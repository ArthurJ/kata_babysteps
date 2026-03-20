//! Unit tests for statement parser

use chumsky::Parser;
use crate::ast::stmt::Stmt;
use crate::lexer::KataLexer;
use crate::parser::common::convert_result;
use crate::parser::error::ParseError;
use crate::parser::expr::expression;
use crate::parser::stmt::statement;

fn parse_stmt(source: &str) -> Result<Stmt, Vec<ParseError>> {
    let tokens = KataLexer::lex_with_indent(source)
        .map_err(|e| e.into_iter().map(|e| ParseError::new(e.to_string(), e.span().clone())).collect::<Vec<_>>())?;
    convert_result(statement(expression()).parse(tokens))
}

#[test]
fn test_let_binding() {
    let result = parse_stmt("let x 42");
    assert!(result.is_ok());
    match result.unwrap() {
        Stmt::Let { pattern, value: _ } => {
            assert_eq!(pattern.to_string(), "x");
        }
        _ => panic!("Expected let binding"),
    }
}

#[test]
fn test_var_binding() {
    let result = parse_stmt("var counter 0");
    assert!(result.is_ok());
    match result.unwrap() {
        Stmt::Var { pattern, value: _ } => {
            assert_eq!(pattern.to_string(), "counter");
        }
        _ => panic!("Expected var binding"),
    }
}

#[test]
fn test_break() {
    let result = parse_stmt("break");
    assert!(result.is_ok());
    assert_eq!(result.unwrap(), Stmt::Break);
}

#[test]
fn test_continue() {
    let result = parse_stmt("continue");
    assert!(result.is_ok());
    assert_eq!(result.unwrap(), Stmt::Continue);
}
