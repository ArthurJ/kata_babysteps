//! Core Type Checker (AST -> TAST converter)
//!
//! The Checker is responsible for validating the AST, performing type inference,
//! enforcing domain separation (pure vs impure), and producing the Typed AST (TAST).

use std::collections::HashMap;

use crate::ast::decl::{TopLevel, FunctionDef, ActionDef};
use crate::ast::expr::Expr;
use crate::ast::stmt::Stmt;
use crate::ast::types::Type;
use crate::lexer::Span;

use crate::tast::expr::{TypedExpr, ExprKind};
use crate::tast::stmt::{TypedStmt, StmtKind};
use crate::tast::decl::{TypedDecl, DeclKind, TypedFunctionDef, TypedActionDef};

use crate::type_checker::environment::Environment;
use crate::type_checker::inference::{unify, instantiate, Substitution, Substitutable};
use crate::type_checker::error::TypeError;
use crate::ast::types::FunctionSig;
use crate::ast::id::Ident;

/// A type constraint to be validated after inference
#[derive(Debug, Clone)]
pub enum Constraint {
    /// name :: Sig
    Signature(Ident, FunctionSig, Span),
    /// Type implements Interface
    Interface(Type, Ident, Span),
}

/// The main Type Checker context
pub struct Checker {
    pub env: Environment,
    
    /// Maps each type variable to its current substitution
    pub substitutions: Substitution,
    
    /// Keeps track of whether we are currently inside an impure Action context
    pub in_action_context: bool,

    /// Constraints collected from 'with' blocks
    pub constraints: Vec<Constraint>,
}

impl Checker {
    pub fn new() -> Self {
        Checker {
            env: Environment::new(),
            substitutions: HashMap::new(),
            in_action_context: false,
            constraints: Vec::new(),
        }
    }

    /// Resolves a prefix application by consuming atoms according to arity.
    /// Returns the typed expression and the remaining unconsumed atoms.
    fn resolve_prefix_apply(&mut self, atoms: &mut Vec<crate::ast::Spanned<Expr>>) -> Result<(TypedExpr, Vec<crate::ast::Spanned<Expr>>), TypeError> {
        if atoms.is_empty() {
            return Err(TypeError::UnboundVariable {
                name: "Unexpected empty application".to_string(),
                span: Span { start: 0, end: 0 },
            });
        }

        let first = atoms.remove(0);
        let span = first.span.clone();

        // 1. If it's a variable, it might be a function with a known arity
        if let Expr::Var { name, .. } = &first.node {
            // We clone the signatures to avoid borrowing self.env while calling self.resolve_prefix_apply
            if let Some(sigs) = self.env.lookup_dispatch(&name.0).cloned() {
                // For simplicity, we assume all signatures for a function have the same arity
                let arity = sigs[0].params.len();
                
                let mut typed_args = Vec::new();
                for _ in 0..arity {
                    if atoms.is_empty() {
                        return Err(TypeError::UnboundVariable {
                            name: format!("Function `{}` expects {} arguments, but only {} were provided. Use '_' for partial application.", name, arity, typed_args.len()),
                            span: span,
                        });
                    }
                    let (arg_expr, _) = self.resolve_prefix_apply(atoms)?;
                    typed_args.push(arg_expr);
                }

                // Now find the signature that unifies with the arguments with the lowest distance score
                let mut best_match = None;
                let mut min_score = usize::MAX;
                let mut ambiguous = false;

                for sig in &sigs {
                    let inst_sig = FunctionSig {
                        params: sig.params.iter().map(|t| instantiate(t)).collect(),
                        return_type: instantiate(&sig.return_type),
                    };

                    let mut temp_subst = self.substitutions.clone();
                    let mut possible = true;
                    let mut current_score = 0;

                    for (p_formal, p_actual) in inst_sig.params.iter().zip(typed_args.iter()) {
                        if let Ok((s, score)) = unify(p_formal, &p_actual.typ, &self.env, &span) {
                            temp_subst = crate::type_checker::inference::compose(&temp_subst, &s);
                            current_score += score;
                        } else {
                            possible = false;
                            break;
                        }
                    }

                    if possible {
                        if current_score < min_score {
                            min_score = current_score;
                            best_match = Some((inst_sig, temp_subst));
                            ambiguous = false;
                        } else if current_score == min_score {
                            // Only mark as ambiguous if they don't point to the same underlying commutative block
                            // (We don't track AST pointers here yet, but we will ignore the error for now as a simple heuristic)
                            // In a full implementation, we'd check if `sig` and `best_match` came from the same @comutative declaration.
                            // ambiguous = true;
                        }
                    }
                }

                if ambiguous {
                    return Err(TypeError::UnboundVariable {
                        name: format!("Ambiguous Dispatch for `{}` with given arguments. Multiple signatures have the same distance score.", name.0),
                        span: span,
                    });
                }

                if let Some((inst_sig, matched_subst)) = best_match {
                    self.substitutions = matched_subst;
                    let final_ret_type = inst_sig.return_type.apply(&self.substitutions);
                    let typed_func = TypedExpr {
                        kind: ExprKind::Var(name.clone()),
                        typ: Type::function(inst_sig.params, inst_sig.return_type),
                        span: span.clone(),
                    };

                    return Ok((TypedExpr {
                        kind: ExprKind::Apply { func: Box::new(typed_func), args: typed_args },
                        typ: final_ret_type,
                        span: span,
                    }, atoms.clone()));
                } else {
                    let arg_types: Vec<String> = typed_args.iter().map(|a| format!("{}", a.typ)).collect();
                    return Err(TypeError::NoMatchingDispatch {
                        func_name: name.0.clone(),
                        args: typed_args.into_iter().map(|a| a.typ).collect(),
                        span: span,
                    });
                }
            }
        }

        // 2. If it's not a known function name, just check it as a simple expression
        let typed_expr = self.check_expr(first)?;
        Ok((typed_expr, atoms.clone()))
    }

    /// Validates all collected constraints against the final substitutions.
    fn validate_constraints(&mut self) -> Result<(), TypeError> {
        let constraints = std::mem::take(&mut self.constraints);
        
        for constraint in constraints {
            match constraint {
                Constraint::Signature(name, sig, span) => {
                    // Apply current substitutions to the signature
                    let applied_sig = FunctionSig {
                        params: sig.params.iter().map(|t| t.apply(&self.substitutions)).collect(),
                        return_type: sig.return_type.apply(&self.substitutions),
                    };
                    
                    // Check if there is an implementation that matches this signature
                    if let Some(available_sigs) = self.env.lookup_dispatch(&name.0) {
                        let mut matched = false;
                        for avail in available_sigs {
                            // If we can unify the required sig with an available one, the constraint is satisfied
                            // Note: we instantiate the available sig to allow it to be more generic than the constraint
                            let inst_avail = FunctionSig {
                                params: avail.params.iter().map(|t| instantiate(t)).collect(),
                                return_type: instantiate(&avail.return_type),
                            };
                            
                            // Check arity
                            if inst_avail.params.len() != applied_sig.params.len() {
                                continue;
                            }

                            // Temporary state to try unification
                            let mut temp_subst = self.substitutions.clone();
                            let mut possible = true;
                            for (p1, p2) in applied_sig.params.iter().zip(inst_avail.params.iter()) {
                                if let Ok((s, _)) = unify(p1, p2, &self.env, &span) {
                                    temp_subst = crate::type_checker::inference::compose(&temp_subst, &s);
                                } else {
                                    possible = false;
                                    break;
                                }
                            }
                            
                            if possible {
                                if let Ok((_s, _)) = unify(&applied_sig.return_type, &inst_avail.return_type, &self.env, &span) {
                                    matched = true;
                                    // We don't necessarily want to commit the unification results back to self.substitutions
                                    // because the constraint is just a check that *some* implementation exists.
                                    // But actually, for generics, we might want to!
                                    // For now, let's just mark as matched.
                                    break;
                                }
                            }
                        }
                        
                        if !matched {
                            return Err(TypeError::UnboundVariable {
                                name: format!("Constraint Mismatch: No implementation found for `{} :: {}`", name, applied_sig),
                                span,
                            });
                        }
                    } else {
                        return Err(TypeError::UnboundVariable {
                            name: format!("Constraint Mismatch: Function `{}` not found", name),
                            span,
                        });
                    }
                }
                Constraint::Interface(typ, interface, span) => {
                    let applied_typ = typ.apply(&self.substitutions);
                    
                    if self.env.satisfies_interface(&applied_typ, &interface.0).is_none() {
                        return Err(TypeError::UnboundVariable {
                            name: format!("Type `{}` does not satisfy interface `{}` (missing one or more members)", applied_typ, interface),
                            span,
                        });
                    }
                }
            }
        }
        
        Ok(())
    }

    /// Entry point for type checking a whole module.
    /// Takes a list of top-level declarations that have already been
    /// topologically sorted and tree-shaken by the DAG.
    pub fn check_module(&mut self, sorted_decls: Vec<crate::ast::Spanned<TopLevel>>) -> Result<Vec<TypedDecl>, TypeError> {
        let mut typed_decls = Vec::new();

        // First pass: Register all type definitions and function signatures in the global environment
        // so that mutually recursive functions or out-of-order uses within the sorted components work.
        for decl in &sorted_decls {
            self.register_global_signature(&decl.node)?;
        }

        // Second pass: Actually type check the bodies
        for decl in sorted_decls {
            let typed_decl = self.check_top_level(decl)?;
            typed_decls.push(typed_decl);
        }

        Ok(typed_decls)
    }

    /// Registers signatures without checking bodies
    fn register_global_signature(&mut self, decl: &TopLevel) -> Result<(), TypeError> {
        match decl {
            TopLevel::Function(f) => {
                // Register the function's signature in the global dispatch table
                self.env.register_dispatch(&f.name.0, f.sig.clone());
            }
            TopLevel::Action(a) => {
                // Actions are also callable, but only from other actions
                let ret_type = a.return_type.clone().unwrap_or_else(|| Type::named("Unit"));
                // For actions, we create a pseudo-signature
                let params = a.params.iter().map(|_| crate::type_checker::inference::fresh_type()).collect();
                let sig = crate::ast::types::FunctionSig::new(params, ret_type);
                
                // Actions MUST be called with '!'
                let mut name_with_bang = a.name.0.clone();
                if !name_with_bang.ends_with('!') {
                    name_with_bang.push('!');
                }
                self.env.register_dispatch(&name_with_bang, sig);
            }
            TopLevel::Data(d) => {
                // Register the type itself
                let typ = if d.type_params.is_empty() {
                    Type::named(&d.name.0)
                } else {
                    let params = d.type_params.iter().map(|p| Type::var(&p.0)).collect();
                    Type::generic(&d.name.0, params)
                };
                self.env.register_type(&d.name.0, typ, true);
            }
            TopLevel::Enum(e) => {
                let typ = if e.type_params.is_empty() {
                    Type::named(&e.name.0)
                } else {
                    let params = e.type_params.iter().map(|p| Type::var(&p.0)).collect();
                    Type::generic(&e.name.0, params)
                };
                self.env.register_type(&e.name.0, typ, true);
            }
            TopLevel::Interface(i) => {
                let mut members = HashMap::new();
                for member in &i.members {
                    match member {
                        crate::ast::decl::InterfaceMember::Signature(name, sig) => {
                            members.insert(name.0.clone(), sig.clone());
                            self.env.register_dispatch(&name.0, sig.clone());
                        }
                        crate::ast::decl::InterfaceMember::FunctionDef(f) => {
                            members.insert(f.name.0.clone(), f.sig.clone());
                            self.env.register_dispatch(&f.name.0, f.sig.clone());
                        }
                    }
                }
                let info = crate::type_checker::environment::InterfaceInfo {
                    name: i.name.0.clone(),
                    extends: i.extends.iter().map(|id| id.0.clone()).collect(),
                    members,
                };
                self.env.register_interface(info, true);
            }
            TopLevel::Implements(impl_def) => {
                // Register the nominal subtyping explicitly for this interface and all its parents!
                let type_name = &impl_def.type_name.name;
                let interface_name = &impl_def.interface.0;
                self.env.register_implementation(type_name, interface_name);
                
                // Helper closure to register parents (simulated with a loop or we can just fetch all parents)
                // Since we don't have an easy recursive closure here, we can do it iteratively
                let mut queue = vec![interface_name.clone()];
                while let Some(current_iface) = queue.pop() {
                    self.env.register_implementation(type_name, &current_iface);
                    if let Some(info) = self.env.interfaces.get(&current_iface) {
                        for parent in &info.extends {
                            queue.push(parent.clone());
                        }
                    }
                }

                // Register all functions in the implementation for multiple dispatch
                for f in &impl_def.implementations {
                    self.env.register_dispatch(&f.name.0, f.sig.clone());
                }
            }
            TopLevel::Import(_) => {}
            TopLevel::Export(_) => {}
            _ => {}
        }
        Ok(())
    }

    // =========================================================================
    // TOP-LEVEL DECLARATIONS
    // =========================================================================

    fn check_top_level(&mut self, spanned_decl: crate::ast::Spanned<TopLevel>) -> Result<TypedDecl, TypeError> {
        let decl = spanned_decl.node;
        let span = spanned_decl.span;
        // For now, we use a dummy span for top-level nodes since the AST doesn't
        // currently store spans on declarations. In a real compiler, the parser
        // would attach spans to `TopLevel` variants.
        // let span = Span { start: 0, end: 0 };

        let kind = match decl {
            TopLevel::Function(f) => DeclKind::Function(self.check_function_def(f)?),
            TopLevel::Action(a) => DeclKind::Action(self.check_action_def(a)?),
            TopLevel::Data(d) => DeclKind::Data(self.check_data_def(d)?),
            TopLevel::Enum(e) => DeclKind::Enum(self.check_enum_def(e)?),
            TopLevel::Interface(i) => DeclKind::Interface(self.check_interface_def(i)?),
            TopLevel::Implements(impl_def) => DeclKind::Implements(self.check_implements(impl_def, span)?),
            TopLevel::Alias(a) => DeclKind::Alias(self.check_alias_def(a)?),
            TopLevel::Statement(s) => {
                // Statements in top-level are treated as being in an "entry-point" action context.
                // This allows `main!` calls at the end of the file.
                self.in_action_context = true;
                let typed_stmt = self.check_action_stmt(crate::ast::Spanned::new(s, span.clone()))?;
                self.in_action_context = false;
                DeclKind::Statement(typed_stmt)
            }
            TopLevel::Import(i) => DeclKind::Import(i),
            TopLevel::Export(e) => DeclKind::Export(e.items.into_iter().map(|id| id.0).collect()),
        };

        Ok(TypedDecl { kind, span: span })
    }

    fn check_data_def(&mut self, d: crate::ast::decl::DataDef) -> Result<crate::tast::decl::TypedDataDef, TypeError> {
        let mut fields = Vec::new();
        if let crate::ast::decl::DataKind::Product(ast_fields) = d.kind {
            for f in ast_fields {
                fields.push(crate::tast::decl::TypedFieldDef {
                    name: f.name,
                    typ: f.type_annotation.unwrap_or_else(|| Type::named("Any")), // Placeholder
                });
            }
        }
        
        Ok(crate::tast::decl::TypedDataDef {
            name: d.name,
            type_params: d.type_params,
            fields,
        })
    }

    fn check_interface_def(&mut self, i: crate::ast::decl::InterfaceDef) -> Result<crate::tast::decl::TypedInterfaceDef, TypeError> {
        let mut typed_members = Vec::new();
        for member in i.members {
            match member {
                crate::ast::decl::InterfaceMember::Signature(name, sig) => {
                    typed_members.push(crate::tast::decl::TypedInterfaceMember::Signature(name, sig));
                }
                crate::ast::decl::InterfaceMember::FunctionDef(f) => {
                    typed_members.push(crate::tast::decl::TypedInterfaceMember::FunctionDef(self.check_function_def(f)?));
                }
            }
        }

        Ok(crate::tast::decl::TypedInterfaceDef {
            name: i.name,
            extends: i.extends,
            members: typed_members,
        })
    }

    fn check_implements(&mut self, impl_def: crate::ast::decl::ImplDef, span: Span) -> Result<crate::tast::decl::TypedImplDef, TypeError> {
        // 1. Check Orphan Rule
        crate::type_checker::interfaces::check_orphan_rule(&self.env, &impl_def, span)?;

        // 2. Validate interface implementation contract
        crate::type_checker::interfaces::validate_interface_impl(&self.env, &impl_def, span)?;

        // 3. Type check each function implementation body
        let mut typed_impls = Vec::new();
        for f in impl_def.implementations {
            typed_impls.push(self.check_function_def(f)?);
        }

        Ok(crate::tast::decl::TypedImplDef {
            type_name: Type::named(impl_def.type_name.name),
            interface: impl_def.interface,
            implementations: typed_impls,
        })
    }

    fn check_enum_def(&mut self, e: crate::ast::decl::EnumDef) -> Result<crate::tast::decl::TypedEnumDef, TypeError> {
        let mut typed_variants = Vec::new();
        for v in e.variants {
            let payload = match v.payload {
                crate::ast::decl::VariantPayload::Unit => crate::tast::decl::TypedVariantPayload::Unit,
                crate::ast::decl::VariantPayload::Typed(t) => crate::tast::decl::TypedVariantPayload::Typed(t),
                _ => crate::tast::decl::TypedVariantPayload::Unit, // Placeholder for FixedValue/Predicated
            };
            typed_variants.push(crate::tast::decl::TypedVariantDef {
                name: v.name,
                payload,
            });
        }

        Ok(crate::tast::decl::TypedEnumDef {
            name: e.name,
            type_params: e.type_params,
            variants: typed_variants,
        })
    }

    fn check_alias_def(&mut self, a: crate::ast::decl::AliasDef) -> Result<crate::tast::decl::TypedAliasDef, TypeError> {
        Ok(crate::tast::decl::TypedAliasDef {
            name: a.name,
            target: a.target,
        })
    }

    fn check_function_def(&mut self, func: FunctionDef) -> Result<TypedFunctionDef, TypeError> {
        log::debug!("Checking function definition: {}", func.name.0);
        // Pure functions cannot have side effects
        self.in_action_context = false;
        
        self.env.enter_scope();

        // 1. Bind parameters to their types from the signature
        // In Kata, the lambda clauses actually bind the names, but the signature
        // dictates their expected types.
        
        let mut typed_clauses = Vec::new();
        
        for clause in func.clauses {
            self.env.enter_scope();
            
            let mut patterns = clause.patterns.clone();
            
            // If patterns are empty, fill with wildcards (handles 'otherwise:')
            if patterns.is_empty() {
                for _ in 0..func.sig.params.len() {
                    patterns.push(crate::ast::Spanned::new(crate::ast::pattern::Pattern::Wildcard, crate::lexer::Span { start: 0, end: 0 }));
                }
            }

            // Check patterns against the signature parameters
            if patterns.len() != func.sig.params.len() {
                return Err(TypeError::UnboundVariable {
                    name: format!("Arity Mismatch in function `{}`: expected {}, but found {}", func.name.0, func.sig.params.len(), patterns.len()),
                    span: Span { start: 0, end: 0 },
                });
            }

            for (pattern, param_type) in patterns.iter().zip(func.sig.params.iter()) {
                if let Err(e) = self.check_pattern(pattern, param_type, &Span { start: 0, end: 0 }) {
                    if let TypeError::TypeMismatch { expected, found, span } = e {
                        return Err(TypeError::UnboundVariable {
                            name: format!("Type Mismatch in function `{}` pattern: expected `{}`, but found `{}`", func.name.0, expected, found),
                            span,
                        });
                    }
                    return Err(e);
                }
            }
            
            // 2. Process 'with' bindings (if any)
            for binding in clause.with.clone() {
                match binding {
                    crate::ast::expr::WithBinding::Value { name, value } => {
                        let typed_val = self.check_expr(value)?;
                        self.env.bind_var(&name.0, typed_val.typ);
                    }
                    crate::ast::expr::WithBinding::Signature { name, sig } => {
                        self.constraints.push(Constraint::Signature(name, sig, Span { start: 0, end: 0 }));
                    }
                    crate::ast::expr::WithBinding::Interface { typ, interface } => {
                        self.constraints.push(Constraint::Interface(typ, interface, Span { start: 0, end: 0 }));
                    }
                }
            }

            // 3. Check guards (if any)
            // TODO: Implement guard checking

            let typed_body = if let Some(body) = clause.body {
                let mut checked_body = self.check_expr(body)?;
                
                // Unify the body's inferred type with the declared return type
                let s = match unify(&checked_body.typ, &func.sig.return_type, &self.env, &checked_body.span) {
                    Ok((s, _)) => s,
                    Err(e) => {
                        if let TypeError::TypeMismatch { expected, found, span } = e {
                            return Err(TypeError::UnboundVariable {
                                name: format!("Type Mismatch in function `{}` return: expected `{}`, but found `{}`", func.name.0, expected, found),
                                span,
                            });
                        }
                        return Err(e);
                    }
                };
                self.substitutions = crate::type_checker::inference::compose(&self.substitutions, &s);
                
                // Apply the final substitutions to the body's type
                checked_body.typ = checked_body.typ.apply(&self.substitutions);
                Some(checked_body)
            } else {
                None
            };
            
            // 4. Validate all constraints for this clause
            self.validate_constraints()?;

            typed_clauses.push(crate::tast::expr::TypedLambdaClause {
                patterns: patterns.into_iter().map(|p| p.node).collect(),
                guards: vec![],
                body: typed_body,
                with: vec![], // TODO: TAST should probably store typed with bindings
            });
            
            self.env.exit_scope();
        }

        self.env.exit_scope();

        Ok(TypedFunctionDef {
            name: func.name,
            sig: func.sig,
            arity: func.arity,
            directives: func.directives,
            clauses: typed_clauses,
        })
    }

    fn check_action_def(&mut self, action: ActionDef) -> Result<TypedActionDef, TypeError> {
        // Actions CAN have side effects
        self.in_action_context = true;
        
        self.env.enter_scope();

        let return_type = action.return_type.unwrap_or_else(|| Type::named("Unit"));

        // Bind parameters to fresh type variables (since actions often don't have explicit sigs)
        for param in &action.params {
            self.env.bind_var(&param.0, crate::type_checker::inference::fresh_type());
        }

        let mut typed_body = Vec::new();
        for stmt in action.body {
            typed_body.push(self.check_action_stmt(stmt)?);
        }

        // TODO: Verify that if the action returns a value, the last statement matches the return type.

        self.env.exit_scope();
        self.in_action_context = false;

        Ok(TypedActionDef {
            name: action.name,
            params: action.params,
            return_type,
            directives: action.directives,
            body: typed_body,
        })
    }

    // =========================================================================
    // PATTERN MATCHING
    // =========================================================================

    /// Checks a pattern against an expected type, binding extracted variables to the environment.
    /// Returns the type that the pattern actually matches (usually the same as expected_type, 
    /// but unified/instantiated if expected_type had generics).
    fn check_pattern(&mut self, spanned_pattern: &crate::ast::Spanned<crate::ast::pattern::Pattern>, expected_type: &Type, dummy_span: &Span) -> Result<Type, TypeError> {
        let span = &spanned_pattern.span;
        let pattern = &spanned_pattern.node;
        match pattern {
            crate::ast::pattern::Pattern::Wildcard => {
                Ok(expected_type.clone())
            }
            crate::ast::pattern::Pattern::Literal(lit) => {
                let lit_type = match lit {
                    crate::ast::id::Literal::Int(_) => Type::named("Int"),
                    crate::ast::id::Literal::Float(_) => Type::named("Float"),
                    crate::ast::id::Literal::String(_) => Type::named("Text"),
                    crate::ast::id::Literal::Bool(_) => Type::named("Bool"),
                    crate::ast::id::Literal::Bytes(_) => Type::named("Bytes"),
                    crate::ast::id::Literal::Unit => Type::named("Unit"),
                };
                
                let (s, _) = unify(&lit_type, expected_type, &self.env, span)?;
                self.substitutions = crate::type_checker::inference::compose(&self.substitutions, &s);
                Ok(lit_type.apply(&self.substitutions))
            }
            crate::ast::pattern::Pattern::Var(id) => {
                // Bind the variable to the expected type in the current environment
                self.env.bind_var(&id.0, expected_type.clone());
                Ok(expected_type.clone())
            }
            crate::ast::pattern::Pattern::Tuple(patterns) => {
                // If the expected type is a tuple, verify lengths and check each element
                if let Type::Tuple(expected_types) = expected_type {
                    if patterns.len() != expected_types.len() {
                        return Err(TypeError::TypeMismatch {
                            expected: expected_type.clone(),
                            found: Type::tuple(patterns.iter().map(|_| crate::type_checker::inference::fresh_type()).collect()),
                            span: span.clone(),
                        });
                    }

                    let mut matched_types = Vec::new();
                    for (p, t) in patterns.iter().zip(expected_types.iter()) {
                        matched_types.push(self.check_pattern(p, t, span)?);
                    }
                    Ok(Type::tuple(matched_types))
                } else {
                    // Try to unify expected_type with a generic tuple type
                    let fresh_tuple = Type::tuple(patterns.iter().map(|_| crate::type_checker::inference::fresh_type()).collect());
                    let (s, _) = unify(&fresh_tuple, expected_type, &self.env, span)?;
                    self.substitutions = crate::type_checker::inference::compose(&self.substitutions, &s);
                    
                    let resolved_tuple = fresh_tuple.apply(&self.substitutions);
                    if let Type::Tuple(resolved_types) = resolved_tuple {
                        let mut matched_types = Vec::new();
                        for (p, t) in patterns.iter().zip(resolved_types.iter()) {
                            matched_types.push(self.check_pattern(p, t, span)?);
                        }
                        Ok(Type::tuple(matched_types))
                    } else {
                        unreachable!()
                    }
                }
            }
            crate::ast::pattern::Pattern::List { elements, rest } => {
                // Determine the element type of the list
                let elem_type = if let Type::Named { name, params } = expected_type {
                    if name.name == "List" && params.len() == 1 {
                        params[0].clone()
                    } else {
                        crate::type_checker::inference::fresh_type()
                    }
                } else {
                    crate::type_checker::inference::fresh_type()
                };

                let mut matched_elem_type = elem_type.clone();
                for p in elements {
                    let t = self.check_pattern(p, &matched_elem_type, span)?;
                    let (s, _) = unify(&t, &matched_elem_type, &self.env, span)?;
                    self.substitutions = crate::type_checker::inference::compose(&self.substitutions, &s);
                    matched_elem_type = matched_elem_type.apply(&self.substitutions);
                }

                let list_type = Type::generic("List", vec![matched_elem_type]);
                let (s, _) = unify(&list_type, expected_type, &self.env, span)?;
                self.substitutions = crate::type_checker::inference::compose(&self.substitutions, &s);
                
                if let Some(r) = rest {
                    self.check_pattern(r, &list_type.apply(&self.substitutions), span)?;
                }

                Ok(list_type.apply(&self.substitutions))
            }
            crate::ast::pattern::Pattern::Cons { head, tail } => {
                let elem_type = crate::type_checker::inference::fresh_type();
                let list_type = Type::generic("List", vec![elem_type.clone()]);
                
                let (s, _) = unify(&list_type, expected_type, &self.env, span)?;
                self.substitutions = crate::type_checker::inference::compose(&self.substitutions, &s);
                
                let resolved_elem = elem_type.apply(&self.substitutions);
                let resolved_list = list_type.apply(&self.substitutions);
                
                self.check_pattern(head, &resolved_elem, span)?;
                self.check_pattern(tail, &resolved_list, span)?;
                
                Ok(resolved_list)
            }
            crate::ast::pattern::Pattern::Variant { name: _, args } => {
                // TODO: Full Enum variant resolution requires looking up the enum definition
                // For now, we'll do a placeholder check where we create fresh types for args
                let mut dummy_args = Vec::new();
                for p in args {
                    let fresh = crate::type_checker::inference::fresh_type();
                    dummy_args.push(self.check_pattern(p, &fresh, span)?);
                }
                Ok(expected_type.clone())
            }
            // ... Range, As (binding patterns)
            _ => Err(TypeError::UnboundVariable { 
                name: "Unimplemented pattern check".to_string(), 
                span: span.clone() 
            }),
        }
    }

    // =========================================================================
    // STATEMENTS (Action Context Only)
    // =========================================================================

    fn check_action_stmt(&mut self, spanned_stmt: crate::ast::Spanned<Stmt>) -> Result<TypedStmt, TypeError> {
        let span = spanned_stmt.span;
        let stmt = spanned_stmt.node;
        if !self.in_action_context {
            // This shouldn't happen structurally if the parser is correct,
            // but we double-check that statements only appear in actions.
            panic!("Compiler Bug: Found statement outside of an action context.");
        }

        // let span = Span { start: 0, end: 0 };

        match stmt {
            Stmt::Expr(expr) => {
                let typed_expr = self.check_expr(expr)?;
                Ok(TypedStmt {
                    kind: StmtKind::Expr(typed_expr),
                    span: span,
                })
            }
            Stmt::Let { pattern, value } => {
                let typed_value = self.check_expr(value)?;

                if let crate::ast::pattern::Pattern::Var(id) = &pattern.node {                    self.env.bind_var(&id.0, typed_value.typ.clone());
                } else {
                    // TODO: Handle complex pattern destructurings
                }

                Ok(TypedStmt {
                    kind: StmtKind::Let {
                        pattern: pattern.node,
                        value: typed_value,
                    },
                    span: span,
                })
            }
            Stmt::Var { pattern, value } => {
                let typed_value = self.check_expr(value)?;

                if let crate::ast::pattern::Pattern::Var(id) = &pattern.node {                    self.env.bind_var(&id.0, typed_value.typ.clone());
                }

                Ok(TypedStmt {
                    kind: StmtKind::Var {
                        pattern: pattern.node,
                        value: typed_value,
                    },
                    span: span,
                })
            }
            Stmt::Assign { name, value } => {
                let typed_value = self.check_expr(value)?;
                
                if let Some(expected_typ) = self.env.lookup_var(&name.0) {
                    let (s, _) = unify(&typed_value.typ, expected_typ, &self.env, &span)?;
                    self.substitutions = crate::type_checker::inference::compose(&self.substitutions, &s);
                } else {
                    return Err(TypeError::UnboundVariable { name: name.0, span: span });
                }

                Ok(TypedStmt {
                    kind: StmtKind::Assign {
                        name,
                        value: typed_value,
                    },
                    span: span,
                })
            }
            Stmt::Return(expr) => {
                let typed_expr = self.check_expr(expr)?;
                Ok(TypedStmt {
                    kind: StmtKind::Return(typed_expr),
                    span: span,
                })
            }
            Stmt::Break => Ok(TypedStmt { kind: StmtKind::Break, span: span }),
            Stmt::Continue => Ok(TypedStmt { kind: StmtKind::Continue, span: span }),
            Stmt::Loop { body } => {
                let mut typed_body = Vec::new();
                self.env.enter_scope();
                for s in body {
                    typed_body.push(self.check_action_stmt(s)?);
                }
                self.env.exit_scope();
                Ok(TypedStmt {
                    kind: StmtKind::Loop { body: typed_body },
                    span: span,
                })
            }
            Stmt::Match { value, cases } => {
                let typed_value = self.check_expr(value)?;
                let mut typed_cases = Vec::new();

                for case in cases {
                    self.env.enter_scope();
                    
                    // Check pattern and bind variables
                    self.check_pattern(&case.pattern, &typed_value.typ, &span)?;
                    
                    let mut typed_body = Vec::new();
                    for s in case.body {
                        typed_body.push(self.check_action_stmt(s)?);
                    }
                    typed_cases.push(crate::tast::stmt::TypedMatchCase {
                        pattern: case.pattern.node,
                        body: typed_body,
                    });
                    
                    self.env.exit_scope();
                }

                Ok(TypedStmt {
                    kind: StmtKind::Match {
                        value: typed_value,
                        cases: typed_cases,
                    },
                    span: span,
                })
            }
            // ... Other statements (For, Select) would be implemented here
            _ => Err(TypeError::UnboundVariable { 
                name: "Unimplemented statement check".to_string(), 
                span: span 
            }),
        }
    }

    // =========================================================================
    // EXPRESSIONS
    // =========================================================================

    fn check_expr(&mut self, spanned_expr: crate::ast::Spanned<Expr>) -> Result<TypedExpr, TypeError> {
        let span = spanned_expr.span;
        let expr = spanned_expr.node;
        // let span = Span { start: 0, end: 0 };

        match expr {
            Expr::Literal(lit) => {
                let typ = match lit {
                    crate::ast::id::Literal::Int(_) => Type::named("Int"),
                    crate::ast::id::Literal::Float(_) => Type::named("Float"),
                    crate::ast::id::Literal::String(_) => Type::named("Text"),
                    crate::ast::id::Literal::Bool(_) => Type::named("Bool"),
                    crate::ast::id::Literal::Bytes(_) => Type::named("Bytes"),
                    crate::ast::id::Literal::Unit => Type::named("Unit"),
                };
                Ok(TypedExpr {
                    kind: ExprKind::Literal(lit),
                    typ,
                    span: span,
                })
            }
            
            Expr::Var { name, type_ascription } => {
                // 1. Try to find the variable in the local environment
                if let Some(typ) = self.env.lookup_var(&name.0) {
                    let mut final_typ = typ.clone();
                    
                    // If the user provided a type ascription (e.g., `x::Int`), unify it!
                    if let Some(asc) = type_ascription {
                        let (s, _) = unify(&final_typ, &asc, &self.env, &span)?;
                        self.substitutions = crate::type_checker::inference::compose(&self.substitutions, &s);
                        final_typ = final_typ.apply(&self.substitutions);
                    }
                    
                    return Ok(TypedExpr {
                        kind: ExprKind::Var(name),
                        typ: final_typ,
                        span: span,
                    });
                }
                
                // 2. If not local, is it a global function?
                if let Some(sigs) = self.env.lookup_dispatch(&name.0) {
                    // If it's a function but we are just referencing it (not calling it directly),
                    // we need to instantiate its signature into a Function Type.
                    // For multiple dispatch, referencing a function without calling it is ambiguous
                    // unless type inference can lock it down. For now, we take the first signature.
                    let sig = &sigs[0];
                    let func_type = Type::function(sig.params.clone(), sig.return_type.clone());
                    let inst_type = instantiate(&func_type);
                    
                    return Ok(TypedExpr {
                        kind: ExprKind::Var(name),
                        typ: inst_type,
                        span: span,
                    });
                }

                // 3. Variable not found
                Err(TypeError::UnboundVariable {
                    name: name.0,
                    span: span,
                })
            }

            Expr::Apply { ref func, ref args } | Expr::ExplicitApply { ref func, ref args } => {
                let is_explicit = matches!(expr, Expr::ExplicitApply { .. });
                // If it's an ExplicitApply ($), we check it as a single unit
                if is_explicit {
                    let typed_func = self.check_expr(*func.clone())?;
                    let mut typed_args = Vec::new();
                    for arg in args {
                        typed_args.push(self.check_expr(arg.clone())?);
                    }
                    let arg_types: Vec<Type> = typed_args.iter().map(|a| a.typ.clone()).collect();
                    let ret_type = crate::type_checker::inference::fresh_type();
                    let expected_func_type = Type::function(arg_types, ret_type.clone());
                    let (s, _) = unify(&typed_func.typ, &expected_func_type, &self.env, &span)?;
                    self.substitutions = crate::type_checker::inference::compose(&self.substitutions, &s);
                    return Ok(TypedExpr {
                        kind: ExprKind::Apply { func: Box::new(typed_func), args: typed_args },
                        typ: ret_type.apply(&self.substitutions),
                        span: span,
                    });
                }

                // For implicit Apply (prefix notation), we need to resolve arity
                // We treat the current Apply as a flat sequence of atoms: [func] + args
                let mut atoms = Vec::new();
                atoms.push(*func.clone());
                atoms.extend(args.iter().cloned());

                let (typed_expr, remaining) = self.resolve_prefix_apply(&mut atoms)?;
                
                if !remaining.is_empty() {
                    // This happens if we have more arguments than the function can take
                    // e.g. (f x y) where f is arity 1.
                    // In prefix notation, this might be valid if it's curried or if
                    // the caller expects a partial application.
                    // For now, let's assume it's an error if we have leftovers in a top-level apply.
                }

                Ok(typed_expr)
            }
            
            Expr::Tuple(exprs) => {
                let mut typed_exprs = Vec::new();
                let mut types = Vec::new();
                for e in exprs {
                    let typed_e = self.check_expr(e)?;
                    types.push(typed_e.typ.clone());
                    typed_exprs.push(typed_e);
                }
                Ok(TypedExpr {
                    kind: ExprKind::Tuple(typed_exprs),
                    typ: Type::tuple(types),
                    span: span,
                })
            }
            
            Expr::List(exprs) => {
                let mut typed_exprs = Vec::new();
                let elem_type = crate::type_checker::inference::fresh_type();
                
                for e in exprs {
                    let typed_e = self.check_expr(e)?;
                    let (s, _) = unify(&typed_e.typ, &elem_type, &self.env, &span)?;
                    self.substitutions = crate::type_checker::inference::compose(&self.substitutions, &s);
                    typed_exprs.push(typed_e);
                }
                
                let final_elem_type = elem_type.apply(&self.substitutions);
                let list_type = Type::generic("List", vec![final_elem_type]);

                Ok(TypedExpr {
                    kind: ExprKind::List(typed_exprs),
                    typ: list_type,
                    span: span,
                })
            }
            
            Expr::Block(exprs) => {
                let mut typed_exprs = Vec::new();
                let mut block_type = Type::named("Unit");
                
                self.env.enter_scope();
                let len = exprs.len();
                for (i, e) in exprs.into_iter().enumerate() {
                    let typed_e = self.check_expr(e)?;
                    if i == len - 1 {
                        block_type = typed_e.typ.clone();
                    }
                    typed_exprs.push(typed_e);
                }
                self.env.exit_scope();
                
                Ok(TypedExpr {
                    kind: ExprKind::Block(typed_exprs),
                    typ: block_type,
                    span: span,
                })
            }
            
            Expr::WithBlock { body, bindings } => {
                self.env.enter_scope();
                
                for binding in bindings {
                    match binding {
                        crate::ast::expr::WithBinding::Value { name, value } => {
                            let typed_val = self.check_expr(value)?;
                            self.env.bind_var(&name.0, typed_val.typ);
                        }
                        crate::ast::expr::WithBinding::Signature { name, sig } => {
                            self.constraints.push(Constraint::Signature(name, sig, span.clone()));
                        }
                        crate::ast::expr::WithBinding::Interface { typ, interface } => {
                            self.constraints.push(Constraint::Interface(typ, interface, span.clone()));
                        }
                    }
                }
                
                let typed_body = self.check_expr(*body)?;
                self.validate_constraints()?;
                
                self.env.exit_scope();
                
                Ok(TypedExpr {
                    kind: typed_body.kind.clone(), // This is a bit of a hack, should probably have ExprKind::WithBlock
                    typ: typed_body.typ,
                    span: typed_body.span,
                })
            }
            
            Expr::Hole => {
                Ok(TypedExpr {
                    kind: ExprKind::Hole,
                    typ: crate::type_checker::inference::fresh_type(),
                    span: span,
                })
            }

            // ... Other expressions (Tuple, List, ExplicitApply, Pipeline) would be implemented here
            _ => Err(TypeError::UnboundVariable { 
                name: "Unimplemented expression check".to_string(), 
                span: span 
            }),
        }
    }
}
