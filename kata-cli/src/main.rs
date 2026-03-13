use clap::{Parser, Subcommand};
use miette::{IntoDiagnostic, Result};
use std::path::PathBuf;
use std::time::Instant;

mod ast;
mod codegen;
mod error;
mod ir;
mod jit;
mod lexer;
mod parser;
mod recursion_analysis;
mod type_checker;
mod typed_ast;
mod repl;

use codegen::aot_compiler::AOTCompiler;
use codegen::tree_shaker::TreeShaker;
use ir::IRBuilder;
use type_checker::TypeChecker;
use std::fs;

/// Kata Language Compiler & CLI
#[derive(Parser, Debug)]
#[command(name = "kata")]
#[command(author = "Kata Lang Contributors")]
#[command(version = "0.1.0")]
#[command(about = "Compilador e REPL para a linguagem Kata", long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand, Debug)]
enum Commands {
    /// Compila um arquivo Kata e suas dependências
    Build {
        /// O arquivo de entrada principal (.kata)
        entry_file: PathBuf,
    },
    /// Compila e executa o binário imediatamente
    Run {
        /// O arquivo de entrada principal (.kata)
        entry_file: PathBuf,
    },
    /// Executa testes anotados com @test no arquivo ou diretório
    Test {
        /// O alvo do teste (diretório atual se não especificado)
        target: Option<PathBuf>,
    },
    /// Inicia o ambiente interativo (Read-Eval-Print Loop)
    Repl,
}

fn bootstrap_stdlib() -> ast::ModuleAST {
    let types_src = include_str!("core/types.kata");
    let io_src = include_str!("core/io.kata");
    let csp_src = include_str!("core/csp.kata");
    let prelude_src = include_str!("core/prelude.kata");

    let mut stdlib_ast = ast::ModuleAST { declarations: vec![] };

    let files = vec![types_src, io_src, csp_src, prelude_src];
    for src in files {
        let mut parser = parser::Parser::new(src);
        match parser.parse_module() {
            Ok(mut ast) => {
                stdlib_ast.declarations.append(&mut ast.declarations);
            }
            Err(e) => {
                println!("Falha ao parsear StdLib: {:?}", miette::Report::new(e));
                std::process::exit(1);
            }
        }
    }

    stdlib_ast
}

/// Processa imports recursivamente e coleta declarações e exports
fn process_imports(
    entry_file: &PathBuf,
    processed: &mut std::collections::HashSet<PathBuf>,
) -> (Vec<ast::TopLevelDecl>, Vec<String>) {
    let mut all_decls = Vec::new();
    let mut exported_names = Vec::new();

    // Evita processar o mesmo arquivo múltiplas vezes
    if processed.contains(entry_file) {
        return (all_decls, exported_names);
    }
    processed.insert(entry_file.clone());

    let source = match fs::read_to_string(entry_file) {
        Ok(s) => s,
        Err(e) => {
            eprintln!("Aviso: Não foi possível ler {}: {}", entry_file.display(), e);
            return (all_decls, exported_names);
        }
    };

    let mut parser = parser::Parser::new(&source);
    let ast = match parser.parse_module() {
        Ok(a) => {
            eprintln!("DEBUG process_imports: {} declarações parseadas em {:?}", a.declarations.len(), entry_file);
            for decl in &a.declarations {
                eprintln!("DEBUG   - Parse: {:?}", std::mem::discriminant(decl));
            }
            a
        }
        Err(e) => {
            eprintln!("Erro ao parsear {}: {:?}", entry_file.display(), e);
            return (all_decls, exported_names);
        }
    };

    // Extrai o diretório base para resolver imports relativos
    let base_dir = entry_file.parent().map(|p| p.to_path_buf()).unwrap_or_else(|| PathBuf::from("."));

    for decl in ast.declarations {
        match &decl {
            ast::TopLevelDecl::Import { path, .. } => {
                // Resolve o caminho do import
                if let Some(ast::Ident::Func(module_name)) = path.first() {
                    let module_path = base_dir.join(format!("{}.kata", module_name));
                    if module_path.exists() {
                        let (imported_decls, imported_exports) = process_imports(&module_path, processed);
                        all_decls.extend(imported_decls);
                        exported_names.extend(imported_exports);
                    } else {
                        eprintln!("Aviso: Módulo não encontrado: {}", module_path.display());
                    }
                }
            }
            ast::TopLevelDecl::Export(names) => {
                // Coleta nomes exportados deste módulo
                for name in names {
                    if let ast::Ident::Func(n) | ast::Ident::Action(n) = name {
                        exported_names.push(n.clone());
                    }
                }
                // Mantém o export na AST para processamento posterior
                all_decls.push(decl);
            }
            _ => {
                all_decls.push(decl);
            }
        }
    }

    (all_decls, exported_names)
}

fn compile_pipeline(entry_file: &PathBuf) -> Result<Vec<u8>> {
    // DEBUG: Ver parsing direto primeiro
    let debug_source = fs::read_to_string(entry_file).into_diagnostic()?;
    let mut debug_parser = parser::Parser::new(&debug_source);
    match debug_parser.parse_module() {
        Ok(ast) => {
            eprintln!("DEBUG: Parsing bem-sucedido! {} declarações", ast.declarations.len());
            for (i, decl) in ast.declarations.iter().enumerate() {
                eprintln!("DEBUG   [{}]: {:?}", i, std::mem::discriminant(decl));
            }
        }
        Err(e) => {
            eprintln!("DEBUG: Erro no parsing: {:?}", e);
        }
    }

    // Processa o arquivo de entrada e seus imports recursivamente
    let mut processed_files = std::collections::HashSet::new();
    let (mut user_decls, imported_exports) = process_imports(entry_file, &mut processed_files);
    eprintln!("DEBUG compile_pipeline: {} declarações do usuário", user_decls.len());
    for decl in &user_decls {
        eprintln!("DEBUG   - Declaração: {:?}", std::mem::discriminant(decl));
    }

    // Bootstrapping mágico: Colocamos o teto arquitetural da linguagem
    // acima das declarações do usuário.
    let mut master_ast = bootstrap_stdlib();
    master_ast.declarations.append(&mut user_decls);

    let mut tc = TypeChecker::new();
    tc.discover(&master_ast).unwrap();
    let typed_ast = tc.resolve_module(master_ast).unwrap();

    let mut ir_builder = IRBuilder::new();
    let mut all_functions = Vec::new();
    let mut top_level_roots = Vec::new();

    // Gera a IR para todas as funções e colhe as chamadas de nível raiz
    for decl in &typed_ast.declarations {
        if let typed_ast::TypedTopLevel::Definition { name, expr } = decl {
            if let ast::Ident::Func(n) = name {
                if let Some(sig) = tc.env.get_signature(n) {
                    let signature = sig.clone();
                    all_functions.push(ir_builder.build_function(n, signature, expr));
                }
            }
        } else if let typed_ast::TypedTopLevel::ActionDef { name, body, .. } = decl {
             if let ast::Ident::Func(n) | ast::Ident::Action(n) = name {
                 let lookup_name = if n.ends_with('!') { n.clone() } else { format!("{}!", n) };
                 let export_name = if lookup_name == "main!" { "kata_main".to_string() } else { lookup_name.clone() };
                 
                 top_level_roots.push(export_name.clone());

                 if let Some(sig) = tc.env.get_signature(&lookup_name) {
                     let signature = sig.clone();
                     let ir_func = ir_builder.build_action(&export_name, signature, body);
                     all_functions.push(ir_func);
                 }
             }
        }
    }

    // Adiciona as funções marcadas como 'export' aos roots para que o Tree Shaker não as elimine
    for decl in &user_decls {
        if let ast::TopLevelDecl::Export(names) = decl {
            for name in names {
                if let ast::Ident::Func(n) | ast::Ident::Action(n) = name {
                    top_level_roots.push(n.clone());
                }
            }
        }
    }

    // Adiciona as funções exportadas dos imports aos roots
    for exported_name in &imported_exports {
        top_level_roots.push(exported_name.clone());
    }

    let shaker = TreeShaker::new(all_functions, top_level_roots.clone());
    let shaken_funcs = shaker.shake();
    
    // DEBUG: Ver quais funções sobreviveram
    println!("DEBUG: Funções compiladas: {:?}", shaken_funcs.iter().map(|f| &f.name).collect::<Vec<_>>());

    let mut compiler = AOTCompiler::new("kata_module");
    
    for f in &shaken_funcs {
        compiler.compile_function(f).unwrap();
    }
    
    // Apenas main/kata_main é o entrypoint padrão - outras actions são chamadas explicitamente no código
    let main_entrypoint = if top_level_roots.contains(&"main".to_string()) || top_level_roots.contains(&"main!".to_string()) {
        vec!["main".to_string()]
    } else if top_level_roots.contains(&"kata_main".to_string()) {
        vec!["kata_main".to_string()]
    } else {
        vec![]
    };

    // Compilar wrapper main que chama a action main (se existir)
    if !main_entrypoint.is_empty() {
        compiler.compile_system_main(main_entrypoint).unwrap();
    }

    Ok(compiler.finish())
}

fn main() -> Result<()> {
    let start_time = Instant::now();
    let cli = Cli::parse();

    match &cli.command {
        Commands::Build { entry_file } => {
            if !entry_file.exists() {
                miette::bail!("Erro: O arquivo de entrada '{}' não foi encontrado.", entry_file.display());
            }
            println!("Compilando AOT: {}", entry_file.display());
            
            let obj_bytes = compile_pipeline(entry_file)?;
            
            // Escreve output.o
            let out_o_path = entry_file.with_extension("o");
            let out_bin_path = entry_file.with_extension(""); // Binário final
            
            fs::write(&out_o_path, obj_bytes).into_diagnostic()?;
            println!("Objeto Cranelift gerado em: {}", out_o_path.display());
            
            // Fase 8: Cross-Linking Final!
            // Invoca o compilador nativo para linkar o objeto .o com o kata-runtime
            println!("Linkando executável final...");

            // Tenta achar o diretório do runtime relativo ao workspace para o link (No futuro, isso pode ser instalado em /usr/local/lib)
            let current_dir = std::env::current_dir().into_diagnostic()?;
            let runtime_dir = current_dir.parent().unwrap().join("kata-runtime").join("target").join("release");

            let status = std::process::Command::new("cc")
                .arg(&out_o_path)
                .arg("-o")
                .arg(&out_bin_path)
                // Aponta pro diretório do libkata_runtime.a gerado no build do runtime
                .arg(format!("-L{}", runtime_dir.display()))
                .arg("-lkata_runtime")
                // Dependências pesadas do S.O. para suportar os canais do Tokio/Crossbeam
                .arg("-lpthread")
                .arg("-ldl")
                .arg("-lm")
                .status();

            match status {
                Ok(s) if s.success() => {
                    println!("Executável gerado com sucesso em: ./{}", out_bin_path.display());
                    // Remove o .o temporário para manter o diretório limpo
                    let _ = fs::remove_file(&out_o_path);
                }
                Ok(s) => {
                    miette::bail!("Falha no processo de Linkagem (cc). Código de saída: {}", s);
                }
                Err(e) => {
                    miette::bail!("Erro crítico: O compilador 'cc' (GCC/Clang) não foi encontrado no sistema ou não pôde ser executado. O arquivo '.o' foi mantido. Detalhe: {}", e);
                }
            }

            print_elapsed(start_time, "Build");
        }
        Commands::Run { entry_file } => {
            if !entry_file.exists() {
                miette::bail!("Erro: O arquivo de entrada '{}' não foi encontrado.", entry_file.display());
            }
            println!("Compilando pipeline para Run: {}", entry_file.display());
            let _obj_bytes = compile_pipeline(entry_file)?;
            println!("Pipeline completo! Execução nativa simulada (Linker desativado nesta Fase)");
            print_elapsed(start_time, "Run");
        }
        Commands::Test { target } => {
            let path = target.clone().unwrap_or_else(|| PathBuf::from("."));
            if !path.exists() {
                miette::bail!("Erro: O alvo de teste '{}' não foi encontrado.", path.display());
            }
            println!("Varrendo testes em: {}", path.display());
            print_elapsed(start_time, "Test");
        }
        Commands::Repl => {
            // Entregamos o controle ao REPL
            repl::start()?;
            print_elapsed(start_time, "Sessão REPL");
        }
    }

    Ok(())
}

fn print_elapsed(start: Instant, prefix: &str) {
    let elapsed = start.elapsed();
    println!("{} finalizado em {:?}", prefix, elapsed);
}
