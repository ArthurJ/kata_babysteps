//! Literal value parsers for Kata Language
//!
//! Parses literal values: integers, floats, strings, bytes, booleans, unit.

use chumsky::prelude::*;
use crate::lexer::{Token, SpannedToken};
use crate::ast::id::Literal;
use super::common::{int_literal, float_literal, string_literal, bytes_literal, token, ParserError, ParserSpan};

/// Parse any literal value
pub fn literal() -> impl Parser<SpannedToken, Literal, Error = ParserError> + Clone {
    choice((
        // Unit: () - must come before other tokens
        token(Token::LParen)
            .then_ignore(token(Token::RParen))
            .map(|_| Literal::Unit),

        // Boolean: True or False keywords
        filter_map(|_span: ParserSpan, spanned: SpannedToken| {
            match &spanned.token {
                Token::Ident(s) if s == "True" => Ok(Literal::Bool(true)),
                Token::Ident(s) if s == "False" => Ok(Literal::Bool(false)),
                _ => Err(ParserError::custom(_span, "expected literal".to_string())),
            }
        }),

        // Float before int to avoid ambiguity
        float_literal().map(Literal::Float),

        // Integer
        int_literal().map(Literal::Int),

        // String
        string_literal().map(Literal::String),

        // Bytes
        bytes_literal().map(Literal::Bytes),
    ))
}

/// Parse a numeric literal (int or float)
pub fn numeric() -> impl Parser<SpannedToken, Literal, Error = ParserError> + Clone {
    choice((
        float_literal().map(Literal::Float),
        int_literal().map(Literal::Int),
    ))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::lexer::KataLexer;
    use super::super::common::convert_result;
    use super::super::error::ParseError;

    fn parse_literal(source: &str) -> Result<Literal, Vec<ParseError>> {
        let tokens = KataLexer::lex_with_indent(source)
            .map_err(|e| e.into_iter().map(|e| ParseError::new(e.to_string(), e.span().clone())).collect::<Vec<_>>())?;
        convert_result(literal().parse(tokens))
    }

    #[test]
    fn test_int_literal() {
        let result = parse_literal("42");
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), Literal::Int("42".to_string()));
    }

    #[test]
    fn test_float_literal() {
        let result = parse_literal("3.14");
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), Literal::Float("3.14".to_string()));
    }

    #[test]
    fn test_string_literal() {
        let result = parse_literal("\"hello\"");
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), Literal::String("hello".to_string()));
    }

    #[test]
    fn test_bool_true() {
        let result = parse_literal("True");
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), Literal::Bool(true));
    }

    #[test]
    fn test_bool_false() {
        let result = parse_literal("False");
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), Literal::Bool(false));
    }

    #[test]
    fn test_unit_literal() {
        let result = parse_literal("()");
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), Literal::Unit);
    }

    #[test]
    fn test_bytes_literal() {
        let result = parse_literal("b\"data\"");
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), Literal::Bytes("data".to_string()));
    }
}