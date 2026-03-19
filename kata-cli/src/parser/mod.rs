//! Parser module for Kata Language
//!
//! This module parses tokens into an Abstract Syntax Tree (AST).
//! Uses chumsky parser combinators for a clean, declarative parser.
//!
//! ## Structure
//!
//! - `error`: Parse errors with spans
//! - `common`: Utility parsers and combinators
//! - `literal`: Literal value parsers (Int, Float, String, etc.)
//! - `expr`: Expression parsers (Apply, Lambda, Pipeline, etc.)
//! - `pattern`: Pattern matching parsers
//! - `type`: Type expression parsers
//! - `stmt`: Statement parsers (Let, Var, Match, etc.)
//! - `decl`: Top-level declaration parsers

pub mod error;
pub mod common;
pub mod literal;
pub mod expr;
pub mod pattern;
pub mod r#type;
pub mod stmt;
pub mod decl;

#[cfg(test)]
mod decl_tests;

#[cfg(test)]
pub mod tests;

use chumsky::prelude::*;
use crate::lexer::SpannedToken;
use crate::ast::decl::Module;
use error::ParseError;
use common::convert_result;
use decl::module;

/// Parse a complete module from tokens
pub fn parse_module(tokens: Vec<SpannedToken>) -> Result<Module, Vec<ParseError>> {
    log::debug!("parse_module: tokens = {:?}", tokens);
    convert_result(module().parse(tokens))
}

/// Parse tokens and return either a successful AST or errors
pub fn parse(tokens: Vec<SpannedToken>) -> Result<Module, Vec<ParseError>> {
    parse_module(tokens)
}
