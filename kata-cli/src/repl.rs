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

                // Bootstrapping do REPL
                let types_src = include_str!("core/types.kata");
                let io_src = include_str!("core/io.kata");
                let csp_src = include_str!("core/csp.kata");
                let prelude_src = include_str!("core/prelude.kata");

                let mut master_ast = crate::ast::ModuleAST { declarations: vec![] };
                for src in [types_src, io_src, csp_src, prelude_src] {
                    if let Ok(mut core_ast) = crate::parser::Parser::new(src).parse_module() {
                        master_ast.declarations.append(&mut core_ast.declarations);
                    }
                }

                let mut parser = crate::parser::Parser::new(&line);
                match parser.parse_module() {
                    Ok(mut user_ast) => {
                        master_ast.declarations.append(&mut user_ast.declarations);

                        // Fase 3: Type Checker
                        let mut tc = crate::type_checker::TypeChecker::new();

                        match tc.discover(&master_ast) {
                            Ok(_) => {
                                match tc.resolve_module(master_ast) {
                                    Ok(typed_ast) => {
                                        let mut printed_result = false;

                                        // Fase 4: IR Builder e JIT Compilation
                                        let mut ir_builder = crate::ir::IRBuilder::new();

                                        for decl in &typed_ast.declarations {
                                            if let crate::typed_ast::TypedTopLevel::Definition { name, expr } = decl {
                                                let sig = crate::type_checker::FuncSignature {
                                                    name: "repl_eval".to_string(),
                                                    arity: 0,
                                                    args_types: vec![],
                                                    return_type: expr.ty.clone(),
                                                    is_action: false,
                                                    ffi_binding: None,
                                                };

                                                let ir_func = ir_builder.build_function("repl_eval", sig, expr);

                                                log::debug!("REPL: Compilando IR function '{}' com tipo de retorno {:?}", ir_func.name, expr.ty);
                                                log::trace!("REPL: IR root = {:?}", ir_func.ctx.arena[ir_func.root]);

                                                // Executa via JIT compiler nativo usando o mesmo codegen do AOT
                                                match execute_with_jit(&mut jit_compiler, &ir_func, &expr.ty) {
                                                    Ok(result_str) => {
                                                        println!("=> {} :: {}", result_str, expr.ty);
                                                    }
                                                    Err(e) => {
                                                        // Se o JIT falhar, mostra o IR para debug
                                                        println!("[JIT Error] {}. IR: {:?}", e, ir_func.ctx.arena[ir_func.root]);
                                                    }
                                                }

                                                printed_result = true;
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
