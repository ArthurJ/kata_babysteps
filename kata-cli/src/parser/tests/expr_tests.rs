//! Unit tests for expression parser

use chumsky::Parser;
use crate::ast::expr::Expr;
use crate::ast::id::{Ident, Literal};
use crate::lexer::KataLexer;
use crate::parser::common::convert_result;
use crate::parser::error::ParseError;
use crate::parser::expr::expression;

fn parse_expr(source: &str) -> Result<Expr, Vec<ParseError>> {
    let tokens = KataLexer::lex_with_indent(source)
        .map_err(|e| e.into_iter().map(|e| ParseError::new(e.to_string(), e.span().clone())).collect::<Vec<_>>())?;
    convert_result(expression().parse(tokens))
}

#[test]
fn test_literal_int() {
    let result = parse_expr("42");
    assert!(result.is_ok());
    assert_eq!(result.unwrap(), Expr::Literal(Literal::Int("42".to_string())));
}

#[test]
fn test_var() {
    let result = parse_expr("x");
    assert!(result.is_ok());
    assert_eq!(result.unwrap(), Expr::Var { name: Ident::new("x"), type_ascription: None });
}

#[test]
fn test_hole() {
    let result = parse_expr("_");
    assert!(result.is_ok());
    assert_eq!(result.unwrap(), Expr::Hole);
}

#[test]
fn test_tuple() {
    // In Kata's unified theory, (1 2 3) is always a tuple
    let result = parse_expr("(1 2 3)");
    assert!(result.is_ok(), "Parse failed: {:?}", result.err());
    let expr = result.unwrap();
    match expr {
        Expr::Tuple(items) => assert_eq!(items.len(), 3),
        _ => panic!("Expected tuple, got: {:?}", expr),
    }
}

#[test]
fn test_apply() {
    let result = parse_expr("+ 1 2");
    assert!(result.is_ok(), "Parse failed: {:?}", result.err());
    let expr = result.unwrap();
    match expr {
        Expr::Apply { func, args } => {
            assert!(matches!(*func, Expr::Var { name: ref v, type_ascription: _ } if v.0 == "+"), "Expected func to be Var(+), got: {:?}", func);
            assert_eq!(args.len(), 2, "Expected 2 args, got: {:?}", args);
        }
        _ => panic!("Expected apply, got: {:?}", expr),
    }
}

#[test]
fn test_pipeline() {
    let result = parse_expr("x |> f");
    assert!(result.is_ok());
    let expr = result.unwrap();
    match expr {
        Expr::Pipeline { value, func } => {
            assert!(matches!(*value, Expr::Var { name: ref v, type_ascription: _ } if v.0 == "x"));
            assert!(matches!(*func, Expr::Var { name: ref v, type_ascription: _ } if v.0 == "f"));
        }
        _ => panic!("Expected pipeline"),
    }
}

#[test]
fn test_list() {
    let result = parse_expr("[1, 2, 3]");
    assert!(result.is_ok());
    let expr = result.unwrap();
    match expr {
        Expr::List(items) => assert_eq!(items.len(), 3),
        _ => panic!("Expected list"),
    }
}

#[test]
fn test_explicit_apply() {
    let result = parse_expr("$(+ 1 2)");
    assert!(result.is_ok());
    let expr = result.unwrap();
    match expr {
        Expr::ExplicitApply { func, args } => {
            assert!(matches!(*func, Expr::Var { name: ref v, type_ascription: _ } if v.0 == "+"));
            assert_eq!(args.len(), 2);
        }
        _ => panic!("Expected explicit apply"),
    }
}
