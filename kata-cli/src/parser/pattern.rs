//! Pattern parsers for Kata Language
//!
//! Parses patterns used in:
//! - Lambda clauses: λ (pattern): body
//! - Match expressions: match x { pattern: body }
//! - Destructuring: let (a b) = tuple

use chumsky::prelude::*;
use crate::lexer::{Token, SpannedToken};
use crate::ast::id::{Ident, Literal};
use crate::ast::pattern::Pattern;
use crate::ast::Spanned;
use super::common::{ident, pure_ident, token, between, ParserError, ParserSpan};
use super::literal::literal;
use super::r#type::type_expr;

/// Parse any pattern (including Cons)
pub fn pattern() -> impl Parser<SpannedToken, Spanned<Pattern>, Error = ParserError> + Clone {
    full_pattern()
}

/// Parse a base pattern (excluding top-level Cons, used for lambda arguments to avoid `:` collision)
pub fn base_pattern() -> impl Parser<SpannedToken, Spanned<Pattern>, Error = ParserError> + Clone {
    recursive(|base_pat| {
        let atom = choice((
            tuple_pattern(full_pattern()),
            list_pattern(full_pattern()),
            array_pattern(full_pattern()),
            variant_pattern(base_pat.clone()),
            range_pattern(),
            literal().map_with_span(|lit, span| Spanned::new(Pattern::Literal(lit), span.into())),
            token(Token::Hole).map_with_span(|_, span| Spanned::new(Pattern::Wildcard, span.into())),
            pure_ident().map_with_span(|s, span| {
                log::debug!("base_pattern: ident matched '{}'", s);
                Spanned::new(Pattern::Var(Ident::new(s)), span.into())
            }),
        ));

        // Typed pattern: pattern::Type (higher precedence than Or)
        let typed = atom.clone()
            .then(token(Token::DoubleColon).ignore_then(type_expr()).or_not())
            .map_with_span(|(pattern, type_annotation), span| {
                match type_annotation {
                    Some(type_annotation) => Spanned::new(Pattern::Typed {
                        pattern: Box::new(pattern),
                        type_annotation: type_annotation.node,
                    }, span.into()),
                    None => pattern,
                }
            });

        // Or pattern: p1 | p2 | p3 (lowest precedence)
        typed.clone()
            .then(token(Token::Pipe).ignore_then(typed.clone()).repeated())
            .map_with_span(|(first, rest): (Spanned<Pattern>, Vec<Spanned<Pattern>>), span| {
                if rest.is_empty() {
                    first
                } else {
                    Spanned::new(Pattern::Or(std::iter::once(first).chain(rest).collect()), span.into())
                }
            })
    })
}

/// Parse a full pattern (including Cons)
pub fn full_pattern() -> impl Parser<SpannedToken, Spanned<Pattern>, Error = ParserError> + Clone {
    recursive(|pat| {
        // Atoms are the building blocks that don't have left recursion
        let atom = choice((
            tuple_pattern(pat.clone()),
            list_pattern(pat.clone()),
            array_pattern(pat.clone()),
            variant_pattern(pat.clone()),
            range_pattern(),
            literal().map_with_span(|lit, span| Spanned::new(Pattern::Literal(lit), span.into())),
            token(Token::Hole).map_with_span(|_, span| Spanned::new(Pattern::Wildcard, span.into())),
            pure_ident().map_with_span(|s, span| {
                log::debug!("full_pattern: ident matched '{}'", s);
                Spanned::new(Pattern::Var(Ident::new(s)), span.into())
            }),
        ));

        // Cons pattern is now handled in list_pattern [head : tail].
        let cons = atom.clone();
        // Typed pattern: pattern::Type (higher precedence than Or)
        let typed = cons.clone()
            .then(token(Token::DoubleColon).ignore_then(type_expr()).or_not())
            .map_with_span(|(pattern, type_annotation), span| {
                match type_annotation {
                    Some(type_annotation) => Spanned::new(Pattern::Typed {
                        pattern: Box::new(pattern),
                        type_annotation: type_annotation.node,
                    }, span.into()),
                    None => pattern,
                }
            });

        // Or pattern: p1 | p2 | p3 (lowest precedence)
        typed.clone()
            .then(token(Token::Pipe).ignore_then(typed.clone()).repeated())
            .map_with_span(|(first, rest): (Spanned<Pattern>, Vec<Spanned<Pattern>>), span| {
                if rest.is_empty() {
                    first
                } else {
                    Spanned::new(Pattern::Or(std::iter::once(first).chain(rest).collect()), span.into())
                }
            })
    })
}

/// Parse a tuple pattern: (p1 p2 p3)
fn tuple_pattern(pat: impl Parser<SpannedToken, Spanned<Pattern>, Error = ParserError> + Clone + 'static)
    -> impl Parser<SpannedToken, Spanned<Pattern>, Error = ParserError> + Clone {
    let sep = token(Token::Comma).or(token(Token::Newline)).ignored().repeated().or_not();
    
    between(
        token(Token::LParen),
        token(Token::RParen),
        pat.clone()
            .padded_by(sep)
            .repeated()
            .map(|items| {
                log::debug!("tuple_pattern matched {} items", items.len());
                items
            })
    )
    .map_with_span(|patterns: Vec<Spanned<Pattern>>, span| {
        // In Kata, (x) is just x, but (x y) is a Tuple
        if patterns.len() == 1 {
            patterns.into_iter().next().unwrap()
        } else {
            Spanned::new(Pattern::Tuple(patterns), span.into())
        }
    })
}

/// Parse a list pattern: [p1, p2, ...rest] or [] or [head : tail]
fn list_pattern(pat: impl Parser<SpannedToken, Spanned<Pattern>, Error = ParserError> + Clone + 'static)
    -> impl Parser<SpannedToken, Spanned<Pattern>, Error = ParserError> + Clone {

    let normal_list = pat.clone()
        .then_ignore(token(Token::Comma).or_not())
        .repeated()
        .then(
            token(Token::DotDotDot)
                .ignore_then(pat.clone())
                .or_not()
        )
        .map(|(elements, rest)| {
            Pattern::List {
                elements,
                rest: rest.map(Box::new),
            }
        });

    let cons_list = pat.clone()
        .then(token(Token::Colon).ignore_then(pat.clone()).repeated().at_least(1))
        .map(|(first, rest)| {
            let mut all = vec![first];
            all.extend(rest);
            let mut it = all.into_iter().rev();
            let last = it.next().unwrap();
            let cons = it.fold(last, |tail, head| {
                let tail_span = tail.span.clone();
                Spanned::new(Pattern::Cons {
                    head: Box::new(head),
                    tail: Box::new(tail),
                }, tail_span)
            }); // using tail's span as fallback
            cons.node
        });

    token(Token::LBracket)
        .ignore_then(choice((cons_list, normal_list)))
        .then_ignore(token(Token::RBracket))
        .map_with_span(|pattern, span| Spanned::new(pattern, span.into()))
}

/// Parse an array pattern: {p1, p2, p3}
fn array_pattern(pat: impl Parser<SpannedToken, Spanned<Pattern>, Error = ParserError> + Clone + 'static)
    -> impl Parser<SpannedToken, Spanned<Pattern>, Error = ParserError> + Clone {
    between(
        token(Token::LBrace),
        token(Token::RBrace),
        pat.separated_by(token(Token::Comma).or(token(Token::Newline)).ignored()).at_least(0)
    )
    .map_with_span(|items, span| Spanned::new(Pattern::Array(items), span.into()))
}

/// Parse a variant pattern: Name or Name(args)
fn variant_pattern(pat: impl Parser<SpannedToken, Spanned<Pattern>, Error = ParserError> + Clone + 'static)
    -> impl Parser<SpannedToken, Spanned<Pattern>, Error = ParserError> + Clone {
    type_name()
        .then(
            between(
                token(Token::LParen),
                token(Token::RParen),
                pat.clone()
                    .separated_by(token(Token::Comma).or(token(Token::Newline)).ignored())
                    .at_least(0)
            )
            .or_not()
        )
        .map_with_span(|(name, args): (Ident, Option<Vec<Spanned<Pattern>>>), span| {
            Spanned::new(Pattern::Variant {
                name,
                args: args.unwrap_or_default(),
            }, span.into())
        })
}

/// Parse a range pattern: [1..10] or [1..=10]
fn range_pattern() -> impl Parser<SpannedToken, Spanned<Pattern>, Error = ParserError> + Clone {
    token(Token::LBracket)
        .ignore_then(literal())
        .then(token(Token::DotDot).map(|_| false).or(token(Token::DotDotEqual).map(|_| true)))
        .then(literal())
        .then_ignore(token(Token::RBracket))
        .map_with_span(|((start, inclusive), end): ((Literal, bool), Literal), span| {
            Spanned::new(Pattern::Range { start, end, inclusive }, span.into())
        })
}

/// Parse a type name (identifier starting with uppercase)
fn type_name() -> impl Parser<SpannedToken, Ident, Error = ParserError> + Clone {
    filter_map(|_span: ParserSpan, spanned: SpannedToken| {
        match &spanned.token {
            Token::Ident(s) if s.chars().next().map(|c| c.is_ascii_uppercase()).unwrap_or(false) => {
                Ok(Ident::new(s))
            }
            _ => Err(ParserError::custom(_span, "expected type name".to_string())),
        }
    })
}

// Helper for newline - matches Newline token or nothing
fn newline() -> impl Parser<SpannedToken, (), Error = ParserError> + Clone {
    token(Token::Newline).ignored()
}

