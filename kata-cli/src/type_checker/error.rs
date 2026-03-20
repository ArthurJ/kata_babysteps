//! Type Checker Diagnostics and Errors

use std::fmt;
use crate::ast::types::Type;
use crate::lexer::Span;

/// Errors that can occur during Type Checking and Inference
#[derive(Debug, Clone, PartialEq)]
pub enum TypeError {
    /// Two types cannot be unified (e.g., expected Int, found Float)
    TypeMismatch {
        expected: Type,
        found: Type,
        span: Span,
    },

    /// Occurs check failed: A type variable would resolve to a type containing itself (infinite type)
    InfiniteType {
        var: String,
        typ: Type,
        span: Span,
    },

    /// A variable was used but not declared in any accessible scope
    UnboundVariable {
        name: String,
        span: Span,
    },

    /// No matching signature found for a multiple-dispatch function call
    NoMatchingDispatch {
        func_name: String,
        args: Vec<Type>,
        span: Span,
    },

    /// Multiple signatures match a function call ambiguously
    AmbiguousDispatch {
        func_name: String,
        args: Vec<Type>,
        span: Span,
    },

    /// A pure function attempted to call an impure action (e.g., `echo!`)
    ImpureCallInPureContext {
        action_name: String,
        span: Span,
    },

    /// Orphan Rule violation: Cannot implement foreign trait for foreign types
    OrphanRuleViolation {
        type_name: String,
        interface_name: String,
        span: Span,
    },
    
    /// Arity mismatch: Function called with wrong number of arguments
    ArityMismatch {
        expected: usize,
        found: usize,
        span: Span,
    },
}

impl fmt::Display for TypeError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            TypeError::TypeMismatch { expected, found, .. } => {
                write!(f, "Type Mismatch: expected `{}`, but found `{}`", expected, found)
            }
            TypeError::InfiniteType { var, typ, .. } => {
                write!(f, "Infinite Type: variable `{}` occurs inside `{}`", var, typ)
            }
            TypeError::UnboundVariable { name, .. } => {
                write!(f, "Unbound Variable: `{}` is not defined in this scope", name)
            }
            TypeError::NoMatchingDispatch { func_name, args, .. } => {
                let args_str: Vec<String> = args.iter().map(|a| a.to_string()).collect();
                write!(f, "No Matching Dispatch: no implementation of `{}` accepts arguments ({})", func_name, args_str.join(" "))
            }
            TypeError::AmbiguousDispatch { func_name, args, .. } => {
                let args_str: Vec<String> = args.iter().map(|a| a.to_string()).collect();
                write!(f, "Ambiguous Dispatch: multiple implementations of `{}` match arguments ({})", func_name, args_str.join(" "))
            }
            TypeError::ImpureCallInPureContext { action_name, .. } => {
                write!(f, "Impure Call in Pure Context: pure functions cannot call action `{}`", action_name)
            }
            TypeError::OrphanRuleViolation { type_name, interface_name, .. } => {
                write!(f, "Orphan Rule Violation: cannot implement foreign interface `{}` for foreign type `{}`", interface_name, type_name)
            }
            TypeError::ArityMismatch { expected, found, .. } => {
                write!(f, "Arity Mismatch: expected {} arguments, but found {}", expected, found)
            }
        }
    }
}
