//! Top-Level Declarations for Kata Language
//!
//! This module defines declarations that appear at module level:
//! - Functions and Actions
//! - Data and Enum definitions
//! - Interface and Implementation definitions
//! - Type aliases
//! - Imports and Exports

use super::expr::LambdaClause;
use super::id::{Directive, Ident, QualifiedIdent};
use super::stmt::Stmt;
use super::types::{FunctionSig, Type};
use std::fmt;

// =============================================================================
// MODULE
// =============================================================================

/// A complete Kata source file (module)
#[derive(Debug, Clone, PartialEq)]
pub struct Module {
    /// Module name (from file path)
    pub name: String,
    /// Top-level declarations
    pub declarations: Vec<TopLevel>,
    /// Exports (if any)
    pub exports: Vec<Ident>,
    /// Imports
    pub imports: Vec<Import>,
}

impl Module {
    pub fn new(name: impl Into<String>) -> Self {
        Module {
            name: name.into(),
            declarations: Vec::new(),
            exports: Vec::new(),
            imports: Vec::new(),
        }
    }

    pub fn with_imports(mut self, imports: Vec<Import>) -> Self {
        self.imports = imports;
        self
    }

    pub fn with_declarations(mut self, declarations: Vec<TopLevel>) -> Self {
        self.declarations = declarations;
        self
    }

    pub fn with_exports(mut self, exports: Vec<Ident>) -> Self {
        self.exports = exports;
        self
    }
}

// =============================================================================
// TOP-LEVEL DECLARATIONS
// =============================================================================

/// Top-level declarations in a module
#[derive(Debug, Clone, PartialEq)]
pub enum TopLevel {
    /// Function definition: `name :: Sig => body`
    Function(FunctionDef),

    /// Action definition: `action name (args) body`
    Action(ActionDef),

    /// Data type definition: `data Name (fields)`
    Data(DataDef),

    /// Enum definition: `enum Name | Variant | Variant(T)`
    Enum(EnumDef),

    /// Interface definition: `interface NAME implements ...`
    Interface(InterfaceDef),

    /// Implementation: `Type implements INTERFACE { ... }`
    Implements(ImplDef),

    /// Type alias: `alias NewName Type`
    Alias(AliasDef),

    /// Import declaration: `import module`
    Import(Import),

    /// Export declaration: `export item1 item2`
    Export(Export),
    
    /// Top-level action statement (entry points)
    Statement(Stmt),
}

impl fmt::Display for TopLevel {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            TopLevel::Function(d) => write!(f, "{}", d),
            TopLevel::Action(d) => write!(f, "{}", d),
            TopLevel::Data(d) => write!(f, "{}", d),
            TopLevel::Enum(d) => write!(f, "{}", d),
            TopLevel::Interface(d) => write!(f, "{}", d),
            TopLevel::Implements(d) => write!(f, "{}", d),
            TopLevel::Alias(d) => write!(f, "{}", d),
            TopLevel::Import(d) => write!(f, "{}", d),
            TopLevel::Export(d) => write!(f, "{}", d),
            TopLevel::Statement(s) => write!(f, "{}", s),
        }
    }
}

// =============================================================================
// FUNCTION DEFINITION
// =============================================================================

/// Function definition in the pure domain
///
/// Examples:
/// ```kata
/// soma :: Int Int => Int
/// λ (x y): + x y
/// ```
///
/// With pattern matching:
/// ```kata
/// fib :: Int => Int
/// λ (0): 0
/// λ (1): 1
/// λ (n): + (fib $(- n 1)) (fib $(- n 2))
/// ```
#[derive(Debug, Clone, PartialEq)]
pub struct FunctionDef {
    /// Function name (can be operator like `+`)
    pub name: Ident,
    /// Function signature: `Arg1 Arg2 => Return`
    pub sig: FunctionSig,
    /// Number of parameters (arity)
    pub arity: usize,
    /// Compiler directives: @ffi, @comutative, @cache_strategy, etc.
    pub directives: Vec<Directive>,
    /// Lambda clauses (pattern matching alternatives)
    pub clauses: Vec<LambdaClause>,
}

impl FunctionDef {
    pub fn new(name: impl Into<String>, sig: FunctionSig) -> Self {
        let arity = sig.arity();
        FunctionDef {
            name: Ident::new(name),
            sig,
            arity,
            directives: Vec::new(),
            clauses: Vec::new(),
        }
    }

    pub fn with_directives(mut self, directives: Vec<Directive>) -> Self {
        self.directives = directives;
        self
    }

    pub fn with_clauses(mut self, clauses: Vec<LambdaClause>) -> Self {
        self.clauses = clauses;
        self
    }

    pub fn add_clause(mut self, clause: LambdaClause) -> Self {
        self.clauses.push(clause);
        self
    }

    /// Check if this is a foreign function (@ffi)
    pub fn is_ffi(&self) -> bool {
        self.directives.iter().any(|d| matches!(d, Directive::Ffi { .. }))
    }
}

impl fmt::Display for FunctionDef {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        for directive in &self.directives {
            writeln!(f, "{}", directive)?;
        }
        writeln!(f, "{} :: {}", self.name, self.sig)?;
        for clause in &self.clauses {
            writeln!(f, "{}", clause)?;
        }
        Ok(())
    }
}

// =============================================================================
// ACTION DEFINITION
// =============================================================================

/// Action definition in the impure domain
///
/// Actions can have side effects and use imperative constructs.
///
/// Example:
/// ```kata
/// action main
///     let resultado canal!()
///     echo! "Hello, World!"
/// ```
#[derive(Debug, Clone, PartialEq)]
pub struct ActionDef {
    /// Action name (ends with `!` conventionally, but not required)
    pub name: Ident,
    /// Parameters
    pub params: Vec<Ident>,
    /// Return type (if specified)
    pub return_type: Option<Type>,
    /// Compiler directives: @parallel, @restart, etc.
    pub directives: Vec<Directive>,
    /// Body statements
    pub body: Vec<Stmt>,
}

impl ActionDef {
    pub fn new(name: impl Into<String>) -> Self {
        ActionDef {
            name: Ident::new(name),
            params: Vec::new(),
            return_type: None,
            directives: Vec::new(),
            body: Vec::new(),
        }
    }

    pub fn with_params(mut self, params: Vec<Ident>) -> Self {
        self.params = params;
        self
    }

    pub fn with_return_type(mut self, return_type: Type) -> Self {
        self.return_type = Some(return_type);
        self
    }

    pub fn with_directives(mut self, directives: Vec<Directive>) -> Self {
        self.directives = directives;
        self
    }

    pub fn with_body(mut self, body: Vec<Stmt>) -> Self {
        self.body = body;
        self
    }

    /// Check if this action has @parallel directive
    pub fn is_parallel(&self) -> bool {
        self.directives.iter().any(|d| matches!(d, Directive::Parallel))
    }
}

impl fmt::Display for ActionDef {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        for directive in &self.directives {
            writeln!(f, "{}", directive)?;
        }
        write!(f, "action {}", self.name)?;
        if !self.params.is_empty() {
            write!(f, "(")?;
            for (i, param) in self.params.iter().enumerate() {
                if i > 0 {
                    write!(f, " ")?;
                }
                write!(f, "{}", param)?;
            }
            write!(f, ")")?;
        }
        if let Some(ret) = &self.return_type {
            write!(f, " => {}", ret)?;
        }
        writeln!(f)?;
        for stmt in &self.body {
            writeln!(f, "    {}", stmt)?;
        }
        Ok(())
    }
}

// =============================================================================
// DATA DEFINITION (Product Type)
// =============================================================================

/// Data type definition (product type / struct or refinement)
///
/// Examples:
/// ```kata
/// # Product
/// data Vec2 (x::Float y::Float)
/// 
/// # Refinement
/// data PositiveInt as (Int, > _ 0)
/// ```
#[derive(Debug, Clone, PartialEq)]
pub struct DataDef {
    /// Type name
    pub name: Ident,
    /// Type parameters (generics)
    pub type_params: Vec<Ident>,
    /// Kind of definition
    pub kind: DataKind,
}

/// The kind of data definition
#[derive(Debug, Clone, PartialEq)]
pub enum DataKind {
    /// Product type with named fields
    Product(Vec<FieldDef>),
    /// Refined type based on another type
    Refinement(Type),
}

/// A field in a data definition
#[derive(Debug, Clone, PartialEq)]
pub struct FieldDef {
    /// Field name
    pub name: Ident,
    /// Field type (optional for inference)
    pub type_annotation: Option<Type>,
}

impl DataDef {
    pub fn new(name: impl Into<String>) -> Self {
        DataDef {
            name: Ident::new(name),
            type_params: Vec::new(),
            kind: DataKind::Product(Vec::new()),
        }
    }

    pub fn refinement(name: impl Into<String>, target: Type) -> Self {
        DataDef {
            name: Ident::new(name),
            type_params: Vec::new(),
            kind: DataKind::Refinement(target),
        }
    }

    pub fn with_type_params(mut self, params: Vec<Ident>) -> Self {
        self.type_params = params;
        self
    }

    pub fn with_fields(mut self, fields: Vec<FieldDef>) -> Self {
        self.kind = DataKind::Product(fields);
        self
    }

    pub fn add_field(mut self, name: impl Into<String>, type_annotation: Option<Type>) -> Self {
        if let DataKind::Product(fields) = &mut self.kind {
            fields.push(FieldDef {
                name: Ident::new(name),
                type_annotation,
            });
        }
        self
    }
}

impl fmt::Display for DataDef {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "data {}", self.name)?;
        if !self.type_params.is_empty() {
            for param in &self.type_params {
                write!(f, "::{}", param)?;
            }
        }
        match &self.kind {
            DataKind::Product(fields) => {
                write!(f, " (")?;
                for (i, field) in fields.iter().enumerate() {
                    if i > 0 {
                        write!(f, " ")?;
                    }
                    write!(f, "{}", field.name)?;
                    if let Some(t) = &field.type_annotation {
                        write!(f, "::{}", t)?;
                    }
                }
                write!(f, ")")
            }
            DataKind::Refinement(t) => {
                write!(f, " as {}", t)
            }
        }
    }
}

// =============================================================================
// ENUM DEFINITION (Sum Type)
// =============================================================================

/// Enum definition (sum type / ADT)
///
/// Examples:
/// ```kata
/// enum Bool
///     | True
///     | False
///
/// enum Result::T::E
///     | Ok(T)
///     | Err(E)
///
/// enum IMC
///     | Magreza(< _ 18.5)
///     | Normal(<= _ 25.0)
///     | Obesidade
/// ```
#[derive(Debug, Clone, PartialEq)]
pub struct EnumDef {
    /// Type name
    pub name: Ident,
    /// Type parameters (generics)
    pub type_params: Vec<Ident>,
    /// Variants
    pub variants: Vec<VariantDef>,
}

/// A variant in an enum definition
#[derive(Debug, Clone, PartialEq)]
pub struct VariantDef {
    /// Variant name
    pub name: Ident,
    /// Variant payload: None (unit), Some with type, or predicate
    pub payload: VariantPayload,
}

/// Payload of an enum variant
#[derive(Debug, Clone, PartialEq)]
pub enum VariantPayload {
    /// Unit variant: `True`, `False`, `None`
    Unit,

    /// Typed variant: `Ok(T)`, `Some(value)`
    Typed(Type),

    /// Fixed value variant: `OK(200)`, `NotFound(404)`
    FixedValue(LiteralValue),

    /// Predicated variant: `Magreza(< _ 18.5)`
    Predicated {
        predicate: VariantPredicate,
    },
}

/// Literal value for fixed value variants
#[derive(Debug, Clone, PartialEq)]
pub enum LiteralValue {
    Int(String),
    Float(String),
    String(String),
}

/// Predicate for predicated variants
#[derive(Debug, Clone, PartialEq)]
pub enum VariantPredicate {
    /// Comparison: `< _ 18.5`, `<= _ 25.0`
    Comparison {
        op: CompareOp,
        value: LiteralValue,
    },
}

/// Comparison operator
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CompareOp {
    Lt,  // <
    Le,  // <=
    Gt,  // >
    Ge,  // >=
}

impl EnumDef {
    pub fn new(name: impl Into<String>) -> Self {
        EnumDef {
            name: Ident::new(name),
            type_params: Vec::new(),
            variants: Vec::new(),
        }
    }

    pub fn with_type_params(mut self, params: Vec<Ident>) -> Self {
        self.type_params = params;
        self
    }

    pub fn with_variants(mut self, variants: Vec<VariantDef>) -> Self {
        self.variants = variants;
        self
    }

    pub fn add_unit_variant(mut self, name: impl Into<String>) -> Self {
        self.variants.push(VariantDef {
            name: Ident::new(name),
            payload: VariantPayload::Unit,
        });
        self
    }

    pub fn add_typed_variant(mut self, name: impl Into<String>, typ: Type) -> Self {
        self.variants.push(VariantDef {
            name: Ident::new(name),
            payload: VariantPayload::Typed(typ),
        });
        self
    }
}

impl fmt::Display for EnumDef {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "enum {}", self.name)?;
        for param in &self.type_params {
            write!(f, "::{}", param)?;
        }
        writeln!(f)?;
        for variant in &self.variants {
            write!(f, "    | {}", variant.name)?;
            match &variant.payload {
                VariantPayload::Unit => {}
                VariantPayload::Typed(t) => write!(f, "({})", t)?,
                VariantPayload::FixedValue(v) => write!(f, "({:?})", v)?,
                VariantPayload::Predicated { predicate } => {
                    write!(f, "({:?})", predicate)?;
                }
            }
            writeln!(f)?;
        }
        Ok(())
    }
}

// =============================================================================
// INTERFACE DEFINITION
// =============================================================================

/// Interface definition
///
/// Interfaces define contracts that types must implement.
/// They can include both signatures and default implementations.
///
/// Example:
/// ```kata
/// interface EQ implements HASH
///     = :: A A => Bool
///
///     != :: A A => Bool
///     λ (x y): not $(= x y)
/// ```
#[derive(Debug, Clone, PartialEq)]
pub struct InterfaceDef {
    /// Interface name (ALL_CAPS by convention)
    pub name: Ident,
    /// Parent interfaces (inheritance)
    pub extends: Vec<Ident>,
    /// Interface members (signatures and/or default implementations)
    pub members: Vec<InterfaceMember>,
}

/// A member of an interface
#[derive(Debug, Clone, PartialEq)]
pub enum InterfaceMember {
    /// Signature only: `= :: A A => Bool`
    Signature(Ident, FunctionSig),

    /// Signature with default implementation: `!= :: A A => Bool` + `λ (x y): ...`
    FunctionDef(FunctionDef),
}

impl InterfaceDef {
    pub fn new(name: impl Into<String>) -> Self {
        InterfaceDef {
            name: Ident::new(name),
            extends: Vec::new(),
            members: Vec::new(),
        }
    }

    pub fn with_extends(mut self, extends: Vec<Ident>) -> Self {
        self.extends = extends;
        self
    }

    pub fn with_members(mut self, members: Vec<InterfaceMember>) -> Self {
        self.members = members;
        self
    }

    pub fn add_signature(mut self, name: impl Into<String>, sig: FunctionSig) -> Self {
        self.members.push(InterfaceMember::Signature(Ident::new(name), sig));
        self
    }

    pub fn add_function(mut self, func: FunctionDef) -> Self {
        self.members.push(InterfaceMember::FunctionDef(func));
        self
    }
}

impl fmt::Display for InterfaceDef {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "interface {}", self.name)?;
        if !self.extends.is_empty() {
            write!(f, " implements")?;
            for ext in &self.extends {
                write!(f, " {}", ext)?;
            }
        }
        writeln!(f)?;
        for member in &self.members {
            match member {
                InterfaceMember::Signature(name, sig) => writeln!(f, "    {} :: {}", name, sig)?,
                InterfaceMember::FunctionDef(func) => {
                    for line in func.to_string().lines() {
                        writeln!(f, "    {}", line)?;
                    }
                }
            }
        }
        Ok(())
    }
}

// =============================================================================
// IMPLEMENTATION DEFINITION
// =============================================================================

/// Implementation definition
///
/// Types implement interfaces with specific functions.
///
/// Example:
/// ```kata
/// Int implements NUM
///     @ffi("kata_rt_add_int")
///     + :: Int Int => Int
///
///     @ffi("kata_rt_sub_int")
///     - :: Int Int => Int
/// ```
#[derive(Debug, Clone, PartialEq)]
pub struct ImplDef {
    /// The type implementing the interface
    pub type_name: QualifiedIdent,
    /// The interface being implemented
    pub interface: Ident,
    /// Function implementations
    pub implementations: Vec<FunctionDef>,
}

impl ImplDef {
    pub fn new(type_name: QualifiedIdent, interface: impl Into<String>) -> Self {
        ImplDef {
            type_name,
            interface: Ident::new(interface),
            implementations: Vec::new(),
        }
    }

    pub fn with_implementations(mut self, implementations: Vec<FunctionDef>) -> Self {
        self.implementations = implementations;
        self
    }

    pub fn add_implementation(mut self, func: FunctionDef) -> Self {
        self.implementations.push(func);
        self
    }
}

impl fmt::Display for ImplDef {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        writeln!(f, "{} implements {}", self.type_name, self.interface)?;
        for func in &self.implementations {
            for line in func.to_string().lines() {
                writeln!(f, "    {}", line)?;
            }
        }
        Ok(())
    }
}

// =============================================================================
// TYPE ALIAS
// =============================================================================

/// Type alias definition
///
/// Example:
/// ```kata
/// alias (NUM, != _ 0) as NonZero
/// alias Matrix as MatrizLocal
/// ```
#[derive(Debug, Clone, PartialEq)]
pub struct AliasDef {
    /// New type name
    pub name: Ident,
    /// Underlying type
    pub target: Type,
}

impl AliasDef {
    pub fn new(name: impl Into<String>, target: Type) -> Self {
        AliasDef {
            name: Ident::new(name),
            target,
        }
    }
}

impl fmt::Display for AliasDef {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "alias {} as {}", self.target, self.name)
    }
}

// =============================================================================
// IMPORT AND EXPORT
// =============================================================================

/// Import declaration
#[derive(Debug, Clone, PartialEq)]
pub enum Import {
    /// `import module` - import entire namespace
    Namespace { module: String },

    /// `import module.Item` - import single item
    Item { module: String, item: String },

    /// `import module.(Item1, Item2)` - import multiple items
    Items { module: String, items: Vec<String> },
}

impl fmt::Display for Import {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Import::Namespace { module } => write!(f, "import {}", module),
            Import::Item { module, item } => write!(f, "import {}.{}", module, item),
            Import::Items { module, items } => {
                write!(f, "import {}.(", module)?;
                for (i, item) in items.iter().enumerate() {
                    if i > 0 {
                        write!(f, " ")?;
                    }
                    write!(f, "{}", item)?;
                }
                write!(f, ")")
            }
        }
    }
}

/// Export declaration
#[derive(Debug, Clone, PartialEq)]
pub struct Export {
    /// Items to export
    pub items: Vec<Ident>,
}

impl Export {
    pub fn new(items: Vec<Ident>) -> Self {
        Export { items }
    }
}

impl fmt::Display for Export {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "export")?;
        for item in &self.items {
            write!(f, " {}", item)?;
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_function_def() {
        let func = FunctionDef::new("soma", FunctionSig::binary(
            Type::named("Int"),
            Type::named("Int"),
            Type::named("Int"),
        ));
        assert_eq!(func.name.to_string(), "soma");
        assert_eq!(func.arity, 2);
    }

    #[test]
    fn test_action_def() {
        let action = ActionDef::new("main")
            .with_params(vec![Ident::new("args")]);
        assert_eq!(action.name.to_string(), "main");
        assert_eq!(action.params.len(), 1);
    }

    #[test]
    fn test_data_def() {
        let data = DataDef::new("Vec2")
            .add_field("x", Some(Type::named("Float")))
            .add_field("y", Some(Type::named("Float")));
        if let DataKind::Product(fields) = &data.kind {
            assert_eq!(fields.len(), 2);
        } else {
            panic!("Expected Product kind");
        }
    }

    #[test]
    fn test_enum_def() {
        let enum_def = EnumDef::new("Result")
            .with_type_params(vec![Ident::new("T"), Ident::new("E")])
            .add_typed_variant("Ok", Type::var("T"))
            .add_typed_variant("Err", Type::var("E"));
        assert_eq!(enum_def.variants.len(), 2);
        assert_eq!(enum_def.type_params.len(), 2);
    }

    #[test]
    fn test_interface_def() {
        let interface = InterfaceDef::new("EQ")
            .with_extends(vec![Ident::new("HASH")]);
        assert_eq!(interface.name.to_string(), "EQ");
        assert_eq!(interface.extends.len(), 1);
    }

    #[test]
    fn test_impl_def() {
        let impl_def = ImplDef::new(
            QualifiedIdent::simple("Int"),
            "NUM"
        );
        assert_eq!(impl_def.type_name.to_string(), "Int");
        assert_eq!(impl_def.interface.to_string(), "NUM");
    }

    #[test]
    fn test_import() {
        let import = Import::Item {
            module: "types".to_string(),
            item: "NUM".to_string(),
        };
        assert_eq!(import.to_string(), "import types.NUM");
    }
}