//! Statements for Kata Language (Imperative/Action Domain)
//!
//! This module defines statements used in the action domain.
//! Statements can have side effects and are only allowed inside actions.

use super::id::Ident;
use super::expr::Expr;
use super::pattern::Pattern;
use std::fmt;

// =============================================================================
// STATEMENTS
// =============================================================================

/// A statement in the action domain
#[derive(Debug, Clone, PartialEq)]
pub enum Stmt {
    // === Bindings ===
    /// Immutable binding: `let x expr`
    Let {
        name: Ident,
        value: Expr,
    },

    /// Immutable binding with destructuring: `let (a b) as tuple`
    LetDestructure {
        pattern: Pattern,
        value: Expr,
    },

    /// Mutable binding (action only): `var x expr`
    Var {
        name: Ident,
        value: Expr,
    },

    /// Assignment to mutable variable: `var x (+ x 1)`
    Assign {
        name: Ident,
        value: Expr,
    },

    // === Control Flow (Imperative) ===
    /// Match expression: `match valor { ... }`
    Match {
        value: Expr,
        cases: Vec<MatchCase>,
    },

    /// Infinite loop: `loop corpo`
    Loop {
        body: Vec<Stmt>,
    },

    /// For loop: `for x in lista corpo`
    For {
        var: Ident,
        iterable: Expr,
        body: Vec<Stmt>,
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
    Expr(Expr),

    // === Return (Implicit) ===
    /// Return statement (usually implicit, last expression)
    Return(Expr),

    // === Panic and Assertions ===
    /// Panic: `panic! "message"`
    Panic {
        message: Expr,
    },

    /// Assert: `assert! condition "message"`
    Assert {
        condition: Expr,
        message: Expr,
    },
}

impl Stmt {
    /// Create a let binding
    pub fn let_binding(name: impl Into<String>, value: Expr) -> Self {
        Stmt::Let {
            name: Ident::new(name),
            value,
        }
    }

    /// Create a var binding
    pub fn var_binding(name: impl Into<String>, value: Expr) -> Self {
        Stmt::Var {
            name: Ident::new(name),
            value,
        }
    }

    /// Create an assignment
    pub fn assign(name: impl Into<String>, value: Expr) -> Self {
        Stmt::Assign {
            name: Ident::new(name),
            value,
        }
    }

    /// Create an expression statement
    pub fn expr(e: Expr) -> Self {
        Stmt::Expr(e)
    }

    /// Create a match statement
    pub fn match_stmt(value: Expr, cases: Vec<MatchCase>) -> Self {
        Stmt::Match { value, cases }
    }

    /// Create a loop statement
    pub fn loop_stmt(body: Vec<Stmt>) -> Self {
        Stmt::Loop { body }
    }

    /// Create a for statement
    pub fn for_stmt(var: impl Into<String>, iterable: Expr, body: Vec<Stmt>) -> Self {
        Stmt::For {
            var: Ident::new(var),
            iterable,
            body,
        }
    }

    /// Check if this is a control flow statement
    pub fn is_control_flow(&self) -> bool {
        matches!(self, Stmt::Break | Stmt::Continue | Stmt::Return(_))
    }
}

impl fmt::Display for Stmt {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Stmt::Let { name, value } => {
                write!(f, "let {} {}", name, value)
            }
            Stmt::LetDestructure { pattern, value } => {
                write!(f, "let {} as {}", pattern, value)
            }
            Stmt::Var { name, value } => {
                write!(f, "var {} {}", name, value)
            }
            Stmt::Assign { name, value } => {
                write!(f, "var {} {}", name, value)
            }
            Stmt::Match { value, cases } => {
                write!(f, "match {}\n", value)?;
                for case in cases {
                    write!(f, "    {}", case)?;
                }
                Ok(())
            }
            Stmt::Loop { body } => {
                write!(f, "loop\n")?;
                for stmt in body {
                    write!(f, "    {}\n", stmt)?;
                }
                Ok(())
            }
            Stmt::For { var, iterable, body } => {
                write!(f, "for {} in {}\n", var, iterable)?;
                for stmt in body {
                    write!(f, "    {}\n", stmt)?;
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
            Stmt::Expr(e) => write!(f, "{}", e),
            Stmt::Return(e) => write!(f, "return {}", e),
            Stmt::Panic { message } => write!(f, "panic! {}", message),
            Stmt::Assert { condition, message } => {
                write!(f, "assert! {} {}", condition, message)
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
    pub pattern: Pattern,
    /// Body statements
    pub body: Vec<Stmt>,
}

impl MatchCase {
    pub fn new(pattern: Pattern, body: Vec<Stmt>) -> Self {
        MatchCase { pattern, body }
    }

    pub fn single(pattern: Pattern, stmt: Stmt) -> Self {
        MatchCase {
            pattern,
            body: vec![stmt],
        }
    }
}

impl fmt::Display for MatchCase {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}: ", self.pattern)?;
        for (i, stmt) in self.body.iter().enumerate() {
            if i > 0 {
                write!(f, "\n        ")?;
            }
            write!(f, "{}", stmt)?;
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
    pub body: Vec<Stmt>,
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
            write!(f, "{}", stmt)?;
        }
        Ok(())
    }
}

/// Channel operation in select
#[derive(Debug, Clone, PartialEq)]
pub enum ChannelOp {
    /// Receive operation: `<! rx` or `<!? rx`
    Receive {
        channel: Expr,
        non_blocking: bool, // true for <!?
    },

    /// Send operation: `value !> tx`
    Send {
        value: Expr,
        channel: Expr,
    },
}

impl fmt::Display for ChannelOp {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ChannelOp::Receive { channel, non_blocking } => {
                if *non_blocking {
                    write!(f, "<!? {}", channel)
                } else {
                    write!(f, "<! {}", channel)
                }
            }
            ChannelOp::Send { value, channel } => {
                write!(f, "{} !> {}", value, channel)
            }
        }
    }
}

/// Timeout case in select
#[derive(Debug, Clone, PartialEq)]
pub struct SelectTimeout {
    /// Timeout duration in milliseconds
    pub duration: Expr,
    /// Body to execute on timeout
    pub body: Vec<Stmt>,
}

impl fmt::Display for SelectTimeout {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "timeout {}: ", self.duration)?;
        for (i, stmt) in self.body.iter().enumerate() {
            if i > 0 {
                write!(f, "\n        ")?;
            }
            write!(f, "{}", stmt)?;
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
    pub expr: Expr,
}

impl fmt::Display for ErrorPropagation {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}?", self.expr)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ast::id::Literal;

    #[test]
    fn test_let_stmt() {
        let stmt = Stmt::let_binding("x", Expr::literal(Literal::int("42")));
        assert_eq!(stmt.to_string(), "let x 42");
    }

    #[test]
    fn test_var_stmt() {
        let stmt = Stmt::var_binding("counter", Expr::literal(Literal::int("0")));
        assert_eq!(stmt.to_string(), "var counter 0");
    }

    #[test]
    fn test_loop_stmt() {
        let stmt = Stmt::loop_stmt(vec![
            Stmt::let_binding("x", Expr::literal(Literal::int("1"))),
        ]);
        assert!(stmt.to_string().contains("loop"));
    }

    #[test]
    fn test_for_stmt() {
        let stmt = Stmt::for_stmt(
            "item",
            Expr::var("lista"),
            vec![Stmt::expr(Expr::var("item"))],
        );
        assert!(stmt.to_string().contains("for item in"));
    }

    #[test]
    fn test_match_case() {
        let case = MatchCase::single(
            Pattern::variant("Ok", vec![Pattern::var("value")]),
            Stmt::expr(Expr::var("value")),
        );
        assert!(case.to_string().contains("Ok(value)"));
    }
}