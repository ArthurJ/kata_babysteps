//! Pattern Matching for Kata Language
//!
//! This module defines patterns used in:
//! - Lambda clauses: `λ (0): 0`, `λ (n): + n 1`
//! - Match expressions: `match x { Ok(v): ..., Err(e): ... }`
//! - Destructuring: `let (a b) as tuple`

use super::id::{Ident, Literal};
use super::types::Type;
use super::Spanned;
use std::fmt;

// =============================================================================
// PATTERNS
// =============================================================================

/// A pattern for matching values
#[derive(Debug, Clone, PartialEq)]
pub enum Pattern {
    /// Literal pattern (exact match)
    /// Examples: `0`, `1`, `"hello"`, `True`, `False`
    Literal(Literal),

    /// Variable pattern (captures the value)
    /// Examples: `x`, `value`, `_`
    /// Note: `_` is a special case - it's the wildcard/hole
    Var(Ident),

    /// Wildcard pattern (matches anything, doesn't capture)
    /// Example: `_`
    Wildcard,

    /// Tuple pattern (destructuring)
    /// Examples: `(a b c)`, `(x y)`
    Tuple(Vec<Spanned<Pattern>>),

    /// List pattern (destructuring)
    /// Examples: `[head, ...tail]`, `[a, b, c]`, `[]`
    List {
        elements: Vec<Spanned<Pattern>>,
        rest: Option<Box<Spanned<Pattern>>>,
    },

    /// Array pattern (similar to list but for Array type)
    /// Examples: `{a, b, c}`
    Array(Vec<Spanned<Pattern>>),

    /// Cons pattern (head:tail for lists)
    /// Example: `x:xs`
    Cons {
        head: Box<Spanned<Pattern>>,
        tail: Box<Spanned<Pattern>>,
    },

    /// Constructor/Variant pattern (for enums)
    /// Examples: `Ok(x)`, `Err(e)`, `Some(value)`, `None`
    Variant {
        name: Ident,
        args: Vec<Spanned<Pattern>>,
    },

    /// Or pattern (multiple alternatives)
    /// Example: `A | B | C`
    Or(Vec<Spanned<Pattern>>),

    /// Guarded pattern (pattern with condition)
    /// Example: `n where > n 0`
    Guarded {
        pattern: Box<Spanned<Pattern>>,
        condition: Box<Spanned<Pattern>>, // Simplified - could be Expr
    },

    /// Type constraint pattern (pattern with type annotation)
    /// Example: `x::Int`
    Typed {
        pattern: Box<Spanned<Pattern>>,
        type_annotation: Type,
    },

    /// Range pattern (for numeric values)
    /// Example: `[1..10]` matches values in range
    Range {
        start: Literal,
        end: Literal,
        inclusive: bool,
    },
}

impl Pattern {
    /// Create a literal pattern
    pub fn literal(lit: Literal) -> Self {
        Pattern::Literal(lit)
    }

    /// Create a variable pattern
    pub fn var(name: impl Into<String>) -> Self {
        Pattern::Var(Ident::new(name))
    }

    /// Create a wildcard pattern
    pub fn wildcard() -> Self {
        Pattern::Wildcard
    }

    /// Check if this pattern captures any variables
    pub fn captures_variables(&self) -> bool {
        match self {
            Pattern::Var(_) => true,
            Pattern::Wildcard | Pattern::Literal(_) | Pattern::Range { .. } => false,
            Pattern::Tuple(patterns) => patterns.iter().any(|p| p.node.captures_variables()),
            Pattern::List { elements, rest } => {
                elements.iter().any(|p| p.node.captures_variables())
                    || rest.as_ref().map_or(false, |r| r.node.captures_variables())
            }
            Pattern::Array(patterns) => patterns.iter().any(|p| p.node.captures_variables()),
            Pattern::Cons { head, tail } => head.node.captures_variables() || tail.node.captures_variables(),
            Pattern::Variant { args, .. } => args.iter().any(|p| p.node.captures_variables()),
            Pattern::Or(patterns) => patterns.iter().any(|p| p.node.captures_variables()),
            Pattern::Guarded { pattern, .. } => pattern.node.captures_variables(),
            Pattern::Typed { pattern, .. } => pattern.node.captures_variables(),
        }
    }

    /// Get all variables captured by this pattern
    pub fn captured_variables(&self) -> Vec<&Ident> {
        let mut vars = Vec::new();
        self.collect_variables(&mut vars);
        vars
    }

    fn collect_variables<'a>(&'a self, vars: &mut Vec<&'a Ident>) {
        match self {
            Pattern::Var(v) => vars.push(v),
            Pattern::Wildcard | Pattern::Literal(_) | Pattern::Range { .. } => {}
            Pattern::Tuple(patterns) => {
                for p in patterns {
                    p.node.collect_variables(vars);
                }
            }
            Pattern::List { elements, rest } => {
                for p in elements {
                    p.node.collect_variables(vars);
                }
                if let Some(r) = rest {
                    r.node.collect_variables(vars);
                }
            }
            Pattern::Array(patterns) => {
                for p in patterns {
                    p.node.collect_variables(vars);
                }
            }
            Pattern::Cons { head, tail } => {
                head.node.collect_variables(vars);
                tail.node.collect_variables(vars);
            }
            Pattern::Variant { args, .. } => {
                for p in args {
                    p.node.collect_variables(vars);
                }
            }
            Pattern::Or(patterns) => {
                for p in patterns {
                    p.node.collect_variables(vars);
                }
            }
            Pattern::Guarded { pattern, .. } => pattern.node.collect_variables(vars),
            Pattern::Typed { pattern, .. } => pattern.node.collect_variables(vars),
        }
    }
}

impl fmt::Display for Pattern {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Pattern::Literal(lit) => write!(f, "{}", lit),
            Pattern::Var(v) => write!(f, "{}", v),
            Pattern::Wildcard => write!(f, "_"),
            Pattern::Tuple(patterns) => {
                write!(f, "(")?;
                for (i, p) in patterns.iter().enumerate() {
                    if i > 0 {
                        write!(f, " ")?;
                    }
                    write!(f, "{}", p.node)?;
                }
                write!(f, ")")
            }
            Pattern::List { elements, rest } => {
                write!(f, "[")?;
                for (i, p) in elements.iter().enumerate() {
                    if i > 0 {
                        write!(f, " ")?;
                    }
                    write!(f, "{}", p.node)?;
                }
                if let Some(r) = rest {
                    write!(f, " ...{}", r.node)?;
                }
                write!(f, "]")
            }
            Pattern::Array(patterns) => {
                write!(f, "{{")?;
                for (i, p) in patterns.iter().enumerate() {
                    if i > 0 {
                        write!(f, " ")?;
                    }
                    write!(f, "{}", p.node)?;
                }
                write!(f, "}}")
            }
            Pattern::Cons { head, tail } => {
                write!(f, "{}:{}", head.node, tail.node)
            }
            Pattern::Variant { name, args } => {
                write!(f, "{}", name)?;
                if !args.is_empty() {
                    write!(f, "(")?;
                    for (i, p) in args.iter().enumerate() {
                        if i > 0 {
                            write!(f, " ")?;
                        }
                        write!(f, "{}", p.node)?;
                    }
                    write!(f, ")")?;
                }
                Ok(())
            }
            Pattern::Or(patterns) => {
                for (i, p) in patterns.iter().enumerate() {
                    if i > 0 {
                        write!(f, " | ")?;
                    }
                    write!(f, "{}", p.node)?;
                }
                Ok(())
            }
            Pattern::Guarded { pattern, condition } => {
                write!(f, "{} where {}", pattern.node, condition.node)
            }
            Pattern::Typed { pattern, type_annotation } => {
                write!(f, "{}::{}", pattern.node, type_annotation)
            }
            Pattern::Range { start, end, inclusive } => {
                if *inclusive {
                    write!(f, "[{}..={}]", start, end)
                } else {
                    write!(f, "[{}..{}]", start, end)
                }
            }
        }
    }
}

// =============================================================================
// GUARDS
// =============================================================================

/// A guard condition in pattern matching
///
/// Guards allow conditional matching beyond structural patterns.
///
/// Example:
/// ```kata
/// λ (x y)
///     maior: x
///     menor: y
///     otherwise: y
///     with
///         maior as > x y
///         menor as < x y
/// ```
#[derive(Debug, Clone, PartialEq)]
pub enum Guard {
    /// Boolean expression guard
    Condition(Ident), // Simplified - would be Expr in full implementation

    /// The `otherwise` clause - matches when all other guards fail
    Otherwise,
}

impl fmt::Display for Guard {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Guard::Condition(cond) => write!(f, "{}", cond),
            Guard::Otherwise => write!(f, "otherwise"),
        }
    }
}