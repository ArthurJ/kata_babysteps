//! Interface Coherence Rule Checker (Orphan Rules)

use crate::type_checker::environment::Environment;
use crate::type_checker::error::TypeError;
use crate::ast::decl::ImplDef;
use crate::lexer::Span;
use crate::type_checker::inference::{unify, instantiate, Substitutable};

/// Checks the Orphan Rule for an implementation.
///
/// Rule: To implement an Interface for a Type, at least one of the two
/// (the Interface or the Type) must have been defined in the current module.
pub fn check_orphan_rule(env: &Environment, impl_def: &ImplDef, span: Span) -> Result<(), TypeError> {
    let type_name = &impl_def.type_name.name;
    let interface_name = &impl_def.interface.0;

    // A type is local if it was registered as local in the environment
    let type_is_local = env.is_local_type(type_name);
    
    // An interface is local if it was registered as local in the environment
    let interface_is_local = env.is_local_interface(interface_name);

    // If both are foreign, it's an Orphan Rule violation
    if !type_is_local && !interface_is_local {
        return Err(TypeError::OrphanRuleViolation {
            type_name: type_name.clone(),
            interface_name: interface_name.clone(),
            span,
        });
    }

    Ok(())
}

/// Validates that an implementation satisfies the interface contract.
/// This includes checking that all required members are implemented
/// and that their signatures match.
pub fn validate_interface_impl(env: &Environment, impl_def: &ImplDef, span: Span) -> Result<(), TypeError> {
    let interface_name = &impl_def.interface.0;
    
    let interface_info = env.interfaces.get(interface_name).ok_or_else(|| {
        TypeError::UnboundVariable {
            name: format!("Interface `{}`", interface_name),
            span,
        }
    })?;

    // Check if all members of the interface are implemented
    for (member_name, expected_sig) in &interface_info.members {
        let implementation = impl_def.implementations.iter().find(|f| f.name.0 == *member_name);
        
        match implementation {
            Some(f) => {
                // Validate signature matches
                // We'll instantiate the expected signature and try to unify it with the implementation.
                // Substitute the interface name (Self) AND all generic parameters with the implementing type BEFORE instantiating!
                let mut self_subst = std::collections::HashMap::new();
                self_subst.insert(interface_name.clone(), crate::ast::types::Type::named(&impl_def.type_name.name));
                
                // Also find all free generic variables in the expected signature and map them to the implementor type
                // In Kata, interfaces like HASH use 'hash :: A => Text' where A is the implementing type.
                for p in &expected_sig.params {
                    for var in p.free_type_vars() {
                        if crate::type_checker::inference::is_generic_name(&var) {
                            self_subst.insert(var, crate::ast::types::Type::named(&impl_def.type_name.name));
                        }
                    }
                }
                for var in expected_sig.return_type.free_type_vars() {
                    if crate::type_checker::inference::is_generic_name(&var) {
                        self_subst.insert(var, crate::ast::types::Type::named(&impl_def.type_name.name));
                    }
                }
                
                let expected_sig_subst = crate::ast::types::FunctionSig {
                    params: expected_sig.params.iter().map(|t| {
                        use crate::type_checker::inference::Substitutable;
                        t.apply(&self_subst)
                    }).collect(),
                    return_type: {
                        use crate::type_checker::inference::Substitutable;
                        expected_sig.return_type.apply(&self_subst)
                    },
                };

                let inst_expected = crate::ast::types::FunctionSig {
                    params: expected_sig_subst.params.iter().map(|t| instantiate(t)).collect(),
                    return_type: instantiate(&expected_sig_subst.return_type),
                };

                // Check arity first
                if f.sig.params.len() != inst_expected.params.len() {
                    return Err(TypeError::UnboundVariable {
                        name: format!("Arity Mismatch in member `{}`: expected {}, but found {}", member_name, inst_expected.params.len(), f.sig.params.len()),
                        span,
                    });
                }

                let mut subst = std::collections::HashMap::new();
                // Try to unify params and return type
                for (p1, p2) in f.sig.params.iter().zip(inst_expected.params.iter()) {
                    match unify(&p1.apply(&subst), &p2.apply(&subst), env, &span) {
                        Ok(s) => subst = crate::type_checker::inference::compose(&subst, &s),
                        Err(_) => return Err(TypeError::TypeMismatch {
                            expected: p2.clone(),
                            found: p1.clone(),
                            span,
                        }),
                    }
                }

                if let Err(_) = unify(&f.sig.return_type.apply(&subst), &inst_expected.return_type.apply(&subst), env, &span) {
                    return Err(TypeError::TypeMismatch {
                        expected: inst_expected.return_type.clone(),
                        found: f.sig.return_type.clone(),
                        span,
                    });
                }
            }
            None => {
                // Member not implemented and no default implementation?
                return Err(TypeError::UnboundVariable {
                    name: format!("Member `{}` of interface `{}`", member_name, interface_name),
                    span,
                });
            }
        }
    }

    Ok(())
}
