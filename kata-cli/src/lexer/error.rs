//! Error types for the Kata lexer

use thiserror::Error;

use super::token::Span;

/// Lexer error types
#[derive(Debug, Clone, PartialEq, Error)]
pub enum LexerError {
    /// Unterminated string literal
    #[error("Unterminated string literal at {span}")]
    UnterminatedString { span: Span },

    /// Invalid character in source
    #[error("Invalid character '{ch}' at {span}")]
    InvalidCharacter { ch: char, span: Span },

    /// Invalid number format
    #[error("Invalid number format '{value}' at {span}")]
    InvalidNumber { value: String, span: Span },

    /// Mixed tabs and spaces in indentation
    #[error("Mixed tabs and spaces in indentation at {span}")]
    MixedIndentation { span: Span },

    /// Indentation error (dedent without matching indent)
    #[error("Unmatched dedent at {span}")]
    UnmatchedDedent { span: Span },

    /// Unknown escape sequence in string
    #[error("Unknown escape sequence '{seq}' at {span}")]
    UnknownEscapeSequence { seq: String, span: Span },

    /// Invalid bytes literal
    #[error("Invalid bytes literal at {span}")]
    InvalidBytes { span: Span },

    /// Generic parse error
    #[error("Parse error at {span}: {message}")]
    ParseError { message: String, span: Span },
}

impl LexerError {
    pub fn span(&self) -> &Span {
        match self {
            LexerError::UnterminatedString { span } => span,
            LexerError::InvalidCharacter { span, .. } => span,
            LexerError::InvalidNumber { span, .. } => span,
            LexerError::MixedIndentation { span } => span,
            LexerError::UnmatchedDedent { span } => span,
            LexerError::UnknownEscapeSequence { span, .. } => span,
            LexerError::InvalidBytes { span } => span,
            LexerError::ParseError { span, .. } => span,
        }
    }
}