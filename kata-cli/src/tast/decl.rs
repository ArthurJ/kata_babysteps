//! Typed Top-Level Declarations (TAST) for Kata Language

use crate::ast::id::{Ident, Directive};
use crate::ast::types::{FunctionSig, Type};
use crate::tast::expr::TypedLambdaClause;
use crate::tast::stmt::TypedStmt;
use crate::lexer::Span;

/// A fully typed declaration in a module
#[derive(Debug, Clone, PartialEq)]
pub struct TypedDecl {
    pub kind: DeclKind,
    pub span: Span,
}

#[derive(Debug, Clone, PartialEq)]
pub enum DeclKind {
    /// Function definition
    Function(TypedFunctionDef),

    /// Action definition
    Action(TypedActionDef),

    /// Data type definition
    Data(TypedDataDef),

    /// Enum definition
    Enum(TypedEnumDef),

    /// Interface definition
    Interface(TypedInterfaceDef),

    /// Implementation
    Implements(TypedImplDef),

    /// Type alias
    Alias(TypedAliasDef),

    /// Import
    Import(crate::ast::decl::Import),

    /// Export
    Export(Vec<String>),

    /// Entry point statement
    Statement(TypedStmt),
}

#[derive(Debug, Clone, PartialEq)]
pub struct TypedFunctionDef {
    pub name: Ident,
    pub sig: FunctionSig,
    pub arity: usize,
    pub directives: Vec<Directive>,
    pub clauses: Vec<TypedLambdaClause>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct TypedActionDef {
    pub name: Ident,
    pub params: Vec<Ident>,
    pub return_type: Type, // Always resolved in TAST (defaulting to Unit)
    pub directives: Vec<Directive>,
    pub body: Vec<TypedStmt>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct TypedDataDef {
    pub name: Ident,
    pub type_params: Vec<Ident>,
    pub fields: Vec<TypedFieldDef>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct TypedFieldDef {
    pub name: Ident,
    pub typ: Type,
}

#[derive(Debug, Clone, PartialEq)]
pub struct TypedEnumDef {
    pub name: Ident,
    pub type_params: Vec<Ident>,
    pub variants: Vec<TypedVariantDef>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct TypedVariantDef {
    pub name: Ident,
    pub payload: TypedVariantPayload,
}

#[derive(Debug, Clone, PartialEq)]
pub enum TypedVariantPayload {
    Unit,
    Typed(Type),
}

#[derive(Debug, Clone, PartialEq)]
pub struct TypedInterfaceDef {
    pub name: Ident,
    pub extends: Vec<Ident>,
    pub members: Vec<TypedInterfaceMember>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum TypedInterfaceMember {
    Signature(Ident, FunctionSig),
    FunctionDef(TypedFunctionDef),
}

#[derive(Debug, Clone, PartialEq)]
pub struct TypedImplDef {
    pub type_name: Type,
    pub interface: Ident,
    pub implementations: Vec<TypedFunctionDef>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct TypedAliasDef {
    pub name: Ident,
    pub target: Type,
}
