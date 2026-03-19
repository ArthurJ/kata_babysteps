//! Pattern Matching for Kata Language
//!
//! This module defines patterns used in:
//! - Lambda clauses: `λ (0): 0`, `λ (n): + n 1`
//! - Match expressions: `match x { Ok(v): ..., Err(e): ... }`
//! - Destructuring: `let (a b) as tuple`

use super::id::{Ident, Literal};
use super::types::Type;
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
    Tuple(Vec<Pattern>),

    /// List pattern (destructuring)
    /// Examples: `[head, ...tail]`, `[a, b, c]`, `[]`
    List {
        elements: Vec<Pattern>,
        rest: Option<Box<Pattern>>,
    },

    /// Array pattern (similar to list but for Array type)
    /// Examples: `{a, b, c}`
    Array(Vec<Pattern>),

    /// Constructor/Variant pattern (for enums)
    /// Examples: `Ok(x)`, `Err(e)`, `Some(value)`, `None`
    Variant {
        name: Ident,
        args: Vec<Pattern>,
    },

    /// Or pattern (multiple alternatives)
    /// Example: `A | B | C`
    Or(Vec<Pattern>),

    /// Guarded pattern (pattern with condition)
    /// Example: `n where > n 0`
    Guarded {
        pattern: Box<Pattern>,
        condition: Box<Pattern>, // Simplified - could be Expr
    },

    /// Type constraint pattern (pattern with type annotation)
    /// Example: `x::Int`
    Typed {
        pattern: Box<Pattern>,
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

    /// Create a tuple pattern
    pub fn tuple(patterns: Vec<Pattern>) -> Self {
        Pattern::Tuple(patterns)
    }

    /// Create a variant pattern (for enums)
    pub fn variant(name: impl Into<String>, args: Vec<Pattern>) -> Self {
        Pattern::Variant {
            name: Ident::new(name),
            args,
        }
    }

    /// Create an empty list pattern
    pub fn empty_list() -> Self {
        Pattern::List {
            elements: Vec::new(),
            rest: None,
        }
    }

    /// Check if this pattern captures any variables
    pub fn captures_variables(&self) -> bool {
        match self {
            Pattern::Var(_) => true,
            Pattern::Wildcard | Pattern::Literal(_) | Pattern::Range { .. } => false,
            Pattern::Tuple(patterns) => patterns.iter().any(|p| p.captures_variables()),
            Pattern::List { elements, rest } => {
                elements.iter().any(|p| p.captures_variables())
                    || rest.as_ref().map_or(false, |r| r.captures_variables())
            }
            Pattern::Array(patterns) => patterns.iter().any(|p| p.captures_variables()),
            Pattern::Variant { args, .. } => args.iter().any(|p| p.captures_variables()),
            Pattern::Or(patterns) => patterns.iter().any(|p| p.captures_variables()),
            Pattern::Guarded { pattern, .. } => pattern.captures_variables(),
            Pattern::Typed { pattern, .. } => pattern.captures_variables(),
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
                    p.collect_variables(vars);
                }
            }
            Pattern::List { elements, rest } => {
                for p in elements {
                    p.collect_variables(vars);
                }
                if let Some(r) = rest {
                    r.collect_variables(vars);
                }
            }
            Pattern::Array(patterns) => {
                for p in patterns {
                    p.collect_variables(vars);
                }
            }
            Pattern::Variant { args, .. } => {
                for p in args {
                    p.collect_variables(vars);
                }
            }
            Pattern::Or(patterns) => {
                for p in patterns {
                    p.collect_variables(vars);
                }
            }
            Pattern::Guarded { pattern, .. } => pattern.collect_variables(vars),
            Pattern::Typed { pattern, .. } => pattern.collect_variables(vars),
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
                    write!(f, "{}", p)?;
                }
                write!(f, ")")
            }
            Pattern::List { elements, rest } => {
                write!(f, "[")?;
                for (i, p) in elements.iter().enumerate() {
                    if i > 0 {
                        write!(f, " ")?;
                    }
                    write!(f, "{}", p)?;
                }
                if let Some(r) = rest {
                    write!(f, " ...{}", r)?;
                }
                write!(f, "]")
            }
            Pattern::Array(patterns) => {
                write!(f, "{{")?;
                for (i, p) in patterns.iter().enumerate() {
                    if i > 0 {
                        write!(f, " ")?;
                    }
                    write!(f, "{}", p)?;
                }
                write!(f, "}}")
            }
            Pattern::Variant { name, args } => {
                write!(f, "{}", name)?;
                if !args.is_empty() {
                    write!(f, "(")?;
                    for (i, p) in args.iter().enumerate() {
                        if i > 0 {
                            write!(f, " ")?;
                        }
                        write!(f, "{}", p)?;
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
                    write!(f, "{}", p)?;
                }
                Ok(())
            }
            Pattern::Guarded { pattern, condition } => {
                write!(f, "{} where {}", pattern, condition)
            }
            Pattern::Typed { pattern, type_annotation } => {
                write!(f, "{}::{}", pattern, type_annotation)
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_literal_pattern() {
        let p = Pattern::literal(Literal::int("42"));
        assert_eq!(p.to_string(), "42");
        assert!(!p.captures_variables());
    }

    #[test]
    fn test_var_pattern() {
        let p = Pattern::var("x");
        assert_eq!(p.to_string(), "x");
        assert!(p.captures_variables());
        assert_eq!(p.captured_variables().len(), 1);
    }

    #[test]
    fn test_tuple_pattern() {
        let p = Pattern::tuple(vec![
            Pattern::var("a"),
            Pattern::var("b"),
            Pattern::wildcard(),
        ]);
        assert_eq!(p.to_string(), "(a b _)");
        assert!(p.captures_variables());
        assert_eq!(p.captured_variables().len(), 2);
    }

    #[test]
    fn test_variant_pattern() {
        let p = Pattern::variant("Ok", vec![Pattern::var("value")]);
        assert_eq!(p.to_string(), "Ok(value)");
    }

    #[test]
    fn test_wildcard_pattern() {
        let p = Pattern::wildcard();
        assert_eq!(p.to_string(), "_");
        assert!(!p.captures_variables());
    }

    #[test]
    fn test_or_pattern() {
        let p = Pattern::Or(vec![
            Pattern::literal(Literal::int("0")),
            Pattern::literal(Literal::int("1")),
        ]);
        assert_eq!(p.to_string(), "0 | 1");
    }

    #[test]
    fn test_guard_display() {
        assert_eq!(Guard::Otherwise.to_string(), "otherwise");
        assert_eq!(Guard::Condition(Ident::new("maior")).to_string(), "maior");
    }
}