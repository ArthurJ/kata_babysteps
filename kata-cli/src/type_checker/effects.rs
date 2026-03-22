//! Domain effect validator (Pure vs Impure separation)
//!
//! This module traverses the Typed AST (TAST) to ensure that no pure function
//! contains any calls to or references to impure actions.

use crate::tast::decl::{DeclKind, TypedDecl, TypedFunctionDef, TypedInterfaceMember};
use crate::tast::expr::{ExprKind, TypedExpr, TypedLambdaClause};
use crate::type_checker::error::TypeError;

/// Entry point to audit the effects of a module.
/// It iterates over all declarations and enforces purity rules.
pub fn audit_effects(decls: &[TypedDecl]) -> Result<(), TypeError> {
    for decl in decls {
        match &decl.kind {
            DeclKind::Function(func) => {
                audit_function_def(func)?;
            }
            DeclKind::Interface(interface_def) => {
                for member in &interface_def.members {
                    if let TypedInterfaceMember::FunctionDef(func) = member {
                        audit_function_def(func)?;
                    }
                }
            }
            DeclKind::Implements(impl_def) => {
                for func in &impl_def.implementations {
                    audit_function_def(func)?;
                }
            }
            _ => {}
        }
    }
    Ok(())
}

fn audit_function_def(func: &TypedFunctionDef) -> Result<(), TypeError> {
    for clause in &func.clauses {
        audit_lambda_clause(clause)?;
    }
    Ok(())
}

fn audit_lambda_clause(clause: &TypedLambdaClause) -> Result<(), TypeError> {
    if let Some(body) = &clause.body {
        audit_expr(body)?;
    }

    for guard in &clause.guards {
        audit_expr(&guard.body)?;
    }

    for with in &clause.with {
        audit_expr(&with.value)?;
    }

    Ok(())
}

fn audit_expr(expr: &TypedExpr) -> Result<(), TypeError> {
    match &expr.kind {
        ExprKind::Var(ident) => {
            if ident.is_action() {
                return Err(TypeError::ImpureCallInPureContext {
                    action_name: ident.0.clone(),
                    span: expr.span,
                });
            }
        }
        ExprKind::QualifiedRef(qident) => {
            if qident.name.ends_with('!') {
                return Err(TypeError::ImpureCallInPureContext {
                    action_name: qident.to_string(),
                    span: expr.span,
                });
            }
        }
        ExprKind::Tuple(exprs)
        | ExprKind::List(exprs)
        | ExprKind::Array(exprs)
        | ExprKind::Block(exprs) => {
            for e in exprs {
                audit_expr(e)?;
            }
        }
        ExprKind::Cons { head, tail } => {
            audit_expr(head)?;
            audit_expr(tail)?;
        }
        ExprKind::Tensor { elements, .. } => {
            for e in elements {
                audit_expr(e)?;
            }
        }
        ExprKind::Range {
            start, end, step, ..
        } => {
            audit_expr(start)?;
            audit_expr(end)?;
            if let Some(s) = step {
                audit_expr(s)?;
            }
        }
        ExprKind::Dict(pairs) => {
            for (k, v) in pairs {
                audit_expr(k)?;
                audit_expr(v)?;
            }
        }
        ExprKind::Set(exprs) => {
            for e in exprs {
                audit_expr(e)?;
            }
        }
        ExprKind::Apply { func, args } | ExprKind::ExplicitApply { func, args } => {
            audit_expr(func)?;
            for arg in args {
                audit_expr(arg)?;
            }
        }
        ExprKind::Method { object, method, args } => {
            audit_expr(object)?;
            if method.is_action() {
                return Err(TypeError::ImpureCallInPureContext {
                    action_name: method.0.clone(),
                    span: expr.span,
                });
            }
            for arg in args {
                audit_expr(arg)?;
            }
        }
        ExprKind::Field { object, .. } => {
            audit_expr(object)?;
        }
        ExprKind::Index { object, index } => {
            audit_expr(object)?;
            audit_expr(index)?;
        }
        ExprKind::Lambda { clauses } => {
            for clause in clauses {
                audit_lambda_clause(clause)?;
            }
        }
        ExprKind::Pipeline { value, func } => {
            audit_expr(value)?;
            audit_expr(func)?;
        }
        ExprKind::TypeCast { value, .. } => {
            audit_expr(value)?;
        }
        ExprKind::WithBlock { body, bindings } => {
            audit_expr(body)?;
            for b in bindings {
                audit_expr(&b.value)?;
            }
        }
        ExprKind::Literal(_) | ExprKind::Hole => {
            // Safe, no operations.
        }
    }

    Ok(())
}
