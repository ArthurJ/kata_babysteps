//! Environment (Symbol Table) for Kata Type Checker
//!
//! The Environment manages lexical scopes, tracking variables, types,
//! and function/interface implementations. It is structured as a stack
//! of scopes to support shadowing and isolated implementations.

use std::collections::{HashMap, HashSet};
use crate::ast::types::{Type, FunctionSig};

/// Information about a defined interface
#[derive(Debug, Clone)]
pub struct InterfaceInfo {
    pub name: String,
    /// Parent interfaces (inheritance)
    pub extends: Vec<String>,
    /// Member signatures (method name -> signature)
    pub members: HashMap<String, FunctionSig>,
}

/// A single lexical scope containing local variables and type bindings
#[derive(Debug, Clone, Default)]
pub struct Scope {
    /// Maps variable names to their inferred or explicit types
    pub variables: HashMap<String, Type>,
    
    /// Maps generic type variables to concrete types (used during instantiation/inference)
    pub type_bindings: HashMap<String, Type>,
}

impl Scope {
    pub fn new() -> Self {
        Self::default()
    }
}

/// The Type Checking Environment
///
/// Manages a stack of scopes. The top of the stack is the most local scope.
/// Lookups start from the top and proceed downwards to the global scope.
#[derive(Debug, Clone)]
pub struct Environment {
    /// Stack of lexical scopes
    scopes: Vec<Scope>,
    
    /// Global registry of multiple-dispatch functions / interfaces
    /// Maps a function name (e.g., "+") to a list of its implementations (Signatures)
    pub dispatch_table: HashMap<String, Vec<FunctionSig>>,
    
    /// Global registry of custom types (Data, Enum)
    /// Maps type name to its definition (could be expanded to store full structure)
    pub defined_types: HashMap<String, Type>,

    /// Global registry of interfaces
    pub interfaces: HashMap<String, InterfaceInfo>,

    /// Tracks which types were defined in the current module (for Orphan Rule)
    pub local_types: HashSet<String>,

    /// Tracks which interfaces were defined in the current module (for Orphan Rule)
    pub local_interfaces: HashSet<String>,

    /// Registry of which types formally implement which interfaces (Nominal Subtyping)
    /// Maps a type name (e.g. "Float") to a set of interface names (e.g. "NUM", "EQ")
    pub type_implements: HashMap<String, HashSet<String>>,
}

impl Environment {
    /// Creates a new, empty environment with a single global scope
    pub fn new() -> Self {
        Environment {
            scopes: vec![Scope::new()],
            dispatch_table: HashMap::new(),
            defined_types: HashMap::new(),
            interfaces: HashMap::new(),
            local_types: HashSet::new(),
            local_interfaces: HashSet::new(),
            type_implements: HashMap::new(),
        }
    }

    /// Pushes a new lexical scope onto the stack (e.g., entering a lambda or block)
    pub fn enter_scope(&mut self) {
        self.scopes.push(Scope::new());
    }

    /// Pops the current lexical scope from the stack (e.g., exiting a lambda)
    /// This automatically discards any local variable bindings or local implementations,
    /// preventing them from leaking into the outer scope.
    pub fn exit_scope(&mut self) {
        if self.scopes.len() > 1 {
            self.scopes.pop();
        } else {
            panic!("Kata Compiler Bug: Attempted to pop the global scope!");
        }
    }

    /// Binds a variable to a type in the current (most local) scope
    pub fn bind_var(&mut self, name: &str, typ: Type) {
        let current_scope = self.scopes.last_mut().unwrap();
        current_scope.variables.insert(name.to_string(), typ);
    }

    /// Looks up a variable's type, starting from the most local scope and moving outward
    pub fn lookup_var(&self, name: &str) -> Option<&Type> {
        for scope in self.scopes.iter().rev() {
            if let Some(typ) = scope.variables.get(name) {
                return Some(typ);
            }
        }
        None
    }

    /// Registers a new multiple-dispatch implementation for a function globally
    pub fn register_dispatch(&mut self, name: &str, sig: FunctionSig) {
        self.dispatch_table
            .entry(name.to_string())
            .or_insert_with(Vec::new)
            .push(sig);
    }

    /// Looks up all available signatures for a given function name
    pub fn lookup_dispatch(&self, name: &str) -> Option<&Vec<FunctionSig>> {
        self.dispatch_table.get(name)
    }

    /// Registers a custom type (Data, Enum) globally
    pub fn register_type(&mut self, name: &str, typ: Type, is_local: bool) {
        self.defined_types.insert(name.to_string(), typ);
        if is_local {
            self.local_types.insert(name.to_string());
        }
    }

    /// Checks if a type is defined in the environment
    pub fn is_type_defined(&self, name: &str) -> bool {
        self.defined_types.contains_key(name)
    }

    /// Registers an interface globally
    pub fn register_interface(&mut self, info: InterfaceInfo, is_local: bool) {
        let name = info.name.clone();
        self.interfaces.insert(name.clone(), info);
        if is_local {
            self.local_interfaces.insert(name);
        }
    }

    /// Checks if a type or interface is local to this module
    pub fn is_local_type(&self, name: &str) -> bool {
        self.local_types.contains(name)
    }

    pub fn is_local_interface(&self, name: &str) -> bool {
        self.local_interfaces.contains(name)
    }

    /// Registers that a specific type implements a specific interface
    pub fn register_implementation(&mut self, type_name: &str, interface_name: &str) {
        self.type_implements
            .entry(type_name.to_string())
            .or_insert_with(HashSet::new)
            .insert(interface_name.to_string());
    }

    /// Checks if a type satisfies an interface, including transitive parent interfaces.
    pub fn satisfies_interface(&self, typ: &Type, interface_name: &str) -> Option<usize> {
        // Fast path: Nominal subtyping check
        if let Type::Named { name, params: _ } = typ {
            if let Some(interfaces) = self.type_implements.get(&name.name) {
                if interfaces.contains(interface_name) {
                    return Some(1);
                }
            }
        }

        let info = self.interfaces.get(interface_name)?;

        for (member_name, _) in &info.members {
            if self.lookup_dispatch(member_name).is_none() {
                return None;
            }
        }

        let mut max_depth = 1;
        for parent in &info.extends {
            if let Some(depth) = self.satisfies_interface(typ, parent) {
                if depth + 1 > max_depth {
                    max_depth = depth + 1;
                }
            } else {
                return None;
            }
        }

        Some(max_depth)
    }
}
