//! Common parser utilities for Kata Language
//!
//! Provides combinators and helper parsers shared across all parser modules.

use chumsky::prelude::*;
use crate::lexer::{Token, SpannedToken};
use super::error::ParseError;

/// Type alias for the span type used in parsers
pub type ParserSpan = std::ops::Range<usize>;

/// Type alias for the error type used in parsers
pub type ParserError = Simple<SpannedToken, ParserSpan>;

// ============================================================================
// TOKEN MATCHERS
// ============================================================================

/// Match any identifier token
pub fn ident() -> impl Parser<SpannedToken, String, Error = ParserError> + Clone {
    filter_map(|_span: ParserSpan, spanned: SpannedToken| {
        match spanned.token {
            Token::Ident(s) => {
                log::debug!("ident matched: '{}'", s);
                Ok(s)
            }
            _ => {
                // log::debug!("ident failed: found {:?}", spanned.token);
                Err(ParserError::custom(_span, "expected identifier".to_string()))
            }
        }
    })
}

/// Match a specific identifier by name
pub fn ident_named(name: &str) -> impl Parser<SpannedToken, String, Error = ParserError> + Clone {
    let name = name.to_string();
    filter_map(move |_span: ParserSpan, spanned: SpannedToken| {
        match &spanned.token {
            Token::Ident(s) if s == &name => Ok(s.clone()),
            _ => Err(ParserError::custom(_span, format!("expected '{}'", name))),
        }
    })
}

/// Match any integer literal
pub fn int_literal() -> impl Parser<SpannedToken, String, Error = ParserError> + Clone {
    filter_map(|_span: ParserSpan, spanned: SpannedToken| {
        match spanned.token {
            Token::Int(s) => Ok(s),
            _ => Err(ParserError::custom(_span, "expected integer".to_string())),
        }
    })
}

/// Match any float literal
pub fn float_literal() -> impl Parser<SpannedToken, String, Error = ParserError> + Clone {
    filter_map(|_span: ParserSpan, spanned: SpannedToken| {
        match spanned.token {
            Token::Float(s) => Ok(s),
            _ => Err(ParserError::custom(_span, "expected float".to_string())),
        }
    })
}

/// Match any string literal
pub fn string_literal() -> impl Parser<SpannedToken, String, Error = ParserError> + Clone {
    filter_map(|_span: ParserSpan, spanned: SpannedToken| {
        match spanned.token {
            Token::String(s) => Ok(s),
            _ => Err(ParserError::custom(_span, "expected string".to_string())),
        }
    })
}

/// Match any bytes literal
pub fn bytes_literal() -> impl Parser<SpannedToken, String, Error = ParserError> + Clone {
    filter_map(|_span: ParserSpan, spanned: SpannedToken| {
        match spanned.token {
            Token::Bytes(s) => Ok(s),
            _ => Err(ParserError::custom(_span, "expected bytes".to_string())),
        }
    })
}

/// Match a specific token
pub fn token(expected: Token) -> impl Parser<SpannedToken, Token, Error = ParserError> + Clone {
    let expected_clone = expected.clone();
    filter_map(move |_span: ParserSpan, spanned: SpannedToken| {
        if spanned.token == expected_clone {
            Ok(spanned.token.clone())
        } else {
            if matches!(spanned.token, Token::Action) || matches!(expected_clone, Token::Action) {
                log::debug!("token: expected {:?}, found {:?}", expected_clone, spanned.token);
            }
            Err(ParserError::custom(_span, format!("expected {:?}", expected_clone)))
        }
    })
}

/// Match a keyword token
pub fn keyword(kw: Token) -> impl Parser<SpannedToken, (), Error = ParserError> + Clone {
    token(kw).ignored()
}

/// Match the hole token (_)
pub fn hole() -> impl Parser<SpannedToken, (), Error = ParserError> + Clone {
    token(Token::Hole).ignored()
}

// ============================================================================
// STRUCTURAL COMBINATORS
// ============================================================================

/// Parse items between delimiters
pub fn between<A, B, C, P1, P2, P3>(
    open: P1,
    close: P2,
    inner: P3,
) -> impl Parser<SpannedToken, C, Error = ParserError> + Clone
where
    P1: Parser<SpannedToken, A, Error = ParserError> + Clone,
    P2: Parser<SpannedToken, B, Error = ParserError> + Clone,
    P3: Parser<SpannedToken, C, Error = ParserError> + Clone,
{
    open.ignore_then(inner).then_ignore(close)
}

/// Parse items separated by a delimiter
pub fn separated<A, B, P1, P2>(
    item: P1,
    separator: P2,
) -> impl Parser<SpannedToken, Vec<A>, Error = ParserError> + Clone
where
    P1: Parser<SpannedToken, A, Error = ParserError> + Clone,
    P2: Parser<SpannedToken, B, Error = ParserError> + Clone,
{
    item.separated_by(separator).at_least(0)
}

/// Parse items separated by a delimiter (at least one)
pub fn separated1<A, B, P1, P2>(
    item: P1,
    separator: P2,
) -> impl Parser<SpannedToken, Vec<A>, Error = ParserError> + Clone
where
    P1: Parser<SpannedToken, A, Error = ParserError> + Clone,
    P2: Parser<SpannedToken, B, Error = ParserError> + Clone,
{
    item.separated_by(separator).at_least(1)
}

// ============================================================================
// INDENTATION HANDLING
// ============================================================================

/// Match an INDENT token
pub fn indent() -> impl Parser<SpannedToken, (), Error = ParserError> + Clone {
    token(Token::Indent).ignored()
}

/// Match a DEDENT token
pub fn dedent() -> impl Parser<SpannedToken, (), Error = ParserError> + Clone {
    token(Token::Dedent).ignored()
}

/// Match a newline (end of line)
pub fn newline() -> impl Parser<SpannedToken, (), Error = ParserError> + Clone {
    token(Token::Newline).ignored()
}

/// Parse an indented block
pub fn indented_block<A, P>(
    inner: P,
) -> impl Parser<SpannedToken, Vec<A>, Error = ParserError> + Clone
where
    P: Parser<SpannedToken, A, Error = ParserError> + Clone,
{
    indent()
        .ignore_then(inner.repeated())
        .then_ignore(dedent())
}

// ============================================================================
// HELPER FUNCTIONS
// ============================================================================

/// Get span from a parser result
pub fn with_span<A, P>(parser: P) -> impl Parser<SpannedToken, (A, ParserSpan), Error = ParserError> + Clone
where
    P: Parser<SpannedToken, A, Error = ParserError> + Clone,
{
    parser.map_with_span(|value, span| (value, span))
}

/// Parse an optional item with a default
pub fn optional_or<A: Default, P>(parser: P) -> impl Parser<SpannedToken, A, Error = ParserError> + Clone
where
    P: Parser<SpannedToken, A, Error = ParserError> + Clone,
{
    parser.or_not().map(|opt| opt.unwrap_or_default())
}

/// Convert Simple error to ParseError
pub fn to_parse_error(e: ParserError) -> ParseError {
    let span = crate::lexer::Span::new(e.span().start, e.span().end);
    let found = e.found().map(|t| format!("{:?}", t.token));
    let expected: Vec<String> = e.expected()
        .filter_map(|opt| opt.as_ref().map(|t| format!("{:?}", t)))
        .collect();
    if expected.is_empty() {
        ParseError::new(format!("unexpected token"), span)
    } else {
        ParseError::expected_found(expected, found, span)
    }
}

/// Convert ParseError from result
pub fn convert_result<T>(result: Result<T, Vec<ParserError>>) -> Result<T, Vec<ParseError>> {
    result.map_err(|errors| errors.into_iter().map(to_parse_error).collect())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::lexer::KataLexer;

    fn parse_tokens(source: &str) -> Vec<SpannedToken> {
        KataLexer::lex_with_indent(source).unwrap()
    }

    #[test]
    fn test_ident_parser() {
        let tokens = parse_tokens("foo bar baz");
        let result: Result<Vec<String>, _> = ident().repeated().parse(tokens);
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), vec!["foo".to_string(), "bar".to_string(), "baz".to_string()]);
    }

    #[test]
    fn test_int_literal_parser() {
        let tokens = parse_tokens("42");
        let result: Result<String, _> = int_literal().parse(tokens);
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), "42");
    }

    #[test]
    fn test_string_literal_parser() {
        let tokens = parse_tokens("\"hello\"");
        let result: Result<String, _> = string_literal().parse(tokens);
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), "hello");
    }
}