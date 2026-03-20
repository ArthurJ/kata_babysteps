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
use super::common::{ident, token, between, ParserError, ParserSpan};
use super::literal::literal;
use super::r#type::type_expr;

/// Parse any pattern (including Cons)
pub fn pattern() -> impl Parser<SpannedToken, Pattern, Error = ParserError> + Clone {
    full_pattern()
}

/// Parse a base pattern (excluding top-level Cons, used for lambda arguments to avoid `:` collision)
pub fn base_pattern() -> impl Parser<SpannedToken, Pattern, Error = ParserError> + Clone {
    recursive(|base_pat| {
        let atom = choice((
            tuple_pattern(full_pattern()),
            list_pattern(full_pattern()),
            array_pattern(full_pattern()),
            variant_pattern(base_pat.clone()),
            range_pattern(),
            literal().map(Pattern::Literal),
            token(Token::Hole).map(|_| Pattern::Wildcard),
            ident().map(|s| {
                log::debug!("base_pattern: ident matched '{}'", s);
                Pattern::Var(Ident::new(s))
            }),
        ));

        // Typed pattern: pattern::Type (higher precedence than Or)
        let typed = atom.clone()
            .then(token(Token::DoubleColon).ignore_then(type_expr()).or_not())
            .map(|(pattern, type_annotation)| {
                match type_annotation {
                    Some(type_annotation) => Pattern::Typed {
                        pattern: Box::new(pattern),
                        type_annotation,
                    },
                    None => pattern,
                }
            });

        // Or pattern: p1 | p2 | p3 (lowest precedence)
        typed.clone()
            .then(token(Token::Pipe).ignore_then(typed.clone()).repeated())
            .map(|(first, rest): (Pattern, Vec<Pattern>)| {
                if rest.is_empty() {
                    first
                } else {
                    Pattern::Or(std::iter::once(first).chain(rest).collect())
                }
            })
    })
}

/// Parse a full pattern (including Cons)
pub fn full_pattern() -> impl Parser<SpannedToken, Pattern, Error = ParserError> + Clone {
    recursive(|pat| {
        // Atoms are the building blocks that don't have left recursion
        let atom = choice((
            tuple_pattern(pat.clone()),
            list_pattern(pat.clone()),
            array_pattern(pat.clone()),
            variant_pattern(pat.clone()),
            range_pattern(),
            literal().map(Pattern::Literal),
            token(Token::Hole).map(|_| Pattern::Wildcard),
            ident().map(|s| {
                log::debug!("full_pattern: ident matched '{}'", s);
                Pattern::Var(Ident::new(s))
            }),
        ));

        // Cons pattern: head : tail (right-associative)
        let cons = atom.clone()
            .then(token(Token::Colon).ignore_then(atom.clone()).repeated())
            .map(|(first, rest): (Pattern, Vec<Pattern>)| {
                if rest.is_empty() {
                    first
                } else {
                    let mut all = vec![first];
                    all.extend(rest);
                    
                    let mut it = all.into_iter().rev();
                    let last = it.next().unwrap();
                    it.fold(last, |tail, head| Pattern::Cons {
                        head: Box::new(head),
                        tail: Box::new(tail),
                    })
                }
            });

        // Typed pattern: pattern::Type (higher precedence than Or)
        let typed = cons.clone()
            .then(token(Token::DoubleColon).ignore_then(type_expr()).or_not())
            .map(|(pattern, type_annotation)| {
                match type_annotation {
                    Some(type_annotation) => Pattern::Typed {
                        pattern: Box::new(pattern),
                        type_annotation,
                    },
                    None => pattern,
                }
            });

        // Or pattern: p1 | p2 | p3 (lowest precedence)
        typed.clone()
            .then(token(Token::Pipe).ignore_then(typed.clone()).repeated())
            .map(|(first, rest): (Pattern, Vec<Pattern>)| {
                if rest.is_empty() {
                    first
                } else {
                    Pattern::Or(std::iter::once(first).chain(rest).collect())
                }
            })
    })
}

/// Parse a tuple pattern: (p1 p2 p3)
fn tuple_pattern(pat: impl Parser<SpannedToken, Pattern, Error = ParserError> + Clone + 'static)
    -> impl Parser<SpannedToken, Pattern, Error = ParserError> + Clone {
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
    .map(|patterns: Vec<Pattern>| {
        // In Kata, (x) is just x, but (x y) is a Tuple
        if patterns.len() == 1 {
            patterns.into_iter().next().unwrap()
        } else {
            Pattern::Tuple(patterns)
        }
    })
}

/// Parse a list pattern: [p1, p2, ...rest] or []
fn list_pattern(pat: impl Parser<SpannedToken, Pattern, Error = ParserError> + Clone + 'static)
    -> impl Parser<SpannedToken, Pattern, Error = ParserError> + Clone {
    token(Token::LBracket)
        .ignore_then(
            pat.clone()
                .then_ignore(token(Token::Comma).or_not())
                .repeated()
                .then(
                    token(Token::DotDotDot)
                        .ignore_then(pat.clone())
                        .or_not()
                )
                .then_ignore(token(Token::RBracket))
        )
        .map(|(elements, rest): (Vec<Pattern>, Option<Pattern>)| {
            Pattern::List {
                elements,
                rest: rest.map(Box::new),
            }
        })
}

/// Parse an array pattern: {p1, p2, p3}
fn array_pattern(pat: impl Parser<SpannedToken, Pattern, Error = ParserError> + Clone + 'static)
    -> impl Parser<SpannedToken, Pattern, Error = ParserError> + Clone {
    between(
        token(Token::LBrace),
        token(Token::RBrace),
        pat.separated_by(token(Token::Comma).or(token(Token::Newline)).ignored()).at_least(0)
    )
    .map(Pattern::Array)
}

/// Parse a variant pattern: Name or Name(args)
fn variant_pattern(pat: impl Parser<SpannedToken, Pattern, Error = ParserError> + Clone + 'static)
    -> impl Parser<SpannedToken, Pattern, Error = ParserError> + Clone {
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
        .map(|(name, args): (Ident, Option<Vec<Pattern>>)| {
            Pattern::Variant {
                name,
                args: args.unwrap_or_default(),
            }
        })
}

/// Parse a range pattern: [1..10] or [1..=10]
fn range_pattern() -> impl Parser<SpannedToken, Pattern, Error = ParserError> + Clone {
    token(Token::LBracket)
        .ignore_then(literal())
        .then(token(Token::DotDot).map(|_| false).or(token(Token::DotDotEqual).map(|_| true)))
        .then(literal())
        .then_ignore(token(Token::RBracket))
        .map(|((start, inclusive), end): ((Literal, bool), Literal)| {
            Pattern::Range { start, end, inclusive }
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::lexer::KataLexer;
    use super::super::common::convert_result;
    use super::super::error::ParseError;

    fn parse_pattern(source: &str) -> Result<Pattern, Vec<ParseError>> {
        let tokens = KataLexer::lex_with_indent(source)
            .map_err(|e| e.into_iter().map(|e| ParseError::new(e.to_string(), e.span().clone())).collect::<Vec<_>>())?;
        convert_result(pattern().parse(tokens))
    }

    #[test]
    fn test_wildcard() {
        let result = parse_pattern("_");
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), Pattern::Wildcard);
    }

    #[test]
    fn test_var_pattern() {
        let result = parse_pattern("x");
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), Pattern::Var(Ident::new("x")));
    }

    #[test]
    fn test_literal_pattern() {
        let result = parse_pattern("42");
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), Pattern::Literal(Literal::Int("42".to_string())));
    }

    #[test]
    fn test_tuple_pattern() {
        let result = parse_pattern("(a b c)");
        assert!(result.is_ok());
        let pat = result.unwrap();
        match pat {
            Pattern::Tuple(items) => assert_eq!(items.len(), 3),
            _ => panic!("Expected tuple pattern"),
        }
    }

    #[test]
    fn test_variant_pattern() {
        let result = parse_pattern("Ok(value)");
        assert!(result.is_ok());
        let pat = result.unwrap();
        match pat {
            Pattern::Variant { name, args } => {
                assert_eq!(name.0, "Ok");
                assert_eq!(args.len(), 1);
            }
            _ => panic!("Expected variant pattern"),
        }
    }

    #[test]
    fn test_unit_variant() {
        let result = parse_pattern("None");
        assert!(result.is_ok());
        let pat = result.unwrap();
        match pat {
            Pattern::Variant { name, args } => {
                assert_eq!(name.0, "None");
                assert!(args.is_empty());
            }
            _ => panic!("Expected variant pattern"),
        }
    }
}
