//! Unit tests for declaration parser

use chumsky::Parser;
use crate::ast::decl::{Import, Module, TopLevel};
use crate::lexer::KataLexer;
use crate::parser::common::convert_result;
use crate::parser::error::ParseError;
use crate::parser::decl::{module, top_level};
use crate::parser::expr::expression;

fn parse_module(source: &str) -> Result<Module, Vec<ParseError>> {
    let tokens = KataLexer::lex_with_indent(source)
        .map_err(|e| e.into_iter().map(|e| ParseError::new(e.to_string(), e.span().clone())).collect::<Vec<_>>())?;
    convert_result(module().parse(tokens))
}

fn parse_top(source: &str) -> Result<TopLevel, Vec<ParseError>> {
    let tokens = KataLexer::lex_with_indent(source).unwrap();
    let expr = expression();
    convert_result(top_level(expr).parse(tokens).map(|s| s.node))
}

#[test]
fn test_action_unit() {
    let source = "action foo\n    echo! 1";
    let result = parse_top(source);
    assert!(result.is_ok(), "Parse failed: {:?}", result.err());
    match result.unwrap() {
        TopLevel::Action(a) => assert_eq!(a.name.0, "foo"),
        _ => panic!("Expected Action"),
    }
}

#[test]
fn test_empty_module() {
    let result = parse_module("");
    assert!(result.is_ok());
    let module = result.unwrap();
    assert!(module.declarations.is_empty());
}

#[test]
fn test_import() {
    let result = parse_module("import types");
    assert!(result.is_ok());
    let module = result.unwrap();
    assert_eq!(module.imports.len(), 1);
}

#[test]
fn test_import_item() {
    let result = parse_module("import types.NUM");
    assert!(result.is_ok());
    let module = result.unwrap();
    match &module.imports[0] {
        Import::Item { module, item } => {
            assert_eq!(module, "types");
            assert_eq!(item, "NUM");
        }
        _ => panic!("Expected item import"),
    }
}
