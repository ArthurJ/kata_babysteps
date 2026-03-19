//! Abstract Syntax Tree for Kata Language
//!
//! This module defines the AST structures that represent a Kata program.
//! The parser produces these structures from tokens, and subsequent
//! compilation phases (type checking, IR generation) consume them.

pub mod id;
pub mod types;
pub mod pattern;
pub mod expr;
pub mod stmt;
pub mod decl;

#[cfg(test)]
mod tests;

// Re-export with explicit visibility to avoid conflicts
pub use id::{Ident, QualifiedIdent, Literal, Directive, CacheStrategy, RestartPolicy};
pub use types::{Type, Predicate, CompareOp, LiteralValue, FunctionSig};
pub use pattern::{Pattern, Guard};
pub use expr::{Expr, LambdaClause, GuardClause, GuardCondition, WithBinding, WithBindingKind};
pub use stmt::{Stmt, MatchCase, SelectCase, ChannelOp, SelectTimeout, ErrorPropagation};
pub use decl::{
    Module, TopLevel, FunctionDef, ActionDef, DataDef, DataKind, FieldDef,
    EnumDef, VariantDef, VariantPayload, InterfaceDef, InterfaceMember,
    ImplDef, AliasDef, Import, Export,
};