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
