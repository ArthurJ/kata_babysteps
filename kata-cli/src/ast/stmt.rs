//! Statements for Kata Language (Imperative/Action Domain)
//!
//! This module defines statements used in the action domain.
//! Statements can have side effects and are only allowed inside actions.

use super::id::Ident;
use super::expr::Expr;
use super::pattern::Pattern;
use super::Spanned;
use std::fmt;

// =============================================================================
// STATEMENTS
// =============================================================================

/// A statement in the action domain
#[derive(Debug, Clone, PartialEq)]
pub enum Stmt {
    // === Bindings ===
    /// Immutable binding: `let x 1` or `let x::Int 1` or `let (a b) pair`
    Let {
        pattern: Spanned<Pattern>,
        value: Spanned<Expr>,
    },

    /// Mutable binding (action only): `var x 1` or `var x::Int 1`
    Var {
        pattern: Spanned<Pattern>,
        value: Spanned<Expr>,
    },

    /// Assignment to mutable variable: `var x (+ x 1)`
    Assign {
        name: Ident,
        value: Spanned<Expr>,
    },

    // === Control Flow (Imperative) ===
    /// Match expression: `match valor { ... }`
    Match {
        value: Spanned<Expr>,
        cases: Vec<MatchCase>,
    },

    /// Infinite loop: `loop corpo`
    Loop {
        body: Vec<Spanned<Stmt>>,
    },

    /// For loop: `for x in lista corpo`
    For {
        var: Ident,
        iterable: Spanned<Expr>,
        body: Vec<Spanned<Stmt>>,
    },

    /// Break statement (inside loops)
    Break,

    /// Continue statement (inside loops)
    Continue,

    // === Concurrency (CSP) ===
    /// Select statement: multiplexing over channels
    Select {
        cases: Vec<SelectCase>,
        timeout: Option<SelectTimeout>,
    },

    // === Expression Statement ===
    /// Expression as statement (for side effects)
    Expr(Spanned<Expr>),

    // === Return (Implicit) ===
    /// Return statement (usually implicit, last expression)
    Return(Spanned<Expr>),

    // === Panic and Assertions ===
    /// Panic: `panic! "message"`
    Panic {
        message: Spanned<Expr>,
    },

    /// Assert: `assert! condition "message"`
    Assert {
        condition: Spanned<Expr>,
        message: Spanned<Expr>,
    },
}

impl Stmt {
    /// Check if this is a control flow statement
    pub fn is_control_flow(&self) -> bool {
        matches!(self, Stmt::Break | Stmt::Continue | Stmt::Return(_))
    }
}

impl fmt::Display for Stmt {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Stmt::Let { pattern, value } => {
                write!(f, "let {} {}", pattern.node, value.node)
            }
            Stmt::Var { pattern, value } => {
                write!(f, "var {} {}", pattern.node, value.node)
            }
            Stmt::Assign { name, value } => {
                write!(f, "var {} {}", name, value.node)
            }
            Stmt::Match { value, cases } => {
                write!(f, "match {}\n", value.node)?;
                for case in cases {
                    write!(f, "    {}", case)?;
                }
                Ok(())
            }
            Stmt::Loop { body } => {
                write!(f, "loop\n")?;
                for stmt in body {
                    write!(f, "    {}\n", stmt.node)?;
                }
                Ok(())
            }
            Stmt::For { var, iterable, body } => {
                write!(f, "for {} in {}\n", var, iterable.node)?;
                for stmt in body {
                    write!(f, "    {}\n", stmt.node)?;
                }
                Ok(())
            }
            Stmt::Break => write!(f, "break"),
            Stmt::Continue => write!(f, "continue"),
            Stmt::Select { cases, timeout } => {
                write!(f, "select\n")?;
                for case in cases {
                    write!(f, "    {}\n", case)?;
                }
                if let Some(t) = timeout {
                    write!(f, "    {}\n", t)?;
                }
                Ok(())
            }
            Stmt::Expr(e) => write!(f, "{}", e.node),
            Stmt::Return(e) => write!(f, "return {}", e.node),
            Stmt::Panic { message } => write!(f, "panic! {}", message.node),
            Stmt::Assert { condition, message } => {
                write!(f, "assert! {} {}", condition.node, message.node)
            }
        }
    }
}

// =============================================================================
// MATCH CASE (Action Domain)
// =============================================================================

/// A case in a match statement
#[derive(Debug, Clone, PartialEq)]
pub struct MatchCase {
    /// Pattern to match
    pub pattern: Spanned<Pattern>,
    /// Body statements
    pub body: Vec<Spanned<Stmt>>,
}

impl MatchCase {
    pub fn new(pattern: Spanned<Pattern>, body: Vec<Spanned<Stmt>>) -> Self {
        MatchCase { pattern, body }
    }
}

impl fmt::Display for MatchCase {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}: ", self.pattern.node)?;
        for (i, stmt) in self.body.iter().enumerate() {
            if i > 0 {
                write!(f, "\n        ")?;
            }
            write!(f, "{}", stmt.node)?;
        }
        Ok(())
    }
}

// =============================================================================
// SELECT CASE (CSP)
// =============================================================================

/// A case in a select statement
///
/// Select multiplexes over multiple channel operations.
#[derive(Debug, Clone, PartialEq)]
pub struct SelectCase {
    /// Channel operation to wait on
    pub operation: ChannelOp,
    /// Variable to bind (if receiving) or value to send
    pub binding: Option<Ident>,
    /// Body to execute when this case is selected
    pub body: Vec<Spanned<Stmt>>,
}

impl fmt::Display for SelectCase {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match &self.binding {
            Some(var) => write!(f, "case {} -> {}: ", self.operation, var)?,
            None => write!(f, "case {}: ", self.operation)?,
        }
        for (i, stmt) in self.body.iter().enumerate() {
            if i > 0 {
                write!(f, "\n        ")?;
            }
            write!(f, "{}", stmt.node)?;
        }
        Ok(())
    }
}

/// Channel operation in select
#[derive(Debug, Clone, PartialEq)]
pub enum ChannelOp {
    /// Receive operation: `<! rx` or `<!? rx`
    Receive {
        channel: Spanned<Expr>,
        non_blocking: bool, // true for <!?
    },

    /// Send operation: `value !> tx`
    Send {
        value: Spanned<Expr>,
        channel: Spanned<Expr>,
    },
}

impl fmt::Display for ChannelOp {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ChannelOp::Receive { channel, non_blocking } => {
                if *non_blocking {
                    write!(f, "<!? {}", channel.node)
                } else {
                    write!(f, "<! {}", channel.node)
                }
            }
            ChannelOp::Send { value, channel } => {
                write!(f, "{} !> {}", value.node, channel.node)
            }
        }
    }
}

/// Timeout case in select
#[derive(Debug, Clone, PartialEq)]
pub struct SelectTimeout {
    /// Timeout duration in milliseconds
    pub duration: Spanned<Expr>,
    /// Body to execute on timeout
    pub body: Vec<Spanned<Stmt>>,
}

impl fmt::Display for SelectTimeout {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "timeout {}: ", self.duration.node)?;
        for (i, stmt) in self.body.iter().enumerate() {
            if i > 0 {
                write!(f, "\n        ")?;
            }
            write!(f, "{}", stmt.node)?;
        }
        Ok(())
    }
}

// =============================================================================
// ERROR PROPAGATION (?)
// =============================================================================

/// Error propagation operator: `expr?`
///
/// In actions, `?` after an expression unwraps a Result/Optional
/// or returns early with the error.
#[derive(Debug, Clone, PartialEq)]
pub struct ErrorPropagation {
    pub expr: Spanned<Expr>,
}

impl fmt::Display for ErrorPropagation {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}?", self.expr.node)
    }
}
