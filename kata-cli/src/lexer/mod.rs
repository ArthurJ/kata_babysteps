//! Lexer module for Kata Language
//!
//! Tokenizes source code into a stream of tokens using logos.
//! Handles indent/dedent synthesis for significant whitespace.

pub mod token;
mod lexer;
mod error;

pub use token::{Token, Span, SpannedToken};
pub use lexer::KataLexer;
pub use error::LexerError;

/// Result type for lexer operations
pub type LexerResult<T> = Result<T, LexerError>;