use crate::ast::{Ident, Literal, Pattern};
use crate::type_checker::Type;

/// A Árvore Sintática Tipada (Pós-Semantic Analysis).
/// Cada nó nesta árvore já passou pelo Type Checker e contém
/// uma anotação infalível do seu tipo de retorno.
#[derive(Debug, Clone, PartialEq)]
pub struct TypedExpr {
    pub expr: TypedDataExpr,
    pub ty: Type,
}

#[derive(Debug, Clone, PartialEq)]
pub enum TypedDataExpr {
    Literal(Literal),
    Identifier(Ident),
    
    /// Uma tupla estrita (ex: `(1 "A")` gera `Type::Tuple([Int, Text])`)
    Tuple(Vec<TypedExpr>),
    
    /// Aplicação de Função totalmente resolvida e validada.
    Call {
        target: Box<TypedExpr>,
        args: Vec<TypedExpr>,
    },
    
    /// O encadeamento Pipeline já desaçucarado e validado.
    Pipe {
        left: Box<TypedExpr>,
        right: Box<TypedExpr>,
    },
    
    LambdaGroup {
        branches: Vec<TypedLambdaBranch>,
    },

    GuardBlock {
        branches: Vec<TypedGuardBranch>,
        otherwise: Box<TypedExpr>,
    },
    
    ScopedBlock {
        bindings: Vec<TypedBinding>,
        body: Box<TypedExpr>,
        with_clauses: Vec<TypedBinding>,
    },
}

#[derive(Debug, Clone, PartialEq)]
pub struct TypedGuardBranch {
    pub condition: TypedExpr,
    pub result: TypedExpr,
}

#[derive(Debug, Clone, PartialEq)]
pub struct TypedBinding {
    pub pattern: Pattern,
    pub expr: TypedExpr,
}

#[derive(Debug, Clone, PartialEq)]
pub struct TypedLambdaBranch {
    pub params: Vec<Pattern>, // No futuro, isso será TypedPattern se inferirmos
    pub body: Box<TypedExpr>,
}

/// Como as Actions operam sobre variáveis, a árvore delas também consome TypedExpr agora.
#[derive(Debug, Clone, PartialEq)]
pub enum TypedActionStmt {
    Expr(TypedExpr),
    ActionCall {
        target: Ident,
        args: Vec<TypedExpr>,
    },
    LetBind {
        pattern: Pattern,
        expr: TypedExpr,
    },
    VarBind {
        name: Ident,
        expr: TypedExpr,
    },
    Assign {
        name: Ident,
        expr: TypedExpr,
    },
    Loop(Vec<TypedActionStmt>),
    Return(TypedExpr),
}

/// A Raiz tipada e limpa do Módulo
#[derive(Debug, Clone, PartialEq)]
pub enum TypedTopLevel {
    Definition {
        name: Ident,
        expr: TypedExpr,
    },
    ActionDef {
        attrs: Vec<crate::ast::TopLevelAttr>,
        name: Ident,
        params: Vec<Pattern>,
        body: Vec<TypedActionStmt>,
    },
    // Imports, Exports e Interfaces continuam os mesmos conceitualmente, 
    // mas iremos focar apenas nos executáveis nesta etapa.
}

#[derive(Debug, Clone, PartialEq)]
pub struct TypedModule {
    pub declarations: Vec<TypedTopLevel>,
}
