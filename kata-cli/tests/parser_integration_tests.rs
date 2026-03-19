//! Integration tests for Kata parser
//!
//! These tests verify that the parser correctly handles complete Kata source files.

use kata::lexer::KataLexer;
use kata::parser::parse;

/// Helper function to lex and parse source code
use kata::ast::decl::Module;

fn lex_and_parse(source: &str) -> Result<Module, Vec<Box<dyn std::error::Error>>> {
    let tokens = KataLexer::lex_with_indent(source)
        .map_err(|errors| errors.into_iter().map(|e| Box::new(e) as Box<dyn std::error::Error>).collect::<Vec<_>>())?;
    parse(tokens).map_err(|errors| errors.into_iter().map(|e| Box::new(e) as Box<dyn std::error::Error>).collect::<Vec<_>>())
}

#[test]
fn test_empty_module() {
    let result = lex_and_parse("");
    assert!(result.is_ok());
    let module = result.unwrap();
    assert!(module.imports.is_empty());
    assert!(module.declarations.is_empty());
}

#[test]
fn test_import_namespace() {
    let result = lex_and_parse("import types");
    assert!(result.is_ok());
    let module = result.unwrap();
    assert_eq!(module.imports.len(), 1);
}

#[test]
fn test_simple_function() {
    let source = r#"
soma :: Int Int => Int
lambda (x y)
    + x y
"#;
    let result = lex_and_parse(source);
    assert!(result.is_ok(), "Parse failed: {:?}", result.err());
    let module = result.unwrap();
    assert_eq!(module.declarations.len(), 1);
}

#[test]
fn test_simple_action() {
    let source = r#"
action main
    echo! "Hello, World!"
"#;
    let result = lex_and_parse(source);
    assert!(result.is_ok(), "Parse failed: {:?}", result.err());
    let module = result.unwrap();
    assert_eq!(module.declarations.len(), 1);
}

#[test]
fn test_data_definition() {
    let source = r#"
data Vec2 (x::Float y::Float)
"#;
    let result = lex_and_parse(source);
    assert!(result.is_ok(), "Parse failed: {:?}", result.err());
    let module = result.unwrap();
    assert_eq!(module.declarations.len(), 1);
}

#[test]
fn test_select_csp_parsing() {
    let source = r#"
action test_csp (rx_a rx_b tx_c)
    loop
        select
            case (<! rx_a) -> valor_a:
                echo! valor_a

            case ("Ping" !> tx_c):
                echo! "Enviado sinal para C"

            timeout 1000:
                echo! "Inatividade detetada"
"#;
    let result = lex_and_parse(source);
    assert!(result.is_ok(), "Parse failed: {:?}", result.err());
    let module = result.unwrap();
    assert_eq!(module.declarations.len(), 1);
    
    use kata::ast::decl::TopLevel;
    use kata::ast::stmt::Stmt;
    
    match &module.declarations[0] {
        TopLevel::Action(action) => {
            // Find the loop, then the select
            match &action.body[0] {
                Stmt::Loop { body } => {
                    match &body[0] {
                        Stmt::Select { cases, timeout } => {
                            assert_eq!(cases.len(), 2);
                            assert!(timeout.is_some());
                        }
                        _ => panic!("Expected Select stmt"),
                    }
                }
                _ => panic!("Expected Loop stmt"),
            }
        }
        _ => panic!("Expected action declaration"),
    }
}
