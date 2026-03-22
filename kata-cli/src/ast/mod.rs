pub mod id;
pub mod types;
pub mod pattern;
pub mod expr;
pub mod stmt;
pub mod decl;

#[cfg(test)]
pub mod tests;

use crate::lexer::token::Span;

/// A wrapper for AST nodes that includes their source code span.
#[derive(Debug, Clone, PartialEq)]
pub struct Spanned<T> {
    pub node: T,
    pub span: Span,
}

impl<T> Spanned<T> {
    pub fn new(node: T, span: Span) -> Self {
        Self { node, span }
    }

    /// Map the inner node while preserving the span.
    pub fn map<U, F>(self, f: F) -> Spanned<U>
    where
        F: FnOnce(T) -> U,
    {
        Spanned {
            node: f(self.node),
            span: self.span,
        }
    }
}

// Re-export with explicit visibility to avoid conflicts
pub use id::{Ident, QualifiedIdent, Literal, Directive, CacheStrategy, RestartPolicy};
pub use types::{Type, Predicate, CompareOp, LiteralValue, FunctionSig};
pub use pattern::{Pattern, Guard};
pub use expr::{Expr, LambdaClause, GuardClause, GuardCondition, WithBinding};
pub use stmt::{Stmt, MatchCase, SelectCase, ChannelOp, SelectTimeout, ErrorPropagation};
pub use decl::{
    Module, TopLevel, FunctionDef, ActionDef, DataDef, DataKind, FieldDef,
    EnumDef, VariantDef, VariantPayload, InterfaceDef, InterfaceMember,
    ImplDef, AliasDef, Import, Export,
};
pub use crate::lexer::token::Span as LexerSpan;
