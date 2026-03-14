use miette::{IntoDiagnostic, Result};
use rustyline::error::ReadlineError;
use rustyline::{Editor, Config, KeyCode, KeyEvent, Modifiers, Cmd};
use rustyline::validate::{ValidationContext, ValidationResult, Validator};
use rustyline::Helper;
use rustyline::hint::Hinter;
use rustyline::highlight::Highlighter;
use rustyline::completion::Completer;
use rustyline_derive::{Completer, Helper, Highlighter, Hinter};
use std::time::Instant;

use crate::codegen::jit_compiler::JITCompiler;
use crate::lexer::KataLexer;

// Um Helper simples que implementa apenas a Validação para o REPL
#[derive(Helper, Completer, Hinter, Highlighter)]
struct KataReplHelper {}

impl Validator for KataReplHelper {
    fn validate(&self, ctx: &mut ValidationContext) -> rustyline::Result<ValidationResult> {
        let input = ctx.input();

        // Verifica fechamento de parênteses/chaves/colchetes
        let mut p_count = 0;
        let mut b_count = 0;
        let mut c_count = 0;

        for ch in input.chars() {
            match ch {
                '(' => p_count += 1,
                ')' => p_count -= 1,
                '[' => b_count += 1,
                ']' => b_count -= 1,
                '{' => c_count += 1,
                '}' => c_count -= 1,
                _ => {}
            }
        }

        if p_count > 0 || b_count > 0 || c_count > 0 {
            // Faltam fechamentos
            return Ok(ValidationResult::Incomplete);
        }

        // Sem heurísticas complexas: Se os blocos estruturais de parênteses estão fechados,
        // o Enter normal submete o código para a compilação.
        // O usuário usará Alt+Enter para forçar múltiplas linhas dentro de blocos de Significant Whitespace.
        Ok(ValidationResult::Valid(None))
    }

    fn validate_while_typing(&self) -> bool {
        false
    }
}

pub fn start() -> Result<()> {
    crate::logger::init_logger();
    println!("Kata-Lang REPL (v0.1.0)");
    println!("Dica: Digite .exit ou .quit para sair, ou pressione Ctrl+D.");
    println!("Dica: Use Alt+Enter (Option+Enter) para quebrar linha manualmente sem executar.");

    let config = Config::builder()
        .auto_add_history(true)
        .build();
    let mut rl = Editor::with_config(config).into_diagnostic()?;
    rl.set_helper(Some(KataReplHelper {}));

    // Mapeia Alt+Enter (ou Meta/Esc+Enter) para inserir uma nova linha (Newline) no buffer
    rl.bind_sequence(KeyEvent(KeyCode::Enter, Modifiers::ALT), Cmd::Insert(1, "\n".to_string()));

    // JIT Compiler reutilizável durante a sessão REPL
    let mut jit_compiler = JITCompiler::new();
    // Contador para nomes únicos de expressões REPL
    let mut repl_counter: u64 = 0;
    // Declarações do usuário acumuladas
    let mut user_decls: Vec<crate::ast::TopLevelDecl> = Vec::new();
    // Funções já compiladas no JIT (para evitar recompilação)
    let mut compiled_functions: std::collections::HashSet<String> = std::collections::HashSet::new();

    // Bootstrapping do core
    let types_src = include_str!("core/types.kata");
    let io_src = include_str!("core/io.kata");
    let csp_src = include_str!("core/csp.kata");
    let prelude_src = include_str!("core/prelude.kata");
    let mut core_ast = crate::ast::ModuleAST { declarations: vec![] };
    for src in [types_src, io_src, csp_src, prelude_src] {
        if let Ok(mut ast) = crate::parser::Parser::new(src).parse_module() {
            core_ast.declarations.append(&mut ast.declarations);
        }
    }

    loop {
        let readline = rl.readline("#--> ");
        match readline {
            Ok(line) => {
                let input = line.trim();

                // Ignora linhas em branco
                if input.is_empty() {
                    continue;
                }

                // Comandos meta do REPL
                if input == ".exit" || input == ".quit" {
                    println!("Saindo...");
                    break;
                }

                if input == ".env" {
                    println!("Ambiente: (funcionalidade em desenvolvimento)");
                    continue;
                }

                if input == ".clear" {
                    jit_compiler = JITCompiler::new(); // Recria o JIT para limpar
                    println!("Ambiente limpo.");
                    continue;
                }

                if input == ".help" {
                    println!("Comandos disponíveis:");
                    println!("  .env    - Mostra funções e variáveis definidas");
                    println!("  .clear  - Limpa o ambiente");
                    println!("  .help   - Mostra esta ajuda");
                    println!("  .exit   - Sai do REPL");
                    println!("  .quit   - Sai do REPL (alias)");
                    continue;
                }

                // Profiling por linha avaliada
                let eval_start = Instant::now();

                // Parse da entrada do usuário
                let mut parser = crate::parser::Parser::new(&line);
                match parser.parse_module() {
                    Ok(mut user_ast) => {
                        // Adiciona novas declações à lista persistente
                        user_decls.append(&mut user_ast.declarations);

                        // Cria um AST combinado com o core + todas as declarações do usuário
                        let mut combined_ast = crate::ast::ModuleAST {
                            declarations: core_ast.declarations.clone(),
                        };
                        combined_ast.declarations.extend(user_decls.clone());

                        // Fase 3: Type Checker (novo a cada iteração, mas com todas as declarações)
                        let mut tc = crate::type_checker::TypeChecker::new();

                        match tc.discover(&combined_ast) {
                            Ok(_) => {
                                match tc.resolve_module(combined_ast) {
                                    Ok(typed_ast) => {
                                        let mut printed_result = false;

                                        // Fase 4: IR Builder e JIT Compilation
                                        let mut ir_builder = crate::ir::IRBuilder::new();

                                        // Primeiro: armazena novas definições do usuário
                                        let mut new_user_defs: Vec<crate::typed_ast::TypedTopLevel> = Vec::new();
                                        let mut has_repl_expr = false;

                                        for decl in &typed_ast.declarations {
                                            if let crate::typed_ast::TypedTopLevel::Definition { name, expr } = decl {
                                                // Filtra apenas declarações do usuário (não as do core)
                                                let is_user_decl = match name {
                                                    crate::ast::Ident::Func(n) => {
                                                        // É uma definição do usuário se não for do core e não for repl_eval
                                                        !n.starts_with("impl_") &&
                                                        !n.starts_with("__") &&
                                                        !n.starts_with("kata_rt") &&
                                                        n != "repl_eval" &&
                                                        n != "repl_eval_lambda"
                                                    },
                                                    _ => false,
                                                };

                                                if is_user_decl {
                                                    new_user_defs.push(decl.clone());
                                                }

                                                // Processa expressões repl_eval/repl_eval_lambda
                                                let is_repl_expr = match name {
                                                    crate::ast::Ident::Func(n) => n == "repl_eval" || n == "repl_eval_lambda",
                                                    _ => false,
                                                };

                                                if is_repl_expr {
                                                    has_repl_expr = true;
                                                    // Gera nome único para evitar duplicatas
                                                    let repl_name = format!("repl_eval_{}", repl_counter);
                                                    repl_counter += 1;

                                                    let sig = crate::type_checker::FuncSignature {
                                                        name: repl_name.clone(),
                                                        arity: 0,
                                                        args_types: vec![],
                                                        return_type: expr.ty.clone(),
                                                        is_action: false,
                                                        ffi_binding: None,
                                                    };

                                                    let ir_func = ir_builder.build_function(&repl_name, sig, expr);

                                                    log::debug!("REPL: Compilando IR function '{}' com tipo de retorno {:?}", ir_func.name, expr.ty);

                                                    // Verifica se todos os símbolos existem antes de compilar
                                                    if let Err(e) = check_symbols_exist(&ir_func, &compiled_functions) {
                                                        println!("[Erro] {}", e);
                                                        printed_result = true;
                                                        continue;
                                                    }

                                                    // Executa via JIT compiler
                                                    match execute_with_jit(&mut jit_compiler, &ir_func, &expr.ty) {
                                                        Ok(result_str) => {
                                                            println!("=> {} :: {}", result_str, expr.ty);
                                                        }
                                                        Err(e) => {
                                                            println!("[JIT Error] {}. IR: {:?}", e, ir_func.ctx.arena[ir_func.root]);
                                                        }
                                                    }

                                                    printed_result = true;
                                                }
                                            }
                                        }

                                        // Compila TODAS as definições do usuário no JIT (não apenas as novas)
                                        for decl in &typed_ast.declarations {
                                            if let crate::typed_ast::TypedTopLevel::Definition { name, expr } = decl {
                                                let func_name = match name {
                                                    crate::ast::Ident::Func(n) | crate::ast::Ident::Symbol(n) => n.clone(),
                                                    _ => continue,
                                                };

                                                // Filtra apenas definições do usuário (não core, não repl_eval)
                                                let is_user_func = !func_name.starts_with("impl_") &&
                                                                   !func_name.starts_with("__") &&
                                                                   !func_name.starts_with("kata_rt") &&
                                                                   !func_name.starts_with("repl_eval");

                                                if !is_user_func {
                                                    continue;
                                                }

                                                // Obtém assinatura do type checker
                                                let sig = if let Some(s) = tc.env.get_signature(&func_name) {
                                                    s.clone()
                                                } else {
                                                    // Assinatura padrão se não encontrada
                                                    crate::type_checker::FuncSignature {
                                                        name: func_name.clone(),
                                                        arity: 0,
                                                        args_types: vec![],
                                                        return_type: expr.ty.clone(),
                                                        is_action: false,
                                                        ffi_binding: None,
                                                    }
                                                };

                                                let ir_func = ir_builder.build_function(&func_name, sig, expr);
                                                log::debug!("REPL: Compilando definição do usuário '{}'", func_name);

                                                // Compila no JIT (mas não executa) apenas se ainda não foi compilada
                                                if !compiled_functions.contains(&func_name) {
                                                    if let Err(e) = jit_compiler.compile_function(&ir_func) {
                                                        log::warn!("Falha ao compilar '{}': {}", func_name, e);
                                                    } else {
                                                        compiled_functions.insert(func_name.clone());
                                                        log::debug!("REPL: Função '{}' compilada com sucesso", func_name);
                                                    }
                                                } else {
                                                    log::debug!("REPL: Função '{}' já compilada, pulando", func_name);
                                                }
                                            }
                                        }

                                        if !printed_result {
                                            // Se não for uma Definition isolada, apenas avisa que tipou
                                            println!("{:#?}", typed_ast);
                                            println!("\nType Check: OK");
                                        }
                                    }
                                    Err(e) => println!("{:?}", miette::Report::new(e))
                                }
                            }
                            Err(e) => println!("{:?}", miette::Report::new(e))
                        }
                    }
                    Err(e) => {
                        println!("{:?}", miette::Report::new(e));
                    }
                }

                let elapsed = eval_start.elapsed();
                println!("[{:?}]", elapsed);
            }
            Err(ReadlineError::Interrupted) => {
                println!("(Ctrl+C) Digite .exit para sair ou pressione Ctrl+D.");
                continue;
            }
            Err(ReadlineError::Eof) => {
                println!("Saindo...");
                break;
            }
            Err(err) => {
                println!("Erro de leitura no REPL: {:?}", err);
                break;
            }
        }
    }

    Ok(())
}

/// Verifica se todos os símbolos referenciados na IR existem no ambiente
fn check_symbols_exist(ir_func: &crate::ir::IRFunction, compiled_functions: &std::collections::HashSet<String>) -> Result<(), String> {
    use crate::ir::IRValue;

    for (_id, value) in ir_func.ctx.arena.iter() {
        match value {
            IRValue::Call { target, .. } => {
                // Ignora operações inline e funções do core
                if target.starts_with("impl_") ||
                   target.starts_with("kata_rt") ||
                   target.starts_with("__") ||
                   target == "+" || target == "-" || target == "*" || target == "/" ||
                   target == "==" || target == "<" || target == ">" ||
                   target.starts_with("repl_eval") {
                    continue;
                }
                // Verifica se a função foi compilada
                if !compiled_functions.contains(target) {
                    return Err(format!("Função '{}' não definida", target));
                }
            }
            IRValue::FuncPtr(name) => {
                // Ignora funções do core
                if name.starts_with("impl_") ||
                   name.starts_with("kata_rt") ||
                   name.starts_with("__") ||
                   name.starts_with("repl_eval") {
                    continue;
                }
                // Verifica se a função foi compilada
                if !compiled_functions.contains(name) {
                    return Err(format!("Função '{}' não definida", name));
                }
            }
            _ => {}
        }
    }
    Ok(())
}

/// Executa uma IRFunction no JIT e retorna o resultado formatado como string
fn execute_with_jit(jit: &mut JITCompiler, ir_func: &crate::ir::IRFunction, ty: &crate::type_checker::Type) -> Result<String, String> {
    match ty {
        crate::type_checker::Type::Int => {
            jit.compile_and_run_i64(ir_func).map(|v| v.to_string())
        }
        crate::type_checker::Type::Float => {
            jit.compile_and_run_f64(ir_func).map(|v| v.to_string())
        }
        crate::type_checker::Type::Bool => {
            jit.compile_and_run_i64(ir_func).map(|v| (v != 0).to_string())
        }
        _ => {
            // Para tipos complexos, tenta compilar e retorna ponteiro
            jit.compile_and_run_ptr(ir_func).map(|v| format!("{:p}", v))
        }
    }
}
