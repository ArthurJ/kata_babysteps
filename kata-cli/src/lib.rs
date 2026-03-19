//! Kata Language Compiler
//!
//! A functional programming language with prefix notation,
//! strict separation of data/functions/actions, and CSP concurrency.

pub mod lexer;
pub mod ast;
pub mod parser;
pub mod type_checker;
pub mod ir;
pub mod codegen;