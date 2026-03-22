use chumsky::Parser;
use crate::lexer::KataLexer;
use crate::parser::common::convert_result;
use crate::parser::error::ParseError;
use crate::ast::types::Type;
use crate::parser::r#type::type_expr;

fn parse_type(source: &str) -> Result<Type, Vec<ParseError>> {
    let tokens = KataLexer::lex_with_indent(source)
        .map_err(|e| e.into_iter().map(|e| ParseError::new(e.to_string(), e.span().clone())).collect::<Vec<_>>())?;
    convert_result(type_expr().parse(tokens).map(|s| s.node))
}

#[test]
fn test_simple_type() {
    let result = parse_type("Int");
    assert!(result.is_ok());
    assert!(matches!(result.unwrap(), Type::Named { .. }));
}

#[test]
fn test_type_variable() {
    // In Kata, T is CamelCase and indistinguishable from a named type at parse time
    let result = parse_type("T");
    assert!(result.is_ok());
    assert!(matches!(result.unwrap(), Type::Named { .. }));
}

#[test]
fn test_tuple_type() {
    let result = parse_type("(Int Float Text)");
    assert!(result.is_ok());
    match result.unwrap() {
        Type::Tuple(types) => assert_eq!(types.len(), 3),
        _ => panic!("Expected tuple type"),
    }
}

#[test]
fn test_function_type() {
    let result = parse_type("Int -> Float");
    assert!(result.is_ok());
    match result.unwrap() {
        Type::Function { params, return_type } => {
            assert_eq!(params.len(), 1);
            assert!(matches!(*return_type, Type::Named { .. }));
        }
        _ => panic!("Expected function type"),
    }
}
