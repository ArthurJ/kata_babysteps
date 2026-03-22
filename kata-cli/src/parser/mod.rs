//! Parser module for Kata Language
//!
//! This module defines the parsers that convert a stream of tokens into an AST.
//! It uses the chumsky library for parser combinators.

pub mod common;
pub mod error;
pub mod literal;
pub mod r#type;
pub mod pattern;
pub mod expr;
pub mod stmt;
pub mod decl;

#[cfg(test)]
pub mod tests;

use chumsky::prelude::*;
use crate::lexer::SpannedToken;
use crate::ast::decl::Module;
use common::convert_result;
use error::ParseError;
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
