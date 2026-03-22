//! Dependency Graph Builder (DAG)
//!
//! This module analyzes the AST to build a directed acyclic graph (DAG) of dependencies
//! between top-level declarations. This allows the compiler to:
//! 1. Determine the correct topological order for type inference.
//! 2. Detect circular dependencies.
//! 3. Perform dead code elimination (tree shaking).

use std::collections::{HashMap, HashSet, VecDeque};
use crate::ast::decl::{Module, TopLevel};
use crate::ast::id::Ident;
use crate::ast::expr::Expr;
use crate::ast::stmt::Stmt;
use crate::ast::Spanned;

/// A node in the dependency graph
#[derive(Debug, Clone)]
pub struct DependencyNode {
    pub id: Ident,
    pub dependencies: HashSet<Ident>,
    pub declaration: Spanned<TopLevel>,
}

/// The Dependency Graph (DAG) for a module
#[derive(Debug, Clone)]
pub struct DependencyGraph {
    pub nodes: HashMap<Ident, DependencyNode>,
}

impl DependencyGraph {
    /// Creates a new DAG from a parsed Module
    pub fn from_module(module: &Module) -> Self {
        let mut nodes = HashMap::new();

        for decl in &module.declarations {
            let id = get_declaration_id(decl);
            let dependencies = find_dependencies(decl);
            
            nodes.insert(id.clone(), DependencyNode {
                id,
                dependencies,
                declaration: decl.clone(),
            });
        }

        DependencyGraph { nodes }
    }

    /// Returns the declarations in topological order.
    pub fn topological_sort(&self) -> Result<Vec<Spanned<TopLevel>>, String> {
        let mut result = Vec::new();
        let mut visited = HashSet::new();
        let mut visiting = HashSet::new();
        let mut order = Vec::new();

        for id in self.nodes.keys() {
            self.dfs_order(id, &mut visited, &mut order);
        }

        visited.clear();
        for id in order.iter().rev() {
            if !visited.contains(id) {
                self.visit_allow_cycles(id, &mut visited, &mut visiting, &mut result);
            }
        }

        Ok(result)
    }

    fn dfs_order(&self, id: &Ident, visited: &mut HashSet<Ident>, order: &mut Vec<Ident>) {
        if !visited.contains(id) {
            visited.insert(id.clone());
            if let Some(node) = self.nodes.get(id) {
                for dep in &node.dependencies {
                    if self.nodes.contains_key(dep) {
                        self.dfs_order(dep, visited, order);
                    }
                }
            }
            order.push(id.clone());
        }
    }

    fn visit_allow_cycles(
        &self,
        id: &Ident,
        visited: &mut HashSet<Ident>,
        visiting: &mut HashSet<Ident>,
        result: &mut Vec<Spanned<TopLevel>>
    ) {
        if visiting.contains(id) || visited.contains(id) {
            return;
        }
        
        visiting.insert(id.clone());
        
        if let Some(node) = self.nodes.get(id) {
            for dep in &node.dependencies {
                if self.nodes.contains_key(dep) && !visited.contains(dep) {
                    self.visit_allow_cycles(dep, visited, visiting, result);
                }
            }
            result.push(node.declaration.clone());
        }
        
        visiting.remove(id);
        visited.insert(id.clone());
    }

    /// Filters the graph to keep only nodes reachable from the given entry points (Tree Shaking).
    pub fn reachability_analysis(&self, roots: &[Ident]) -> Vec<Spanned<TopLevel>> {
        let mut reachable_ids = HashSet::new();
        let mut queue = VecDeque::new();

        for root in roots {
            queue.push_back(root.clone());
        }

        while let Some(id) = queue.pop_front() {
            if !reachable_ids.contains(&id) {
                if let Some(node) = self.nodes.get(&id) {
                    reachable_ids.insert(id.clone());
                    for dep in &node.dependencies {
                        queue.push_back(dep.clone());
                    }
                }
            }
        }

        self.nodes.values()
            .filter(|n| reachable_ids.contains(&n.id))
            .map(|n| n.declaration.clone())
            .collect()
    }
}

// =============================================================================
// HELPERS FOR DEPENDENCY DISCOVERY
// =============================================================================

fn get_declaration_id(spanned_decl: &Spanned<TopLevel>) -> Ident {
    match &spanned_decl.node {
        TopLevel::Function(f) => f.name.clone(),
        TopLevel::Action(a) => a.name.clone(),
        TopLevel::Data(d) => d.name.clone(),
        TopLevel::Enum(e) => e.name.clone(),
        TopLevel::Interface(i) => i.name.clone(),
        TopLevel::Implements(i) => Ident::new(format!("impl_{}_{}", i.type_name, i.interface)),
        TopLevel::Alias(d) => d.name.clone(),
        TopLevel::Statement(_) => Ident::new("__top_level_stmt"),
        TopLevel::Import(_) => Ident::new("__import"),
        TopLevel::Export(_) => Ident::new("__export"),
    }
}

fn find_dependencies(spanned_decl: &Spanned<TopLevel>) -> HashSet<Ident> {
    let mut deps = HashSet::new();
    match &spanned_decl.node {
        TopLevel::Function(f) => {
            for param in &f.sig.params { add_type_deps(param, &mut deps); }
            add_type_deps(&f.sig.return_type, &mut deps);
            
            for clause in &f.clauses {
                if let Some(body) = &clause.body { add_expr_deps(body, &mut deps); }
                for with in &clause.with {
                    match with {
                        crate::ast::expr::WithBinding::Value { value, .. } => add_expr_deps(value, &mut deps),
                        crate::ast::expr::WithBinding::Signature { sig, .. } => {
                            for p in &sig.params { add_type_deps(p, &mut deps); }
                            add_type_deps(&sig.return_type, &mut deps);
                        }
                        crate::ast::expr::WithBinding::Interface { typ, interface } => {
                            deps.insert(interface.clone());
                            add_type_deps(typ, &mut deps);
                        }
                    }
                }
            }
        }
        TopLevel::Action(a) => {
            if let Some(ret) = &a.return_type { add_type_deps(ret, &mut deps); }
            for stmt in &a.body { add_stmt_deps(stmt, &mut deps); }
        }
        TopLevel::Data(d) => {
            match &d.kind {
                crate::ast::decl::DataKind::Product(fields) => {
                    for f in fields {
                        if let Some(t) = &f.type_annotation { add_type_deps(t, &mut deps); }
                    }
                }
                crate::ast::decl::DataKind::Refinement(t) => add_type_deps(t, &mut deps),
            }
        }
        TopLevel::Enum(e) => {
            for v in &e.variants {
                match &v.payload {
                    crate::ast::decl::VariantPayload::Typed(t) => add_type_deps(t, &mut deps),
                    _ => {}
                }
            }
        }
        TopLevel::Statement(s) => add_stmt_deps(&Spanned::new(s.clone(), spanned_decl.span), &mut deps),
        _ => {}
    }
    deps
}

fn add_expr_deps(spanned_expr: &Spanned<Expr>, deps: &mut HashSet<Ident>) {
    match &spanned_expr.node {
        Expr::Var { name, .. } => { deps.insert(name.clone()); }
        Expr::Apply { func, args } | Expr::ExplicitApply { func, args } => {
            add_expr_deps(func, deps);
            for arg in args { add_expr_deps(arg, deps); }
        }
        Expr::Pipeline { value, func } => {
            add_expr_deps(value, deps);
            add_expr_deps(func, deps);
        }
        Expr::Tuple(es) | Expr::List(es) | Expr::Array(es) | Expr::Block(es) => {
            for e in es { add_expr_deps(e, deps); }
        }
        Expr::Cons { head, tail } => {
            add_expr_deps(head, deps);
            add_expr_deps(tail, deps);
        }
        Expr::Lambda { clauses } => {
            for c in clauses {
                if let Some(b) = &c.body { add_expr_deps(b, deps); }
                for w in &c.with {
                    match w {
                        crate::ast::expr::WithBinding::Value { value, .. } => add_expr_deps(value, deps),
                        crate::ast::expr::WithBinding::Signature { sig, .. } => {
                            for p in &sig.params { add_type_deps(p, deps); }
                            add_type_deps(&sig.return_type, deps);
                        }
                        crate::ast::expr::WithBinding::Interface { typ, interface } => {
                            deps.insert(interface.clone());
                            add_type_deps(&typ, deps);
                        }
                    }
                }
            }
        }
        Expr::WithBlock { body, bindings } => {
            add_expr_deps(body, deps);
            for b in bindings {
                match b {
                    crate::ast::expr::WithBinding::Value { value, .. } => add_expr_deps(value, deps),
                    crate::ast::expr::WithBinding::Signature { sig, .. } => {
                        for p in &sig.params { add_type_deps(p, deps); }
                        add_type_deps(&sig.return_type, deps);
                    }
                    crate::ast::expr::WithBinding::Interface { typ, interface } => {
                        deps.insert(interface.clone());
                        add_type_deps(typ, deps);
                    }
                }
            }
        }
        _ => {}
    }
}

fn add_stmt_deps(spanned_stmt: &Spanned<Stmt>, deps: &mut HashSet<Ident>) {
    match &spanned_stmt.node {
        Stmt::Let { value, .. } | Stmt::Var { value, .. } | Stmt::Assign { value, .. } => {
            add_expr_deps(value, deps);
        }
        Stmt::Expr(e) | Stmt::Return(e) => add_expr_deps(e, deps),
        Stmt::Match { value, cases } => {
            add_expr_deps(value, deps);
            for c in cases {
                for s in &c.body { add_stmt_deps(s, deps); }
            }
        }
        Stmt::Loop { body } => {
            for s in body { add_stmt_deps(s, deps); }
        }
        Stmt::For { iterable, body, .. } => {
            add_expr_deps(iterable, deps);
            for s in body { add_stmt_deps(s, deps); }
        }
        _ => {}
    }
}

fn add_type_deps(typ: &crate::ast::types::Type, deps: &mut HashSet<Ident>) {
    match typ {
        crate::ast::types::Type::Named { name, params } => {
            deps.insert(Ident::new(name.name.clone()));
            for p in params { add_type_deps(p, deps); }
        }
        crate::ast::types::Type::Tuple(ts) => {
            for t in ts { add_type_deps(t, deps); }
        }
        crate::ast::types::Type::Function { params, return_type } => {
            for t in params { add_type_deps(t, deps); }
            add_type_deps(return_type, deps);
        }
        crate::ast::types::Type::Refined { base, .. } => add_type_deps(base, deps),
        _ => {}
    }
}
