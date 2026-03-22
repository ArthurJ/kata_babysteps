use chumsky::Parser;
use crate::lexer::KataLexer;
use crate::parser::common::convert_result;
use crate::parser::error::ParseError;
use crate::ast::pattern::Pattern;
use crate::ast::id::{Ident, Literal};
use crate::parser::pattern::pattern;

fn parse_pattern(source: &str) -> Result<Pattern, Vec<ParseError>> {
    let tokens = KataLexer::lex_with_indent(source)
        .map_err(|e| e.into_iter().map(|e| ParseError::new(e.to_string(), e.span().clone())).collect::<Vec<_>>())?;
    convert_result(pattern().parse(tokens).map(|s| s.node))
}

#[test]
fn test_wildcard() {
    let result = parse_pattern("_");
    assert!(result.is_ok());
    assert_eq!(result.unwrap(), Pattern::Wildcard);
}

#[test]
fn test_var_pattern() {
    let result = parse_pattern("x");
    assert!(result.is_ok());
    assert_eq!(result.unwrap(), Pattern::Var(Ident::new("x")));
}

#[test]
fn test_literal_pattern() {
    let result = parse_pattern("42");
    assert!(result.is_ok());
    assert_eq!(result.unwrap(), Pattern::Literal(Literal::Int("42".to_string())));
}

#[test]
fn test_tuple_pattern() {
    let result = parse_pattern("(a b c)");
    assert!(result.is_ok());
    let pat = result.unwrap();
    match pat {
        Pattern::Tuple(items) => assert_eq!(items.len(), 3),
        _ => panic!("Expected tuple pattern"),
    }
}

#[test]
fn test_variant_pattern() {
    let result = parse_pattern("Ok(value)");
    assert!(result.is_ok());
    let pat = result.unwrap();
    match pat {
        Pattern::Variant { name, args } => {
            assert_eq!(name.0, "Ok");
            assert_eq!(args.len(), 1);
        }
        _ => panic!("Expected variant pattern"),
    }
}

#[test]
fn test_unit_variant() {
    let result = parse_pattern("None");
    assert!(result.is_ok());
    let pat = result.unwrap();
    match pat {
        Pattern::Variant { name, args } => {
            assert_eq!(name.0, "None");
            assert!(args.is_empty());
        }
        _ => panic!("Expected variant pattern"),
    }
}
