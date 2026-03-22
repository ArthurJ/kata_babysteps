//! Error types for the Kata parser
//!
//! Provides detailed error messages with source locations.

use std::fmt;
use crate::lexer::Span;

/// A parse error with location information
#[derive(Debug, Clone, PartialEq)]
pub struct ParseError {
    /// Error message
    pub message: String,
    /// Source location
    pub span: Span,
    /// Expected tokens (for error recovery suggestions)
    pub expected: Vec<String>,
    /// Found token (what was actually encountered)
    pub found: Option<String>,
}

impl ParseError {
    /// Create a new parse error
    pub fn new(message: impl Into<String>, span: Span) -> Self {
        ParseError {
            message: message.into(),
            span,
            expected: Vec::new(),
            found: None,
        }
    }

    /// Create an error with expected/found information
    pub fn expected_found(expected: Vec<String>, found: Option<String>, span: Span) -> Self {
        let message = match (&expected, &found) {
            (exp, Some(f)) if !exp.is_empty() => {
                format!("Expected {}, found '{}'", format_expected(exp), f)
            }
            (exp, None) if !exp.is_empty() => {
                format!("Expected {}, found end of file", format_expected(exp))
            }
            (_, Some(f)) => format!("Unexpected '{}'", f),
            (_, None) => "Unexpected end of file".to_string(),
        };

        ParseError {
            message,
            span,
            expected,
            found,
        }
    }

    /// Add expected tokens
    pub fn with_expected(mut self, expected: Vec<String>) -> Self {
        self.expected = expected;
        self
    }

    /// Add found token
    pub fn with_found(mut self, found: String) -> Self {
        self.found = Some(found);
        self
    }
}

/// Format expected tokens for display
fn format_expected(expected: &[String]) -> String {
    match expected.len() {
        0 => "something".to_string(),
        1 => expected[0].clone(),
        2 => format!("{} or {}", expected[0], expected[1]),
        n => {
            let last = &expected[n - 1];
            let rest = &expected[..n - 1];
            format!("{}, or {}", rest.join(", "), last)
        }
    }
}

impl fmt::Display for ParseError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Error at {}: {}", self.span, self.message)
    }
}

impl std::error::Error for ParseError {}

/// Result type for parser operations
pub type ParseResult<T> = Result<T, Vec<ParseError>>;

/// Accumulate multiple parse errors
#[derive(Debug, Clone, Default)]
pub struct ErrorCollector {
    errors: Vec<ParseError>,
}

impl ErrorCollector {
    pub fn new() -> Self {
        ErrorCollector { errors: Vec::new() }
    }

    pub fn add(&mut self, error: ParseError) {
        self.errors.push(error);
    }

    pub fn has_errors(&self) -> bool {
        !self.errors.is_empty()
    }

    pub fn into_errors(self) -> Vec<ParseError> {
        self.errors
    }

    pub fn errors(&self) -> &[ParseError] {
        &self.errors
    }
}
