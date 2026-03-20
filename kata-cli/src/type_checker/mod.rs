//! Kata Type Checker
//!
//! Validates types, ensures pure/impure domain separation, enforces Orphan Rules
//! and constructs a dependency graph (DAG) to allow topological compilation and tree shaking.

pub mod environment;
pub mod inference;
pub mod checker;
pub mod effects;
pub mod interfaces;
pub mod dag;
pub mod error;
