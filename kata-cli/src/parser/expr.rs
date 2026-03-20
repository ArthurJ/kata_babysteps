//! Expression parsers for Kata Language
//!
//! Uses single recursive() call with layered choice to avoid stack overflow

use chumsky::prelude::*;
use crate::lexer::{Token, SpannedToken};
use crate::ast::id::{Ident, QualifiedIdent};
use crate::ast::expr::{Expr, LambdaClause, GuardClause, GuardCondition, WithBinding};
use super::common::{ident, token, newline, indent, dedent, between, ParserError, ParserSpan};
use super::literal::literal;
use super::r#type::{type_expr, function_sig};

/// Parse a qualified identifier (Module::Item)
pub fn qualified_ident() -> impl Parser<SpannedToken, QualifiedIdent, Error = ParserError> + Clone {
    ident()
        .then(token(Token::DoubleColon).ignore_then(ident()).repeated())
        .map(|(first, rest)| {
            if rest.is_empty() {
                QualifiedIdent::simple(first)
            } else {
                let all_parts = std::iter::once(first).chain(rest).collect::<Vec<_>>();
                let name = all_parts.last().unwrap().clone();
                let module_parts = &all_parts[..all_parts.len() - 1];
                if module_parts.len() == 1 {
                    QualifiedIdent::qualified(&module_parts[0], name)
                } else {
                    QualifiedIdent::qualified(&module_parts.join("::"), name)
                }
            }
        })
}

/// Parse a simple identifier as an Ident
pub fn simple_ident() -> impl Parser<SpannedToken, Ident, Error = ParserError> + Clone {
    ident().map(Ident::new)
}

/// Match a specific identifier name
fn ident_named(name: &str) -> impl Parser<SpannedToken, String, Error = ParserError> + Clone {
    let name = name.to_string();
    filter_map(move |_span: ParserSpan, spanned: SpannedToken| {
        match &spanned.token {
            Token::Ident(s) if s == &name => Ok(s.clone()),
            _ => Err(ParserError::custom(_span, format!("expected '{}'", name))),
        }
    })
}

/// Parse any expression
/// Uses single recursive() call with all expression types in choice
pub fn expression() -> impl Parser<SpannedToken, Expr, Error = ParserError> + Clone {
    recursive(|expr| {
        // 1. Atoms and Groups (Primary)
        // Note: tuple and explicit_apply use the recursive 'expr' but we'll 
        // handle the application separation there.
        
        let atom = atom_parser(expr.clone());
        
        // Base parser for items inside tuples/lists/etc.
        // This MUST NOT include automatic application to avoid ambiguity
        // but it must be able to reach higher level constructs via parens.
        let item = recursive(|it| {
            let primary = choice((
                // Tuples and explicit apply use 'expr' or 'it'
                tuple_parser(it.clone()),
                range_parser(it.clone()),
                list_parser(it.clone()),
                braced_parser(it.clone()),
                dict_parser(it.clone()),
                set_parser(it.clone()),
                lambda_parser(it.clone()),
                explicit_apply_parser(it.clone()),
                atom.clone(),
            ));

            let field = primary
                .then(
                    token(Token::Dot)
                        .ignore_then(simple_ident())
                        .then(it.clone().repeated()) // Method args use 'it' to avoid greedy apply
                        .repeated()
                )
                .map(|(base, accesses)| {
                    accesses.into_iter().fold(base, |obj, (name, args)| {
                        if args.is_empty() {
                            Expr::Field { object: Box::new(obj), field: name }
                        } else {
                            Expr::Method { object: Box::new(obj), method: name, args }
                        }
                    })
                });

            let cons = field.clone()
                .then(token(Token::Colon).ignore_then(field.clone()).repeated())
                .map(|(first, rest)| {
                    if rest.is_empty() {
                        first
                    } else {
                        let mut all = vec![first];
                        all.extend(rest);
                        let mut it = all.into_iter().rev();
                        let last = it.next().unwrap();
                        it.fold(last, |tail, head| Expr::Cons {
                            head: Box::new(head),
                            tail: Box::new(tail),
                        })
                    }
                });

            let pipeline = cons.clone()
                .then(token(Token::Pipeline).ignore_then(cons.clone()).repeated())
                .map(|(first, rest)| {
                    rest.into_iter().fold(first, |acc, func| {
                        Expr::Pipeline { value: Box::new(acc), func: Box::new(func) }
                    })
                });
            
            pipeline
        });

        // Application: item arg1 arg2 ...
        // Automatic application is the highest level of expression
        item.clone()
            .then(item.clone().repeated())
            .map(|(func, args)| {
                if args.is_empty() {
                    func
                } else {
                    Expr::Apply {
                        func: Box::new(func),
                        args,
                    }
                }
            })
    })
}

/// Atom parser - highest precedence
fn atom_parser<E>(_expr: E) -> impl Parser<SpannedToken, Expr, Error = ParserError> + Clone
where
    E: Parser<SpannedToken, Expr, Error = ParserError> + Clone + 'static,
{
    choice((
        // Hole: _
        token(Token::Hole).map(|_| Expr::Hole),

        // At keyword parsed as a function identifier
        token(Token::At).map(|_| Expr::Var { name: Ident::new("at"), type_ascription: None }),
        token(Token::Channel).map(|_| Expr::Var { name: Ident::new("channel!"), type_ascription: None }),
        token(Token::Queue).map(|_| Expr::Var { name: Ident::new("queue!"), type_ascription: None }),
        token(Token::Broadcast).map(|_| Expr::Var { name: Ident::new("broadcast!"), type_ascription: None }),

        // Unit: ()
        token(Token::LParen)
            .then_ignore(token(Token::RParen))
            .map(|_| Expr::Tuple(vec![])),

        // Literals
        literal().map(Expr::Literal),

        // Variable or Qualified reference
        qualified_ident()
            .then(token(Token::DoubleColon).ignore_then(type_expr()).or_not())
            .map(|(qi, type_ascription)| {
                if qi.is_simple() {
                    Expr::Var {
                        name: Ident::new(qi.name.clone()),
                        type_ascription,
                    }
                } else {
                    // For now, qualified refs don't have ascription in the parser
                    // because Modulo::Item is already unambiguous.
                    Expr::QualifiedRef(qi)
                }
            }),
    ))
}

/// Tuple parser: (e1, e2, e3)
fn tuple_parser<E>(expr: E) -> impl Parser<SpannedToken, Expr, Error = ParserError> + Clone
where
    E: Parser<SpannedToken, Expr, Error = ParserError> + Clone + 'static,
{
    between(
        token(Token::LParen),
        token(Token::RParen),
        // For Kata's prefix notation, elements are separated by whitespace (consumed by lexer)
        // We just need to parse one or more expressions
        expr.clone().then(expr.clone().repeated())
            .map(|(first, rest): (Expr, Vec<Expr>)| {
                if rest.is_empty() {
                    vec![first]
                } else {
                    let mut items = vec![first];
                    items.extend(rest);
                    items
                }
            })
    ).map(|items: Vec<Expr>| {
        if items.len() == 1 {
            items.into_iter().next().unwrap()
        } else {
            Expr::Tuple(items)
        }
    })
}

/// Range parser: [start..end] or [start..step..end]
fn range_parser<E>(expr: E) -> impl Parser<SpannedToken, Expr, Error = ParserError> + Clone
where
    E: Parser<SpannedToken, Expr, Error = ParserError> + Clone + 'static,
{
    let range_op = choice((
        token(Token::DotDot).to(false),
        token(Token::DotDotEqual).to(true),
    ));

    between(
        token(Token::LBracket),
        token(Token::RBracket),
        expr.clone()
            .then(range_op.clone())
            .then(expr.clone())
            .then(range_op.then(expr.clone()).or_not())
    ).map(|(((start, op1_inclusive), middle), end_opt)| {
        if let Some((op2_inclusive, end_expr)) = end_opt {
            // [start..step..end]
            Expr::Range {
                start: Box::new(start),
                step: Some(Box::new(middle)),
                end: Box::new(end_expr),
                inclusive: op2_inclusive,
            }
        } else {
            // [start..end]
            Expr::Range {
                start: Box::new(start),
                step: None,
                end: Box::new(middle),
                inclusive: op1_inclusive,
            }
        }
    })
}

/// List parser: [e1, e2, e3] or [e1 e2 e3]
fn list_parser<E>(expr: E) -> impl Parser<SpannedToken, Expr, Error = ParserError> + Clone
where
    E: Parser<SpannedToken, Expr, Error = ParserError> + Clone + 'static,
{
    let sep = token(Token::Comma).ignored().or(newline().ignored()).or_not();
    between(
        token(Token::LBracket),
        token(Token::RBracket),
        expr.clone().padded_by(sep.clone()).repeated()
    ).map(Expr::List)
}

/// Parser for braced expressions: {e1 e2} (Array) or {e1 e2 ; e3 e4} (Tensor)
fn braced_parser<E>(expr: E) -> impl Parser<SpannedToken, Expr, Error = ParserError> + Clone
where
    E: Parser<SpannedToken, Expr, Error = ParserError> + Clone + 'static,
{
    let item_sep = choice((
        token(Token::Comma).ignored(),
        newline().ignored(),
        indent().ignored(),
        dedent().ignored(),
    )).repeated().or_not();
    
    // A "row" is a sequence of expressions
    let row = expr.clone()
        .padded_by(item_sep)
        .repeated();

    between(
        token(Token::LBrace),
        token(Token::RBrace),
        row.clone().then(token(Token::Semicolon).ignore_then(row).repeated())
    )
    .map(|(first_row, other_rows): (Vec<Expr>, Vec<Vec<Expr>>)| {
        if other_rows.is_empty() {
            // No semicolons - simple Array
            Expr::Array(first_row)
        } else {
            // Semicolons present - it's a Tensor
            let mut elements = first_row.clone();
            let mut row_count = 1;
            let col_count = first_row.len();
            
            for next_row in other_rows {
                if !next_row.is_empty() {
                    elements.extend(next_row);
                    row_count += 1;
                }
            }
            
            // For now support 2D tensors (matrices)
            // dimensions = [rows, cols]
            // Validation of consistent row lengths is deferred to the Type Checker
            // to provide better error messages and spans.
            Expr::Tensor {
                dimensions: vec![row_count, col_count],
                elements,
            }
        }
    })
}

/// Dictionary parser: Dict [(key value)]
fn dict_parser<E>(expr: E) -> impl Parser<SpannedToken, Expr, Error = ParserError> + Clone
where
    E: Parser<SpannedToken, Expr, Error = ParserError> + Clone + 'static,
{
    let sep = token(Token::Comma).ignored().or(newline().ignored()).or_not();
    ident_named("Dict")
        .then_ignore(token(Token::LBracket))
        .then(
            between(
                token(Token::LParen),
                token(Token::RParen),
                expr.clone().then_ignore(newline().or_not()).then(expr.clone())
            ).padded_by(sep.clone()).repeated()
        )
        .then_ignore(token(Token::RBracket))
        .map(|(_, entries)| Expr::Dict(entries))
}

/// Set parser: Set [e1, e2, e3]
fn set_parser<E>(expr: E) -> impl Parser<SpannedToken, Expr, Error = ParserError> + Clone
where
    E: Parser<SpannedToken, Expr, Error = ParserError> + Clone + 'static,
{
    let sep = token(Token::Comma).ignored().or(newline().ignored()).or_not();
    ident_named("Set")
        .then_ignore(token(Token::LBracket))
        .then(expr.clone().padded_by(sep.clone()).repeated())
        .then_ignore(token(Token::RBracket))
        .map(|(_, items)| Expr::Set(items))
}

/// Lambda parser: lambda (pattern) body
fn lambda_parser<E>(expr: E) -> impl Parser<SpannedToken, Expr, Error = ParserError> + Clone
where
    E: Parser<SpannedToken, Expr, Error = ParserError> + Clone + 'static,
{
    let lambda_keyword = choice((
        token(Token::Lambda).map(|_| ()),
        ident_named("lambda").map(|_| ()),
        ident_named("λ").map(|_| ()),
    ));

    let pattern_parser = super::pattern::base_pattern().padded_by(newline().repeated().or_not()).repeated();

    let with_binding = choice((
        // 1. Signature constraint: `+ :: A B => C`
        simple_ident()
            .then_ignore(token(Token::DoubleColon))
            .then(function_sig())
            .map(|(name, sig)| WithBinding::Signature { name, sig }),

        // 2. Interface constraint: `T implements ORD`
        // We use type_expr() for the type on the left
        type_expr()
            .then_ignore(token(Token::Implements))
            .then(simple_ident())
            .map(|(typ, interface)| WithBinding::Interface { typ, interface }),

        // 3. Value binding: `name as expr`
        simple_ident()
            .then_ignore(token(Token::As))
            .then(expr.clone())
            .map(|(name, value)| WithBinding::Value { name, value }),
    ));

    let with_bindings = token(Token::With)
        .ignore_then(newline().repeated().or_not())
        .ignore_then(indent())
        .ignore_then(with_binding.padded_by(newline().repeated().or_not()).repeated())
        .then_ignore(newline().repeated().or_not())
        .then_ignore(dedent());

    let body_parser = choice((
        newline().repeated().at_least(1).ignore_then(indent())
            .ignore_then(expr.clone().then(newline().repeated().or_not().ignore_then(with_bindings.clone()).or_not()))
            .then_ignore(newline().repeated().or_not()).then_ignore(dedent()),
        expr.clone().then(newline().repeated().or_not().ignore_then(with_bindings.clone()).or_not()),
    ));

    let guard_clause = choice((
        simple_ident().map(|i| (i.clone(), GuardCondition::Named(i))),
        token(Token::Otherwise).map(|_| (Ident::new("otherwise"), GuardCondition::Otherwise)),
    ))
        .then_ignore(token(Token::Colon))
        .then(expr.clone())
        .map(|((label, guard), body)| GuardClause {
            label,
            guard,
            body,
        });

    let guards_and_with = newline().repeated().at_least(1)
        .ignore_then(indent())
        .ignore_then(
            guard_clause
                .then_ignore(newline().repeated().or_not())
                .repeated().at_least(1)
        )
        .then(with_bindings.clone().or_not())
        .then_ignore(newline().repeated().or_not())
        .then_ignore(dedent());

    lambda_keyword
        .ignore_then(pattern_parser)
        .then_ignore(token(Token::Colon))
        .then(choice((
            // Either guards (with optional 'with')
            guards_and_with.map(|(guards, with_opt)| (Some(guards), with_opt, None)),
            // Or normal body (with optional 'with')
            body_parser.map(|(body, with_opt)| (None, with_opt, Some(body))),
        )))
        .try_map(|(patterns, (guards_opt, with_opt, body_opt)): (Vec<crate::ast::pattern::Pattern>, (Option<Vec<GuardClause>>, Option<Vec<WithBinding>>, Option<Expr>)), span| {
            let guards = guards_opt.unwrap_or_default();
            let with_bindings = with_opt.unwrap_or_default();
            
            if body_opt.is_none() && guards.is_empty() {
                return Err(ParserError::custom(span, "Lambda clause must have either a body or guard clauses".to_string()));
            }
            
            let clause = LambdaClause {
                patterns,
                guards,
                body: body_opt,
                with: with_bindings,
            };
            Ok(Expr::Lambda { clauses: vec![clause] })
        })
}

/// Explicit application parser: $(func arg1 arg2)
fn explicit_apply_parser<E>(expr: E) -> impl Parser<SpannedToken, Expr, Error = ParserError> + Clone
where
    E: Parser<SpannedToken, Expr, Error = ParserError> + Clone + 'static,
{
    token(Token::Dollar)
        .ignore_then(between(
            token(Token::LParen),
            token(Token::RParen),
            expr.clone().then(expr.clone().repeated())
        ))
        .map(|(func, args)| Expr::ExplicitApply {
            func: Box::new(func),
            args,
        })
}

/// Parse a simple atom expression
pub fn simple_expression() -> impl Parser<SpannedToken, Expr, Error = ParserError> + Clone {
    atom_parser(expression())
}
