//! Type System for Kata Language
//!
//! This module defines the type representation:
//! - Type: all type expressions (primitives, generics, functions, etc.)
//! - FunctionSig: function signatures
//! - Type parameters and refined types

use super::id::{Ident, QualifiedIdent};
use std::fmt;

// =============================================================================
// TYPES
// =============================================================================

/// A type expression in Kata
///
/// Types in Kata are not hardcoded - `Int`, `Float`, `Text` are just names
/// that implement interfaces like `NUM`, `ORD`, `EQ`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Type {
    /// Named type with optional generic parameters
    /// Examples: `Int`, `List::T`, `Result::T::E`, `Optional::Int`
    Named {
        name: QualifiedIdent,
        params: Vec<Type>,
    },

    /// Type variable (generic parameter)
    /// Examples: `T`, `K`, `V` in generic definitions
    Var(Ident),

    /// Tuple type
    /// Examples: `(A, B, C)`, `(Int Float)`
    Tuple(Vec<Type>),

    /// Function type
    /// Examples: `A -> B`, `A B -> C` (multi-argument function)
    Function {
        params: Vec<Type>,
        return_type: Box<Type>,
    },

    /// Refined type with predicate
    /// Example: `(Int, > _ 0)` - positive integer
    Refined {
        base: Box<Type>,
        predicate: Predicate,
    },
}

impl Type {
    /// Create a simple named type without parameters
    pub fn named(name: impl Into<String>) -> Self {
        Type::Named {
            name: QualifiedIdent::simple(name),
            params: Vec::new(),
        }
    }

    /// Create a named type with generic parameters
    pub fn generic(name: impl Into<String>, params: Vec<Type>) -> Self {
        Type::Named {
            name: QualifiedIdent::simple(name),
            params,
        }
    }

    /// Create a type variable
    pub fn var(name: impl Into<String>) -> Self {
        Type::Var(Ident::new(name))
    }

    /// Create a tuple type
    pub fn tuple(types: Vec<Type>) -> Self {
        Type::Tuple(types)
    }

    /// Create a function type
    pub fn function(params: Vec<Type>, return_type: Type) -> Self {
        Type::Function {
            params,
            return_type: Box::new(return_type),
        }
    }

    /// Create a refined type
    pub fn refined(base: Type, predicate: Predicate) -> Self {
        Type::Refined {
            base: Box::new(base),
            predicate,
        }
    }

    /// Check if this is a type variable
    pub fn is_var(&self) -> bool {
        matches!(self, Type::Var(_))
    }

    /// Check if this is a simple (unparameterized) type
    pub fn is_simple(&self) -> bool {
        matches!(self, Type::Named { params, .. } if params.is_empty())
    }
}

impl fmt::Display for Type {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Type::Named { name, params } => {
                if params.is_empty() {
                    write!(f, "{}", name)
                } else {
                    write!(f, "{}::", name)?;
                    for (i, param) in params.iter().enumerate() {
                        if i > 0 {
                            write!(f, "::")?;
                        }
                        write!(f, "{}", param)?;
                    }
                    Ok(())
                }
            }
            Type::Var(v) => write!(f, "{}", v),
            Type::Tuple(types) => {
                write!(f, "(")?;
                for (i, t) in types.iter().enumerate() {
                    if i > 0 {
                        write!(f, " ")?;
                    }
                    write!(f, "{}", t)?;
                }
                write!(f, ")")
            }
            Type::Function { params, return_type } => {
                if params.len() == 1 {
                    write!(f, "{} -> {}", &params[0], return_type)
                } else {
                    write!(f, "(")?;
                    for (i, t) in params.iter().enumerate() {
                        if i > 0 {
                            write!(f, " ")?;
                        }
                        write!(f, "{}", t)?;
                    }
                    write!(f, ") -> {}", return_type)
                }
            }
            Type::Refined { base, predicate } => {
                write!(f, "({}, {})", base, predicate)
            }
        }
    }
}

// =============================================================================
// PREDICATES (for refined types)
// =============================================================================

/// Predicate for refined types
///
/// Examples:
/// - `> _ 0` - greater than zero
/// - `< _ 10` - less than ten
/// - `except Complex` - exclusion predicate
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Predicate {
    /// Comparison with hole: `> _ 0`, `< _ 10`, `>= _ 5`
    Comparison {
        op: CompareOp,
        value: LiteralValue,
    },

    /// Range check: `<= _ 25.0`
    Range {
        op: CompareOp,
        value: LiteralValue,
    },

    /// Exclusion predicate: `except Complex`
    Except(QualifiedIdent),

    /// Combined predicates (AND)
    And(Vec<Predicate>),
}

/// Comparison operator for predicates
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CompareOp {
    Gt,  // >
    Gte, // >=
    Lt,  // <
    Lte, // <=
    Eq,  // =
    Neq, // !=
}

impl fmt::Display for CompareOp {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            CompareOp::Gt => write!(f, ">"),
            CompareOp::Gte => write!(f, ">="),
            CompareOp::Lt => write!(f, "<"),
            CompareOp::Lte => write!(f, "<="),
            CompareOp::Eq => write!(f, "="),
            CompareOp::Neq => write!(f, "!="),
        }
    }
}

/// Literal value in a predicate
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LiteralValue {
    Int(i64),
    Float(String), // String to preserve precision
    Bool(bool),
}

impl fmt::Display for LiteralValue {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            LiteralValue::Int(n) => write!(f, "{}", n),
            LiteralValue::Float(s) => write!(f, "{}", s),
            LiteralValue::Bool(b) => write!(f, "{}", b),
        }
    }
}

impl fmt::Display for Predicate {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Predicate::Comparison { op, value } => {
                write!(f, "{} _ {}", op, value)
            }
            Predicate::Range { op, value } => {
                write!(f, "{} _ {}", op, value)
            }
            Predicate::Except(t) => {
                write!(f, "except {}", t)
            }
            Predicate::And(preds) => {
                for (i, p) in preds.iter().enumerate() {
                    if i > 0 {
                        write!(f, " ")?;
                    }
                    write!(f, "{}", p)?;
                }
                Ok(())
            }
        }
    }
}

// =============================================================================
// FUNCTION SIGNATURE
// =============================================================================

/// Function signature: `Arg1 Arg2 => Return`
///
/// Note: The arrow `=>` separates parameters from return type.
/// The arrow `->` is used for function types (A -> B).
#[derive(Debug, Clone, PartialEq)]
pub struct FunctionSig {
    /// Parameter types (left of =>)
    pub params: Vec<Type>,
    /// Return type (right of =>)
    pub return_type: Type,
}

impl FunctionSig {
    /// Create a new function signature
    pub fn new(params: Vec<Type>, return_type: Type) -> Self {
        FunctionSig { params, return_type }
    }

    /// Create a signature with no parameters
    pub fn nullary(return_type: Type) -> Self {
        FunctionSig {
            params: Vec::new(),
            return_type,
        }
    }

    /// Create a signature with one parameter
    pub fn unary(param: Type, return_type: Type) -> Self {
        FunctionSig {
            params: vec![param],
            return_type,
        }
    }

    /// Create a signature with two parameters
    pub fn binary(param1: Type, param2: Type, return_type: Type) -> Self {
        FunctionSig {
            params: vec![param1, param2],
            return_type,
        }
    }

    /// Get the arity (number of parameters)
    pub fn arity(&self) -> usize {
        self.params.len()
    }

    /// Convert the signature to a function type
    pub fn to_type(&self) -> Type {
        Type::function(self.params.clone(), self.return_type.clone())
    }
}

impl fmt::Display for FunctionSig {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if self.params.is_empty() {
            write!(f, "{}", self.return_type)
        } else {
            for (i, param) in self.params.iter().enumerate() {
                if i > 0 {
                    write!(f, " ")?;
                }
                write!(f, "{}", param)?;
            }
            write!(f, " => {}", self.return_type)
        }
    }
}
