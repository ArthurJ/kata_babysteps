//! Lexer module for Kata Language
//!
//! Tokenizes source code into a stream of tokens using logos.
//! Handles indent/dedent synthesis for significant whitespace.

mod token;
mod lexer;
mod error;

pub use token::Token;
pub use lexer::KataLexer;
pub use error::LexerError;

/// Result type for lexer operations
pub type LexerResult<T> = Result<T, LexerError>;