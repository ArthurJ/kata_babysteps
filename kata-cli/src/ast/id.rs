//! Identifiers and Literals for Kata Language
//!
//! This module defines the fundamental building blocks:
//! - Ident: simple identifiers (variables, functions, operators)
//! - QualifiedIdent: module-qualified identifiers (Module::Item)
//! - Literal: primitive values (int, float, string, etc.)

use std::fmt;

// =============================================================================
// IDENTIFIERS
// =============================================================================

/// A simple identifier (variable, function, type, or operator name)
///
/// Examples: `x`, `minha_funcao`, `+`, `!>`, `<=`
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Ident(pub String);

impl Ident {
    /// Create a new identifier from a string
    pub fn new(name: impl Into<String>) -> Self {
        Ident(name.into())
    }

    /// Check if this is an operator identifier
    pub fn is_operator(&self) -> bool {
        self.0.starts_with(|c: char| matches!(c, '+' | '-' | '*' | '/' | '\\' | '=' | '!' | '<' | '>' | '?' | '|' | '&' | '^' | '~' | '@' | '#' | '$' | '%'))
    }

    /// Check if this is an action identifier (ends with !)
    pub fn is_action(&self) -> bool {
        self.0.ends_with('!')
    }
}

impl fmt::Display for Ident {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl From<&str> for Ident {
    fn from(s: &str) -> Self {
        Ident(s.to_string())
    }
}

impl From<String> for Ident {
    fn from(s: String) -> Self {
        Ident(s)
    }
}

/// A qualified identifier with optional module path
///
/// Examples: `Int`, `List::T`, `Optional::T`, `Modulo::Item`
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct QualifiedIdent {
    /// Module path (e.g., "types" in "types::NUM")
    pub module: Option<String>,
    /// Item name (e.g., "NUM" in "types::NUM")
    pub name: String,
}

impl QualifiedIdent {
    /// Create a simple identifier without module
    pub fn simple(name: impl Into<String>) -> Self {
        QualifiedIdent {
            module: None,
            name: name.into(),
        }
    }

    /// Create a qualified identifier with module
    pub fn qualified(module: impl Into<String>, name: impl Into<String>) -> Self {
        QualifiedIdent {
            module: Some(module.into()),
            name: name.into(),
        }
    }

    /// Check if this is a simple (unqualified) identifier
    pub fn is_simple(&self) -> bool {
        self.module.is_none()
    }
}

impl fmt::Display for QualifiedIdent {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match &self.module {
            Some(m) => write!(f, "{}::{}", m, self.name),
            None => write!(f, "{}", self.name),
        }
    }
}

impl From<Ident> for QualifiedIdent {
    fn from(ident: Ident) -> Self {
        QualifiedIdent::simple(ident.0)
    }
}

// =============================================================================
// LITERALS
// =============================================================================

/// A literal value in the source code
#[derive(Debug, Clone, PartialEq)]
pub enum Literal {
    /// Integer literal (decimal, hex, octal, binary)
    /// Stored as string to preserve the original representation
    Int(String),

    /// Float literal (decimal, or special: nan, inf, -inf)
    /// Stored as string to preserve the original representation
    Float(String),

    /// String literal (double or single quoted)
    String(String),

    /// Bytes literal (b"...")
    Bytes(String),

    /// Boolean literal (True or False)
    Bool(bool),

    /// Unit literal - empty tuple ()
    Unit,
}

impl Literal {
    /// Create an integer literal
    pub fn int(value: impl Into<String>) -> Self {
        Literal::Int(value.into())
    }

    /// Create a float literal
    pub fn float(value: impl Into<String>) -> Self {
        Literal::Float(value.into())
    }

    /// Create a string literal
    pub fn string(value: impl Into<String>) -> Self {
        Literal::String(value.into())
    }

    /// Create a bytes literal
    pub fn bytes(value: impl Into<String>) -> Self {
        Literal::Bytes(value.into())
    }

    /// Create a boolean literal
    pub fn bool(value: bool) -> Self {
        Literal::Bool(value)
    }

    /// Create a unit literal
    pub fn unit() -> Self {
        Literal::Unit
    }
}

impl fmt::Display for Literal {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Literal::Int(s) => write!(f, "{}", s),
            Literal::Float(s) => write!(f, "{}", s),
            Literal::String(s) => write!(f, "\"{}\"", s),
            Literal::Bytes(s) => write!(f, "b\"{}\"", s),
            Literal::Bool(b) => write!(f, "{}", if *b { "True" } else { "False" }),
            Literal::Unit => write!(f, "()"),
        }
    }
}

// =============================================================================
// DIRECTIVES (Annotations)
// =============================================================================

/// Compiler directive/annotation
#[derive(Debug, Clone, PartialEq)]
pub enum Directive {
    /// `@test("description")` - test annotation
    Test { description: String },

    /// `@parallel` - run in separate OS process
    Parallel,

    /// `@comutative` - marks function as commutative
    Comutative,

    /// `@cache_strategy{...}` - memoization configuration
    CacheStrategy {
        strategy: CacheStrategy,
        size: Option<usize>,
        ttl: Option<u64>,
    },

    /// `@ffi("symbol_name")` - foreign function interface
    Ffi { symbol: String },

    /// `@comptime` - compile-time execution
    Comptime,

    /// `@restart{...}` - restart policy for actions
    Restart {
        policy: RestartPolicy,
        tries: Option<usize>,
        delay: Option<u64>,
    },
}

/// Cache strategy for memoization
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CacheStrategy {
    Default,
    Lru,
    Lfu,
    Disabled,
}

/// Restart policy for actions
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RestartPolicy {
    Always,
    OnFailure,
}

impl fmt::Display for Directive {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Directive::Test { description } => write!(f, "@test(\"{}\")", description),
            Directive::Parallel => write!(f, "@parallel"),
            Directive::Comutative => write!(f, "@comutative"),
            Directive::CacheStrategy { strategy, size, ttl } => {
                write!(f, "@cache_strategy{{strategy: {:?}", strategy)?;
                if let Some(s) = size {
                    write!(f, ", size: {}", s)?;
                }
                if let Some(t) = ttl {
                    write!(f, ", ttl: {}", t)?;
                }
                write!(f, "}}")
            }
            Directive::Ffi { symbol } => write!(f, "@ffi(\"{}\")", symbol),
            Directive::Comptime => write!(f, "@comptime"),
            Directive::Restart { policy, tries, delay } => {
                write!(f, "@restart{{policy: {:?}", policy)?;
                if let Some(t) = tries {
                    write!(f, ", tries: {}", t)?;
                }
                if let Some(d) = delay {
                    write!(f, ", delay: {}", d)?;
                }
                write!(f, "}}")
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ident_operator() {
        let plus = Ident::new("+");
        assert!(plus.is_operator());

        let func = Ident::new("minha_funcao");
        assert!(!func.is_operator());
    }

    #[test]
    fn test_ident_action() {
        let action = Ident::new("echo!");
        assert!(action.is_action());

        let func = Ident::new("soma");
        assert!(!func.is_action());
    }

    #[test]
    fn test_qualified_ident() {
        let simple = QualifiedIdent::simple("Int");
        assert!(simple.is_simple());
        assert_eq!(simple.to_string(), "Int");

        let qualified = QualifiedIdent::qualified("types", "NUM");
        assert!(!qualified.is_simple());
        assert_eq!(qualified.to_string(), "types::NUM");
    }

    #[test]
    fn test_literal_display() {
        assert_eq!(Literal::int("42").to_string(), "42");
        assert_eq!(Literal::string("hello").to_string(), "\"hello\"");
        assert_eq!(Literal::bool(true).to_string(), "True");
        assert_eq!(Literal::unit().to_string(), "()");
    }
}