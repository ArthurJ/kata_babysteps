//! Expressions for Kata Language (Pure/Functional Domain)
//!
//! This module defines expressions used in the pure functional domain.
//! Expressions are side-effect free and produce values.

use super::id::{Ident, Literal, QualifiedIdent};
use super::pattern::Pattern;
use super::types::Type;
use super::Spanned;
use std::fmt;

// =============================================================================
// EXPRESSIONS
// =============================================================================

/// An expression in the functional domain
#[derive(Debug, Clone, PartialEq)]
pub enum Expr {
    // === Literals ===
    /// Literal value: `42`, `"hello"`, `True`
    Literal(Literal),

    // === Variables and References ===
    /// Variable reference: `x`, `minha_var` or `x::Int`
    Var {
        name: Ident,
        type_ascription: Option<Type>,
    },

    /// Qualified reference: `Modulo::funcao`
    QualifiedRef(QualifiedIdent),

    // === Collections ===
    /// Tuple expression: `(1 2 3)` or `(a, b, c)`
    Tuple(Vec<Spanned<Expr>>),

    /// List expression: `[1, 2, 3]`
    List(Vec<Spanned<Expr>>),

    /// Cons expression (head:tail for lists)
    /// Example: `x : xs`
    Cons {
        head: Box<Spanned<Expr>>,
        tail: Box<Spanned<Expr>>,
    },

    /// Array expression: `{1, 2, 3}`
    Array(Vec<Spanned<Expr>>),

    /// Tensor expression: `{1 2 ; 3 4}` (matrix with dimension separator)
    Tensor {
        dimensions: Vec<usize>,
        elements: Vec<Spanned<Expr>>,
    },

    /// Range expression: `[1..10]`, `[1..2..100]`, `[1..=10]`
    Range {
        start: Box<Spanned<Expr>>,
        end: Box<Spanned<Expr>>,
        step: Option<Box<Spanned<Expr>>>,
        inclusive: bool,
    },

    /// Dictionary expression: `Dict [("chave" "valor")]`
    Dict(Vec<(Spanned<Expr>, Spanned<Expr>)>),

    /// Set expression: `Set [1, 2, 3]`
    Set(Vec<Spanned<Expr>>),

    // === Function Application ===
    /// Function application (prefix notation): `+ 1 2`, `f x y`
    Apply {
        func: Box<Spanned<Expr>>,
        args: Vec<Spanned<Expr>>,
    },

    /// Explicit application with `$`: `$(+ 1 2)`
    ExplicitApply {
        func: Box<Spanned<Expr>>,
        args: Vec<Spanned<Expr>>,
    },

    /// Method call: `obj.method arg1 arg2`
    Method {
        object: Box<Spanned<Expr>>,
        method: Ident,
        args: Vec<Spanned<Expr>>,
    },

    /// Field access: `obj.field`
    Field {
        object: Box<Spanned<Expr>>,
        field: Ident,
    },

    /// Index access: `arr .at i` or `list i`
    Index {
        object: Box<Spanned<Expr>>,
        index: Box<Spanned<Expr>>,
    },

    // === Lambda and Functions ===
    /// Lambda expression: `λ (x) corpo` or `lambda (x y) corpo`
    Lambda {
        clauses: Vec<LambdaClause>,
    },

    /// Hole for partial application: `_`
    Hole,

    // === Control Flow (Pure) ===
    /// Pipeline: `expr |> f`
    Pipeline {
        value: Box<Spanned<Expr>>,
        func: Box<Spanned<Expr>>,
    },

    /// Conditional expression via pattern matching (in lambda)
    /// This is handled by lambda clauses with guards

    // === Type Operations ===
    /// Type cast/coercion: `Int x`
    TypeCast {
        type_name: QualifiedIdent,
        value: Box<Spanned<Expr>>,
    },

    // === Special Forms ===
    /// Block expression (sequence of expressions, last one is the value)
    Block(Vec<Spanned<Expr>>),

    /// With block: `with bindings...`
    /// Used for guards and type constraints
    WithBlock {
        body: Box<Spanned<Expr>>,
        bindings: Vec<WithBinding>,
    },
}

impl Expr {
    /// Create a literal expression
    pub fn literal(lit: Literal) -> Self {
        Expr::Literal(lit)
    }

    /// Check if this is a literal
    pub fn is_literal(&self) -> bool {
        matches!(self, Expr::Literal(_))
    }

    /// Check if this is a variable
    pub fn is_var(&self) -> bool {
        matches!(self, Expr::Var { .. })
    }

    /// Check if this is a hole
    pub fn is_hole(&self) -> bool {
        matches!(self, Expr::Hole)
    }
}

impl fmt::Display for Expr {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Expr::Literal(lit) => write!(f, "{}", lit),
            Expr::Var { name, type_ascription } => {
                match type_ascription {
                    Some(t) => write!(f, "{}::{}", name, t),
                    None => write!(f, "{}", name),
                }
            }
            Expr::QualifiedRef(q) => write!(f, "{}", q),
            Expr::Tuple(exprs) => {
                write!(f, "(")?;
                for (i, e) in exprs.iter().enumerate() {
                    if i > 0 {
                        write!(f, " ")?;
                    }
                    write!(f, "{}", e.node)?;
                }
                write!(f, ")")
            }
            Expr::List(exprs) => {
                write!(f, "[")?;
                for (i, e) in exprs.iter().enumerate() {
                    if i > 0 {
                        write!(f, " ")?;
                    }
                    write!(f, "{}", e.node)?;
                }
                write!(f, "]")
            }
            Expr::Cons { head, tail } => {
                write!(f, "{} : {}", head.node, tail.node)
            }
            Expr::Array(exprs) => {
                write!(f, "{{")?;
                for (i, e) in exprs.iter().enumerate() {
                    if i > 0 {
                        write!(f, " ")?;
                    }
                    write!(f, "{}", e.node)?;
                }
                write!(f, "}}")
            }
            Expr::Tensor { elements, dimensions } => {
                write!(f, "{{")?;
                // Format tensor with semicolons based on dimensions
                let row_size = dimensions.get(1).copied().unwrap_or(elements.len());
                for (i, e) in elements.iter().enumerate() {
                    if i > 0 {
                        if i % row_size == 0 {
                            write!(f, " ; ")?;
                        } else {
                            write!(f, " ")?;
                        }
                    }
                    write!(f, "{}", e.node)?;
                }
                write!(f, "}}")
            }
            Expr::Range { start, end, step, inclusive } => {
                write!(f, "[{}", start.node)?;
                if let Some(s) = step {
                    write!(f, "..{}..", s.node)?;
                } else {
                    write!(f, "..")?;
                }
                if *inclusive {
                    write!(f, "=")?;
                }
                write!(f, "{}]", end.node)
            }
            Expr::Dict(entries) => {
                write!(f, "Dict [")?;
                for (i, (k, v)) in entries.iter().enumerate() {
                    if i > 0 {
                        write!(f, " ")?;
                    }
                    write!(f, "({} {})", k.node, v.node)?;
                }
                write!(f, "]")
            }
            Expr::Set(exprs) => {
                write!(f, "Set [")?;
                for (i, e) in exprs.iter().enumerate() {
                    if i > 0 {
                        write!(f, " ")?;
                    }
                    write!(f, "{}", e.node)?;
                }
                write!(f, "]")
            }
            Expr::Apply { func, args } => {
                write!(f, "{}", func.node)?;
                for arg in args {
                    write!(f, " {}", arg.node)?;
                }
                Ok(())
            }
            Expr::ExplicitApply { func, args } => {
                write!(f, "$({}", func.node)?;
                for arg in args {
                    write!(f, " {}", arg.node)?;
                }
                write!(f, ")")
            }
            Expr::Method { object, method, args } => {
                write!(f, "{}.{}", object.node, method)?;
                for arg in args {
                    write!(f, " {}", arg.node)?;
                }
                Ok(())
            }
            Expr::Field { object, field } => {
                write!(f, "{}.{}", object.node, field)
            }
            Expr::Index { object, index } => {
                write!(f, "({} .at {})", object.node, index.node)
            }
            Expr::Lambda { clauses } => {
                write!(f, "λ ")?;
                for (i, clause) in clauses.iter().enumerate() {
                    if i > 0 {
                        write!(f, "\n")?;
                    }
                    write!(f, "{}", clause)?;
                }
                Ok(())
            }
            Expr::Hole => write!(f, "_"),
            Expr::Pipeline { value, func } => {
                write!(f, "{} |>", value.node)?;
                write!(f, " {}", func.node)
            }
            Expr::TypeCast { type_name, value } => {
                write!(f, "{} {}", type_name, value.node)
            }
            Expr::Block(exprs) => {
                for (i, e) in exprs.iter().enumerate() {
                    if i > 0 {
                        write!(f, "\n")?;
                    }
                    write!(f, "{}", e.node)?;
                }
                Ok(())
            }
            Expr::WithBlock { body, bindings } => {
                write!(f, "{}", body.node)?;
                write!(f, "\n    with")?;
                for b in bindings {
                    write!(f, "\n        {}", b)?;
                }
                Ok(())
            }
        }
    }
}

// =============================================================================
// LAMBDA CLAUSE
// =============================================================================

/// A clause in a lambda definition
///
/// Lambdas can have multiple clauses with pattern matching:
/// ```kata
/// λ (0): 0
/// λ (1): 1
/// λ (n): + (fib $(- n 1)) (fib $(- n 2))
/// ```
///
/// With guards and bindings:
/// ```kata
/// λ (x y)
///     maior: x
///     otherwise: y
///     with
///         maior as > x y
/// ```
#[derive(Debug, Clone, PartialEq)]
pub struct LambdaClause {
    /// Patterns to match arguments
    pub patterns: Vec<Spanned<Pattern>>,
    /// Guard conditions (optional)
    pub guards: Vec<GuardClause>,
    /// Body expression (optional if guards are present)
    pub body: Option<Spanned<Expr>>,
    /// Bindings for guards (optional)
    /// Used for both value bindings and type constraints
    pub with: Vec<WithBinding>,
}

impl LambdaClause {
    pub fn new(patterns: Vec<Spanned<Pattern>>, body: Spanned<Expr>) -> Self {
        LambdaClause {
            patterns,
            guards: vec![],
            body: Some(body),
            with: vec![],
        }
    }

    pub fn with_guards(patterns: Vec<Spanned<Pattern>>, guards: Vec<GuardClause>, body: Option<Spanned<Expr>>) -> Self {
        LambdaClause {
            patterns,
            guards,
            body,
            with: Vec::new(),
        }
    }

    pub fn with_bindings(patterns: Vec<Spanned<Pattern>>, guards: Vec<GuardClause>, body: Option<Spanned<Expr>>, with: Vec<WithBinding>) -> Self {
        LambdaClause {
            patterns,
            guards,
            body,
            with,
        }
    }
}

impl fmt::Display for LambdaClause {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        for (i, p) in self.patterns.iter().enumerate() {
            if i > 0 {
                write!(f, " ")?;
            }
            write!(f, "{}", p.node)?;
        }
        write!(f, ":")?;
        
        if !self.guards.is_empty() {
            for guard in &self.guards {
                write!(f, "\n    {}: {}", guard.label, guard.body.node)?;
            }
        } else if let Some(ref body) = self.body {
            write!(f, " {}", body.node)?;
        }

        if !self.with.is_empty() {
            write!(f, "\n    with")?;
            for binding in &self.with {
                write!(f, "\n        {}", binding)?;
            }
        }

        Ok(())
    }
}

// =============================================================================
// GUARD CLAUSE
// =============================================================================

/// A guard clause in pattern matching
///
/// Example:
/// ```kata
/// λ (x y)
///     maior: x
///     menor: y
///     otherwise: y
/// ```
#[derive(Debug, Clone, PartialEq)]
pub struct GuardClause {
    /// Label for the guard result
    pub label: Ident,
    /// Guard condition (or "otherwise")
    pub guard: GuardCondition,
    /// Body expression for this guard
    pub body: Spanned<Expr>,
}

/// Guard condition
#[derive(Debug, Clone, PartialEq)]
pub enum GuardCondition {
    /// Named guard (evaluates condition)
    Named(Ident),
    /// Otherwise clause (default)
    Otherwise,
}

impl fmt::Display for GuardCondition {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            GuardCondition::Named(n) => write!(f, "{}", n),
            GuardCondition::Otherwise => write!(f, "otherwise"),
        }
    }
}

// =============================================================================
// WITH BINDING
// =============================================================================

/// A binding in a with block
///
/// Used for guards and type constraints:
/// ```kata
/// with
///     base as calcular_base entrada
///     variante as extrair_variante carga
///     + :: A B => C
///     T implements ORD
/// ```
#[derive(Debug, Clone, PartialEq)]
pub enum WithBinding {
    /// Value binding: `name as expr`
    Value {
        name: Ident,
        value: Spanned<Expr>,
    },
    /// Signature constraint: `+ :: A B => C`
    Signature {
        name: Ident,
        sig: super::types::FunctionSig,
    },
    /// Interface constraint: `T implements ORD`
    Interface {
        typ: Type,
        interface: Ident,
    },
}

impl fmt::Display for WithBinding {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            WithBinding::Value { name, value } => write!(f, "{} as {}", name, value.node),
            WithBinding::Signature { name, sig } => write!(f, "{} :: {}", name, sig),
            WithBinding::Interface { typ, interface } => write!(f, "{} implements {}", typ, interface),
        }
        }
        }