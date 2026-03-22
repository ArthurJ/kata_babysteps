//! Kata Language CLI
//!
//! A compiler and REPL for the Kata programming language.

use clap::Parser;
use kata::lexer::KataLexer;
use std::fs;
use std::path::PathBuf;

/// Kata Language Compiler and REPL
#[derive(Parser, Debug)]
#[command(name = "kata")]
#[command(version, about, long_about = None)]
struct Args {
    /// Input file to process
    #[arg(value_name = "FILE")]
    input: Option<PathBuf>,

    /// Dump tokens from lexer (for debugging)
    #[arg(short = 't', long)]
    dump_tokens: bool,

    /// Dump AST from parser (for debugging)
    #[arg(short = 'a', long)]
    dump_ast: bool,

    /// Process indentation (INDENT/DEDENT tokens)
    #[arg(short, long)]
    indent: bool,
}

fn main() {
    // Initialize logger for debugging
    env_logger::init();

    let args = Args::parse();

    // If no input file and no specific command, show help
    let input_file = match &args.input {
        Some(file) => file,
        None => {
            eprintln!("Error: No input file specified");
            eprintln!("Usage: kata [OPTIONS] <FILE>");
            std::process::exit(1);
        }
    };

    // Read the input file
    let source = match fs::read_to_string(input_file) {
        Ok(content) => content,
        Err(e) => {
            eprintln!("Error reading file '{}': {}", input_file.display(), e);
            std::process::exit(1);
        }
    };

    // Process based on flags
    if args.dump_tokens {
        dump_tokens(&source, args.indent);
    } 
    
    if args.dump_ast {
        dump_ast(&source);
    }
    
    if !args.dump_tokens && !args.dump_ast {
        // Default pipeline execution
        println!("Processing: {}", input_file.display());
        let tokens = match KataLexer::lex_with_indent(&source) {
            Ok(tokens) => {
                println!("Tokenization successful: {} tokens", tokens.len());
                tokens
            }
            Err(errors) => {
                for error in errors {
                    eprintln!("Lexer error: {}", error);
                }
                std::process::exit(1);
            }
        };

        match kata::parser::parse(tokens) {
            Ok(module) => {
                println!("Parsing successful! {} declarations found.", module.declarations.len());
                
                // Run Type Checker
                println!("Type Checking...");
                let mut checker = kata::type_checker::checker::Checker::new();

                // 1. Load core library files
                let core_files = ["src/core/types.kata", "src/core/io.kata", "src/core/csp.kata"];
                for file_path in core_files {
                    if let Ok(core_source) = fs::read_to_string(file_path) {
                        if let Ok(core_tokens) = KataLexer::lex_with_indent(&core_source) {
                            if let Ok(core_module) = kata::parser::parse(core_tokens) {
                                let core_dag = kata::type_checker::dag::DependencyGraph::from_module(&core_module);
                                if let Ok(core_sorted) = core_dag.topological_sort() {
                                    let _ = checker.check_module(core_sorted);
                                }
                            }
                        }
                    }
                }
                
                // For a single file, we can topologically sort the declarations first
                let dag = kata::type_checker::dag::DependencyGraph::from_module(&module);
                match dag.topological_sort() {
                    Ok(sorted_decls) => {
                        match checker.check_module(sorted_decls) {
                            Ok(tast) => {
                                println!("Type Checking successful! Produced TAST with {} declarations.", tast.len());
                            }
                            Err(e) => {
                                eprintln!("=== TYPE ERRORS ===\n");
                                report_type_error(e, &source, input_file.to_str().unwrap_or("unknown.kata"));
                                std::process::exit(1);
                            }
                        }
                    }
                    Err(e) => {
                        eprintln!("=== CYCLIC DEPENDENCY ERROR ===\n");
                        eprintln!("  {}", e);
                        std::process::exit(1);
                    }
                }
            }
            Err(errors) => {
                eprintln!("=== PARSER ERRORS ===\n");
                for error in errors {
                    eprintln!("  {}", error);
                }
                std::process::exit(1);
            }
        }
    }
}

use ariadne::{Report, ReportKind, Label, Source, Color};

/// Reports a type error beautifully using ariadne
fn report_type_error(e: kata::type_checker::error::TypeError, source: &str, filename: &str) {
    let span = match &e {
        kata::type_checker::error::TypeError::TypeMismatch { span, .. } => span,
        kata::type_checker::error::TypeError::InfiniteType { span, .. } => span,
        kata::type_checker::error::TypeError::UnboundVariable { span, .. } => span,
        kata::type_checker::error::TypeError::NoMatchingDispatch { span, .. } => span,
        kata::type_checker::error::TypeError::AmbiguousDispatch { span, .. } => span,
        kata::type_checker::error::TypeError::ImpureCallInPureContext { span, .. } => span,
        kata::type_checker::error::TypeError::OrphanRuleViolation { span, .. } => span,
        kata::type_checker::error::TypeError::CyclicInheritance { span, .. } => span,
        kata::type_checker::error::TypeError::ArityMismatch { span, .. } => span,
    };

    // Fallback if span is a dummy span (0..0) to avoid crashing Ariadne
    let ariadne_span = if span.start == 0 && span.end == 0 {
        0..1
    } else {
        span.start..span.end
    };

    Report::build(ReportKind::Error, (filename, ariadne_span.clone()))
        .with_message(e.to_string())
        .with_label(
            Label::new((filename, ariadne_span))
                .with_message("Type error originated here")
                .with_color(Color::Red),
        )
        .with_note("Please check the function signature, generic variables, or interface constraints.")
        .finish()
        .print((filename, Source::from(source)))
        .unwrap();
}

/// Dump the AST from the parser
fn dump_ast(source: &str) {
    let tokens = match KataLexer::lex_with_indent(source) {
        Ok(t) => t,
        Err(errors) => {
            eprintln!("=== LEXER ERRORS ===\n");
            for error in errors {
                eprintln!("  {}", error);
            }
            std::process::exit(1);
        }
    };

    match kata::parser::parse(tokens) {
        Ok(module) => {
            println!("=== ABSTRACT SYNTAX TREE (AST) ===\n");
            // Print the entire module using Debug formatting
            println!("{:#?}", module);
        }
        Err(errors) => {
            eprintln!("=== PARSER ERRORS ===\n");
            for error in errors {
                eprintln!("  {}", error);
            }
            std::process::exit(1);
        }
    }
}

/// Dump tokens from the lexer
fn dump_tokens(source: &str, process_indent: bool) {
    let result = if process_indent {
        KataLexer::lex_with_indent(source)
    } else {
        KataLexer::lex(source)
    };

    match result {
        Ok(tokens) => {
            println!("=== TOKENS ===\n");
            println!("{:<8} {:<20} {}", "INDEX", "TOKEN", "SPAN");
            println!("{}", "-".repeat(50));

            for (i, token) in tokens.iter().enumerate() {
                println!("{:<8} {:<20} {:?}", i, format!("{}", token.token), token.span);
            }

            println!("\n=== SUMMARY ===");
            println!("Total tokens: {}", tokens.len());

            // Count token types
            let mut counts = std::collections::HashMap::new();
            for token in &tokens {
                let name = match &token.token {
                    kata::lexer::Token::Ident(_) => "Ident",
                    kata::lexer::Token::Int(_) => "Int",
                    kata::lexer::Token::Float(_) => "Float",
                    kata::lexer::Token::String(_) => "String",
                    kata::lexer::Token::Bytes(_) => "Bytes",
                    kata::lexer::Token::Hole => "Hole",
                    kata::lexer::Token::Lambda => "Lambda",
                    kata::lexer::Token::Action => "Action",
                    kata::lexer::Token::Data => "Data",
                    kata::lexer::Token::Enum => "Enum",
                    kata::lexer::Token::Interface => "Interface",
                    kata::lexer::Token::Implements => "Implements",
                    kata::lexer::Token::Alias => "Alias",
                    kata::lexer::Token::Import => "Import",
                    kata::lexer::Token::Export => "Export",
                    kata::lexer::Token::Let => "Let",
                    kata::lexer::Token::Var => "Var",
                    kata::lexer::Token::Match => "Match",
                    kata::lexer::Token::For => "For",
                    kata::lexer::Token::In => "In",
                    kata::lexer::Token::Loop => "Loop",
                    kata::lexer::Token::Break => "Break",
                    kata::lexer::Token::Continue => "Continue",
                    kata::lexer::Token::Case => "Case",
                    kata::lexer::Token::Select => "Select",
                    kata::lexer::Token::Timeout => "Timeout",
                    kata::lexer::Token::With => "With",
                    kata::lexer::Token::Otherwise => "Otherwise",
                    kata::lexer::Token::As => "As",
                    kata::lexer::Token::Unit => "Unit",
                    kata::lexer::Token::Except => "Except",
                    kata::lexer::Token::Channel => "Channel",
                    kata::lexer::Token::Queue => "Queue",
                    kata::lexer::Token::Broadcast => "Broadcast",
                    kata::lexer::Token::At => "At",
                    kata::lexer::Token::Indent => "INDENT",
                    kata::lexer::Token::Dedent => "DEDENT",
                    kata::lexer::Token::Newline => "Newline",
                    kata::lexer::Token::Semicolon => "Semicolon",
                    kata::lexer::Token::Colon => "Colon",
                    kata::lexer::Token::DoubleColon => "DoubleColon",
                    kata::lexer::Token::SimpleArrow => "SimpleArrow",
                    kata::lexer::Token::Arrow => "Arrow",
                    kata::lexer::Token::LParen => "LParen",
                    kata::lexer::Token::RParen => "RParen",
                    kata::lexer::Token::LBracket => "LBracket",
                    kata::lexer::Token::RBracket => "RBracket",
                    kata::lexer::Token::LBrace => "LBrace",
                    kata::lexer::Token::RBrace => "RBrace",
                    kata::lexer::Token::Comma => "Comma",
                    kata::lexer::Token::Dot => "Dot",
                    kata::lexer::Token::DotDot => "DotDot",
                    kata::lexer::Token::DotDotEqual => "DotDotEqual",
                    kata::lexer::Token::DotDotDot => "DotDotDot",
                    kata::lexer::Token::Pipe => "Pipe",
                    kata::lexer::Token::Pipeline => "Pipeline",
                    kata::lexer::Token::Send => "Send",
                    kata::lexer::Token::Receive => "Receive",
                    kata::lexer::Token::ReceiveNonBlocking => "ReceiveNonBlocking",
                    kata::lexer::Token::AtSymbol => "AtSymbol",
                    kata::lexer::Token::Backslash => "Backslash",
                    kata::lexer::Token::Question => "Question",
                    kata::lexer::Token::Dollar => "Dollar",
                    kata::lexer::Token::Eof => "EOF",
                };
                *counts.entry(name).or_insert(0) += 1;
            }

            println!("\nToken counts:");
            for (name, count) in counts.iter() {
                println!("  {}: {}", name, count);
            }
        }
        Err(errors) => {
            eprintln!("=== LEXER ERRORS ===\n");
            for error in errors {
                eprintln!("  {}", error);
            }
            std::process::exit(1);
        }
    }
}