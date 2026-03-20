//! Typed Statements (TAST) for Kata Language

use crate::ast::id::Ident;
use crate::ast::pattern::Pattern;
use crate::tast::expr::TypedExpr;
use crate::lexer::Span;

/// A fully typed statement in the action domain
#[derive(Debug, Clone, PartialEq)]
pub struct TypedStmt {
    pub kind: StmtKind,
    pub span: Span,
}

#[derive(Debug, Clone, PartialEq)]
pub enum StmtKind {
    /// Immutable binding: `let x 1`
    Let {
        pattern: Pattern,
        value: TypedExpr,
    },

    /// Mutable binding: `var x 1`
    Var {
        pattern: Pattern,
        value: TypedExpr,
    },

    /// Assignment: `var x 2`
    Assign {
        name: Ident,
        value: TypedExpr,
    },

    /// Match statement
    Match {
        value: TypedExpr,
        cases: Vec<TypedMatchCase>,
    },

    /// Infinite loop
    Loop {
        body: Vec<TypedStmt>,
    },

    /// For loop
    For {
        var: Ident,
        iterable: TypedExpr,
        body: Vec<TypedStmt>,
    },

    /// Break statement
    Break,

    /// Continue statement
    Continue,

    /// Select statement (CSP)
    Select {
        cases: Vec<TypedSelectCase>,
        timeout: Option<TypedSelectTimeout>,
    },

    /// Expression statement
    Expr(TypedExpr),

    /// Return statement
    Return(TypedExpr),

    /// Panic statement
    Panic {
        message: TypedExpr,
    },

    /// Assert statement
    Assert {
        condition: TypedExpr,
        message: TypedExpr,
    },
}

#[derive(Debug, Clone, PartialEq)]
pub struct TypedMatchCase {
    pub pattern: Pattern,
    pub body: Vec<TypedStmt>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct TypedSelectCase {
    pub operation: TypedChannelOp,
    pub binding: Option<Ident>,
    pub body: Vec<TypedStmt>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum TypedChannelOp {
    Receive {
        channel: TypedExpr,
        non_blocking: bool,
    },
    Send {
        value: TypedExpr,
        channel: TypedExpr,
    },
}

#[derive(Debug, Clone, PartialEq)]
pub struct TypedSelectTimeout {
    pub duration: TypedExpr,
    pub body: Vec<TypedStmt>,
}
