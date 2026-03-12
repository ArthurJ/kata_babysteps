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

                // Profiling por linha avaliada
                let eval_start = Instant::now();

                println!("AST (Parsed):");
                
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
                                        // Apenas para visualização no REPL
                                        let mut printed_ir = false;

                                        // Fase 4: IR Builder e Constant Folding
                                        let mut ir_builder = crate::ir::IRBuilder::new();
                                        let mut jit_engine = crate::jit::JITEngine::new();
                                        
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
                                                
                                                // Executa via JIT compiler nativo
                                                match jit_engine.compile_and_run(&ir_func) {
                                                    Ok(result) => {
                                                        println!("=> {} :: {}", result, expr.ty);
                                                    }
                                                    Err(e) => {
                                                        // Se o JIT falhar por falta de implementação (ex: structs), cai no fallback do AST
                                                        println!("[JIT Unsupported] Fallback to IR: {:?}", ir_func.ctx.arena[ir_func.root]);
                                                    }
                                                }

                                                printed_ir = true;
                                            }
                                        }

                                        if !printed_ir {
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
