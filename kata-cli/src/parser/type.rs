//! Type expression parsers for Kata Language
//!
//! Parses type expressions:
//! - Named types: Int, Float, Text
//! - Generic types: List::T, Result::T::E
//! - Type variables: T, K, V
//! - Tuple types: (A B C)
//! - Function types: A -> B, (A B) -> C
//! - Refined types: (Int, > _ 0)

use chumsky::prelude::*;
use crate::lexer::{Token, SpannedToken};
use crate::ast::id::{Ident, QualifiedIdent};
use crate::ast::types::{Type, FunctionSig, Predicate, CompareOp, LiteralValue};
use crate::ast::Spanned;
use super::common::{ident, token, between, ParserError, ParserSpan};

// ============================================================================
// LAYER 0: INDEPENDENT PARSERS (No recursion needed)
// ============================================================================

/// Parse a type name (identifier starting with uppercase or qualified)
fn type_name() -> impl Parser<SpannedToken, QualifiedIdent, Error = ParserError> + Clone {
    let simple = filter_map(|_span: ParserSpan, spanned: SpannedToken| {
        match &spanned.token {
            Token::Ident(s) if s.chars().next().map(|c| c.is_ascii_uppercase()).unwrap_or(false) => {
                Ok(QualifiedIdent::simple(s))
            }
            _ => Err(ParserError::custom(_span, "expected type name".to_string())),
        }
    });

    let qualified = ident()
        .then(token(Token::DoubleColon).ignore_then(ident()).repeated().at_least(1))
        .map(|(first, rest): (String, Vec<String>)| {
            let all: Vec<_> = std::iter::once(first).chain(rest).collect();
            let name = all.last().unwrap().clone();
            let module = all[..all.len() - 1].join("::");
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

/// Parse a generic type with parameters: Name::T::E
fn generic_type() -> impl Parser<SpannedToken, Spanned<Type>, Error = ParserError> + Clone {
    recursive(|arg| {
        type_name()
            .then(token(Token::DoubleColon).ignore_then(type_arg(arg)).repeated())
            .map_with_span(|(name, params): (QualifiedIdent, Vec<Spanned<Type>>), span| {
                let node = if params.is_empty() {
                    Type::Named { name, params: vec![] }
                } else {
                    Type::Named { name, params: params.into_iter().map(|p| p.node).collect() }
                };
                Spanned::new(node, span.into())
            })
    })
}

/// Parse a type argument for generics (recursive)
fn type_arg(arg: Recursive<'_, SpannedToken, Spanned<Type>, ParserError>)
    -> impl Parser<SpannedToken, Spanned<Type>, Error = ParserError> + Clone + use<'_> {
    choice((
        type_name()
            .then(token(Token::DoubleColon).ignore_then(arg.clone()).repeated())
            .map_with_span(|(name, params): (QualifiedIdent, Vec<Spanned<Type>>), span| {
                Spanned::new(Type::Named { name, params: params.into_iter().map(|p| p.node).collect() }, span.into())
            }),
        type_var().map_with_span(|v, span| Spanned::new(Type::Var(v), span.into())),
        between(
            token(Token::LBracket),
            token(Token::RBracket),
            arg.clone()
        ).map_with_span(|t, span| Spanned::new(Type::Named {
            name: QualifiedIdent::simple("List"),
            params: vec![t.node]
        }, span.into())),
        between(
            token(Token::LParen),
            token(Token::RParen),
            arg.clone()
                .then(arg.clone().repeated())
                .map_with_span(|(first, rest): (Spanned<Type>, Vec<Spanned<Type>>), span| {
                    if rest.is_empty() {
                        first
                    } else {
                        let mut types = vec![first.node];
                        types.extend(rest.into_iter().map(|p| p.node));
                        Spanned::new(Type::Tuple(types), span.into())
                    }
                })
        ),
    ))
}

// ============================================================================
// LAYER 1: SELF-CONTAINED RECURSIVE PARSERS
// ============================================================================

fn list_type_inner(type_inner: impl Parser<SpannedToken, Spanned<Type>, Error = ParserError> + Clone + 'static)
    -> impl Parser<SpannedToken, Spanned<Type>, Error = ParserError> + Clone {
    between(
        token(Token::LBracket),
        token(Token::RBracket),
        type_inner
    )
    .map_with_span(|t, span| Spanned::new(Type::Named {
        name: QualifiedIdent::simple("List"),
        params: vec![t.node]
    }, span.into()))
}

fn tuple_type_inner(type_inner: impl Parser<SpannedToken, Spanned<Type>, Error = ParserError> + Clone + 'static)
    -> impl Parser<SpannedToken, Spanned<Type>, Error = ParserError> + Clone {
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
    .map_with_span(|opt_types, span| {
        let node = match opt_types {
            None => Type::Tuple(vec![]),
            Some((first, rest)) => {
                if rest.is_empty() {
                    return first;
                } else {
                    let mut types = vec![first.node];
                    types.extend(rest.into_iter().map(|p| p.node));
                    Type::Tuple(types)
                }
            }
        };
        Spanned::new(node, span.into())
    })
}

fn function_type_inner(base_type: impl Parser<SpannedToken, Spanned<Type>, Error = ParserError> + Clone + 'static)
    -> impl Parser<SpannedToken, Spanned<Type>, Error = ParserError> + Clone {
    base_type.clone()
        .then(token(Token::SimpleArrow).ignore_then(base_type).repeated().at_least(1))
        .map_with_span(|(first, rest): (Spanned<Type>, Vec<Spanned<Type>>), span| {
            let mut all = vec![first.node];
            all.extend(rest.into_iter().map(|p| p.node));
            
            let mut it = all.into_iter().rev();
            let last = it.next().unwrap();
            let node = it.fold(last, |ret, arg| {
                Type::Function {
                    params: vec![arg],
                    return_type: Box::new(ret),
                }
            });
            Spanned::new(node, span.into())
        })
}

fn refined_type_inner(type_inner: impl Parser<SpannedToken, Spanned<Type>, Error = ParserError> + Clone + 'static)
    -> impl Parser<SpannedToken, Spanned<Type>, Error = ParserError> + Clone {
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
    .map_with_span(|(base, predicates): (Spanned<Type>, Vec<Predicate>), span| {
        let predicate = if predicates.len() == 1 {
            predicates.into_iter().next().unwrap()
        } else {
            Predicate::And(predicates)
        };
        
        Spanned::new(Type::Refined {
            base: Box::new(base.node),
            predicate,
        }, span.into())
    })
}

// ============================================================================
// TOP-LEVEL TYPE PARSER
// ============================================================================

pub fn type_expr() -> impl Parser<SpannedToken, Spanned<Type>, Error = ParserError> + Clone {
    recursive(|typ| {
        let base = choice((
            refined_type_inner(typ.clone()),
            list_type_inner(typ.clone()),
            tuple_type_inner(typ.clone()),
            generic_type(),
            type_var().map_with_span(|v, span| Spanned::new(Type::Var(v), span.into())),
        ));

        choice((
            function_type_inner(base.clone()),
            base,
        ))
    })
}

// ============================================================================
// PREDICATE PARSERS
// ============================================================================

fn predicate() -> impl Parser<SpannedToken, Predicate, Error = ParserError> + Clone {
    choice((
        compare_op()
            .then_ignore(token(Token::Hole))
            .then(literal_value())
            .map(|(op, value)| Predicate::Comparison { op, value }),

        token(Token::Except)
            .ignore_then(type_name())
            .map(Predicate::Except),

        compare_op()
            .then_ignore(token(Token::Hole))
            .then(literal_value())
            .map(|(op, value)| Predicate::Range { op, value }),
    ))
}

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
            FunctionSig::new(params.into_iter().map(|p| p.node).collect(), return_type.node)
        })
}

fn newline() -> impl Parser<SpannedToken, (), Error = ParserError> + Clone {
    token(Token::Newline).ignored()
}

fn ident_named(name: &str) -> impl Parser<SpannedToken, String, Error = ParserError> + Clone {
    let name = name.to_string();
    filter_map(move |_span: ParserSpan, spanned: SpannedToken| {
        match &spanned.token {
            Token::Ident(s) if s == &name => Ok(s.clone()),
            _ => Err(ParserError::custom(_span, format!("expected '{}'", name))),
        }
    })
}

