//! Typed Expression (TAST) for Kata Language
//!
//! Typed expressions are the result of the type checking phase.
//! Every node is guaranteed to have a resolved, concrete type.

use crate::ast::id::{Ident, Literal, QualifiedIdent};
use crate::ast::types::Type;
use crate::ast::pattern::Pattern;
use crate::lexer::Span;

/// A fully typed expression
#[derive(Debug, Clone, PartialEq)]
pub struct TypedExpr {
    /// The actual expression variant
    pub kind: ExprKind,
    /// The resolved type of this expression
    pub typ: Type,
    /// Source location for error reporting
    pub span: Span,
}

#[derive(Debug, Clone, PartialEq)]
pub enum ExprKind {
    /// Literal value: `42`, `"hello"`, `True`
    Literal(Literal),

    /// Variable reference: `x`, `minha_var`
    Var(Ident),

    /// Qualified reference: `Modulo::funcao`
    QualifiedRef(QualifiedIdent),

    /// Tuple expression: `(1 2 3)`
    Tuple(Vec<TypedExpr>),

    /// List expression: `[1, 2, 3]`
    List(Vec<TypedExpr>),

    /// Cons expression: `x : xs`
    Cons {
        head: Box<TypedExpr>,
        tail: Box<TypedExpr>,
    },

    /// Array expression: `{1, 2, 3}`
    Array(Vec<TypedExpr>),

    /// Tensor expression: `{1 2 ; 3 4}`
    Tensor {
        dimensions: Vec<usize>,
        elements: Vec<TypedExpr>,
    },

    /// Range expression: `[1..10]`
    Range {
        start: Box<TypedExpr>,
        end: Box<TypedExpr>,
        step: Option<Box<TypedExpr>>,
        inclusive: bool,
    },

    /// Dictionary expression: `Dict [("chave" "valor")]`
    Dict(Vec<(TypedExpr, TypedExpr)>),

    /// Set expression: `Set [1, 2, 3]`
    Set(Vec<TypedExpr>),

    /// Function application: `+ 1 2`
    Apply {
        func: Box<TypedExpr>,
        args: Vec<TypedExpr>,
    },

    /// Explicit application: `$(+ 1 2)`
    ExplicitApply {
        func: Box<TypedExpr>,
        args: Vec<TypedExpr>,
    },

    /// Method call: `obj.method arg1 arg2`
    Method {
        object: Box<TypedExpr>,
        method: Ident,
        args: Vec<TypedExpr>,
    },

    /// Field access: `obj.field`
    Field {
        object: Box<TypedExpr>,
        field: Ident,
    },

    /// Index access: `arr .at i`
    Index {
        object: Box<TypedExpr>,
        index: Box<TypedExpr>,
    },

    /// Lambda expression: `λ (x) corpo`
    Lambda {
        clauses: Vec<TypedLambdaClause>,
    },

    /// Hole for partial application: `_`
    Hole,

    /// Pipeline: `expr |> f`
    Pipeline {
        value: Box<TypedExpr>,
        func: Box<TypedExpr>,
    },

    /// Type cast/coercion: `Int x`
    TypeCast {
        target_type: Type,
        value: Box<TypedExpr>,
    },

    /// Block expression: `{ expr1; expr2; value }`
    Block(Vec<TypedExpr>),

    /// With block: `expr with bindings`
    WithBlock {
        body: Box<TypedExpr>,
        bindings: Vec<TypedWithBinding>,
    },
}

#[derive(Debug, Clone, PartialEq)]
pub struct TypedLambdaClause {
    pub patterns: Vec<Pattern>,
    pub guards: Vec<TypedGuardClause>,
    pub body: Option<TypedExpr>,
    pub with: Vec<TypedWithBinding>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct TypedGuardClause {
    pub label: Ident,
    pub guard: TypedGuardCondition,
    pub body: TypedExpr,
}

#[derive(Debug, Clone, PartialEq)]
pub enum TypedGuardCondition {
    Named(Ident),
    Otherwise,
}

#[derive(Debug, Clone, PartialEq)]
pub struct TypedWithBinding {
    pub name: Ident,
    pub value: TypedExpr,
}
