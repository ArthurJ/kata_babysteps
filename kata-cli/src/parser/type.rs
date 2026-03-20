//! Type expression parsers for Kata Language
//!
//! Parses type expressions:
//! - Named types: Int, Float, Text
//! - Generic types: List::T, Result::T::E
//! - Type variables: T, K, V
//! - Tuple types: (A B C)
//! - Function types: A -> B, (A B) -> C
//! - Refined types: (Int, > _ 0)
//!
//! Architecture:
//! - Layer 0: Independent parsers (no recursion needed)
//! - Layer 1: Self-contained recursive parsers (use recursive() internally)
//! - Top-level: type_expr() orchestrates everything

use chumsky::prelude::*;
use crate::lexer::{Token, SpannedToken};
use crate::ast::id::{Ident, QualifiedIdent};
use crate::ast::types::{Type, FunctionSig, Predicate, CompareOp, LiteralValue};
use super::common::{ident, token, between, ParserError, ParserSpan};

// ============================================================================
// LAYER 0: INDEPENDENT PARSERS (No recursion needed)
// ============================================================================

/// Parse a type name (identifier starting with uppercase or qualified)
fn type_name() -> impl Parser<SpannedToken, QualifiedIdent, Error = ParserError> + Clone {
    // Single uppercase name
    let simple = filter_map(|_span: ParserSpan, spanned: SpannedToken| {
        match &spanned.token {
            Token::Ident(s) if s.chars().next().map(|c| c.is_ascii_uppercase()).unwrap_or(false) => {
                log::debug!("type_name: simple matched '{}'", s);
                Ok(QualifiedIdent::simple(s))
            }
            _ => Err(ParserError::custom(_span, "expected type name".to_string())),
        }
    });

    // Qualified name: Module::Item
    let qualified = ident()
        .then(token(Token::DoubleColon).ignore_then(ident()).repeated().at_least(1))
        .map(|(first, rest): (String, Vec<String>)| {
            let all: Vec<_> = std::iter::once(first).chain(rest).collect();
            let name = all.last().unwrap().clone();
            let module = all[..all.len() - 1].join("::");
            log::debug!("type_name: qualified matched '{}::{}'", module, name);
            QualifiedIdent::qualified(&module, name)
        });

    choice((qualified, simple))
}

/// Parse a type variable (lowercase identifier)
fn type_var() -> impl Parser<SpannedToken, Ident, Error = ParserError> + Clone {
    filter_map(|_span: ParserSpan, spanned: SpannedToken| {
        match &spanned.token {
            Token::Ident(s) if s.chars().next().map(|c| c.is_ascii_lowercase()).unwrap_or(true) && s != "_" => {
                Ok(Ident::new(s))
            }
            _ => Err(ParserError::custom(_span, "expected type variable".to_string())),
        }
    })
}

/// Parse a simple named type or type variable
fn simple_type() -> impl Parser<SpannedToken, Type, Error = ParserError> + Clone {
    choice((
        // Type name (starts with uppercase): Int, Float, Text
        type_name().map(|name| Type::Named {
            name,
            params: vec![],
        }),

        // Type variable (lowercase): T, K, V
        type_var().map(Type::Var),
    ))
}

/// Parse a generic type with parameters: Name::T::E
/// Uses recursive() internally for nested generics
fn generic_type() -> impl Parser<SpannedToken, Type, Error = ParserError> + Clone {
    recursive(|arg| {
        type_name()
            .then(token(Token::DoubleColon).ignore_then(type_arg(arg)).repeated())
            .map(|(name, params): (QualifiedIdent, Vec<Type>)| {
                if params.is_empty() {
                    Type::Named { name, params: vec![] }
                } else {
                    Type::Named { name, params }
                }
            })
    })
}

/// Parse a type argument for generics (recursive)
/// Takes the recursive parser as parameter
fn type_arg(arg: Recursive<'_, SpannedToken, Type, ParserError>)
    -> impl Parser<SpannedToken, Type, Error = ParserError> + Clone + use<'_> {
    choice((
        // Nested generic: Name::T
        type_name()
            .then(token(Token::DoubleColon).ignore_then(arg.clone()).repeated())
            .map(|(name, params): (QualifiedIdent, Vec<Type>)| {
                Type::Named { name, params }
            }),
        // Type variable: T
        type_var().map(Type::Var),
        // List Type: [T]
        between(
            token(Token::LBracket),
            token(Token::RBracket),
            arg.clone()
        ).map(|t| Type::Named {
            name: QualifiedIdent::simple("List"),
            params: vec![t]
        }),
        // Tuple: (A B) - uses the recursive arg directly, not type_expr_inner
        between(
            token(Token::LParen),
            token(Token::RParen),
            arg.clone()
                .then(arg.clone().repeated())
                .map(|(first, rest): (Type, Vec<Type>)| {
                    if rest.is_empty() {
                        first
                    } else {
                        let mut types = vec![first];
                        types.extend(rest);
                        Type::Tuple(types)
                    }
                })
        ),
    ))
}

/// Inner type expression (used for parsing inside parens)
/// Self-contained with its own recursive()
fn type_expr_inner() -> impl Parser<SpannedToken, Type, Error = ParserError> + Clone {
    recursive(|typ| {
        let base = choice((
            list_type_inner(typ.clone()),
            tuple_type_inner(typ.clone()),
            generic_type(),
            type_var().map(Type::Var),
        ));

        choice((
            // Function type: base -> base -> ...
            function_type_inner(base.clone()),
            // Otherwise just the base
            base,
        ))
    })
}

// ============================================================================
// LAYER 1: SELF-CONTAINED RECURSIVE PARSERS
// ============================================================================

/// Parse a list type: [A]
fn list_type_inner(type_inner: impl Parser<SpannedToken, Type, Error = ParserError> + Clone + 'static)
    -> impl Parser<SpannedToken, Type, Error = ParserError> + Clone {
    between(
        token(Token::LBracket),
        token(Token::RBracket),
        type_inner
    )
    .map(|t| Type::Named {
        name: QualifiedIdent::simple("List"),
        params: vec![t]
    })
}

/// Parse a tuple type: (A B C) or (A, B, C)
fn tuple_type_inner(type_inner: impl Parser<SpannedToken, Type, Error = ParserError> + Clone + 'static)
    -> impl Parser<SpannedToken, Type, Error = ParserError> + Clone {
    let sep = choice((
        token(Token::Comma).ignored(),
        newline().ignored(),
    )).repeated().or_not();

    between(
        token(Token::LParen),
        token(Token::RParen),
        type_inner.clone()
            .then(sep.clone().ignore_then(type_inner.clone()).repeated())
            .or_not()
    )
    .map(|opt_types| {
        match opt_types {
            None => Type::Tuple(vec![]),
            Some((first, rest)) => {
                if rest.is_empty() {
                    first
                } else {
                    let mut types = vec![first];
                    types.extend(rest);
                    Type::Tuple(types)
                }
            }
        }
    })
}

/// Parse a function type: A B -> C
fn function_type_inner(base_type: impl Parser<SpannedToken, Type, Error = ParserError> + Clone + 'static)
    -> impl Parser<SpannedToken, Type, Error = ParserError> + Clone {
    base_type.clone()
        .then(token(Token::SimpleArrow).ignore_then(base_type).repeated().at_least(1))
        .map(|(first, rest): (Type, Vec<Type>)| {
            // Chain function types: A -> B -> C means A -> (B -> C)
            // But Kata-lang might want (A B) -> C? 
            // The spec says A B -> C is multi-argument.
            // Let's implement right-associative for now as is common: A -> B -> C is A -> (B -> C)
            let mut all = vec![first];
            all.extend(rest);
            
            let mut it = all.into_iter().rev();
            let last = it.next().unwrap();
            it.fold(last, |ret, arg| {
                Type::Function {
                    params: vec![arg],
                    return_type: Box::new(ret),
                }
            })
        })
}

/// Parse a refined type: (Type, predicate1, predicate2, ...)
fn refined_type_inner(type_inner: impl Parser<SpannedToken, Type, Error = ParserError> + Clone + 'static)
    -> impl Parser<SpannedToken, Type, Error = ParserError> + Clone {
    between(
        token(Token::LParen),
        token(Token::RParen),
        type_inner.clone()
            .then(
                token(Token::Comma)
                    .ignore_then(predicate())
                    .repeated()
                    .at_least(1)
            )
    )
    .map(|(base, predicates): (Type, Vec<Predicate>)| {
        let predicate = if predicates.len() == 1 {
            predicates.into_iter().next().unwrap()
        } else {
            Predicate::And(predicates)
        };
        
        Type::Refined {
            base: Box::new(base),
            predicate,
        }
    })
}

// ============================================================================
// TOP-LEVEL TYPE PARSER
// ============================================================================

/// Parse any type expression
/// Tries complex types first to avoid partial matches
pub fn type_expr() -> impl Parser<SpannedToken, Type, Error = ParserError> + Clone {
    recursive(|typ| {
        let base = choice((
            refined_type_inner(typ.clone()),
            list_type_inner(typ.clone()),
            tuple_type_inner(typ.clone()),
            generic_type(),
            type_var().map(Type::Var),
        ));

        choice((
            function_type_inner(base.clone()),
            base,
        ))
    })
    .map(|t| {
        log::debug!("type_expr matched: {}", t);
        t
    })
}

/// Recursive type parser for complex types
fn recursive_type() -> impl Parser<SpannedToken, Type, Error = ParserError> + Clone {
    recursive(|typ| {
        choice((
            // Function type: A B -> C
            function_type_inner(typ.clone()),
            // List type: [A]
            list_type_inner(typ.clone()),
            // Tuple type: (A B C)
            tuple_type_inner(typ.clone()),
            // Generic type or simple name
            generic_type(),
            // Type variable
            type_var().map(Type::Var),
        ))
    })
}

/// Parse a tuple type (public version)
pub fn tuple_type(type_inner: impl Parser<SpannedToken, Type, Error = ParserError> + Clone + 'static)
    -> impl Parser<SpannedToken, Type, Error = ParserError> + Clone {
    tuple_type_inner(type_inner)
}

/// Parse a function type (public version)
pub fn function_type(type_inner: impl Parser<SpannedToken, Type, Error = ParserError> + Clone + 'static)
    -> impl Parser<SpannedToken, Type, Error = ParserError> + Clone {
    function_type_inner(type_inner)
}

// ============================================================================
// PREDICATE PARSERS
// ============================================================================

/// Parse a predicate for refined types
fn predicate() -> impl Parser<SpannedToken, Predicate, Error = ParserError> + Clone {
    choice((
        // Comparison: > _ 0, < _ 10, etc.
        compare_op()
            .then_ignore(token(Token::Hole))
            .then(literal_value())
            .map(|(op, value)| Predicate::Comparison { op, value }),

        // Except: except TypeName
        token(Token::Except)
            .ignore_then(type_name())
            .map(Predicate::Except),

        // Range: <= _ 25.0
        compare_op()
            .then_ignore(token(Token::Hole))
            .then(literal_value())
            .map(|(op, value)| Predicate::Range { op, value }),
    ))
}

/// Parse a comparison operator
fn compare_op() -> impl Parser<SpannedToken, CompareOp, Error = ParserError> + Clone {
    choice((
        ident_named(">").map(|_| CompareOp::Gt),
        ident_named(">=").map(|_| CompareOp::Gte),
        ident_named("<").map(|_| CompareOp::Lt),
        ident_named("<=").map(|_| CompareOp::Lte),
        ident_named("=").map(|_| CompareOp::Eq),
        ident_named("!=").map(|_| CompareOp::Neq),
    ))
}

/// Parse a literal value for predicates
fn literal_value() -> impl Parser<SpannedToken, LiteralValue, Error = ParserError> + Clone {
    filter_map(|_span: ParserSpan, spanned: SpannedToken| {
        match &spanned.token {
            Token::Int(s) => s.parse().map(LiteralValue::Int).map_err(|_| {
                ParserError::custom(_span, "expected integer".to_string())
            }),
            Token::Float(s) => Ok(LiteralValue::Float(s.clone())),
            Token::Ident(s) if s == "True" => Ok(LiteralValue::Bool(true)),
            Token::Ident(s) if s == "False" => Ok(LiteralValue::Bool(false)),
            _ => Err(ParserError::custom(_span, "expected literal value".to_string())),
        }
    })
}

/// Parse a function signature: Arg1 Arg2 => Return
pub fn function_sig() -> impl Parser<SpannedToken, FunctionSig, Error = ParserError> + Clone {
    let arg_sep = choice((
        token(Token::Comma).ignored(),
        newline().ignored(),
    )).repeated().or_not();

    type_expr()
        .then_ignore(arg_sep.clone())
        .repeated()
        .then_ignore(token(Token::Arrow))
        .then(type_expr())
        .map(|(params, return_type)| {
            log::debug!("function_sig matched with {} params", params.len());
            FunctionSig::new(params, return_type)
        })
}

// Helper for newline
fn newline() -> impl Parser<SpannedToken, (), Error = ParserError> + Clone {
    token(Token::Newline).ignored()
}

// Helper for matching identifier name
fn ident_named(name: &str) -> impl Parser<SpannedToken, String, Error = ParserError> + Clone {
    let name = name.to_string();
    filter_map(move |_span: ParserSpan, spanned: SpannedToken| {
        match &spanned.token {
            Token::Ident(s) if s == &name => Ok(s.clone()),
            _ => Err(ParserError::custom(_span, format!("expected '{}'", name))),
        }
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::lexer::KataLexer;
    use super::super::common::convert_result;
    use super::super::error::ParseError;

    fn parse_type(source: &str) -> Result<Type, Vec<ParseError>> {
        let tokens = KataLexer::lex_with_indent(source)
            .map_err(|e| e.into_iter().map(|e| ParseError::new(e.to_string(), e.span().clone())).collect::<Vec<_>>())?;
        convert_result(type_expr().parse(tokens))
    }

    #[test]
    fn test_simple_type() {
        let result = parse_type("Int");
        assert!(result.is_ok());
        assert!(matches!(result.unwrap(), Type::Named { .. }));
    }

    #[test]
    fn test_type_variable() {
        // In Kata, T is CamelCase and indistinguishable from a named type at parse time
        let result = parse_type("T");
        assert!(result.is_ok());
        assert!(matches!(result.unwrap(), Type::Named { .. }));
    }

    #[test]
    fn test_tuple_type() {
        let result = parse_type("(Int Float Text)");
        assert!(result.is_ok());
        match result.unwrap() {
            Type::Tuple(types) => assert_eq!(types.len(), 3),
            _ => panic!("Expected tuple type"),
        }
    }

    #[test]
    fn test_function_type() {
        let result = parse_type("Int -> Float");
        assert!(result.is_ok());
        match result.unwrap() {
            Type::Function { params, return_type } => {
                assert_eq!(params.len(), 1);
                assert!(matches!(*return_type, Type::Named { .. }));
            }
            _ => panic!("Expected function type"),
        }
    }
}
