//! Hindley-Milner Type Inference Engine
//!
//! This module provides the core unification and substitution mechanisms
//! required to infer types for variables and expressions that lack explicit
//! type annotations.

use std::collections::{HashMap, HashSet};
use std::sync::atomic::{AtomicUsize, Ordering};

use crate::ast::types::Type;
use crate::lexer::Span;
use crate::type_checker::error::TypeError;

// =============================================================================
// TYPE VARIABLE GENERATION
// =============================================================================

/// A global counter to generate unique type variables (e.g., `t_1`, `t_2`).
static NEXT_TYPE_VAR: AtomicUsize = AtomicUsize::new(1);

/// Generates a fresh, unique type variable name.
pub fn fresh_type_var() -> String {
    let id = NEXT_TYPE_VAR.fetch_add(1, Ordering::SeqCst);
    format!("t_{}", id)
}

/// Creates a new `Type::Var` with a fresh unique name.
pub fn fresh_type() -> Type {
    Type::Var(crate::ast::id::Ident::new(fresh_type_var()))
}

// =============================================================================
// SUBSTITUTIONS
// =============================================================================

/// A mapping from a type variable name to a concrete type.
/// As inference progresses, we learn more about what variables actually are.
pub type Substitution = HashMap<String, Type>;

/// Trait for applying substitutions to types.
pub trait Substitutable {
    /// Recursively replaces type variables with their known concrete types from the substitution.
    fn apply(&self, subst: &Substitution) -> Self;
    /// Returns the set of all free (unbound) type variables in this type.
    fn free_type_vars(&self) -> HashSet<String>;
}

impl Substitutable for Type {
    fn apply(&self, subst: &Substitution) -> Self {
        match self {
            // If it's a variable, check if we have a substitution for it.
            // If we do, apply the substitution recursively (in case it maps to another var).
            Type::Var(id) => {
                if let Some(t) = subst.get(&id.0) {
                    t.apply(subst)
                } else {
                    self.clone()
                }
            }
            // For named types, apply to their generic parameters.
            Type::Named { name, params } => Type::Named {
                name: name.clone(),
                params: params.iter().map(|p| p.apply(subst)).collect(),
            },
            // For tuples, apply to all elements.
            Type::Tuple(types) => Type::Tuple(types.iter().map(|t| t.apply(subst)).collect()),
            // For functions, apply to params and return type.
            Type::Function { params, return_type } => Type::Function {
                params: params.iter().map(|p| p.apply(subst)).collect(),
                return_type: Box::new(return_type.apply(subst)),
            },
            // Refinements apply to their base type.
            Type::Refined { base, predicate } => Type::Refined {
                base: Box::new(base.apply(subst)),
                predicate: predicate.clone(),
            },
        }
    }

    fn free_type_vars(&self) -> HashSet<String> {
        let mut vars = HashSet::new();
        match self {
            Type::Var(id) => {
                vars.insert(id.0.clone());
            }
            Type::Named { params, .. } => {
                for p in params {
                    vars.extend(p.free_type_vars());
                }
            }
            Type::Tuple(types) => {
                for t in types {
                    vars.extend(t.free_type_vars());
                }
            }
            Type::Function { params, return_type } => {
                for t in params {
                    vars.extend(t.free_type_vars());
                }
                vars.extend(return_type.free_type_vars());
            }
            Type::Refined { base: elem, .. } => {
                vars.extend(elem.free_type_vars());
            }
        }
        vars
    }
}

/// Composes two substitutions. `compose(s1, s2)` means apply `s1` first, then `s2`.
pub fn compose(s1: &Substitution, s2: &Substitution) -> Substitution {
    let mut result = s2.clone();
    for (k, v) in s1 {
        result.insert(k.clone(), v.apply(s2));
    }
    result
}

// =============================================================================
// UNIFICATION
// =============================================================================

/// Attempts to unify two types, producing a Substitution that makes them equal.
/// If they cannot be unified, returns a TypeError.
pub fn unify(t1: &Type, t2: &Type, span: &Span) -> Result<Substitution, TypeError> {
    match (t1, t2) {
        // If they are exactly the same type, no substitution is needed.
        (a, b) if a == b => Ok(HashMap::new()),

        // If the left side is a variable, bind it to the right side.
        (Type::Var(id), t) | (t, Type::Var(id)) => bind_var(&id.0, t, span),

        // Unify Tuples: (A B) and (C D) unify if they have the same length and their elements unify.
        (Type::Tuple(types1), Type::Tuple(types2)) if types1.len() == types2.len() => {
            let mut subst = HashMap::new();
            for (ty1, ty2) in types1.iter().zip(types2.iter()) {
                let s1 = unify(&ty1.apply(&subst), &ty2.apply(&subst), span)?;
                subst = compose(&subst, &s1);
            }
            Ok(subst)
        }

        // Unify Functions: A => B and C => D unify if A unifies with C and B unifies with D.
        (Type::Function { params: p1, return_type: r1 }, Type::Function { params: p2, return_type: r2 })
            if p1.len() == p2.len() => {
            let mut subst = HashMap::new();
            for (ty1, ty2) in p1.iter().zip(p2.iter()) {
                let s1 = unify(&ty1.apply(&subst), &ty2.apply(&subst), span)?;
                subst = compose(&subst, &s1);
            }
            let s_ret = unify(&r1.apply(&subst), &r2.apply(&subst), span)?;
            Ok(compose(&subst, &s_ret))
        }

        // Unify Named types: Result::T::E and Result::Int::Text
        (Type::Named { name: n1, params: p1 }, Type::Named { name: n2, params: p2 })
            if n1 == n2 && p1.len() == p2.len() => {
            let mut subst = HashMap::new();
            for (ty1, ty2) in p1.iter().zip(p2.iter()) {
                let s1 = unify(&ty1.apply(&subst), &ty2.apply(&subst), span)?;
                subst = compose(&subst, &s1);
            }
            Ok(subst)
        }

        // If none of the above match, the types are incompatible.
        _ => Err(TypeError::TypeMismatch {
            expected: t1.clone(),
            found: t2.clone(),
            span: span.clone(),
        }),
    }
}

/// Helper function to bind a type variable to a type.
fn bind_var(var_name: &str, typ: &Type, span: &Span) -> Result<Substitution, TypeError> {
    // Occurs check: prevent infinite types like a = [a]
    if typ.free_type_vars().contains(var_name) {
        return Err(TypeError::InfiniteType {
            var: var_name.to_string(),
            typ: typ.clone(),
            span: span.clone(),
        });
    }

    let mut subst = HashMap::new();
    subst.insert(var_name.to_string(), typ.clone());
    Ok(subst)
}

// =============================================================================
// INSTANTIATION
// =============================================================================

/// Instantiates a generic type by replacing all its bound variables with fresh ones.
/// Example: `map :: (A => B) [A] => [B]` becomes `(t_1 => t_2) [t_1] => [t_2]`
/// This is crucial when calling a generic function so we don't accidentally link
/// two independent calls to the same generic function.
pub fn instantiate(typ: &Type) -> Type {
    let mut subst = HashMap::new();
    let free_vars = typ.free_type_vars();

    // For every generic variable (e.g., 'A', 'T'), generate a fresh one (e.g., 't_1').
    for var in free_vars {
        // In Kata, we convention that generic type variables start with uppercase (e.g., T).
        // Fresh variables start with 't_'. We only replace the uppercase generics.
        if var.chars().next().unwrap().is_ascii_uppercase() {
            subst.insert(var, fresh_type());
        }
    }

    typ.apply(&subst)
}
