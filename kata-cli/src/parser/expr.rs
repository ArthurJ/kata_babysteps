//! Expression parsers for Kata Language
//!
//! Uses single recursive() call with layered choice to avoid stack overflow

use chumsky::prelude::*;
use crate::lexer::{Token, SpannedToken};
use crate::ast::id::{Ident, QualifiedIdent};
use crate::ast::expr::{Expr, LambdaClause, GuardClause, GuardCondition, WithBinding};
use crate::ast::Spanned;
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
pub fn expression() -> impl Parser<SpannedToken, Spanned<Expr>, Error = ParserError> + Clone {
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
                .map_with_span(|(base, accesses), span| {
                    accesses.into_iter().fold(base, |obj, (name, args)| {
                        let span_inner = obj.span; // Fold keeps outer span or we should expand it?
                        // For now keep original span for simplicity or map to full
                        if args.is_empty() {
                            Spanned::new(Expr::Field { object: Box::new(obj), field: name }, span.clone().into())
                        } else {
                            Spanned::new(Expr::Method { object: Box::new(obj), method: name, args }, span.clone().into())
                        }
                    })
                });

            let cons = field.clone();

            let pipeline = cons.clone()
                .then(token(Token::Pipeline).ignore_then(cons.clone()).repeated())
                .map_with_span(|(first, rest), span| {
                    rest.into_iter().fold(first, |acc, func| {
                        Spanned::new(Expr::Pipeline { value: Box::new(acc), func: Box::new(func) }, span.clone().into())
                    })
                });
            
            pipeline
        });

        // Application: item arg1 arg2 ...
        // Automatic application is the highest level of expression
        item.clone()
            .then(item.clone().repeated())
            .map_with_span(|(func, args), span| {
                if args.is_empty() {
                    func
                } else {
                    Spanned::new(Expr::Apply {
                        func: Box::new(func),
                        args,
                    }, span.into())
                }
            })
    })
}

/// Atom parser - highest precedence
fn atom_parser<E>(_expr: E) -> impl Parser<SpannedToken, Spanned<Expr>, Error = ParserError> + Clone
where
    E: Parser<SpannedToken, Spanned<Expr>, Error = ParserError> + Clone + 'static,
{
    choice((
        // Hole: _
        token(Token::Hole).map_with_span(|_, span| Spanned::new(Expr::Hole, span.into())),

        // At keyword parsed as a function identifier
        token(Token::At).map_with_span(|_, span| Spanned::new(Expr::Var { name: Ident::new("at"), type_ascription: None }, span.into())),
        token(Token::Channel).map_with_span(|_, span| Spanned::new(Expr::Var { name: Ident::new("channel!"), type_ascription: None }, span.into())),
        token(Token::Queue).map_with_span(|_, span| Spanned::new(Expr::Var { name: Ident::new("queue!"), type_ascription: None }, span.into())),
        token(Token::Broadcast).map_with_span(|_, span| Spanned::new(Expr::Var { name: Ident::new("broadcast!"), type_ascription: None }, span.into())),

        // Unit: ()
        token(Token::LParen)
            .then_ignore(token(Token::RParen))
            .map_with_span(|_, span| Spanned::new(Expr::Tuple(vec![]), span.into())),

        // Literals
        literal().map_with_span(|lit, span| Spanned::new(Expr::Literal(lit), span.into())),

        // Variable or Qualified reference
        qualified_ident()
            .then(token(Token::DoubleColon).ignore_then(type_expr()).or_not())
            .map_with_span(|(qi, type_ascription), span| {
                if qi.is_simple() {
                    Spanned::new(Expr::Var {
                        name: Ident::new(qi.name.clone()),
                        type_ascription: type_ascription.map(|s| s.node),
                    }, span.into())
                } else {
                    // For now, qualified refs don't have ascription in the parser
                    // because Modulo::Item is already unambiguous.
                    Spanned::new(Expr::QualifiedRef(qi), span.into())
                }
            }),
    ))
}

/// Tuple parser: (e1, e2, e3)
fn tuple_parser<E>(expr: E) -> impl Parser<SpannedToken, Spanned<Expr>, Error = ParserError> + Clone
where
    E: Parser<SpannedToken, Spanned<Expr>, Error = ParserError> + Clone + 'static,
{
    between(
        token(Token::LParen),
        token(Token::RParen),
        // For Kata's prefix notation, elements are separated by whitespace (consumed by lexer)
        // We just need to parse one or more expressions
        expr.clone().then(expr.clone().repeated())
            .map(|(first, rest): (Spanned<Expr>, Vec<Spanned<Expr>>)| {
                if rest.is_empty() {
                    vec![first]
                } else {
                    let mut items = vec![first];
                    items.extend(rest);
                    items
                }
            })
    ).map_with_span(|items: Vec<Spanned<Expr>>, span| {
        if items.len() == 1 {
            items.into_iter().next().unwrap()
        } else {
            Spanned::new(Expr::Tuple(items), span.into())
        }
    })
}

/// Range parser: [start..end] or [start..step..end]
fn range_parser<E>(expr: E) -> impl Parser<SpannedToken, Spanned<Expr>, Error = ParserError> + Clone
where
    E: Parser<SpannedToken, Spanned<Expr>, Error = ParserError> + Clone + 'static,
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
    ).map_with_span(|(((start, op1_inclusive), middle), end_opt), span| {
        if let Some((op2_inclusive, end_expr)) = end_opt {
            // [start..step..end]
            Spanned::new(Expr::Range {
                start: Box::new(start),
                step: Some(Box::new(middle)),
                end: Box::new(end_expr),
                inclusive: op2_inclusive,
            }, span.into())
        } else {
            // [start..end]
            Spanned::new(Expr::Range {
                start: Box::new(start),
                step: None,
                end: Box::new(middle),
                inclusive: op1_inclusive,
            }, span.into())
        }
    })
}

/// List parser: [e1, e2, e3] or [e1 e2 e3] or [head : tail]
fn list_parser<E>(expr: E) -> impl Parser<SpannedToken, Spanned<Expr>, Error = ParserError> + Clone
where
    E: Parser<SpannedToken, Spanned<Expr>, Error = ParserError> + Clone + 'static,
{
    let sep = token(Token::Comma).ignored().or(newline().ignored()).or_not();

    let normal_list = expr.clone().padded_by(sep.clone()).repeated()
        .map(|items| Expr::List(items));

    let cons_list = expr.clone()
        .then(token(Token::Colon).ignore_then(expr.clone()).repeated().at_least(1))
        .map(|(first, rest)| {
            let mut all = vec![first];
            all.extend(rest);
            let mut it = all.into_iter().rev();
            let last = it.next().unwrap();
            let cons = it.fold(last, |tail, head| {
                let tail_span = tail.span.clone();
                Spanned::new(Expr::Cons {
                    head: Box::new(head),
                    tail: Box::new(tail),
                }, tail_span)
            }); // using tail's span as fallback
            cons.node
        });

    between(
        token(Token::LBracket),
        token(Token::RBracket),
        choice((cons_list, normal_list))
    ).map_with_span(|items, span| Spanned::new(items, span.into()))
}

/// Parser for braced expressions: {e1 e2} (Array) or {e1 e2 ; e3 e4} (Tensor)
fn braced_parser<E>(expr: E) -> impl Parser<SpannedToken, Spanned<Expr>, Error = ParserError> + Clone
where
    E: Parser<SpannedToken, Spanned<Expr>, Error = ParserError> + Clone + 'static,
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
    .map_with_span(|(first_row, other_rows): (Vec<Spanned<Expr>>, Vec<Vec<Spanned<Expr>>>), span| {
        if other_rows.is_empty() {
            // No semicolons - simple Array
            Spanned::new(Expr::Array(first_row), span.into())
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
            Spanned::new(Expr::Tensor {
                dimensions: vec![row_count, col_count],
                elements,
            }, span.into())
        }
    })
}

/// Dictionary parser: Dict [(key value)]
fn dict_parser<E>(expr: E) -> impl Parser<SpannedToken, Spanned<Expr>, Error = ParserError> + Clone
where
    E: Parser<SpannedToken, Spanned<Expr>, Error = ParserError> + Clone + 'static,
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
        .map_with_span(|(_, entries), span| Spanned::new(Expr::Dict(entries), span.into()))
}

/// Set parser: Set [e1, e2, e3]
fn set_parser<E>(expr: E) -> impl Parser<SpannedToken, Spanned<Expr>, Error = ParserError> + Clone
where
    E: Parser<SpannedToken, Spanned<Expr>, Error = ParserError> + Clone + 'static,
{
    let sep = token(Token::Comma).ignored().or(newline().ignored()).or_not();
    ident_named("Set")
        .then_ignore(token(Token::LBracket))
        .then(expr.clone().padded_by(sep.clone()).repeated())
        .then_ignore(token(Token::RBracket))
        .map_with_span(|(_, items), span| Spanned::new(Expr::Set(items), span.into()))
}

/// Lambda parser: lambda (pattern) body
fn lambda_parser<E>(expr: E) -> impl Parser<SpannedToken, Spanned<Expr>, Error = ParserError> + Clone
where
    E: Parser<SpannedToken, Spanned<Expr>, Error = ParserError> + Clone + 'static,
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
            .map(|(typ, interface)| WithBinding::Interface { typ: typ.node, interface }),

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
        .try_map(|(patterns, (guards_opt, with_opt, body_opt)): (Vec<Spanned<crate::ast::pattern::Pattern>>, (Option<Vec<GuardClause>>, Option<Vec<WithBinding>>, Option<Spanned<Expr>>)), span| {
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
            Ok(Spanned::new(Expr::Lambda { clauses: vec![clause] }, span.into()))
        })
}

/// Explicit application parser: $(func arg1 arg2)
fn explicit_apply_parser<E>(expr: E) -> impl Parser<SpannedToken, Spanned<Expr>, Error = ParserError> + Clone
where
    E: Parser<SpannedToken, Spanned<Expr>, Error = ParserError> + Clone + 'static,
{
    token(Token::Dollar)
        .ignore_then(between(
            token(Token::LParen),
            token(Token::RParen),
            expr.clone().then(expr.clone().repeated())
        ))
        .map_with_span(|(func, args), span| Spanned::new(Expr::ExplicitApply {
            func: Box::new(func),
            args,
        }, span.into()))
}

/// Parse a simple atom expression
pub fn simple_expression() -> impl Parser<SpannedToken, Spanned<Expr>, Error = ParserError> + Clone {
    atom_parser(expression())
}
