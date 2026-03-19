//! Unit tests for declaration parser

use chumsky::Parser;
use crate::ast::decl::{Import, Module};
use crate::lexer::KataLexer;
use crate::parser::common::convert_result;
use crate::parser::decl::module;
use crate::parser::error::ParseError;

fn parse_module(source: &str) -> Result<Module, Vec<ParseError>> {
    let tokens = KataLexer::lex_with_indent(source)
        .map_err(|e| e.into_iter().map(|e| ParseError::new(e.to_string(), e.span().clone())).collect::<Vec<_>>())?;
    convert_result(module().parse(tokens))
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
