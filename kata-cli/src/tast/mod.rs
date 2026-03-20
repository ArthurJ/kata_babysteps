//! Typed Abstract Syntax Tree (TAST)
//!
//! This module defines the heavily-typed version of the AST.
//! In TAST, every expression, pattern, and statement has its exact type
//! fully resolved by the Hindley-Milner type inference engine.
//!
//! TAST is the single source of truth for the Middle-End and Backend.

pub mod expr;
pub mod stmt;
pub mod decl;
