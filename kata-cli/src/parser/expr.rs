//! Expression parsers for Kata Language
//!
//! Uses single recursive() call with layered choice to avoid stack overflow

use chumsky::prelude::*;
use crate::lexer::{Token, SpannedToken};
use crate::ast::id::{Ident, QualifiedIdent};
use crate::ast::expr::{Expr, LambdaClause, GuardClause, GuardCondition, WithBinding, WithBindingKind};
use super::common::{ident, token, newline, indent, dedent, between, separated, ParserError, ParserSpan};
use super::literal::literal;
use super::pattern::pattern;
use super::r#type::type_expr;

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

            let pipeline = field.clone()
                .then(token(Token::Pipeline).ignore_then(field.clone()).repeated())
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

        // Unit: ()
        token(Token::LParen)
            .then_ignore(token(Token::RParen))
            .map(|_| Expr::Tuple(vec![])),

        // Literals
        literal().map(Expr::Literal),

        // Variable or Qualified reference
        qualified_ident().map(|qi| {
            if qi.is_simple() {
                Expr::Var(Ident::new(qi.name.clone()))
            } else {
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

/// List parser: [e1, e2, e3]
fn list_parser<E>(expr: E) -> impl Parser<SpannedToken, Expr, Error = ParserError> + Clone
where
    E: Parser<SpannedToken, Expr, Error = ParserError> + Clone + 'static,
{
    between(
        token(Token::LBracket),
        token(Token::RBracket),
        separated(expr, token(Token::Comma))
    ).map(Expr::List)
}

/// Parser for braced expressions: {e1 e2} (Array) or {e1 e2 ; e3 e4} (Tensor)
fn braced_parser<E>(expr: E) -> impl Parser<SpannedToken, Expr, Error = ParserError> + Clone
where
    E: Parser<SpannedToken, Expr, Error = ParserError> + Clone + 'static,
{
    let item_sep = token(Token::Comma).ignored().or(newline()).repeated().or_not();
    
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
            let mut elements = first_row;
            let mut row_count = 1;
            let col_count = elements.len();
            
            for next_row in other_rows {
                if !next_row.is_empty() {
                    elements.extend(next_row);
                    row_count += 1;
                }
            }
            
            // For now support 2D tensors (matrices)
            // dimensions = [rows, cols]
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
    ident_named("Dict")
        .then_ignore(token(Token::LBracket))
        .then(
            between(
                token(Token::LParen),
                token(Token::RParen),
                expr.clone().then_ignore(newline().or_not()).then(expr.clone())
            ).repeated()
        )
        .then_ignore(token(Token::RBracket))
        .map(|(_, entries)| Expr::Dict(entries))
}

/// Set parser: Set [e1, e2, e3]
fn set_parser<E>(expr: E) -> impl Parser<SpannedToken, Expr, Error = ParserError> + Clone
where
    E: Parser<SpannedToken, Expr, Error = ParserError> + Clone + 'static,
{
    ident_named("Set")
        .then_ignore(token(Token::LBracket))
        .then(separated(expr, token(Token::Comma)))
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

    let pattern_parser = between(
        token(Token::LParen),
        token(Token::RParen),
        pattern().padded_by(newline().repeated().or_not()).repeated()
    );

    let body_parser = choice((
        newline().repeated().at_least(1).ignore_then(indent()).ignore_then(expr.clone()).then_ignore(newline().repeated().or_not()).then_ignore(dedent()),
        expr.clone(),
    ));

    let guard_clauses = simple_ident()
        .then_ignore(token(Token::Colon))
        .then(expr.clone())
        .map(|(label, body): (Ident, Expr)| GuardClause {
            label: label.clone(),
            guard: GuardCondition::Named(label),
            body,
        })
        .repeated()
        .at_least(1);

    let with_binding = simple_ident()
        .then_ignore(token(Token::As))
        .then(choice((
            type_expr().map(|t| WithBindingKind::TypeConstraint(t)),
            expr.clone().map(|e| WithBindingKind::Expr(e)),
        )))
        .map(|(name, kind)| WithBinding { name, kind });

    let with_bindings = token(Token::With)
        .ignore_then(newline().repeated().or_not())
        .ignore_then(indent())
        .ignore_then(with_binding.padded_by(newline().repeated().or_not()).repeated())
        .then_ignore(newline().repeated().or_not())
        .then_ignore(dedent());

    lambda_keyword
        .ignore_then(pattern_parser)
        .then(guard_clauses.or_not())
        .then(with_bindings.or_not())
        .then(body_parser)
        .map(|(((patterns, guards), with_bindings), body)| {
            let clause = LambdaClause {
                patterns,
                guards: guards.unwrap_or_default(),
                body,
                with: with_bindings.unwrap_or_default(),
            };
            Expr::Lambda { clauses: vec![clause] }
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
