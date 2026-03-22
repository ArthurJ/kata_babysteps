//! Statement parsers for Kata Language
//!
//! Parses statements used in the action domain:
//! - let/var bindings
//! - Match expressions
//! - Loop/for/break/continue
//! - Select (CSP)
//! - Return/Panic/Assert

use chumsky::prelude::*;
use crate::lexer::{Token, SpannedToken};
use crate::ast::id::Ident;
use crate::ast::stmt::{Stmt, MatchCase, SelectCase, ChannelOp, SelectTimeout};
use crate::ast::expr::Expr;
use crate::ast::Spanned;
use super::common::{ident, pure_ident, token, newline, indent, dedent, between, ParserError, ParserSpan};
use super::pattern::pattern;

/// Parse any statement (non-recursive version for top-level use)
/// Takes an expression parser to avoid constructing expression() multiple times
pub fn statement<E>(expr: E) -> impl Parser<SpannedToken, Spanned<Stmt>, Error = ParserError> + Clone
where
    E: Parser<SpannedToken, Spanned<Expr>, Error = ParserError> + Clone + 'static,
{
    choice((
        // Let binding: let name expr
        let_binding(expr.clone()),

        // Var binding: var name expr
        var_binding(expr.clone()),

        // Assignment: var name expr (reassignment)
        assignment(expr.clone()),

        // Break
        token(Token::Break).map_with_span(|_, span| Spanned::new(Stmt::Break, span.into())),

        // Continue
        token(Token::Continue).map_with_span(|_, span| Spanned::new(Stmt::Continue, span.into())),

        // Return: return expr
        return_stmt(expr.clone()),

        // Panic: panic! expr
        panic_stmt(expr.clone()),

        // Assert: assert! cond msg
        assert_stmt(expr.clone()),

        // Expression statement
        expr.clone().map_with_span(|e, span| Spanned::new(Stmt::Expr(e), span.into())),
    ))
}

/// Parse statements with support for recursive constructs (match, loop, for, select)
/// Takes an expression parser to avoid constructing expression() multiple times
pub fn recursive_statement<E>(expr: E) -> impl Parser<SpannedToken, Spanned<Stmt>, Error = ParserError> + Clone
where
    E: Parser<SpannedToken, Spanned<Expr>, Error = ParserError> + Clone + 'static,
{
    log::debug!("recursive_statement(): Starting recursive() construction");
    let result = recursive(|stmt| {
        log::debug!("recursive_statement(): Inside recursive closure");
        choice((
            // Let binding: let name expr
            let_binding(expr.clone()),

            // Var binding: var name expr
            var_binding(expr.clone()),

            // Assignment: var name expr (reassignment)
            assignment(expr.clone()),

            // Match: match expr { cases }
            match_stmt(expr.clone(), stmt.clone()),

            // Loop: loop body
            loop_stmt(stmt.clone()),

            // For: for var in iterable body
            for_stmt(expr.clone(), stmt.clone()),

            // Break
            token(Token::Break).map_with_span(|_, span| Spanned::new(Stmt::Break, span.into())),

            // Continue
            token(Token::Continue).map_with_span(|_, span| Spanned::new(Stmt::Continue, span.into())),

            // Select: select! { cases }
            select_stmt(stmt.clone()),

            // Return: return expr
            return_stmt(expr.clone()),

            // Panic: panic! expr
            panic_stmt(expr.clone()),

            // Assert: assert! cond msg
            assert_stmt(expr.clone()),

            // Expression statement
            expr.clone().map_with_span(|e, span| Spanned::new(Stmt::Expr(e), span.into())),
        ))
    });
    log::debug!("recursive_statement(): Complete");
    result
}

/// Parse a let binding: let pattern expr
fn let_binding<E>(expr: E) -> impl Parser<SpannedToken, Spanned<Stmt>, Error = ParserError> + Clone
where
    E: Parser<SpannedToken, Spanned<Expr>, Error = ParserError> + Clone + 'static,
{
    token(Token::Let)
        .ignore_then(pattern())
        .then(expr)
        .map_with_span(|(pattern, value), span| Spanned::new(Stmt::Let { pattern, value }, span.into()))
}

/// Parse a var binding: var pattern expr
fn var_binding<E>(expr: E) -> impl Parser<SpannedToken, Spanned<Stmt>, Error = ParserError> + Clone
where
    E: Parser<SpannedToken, Spanned<Expr>, Error = ParserError> + Clone + 'static,
{
    token(Token::Var)
        .ignore_then(pattern())
        .then(expr)
        .map_with_span(|(pattern, value), span| Spanned::new(Stmt::Var { pattern, value }, span.into()))
}

/// Parse an assignment: var name expr (reassignment to mutable variable)
fn assignment<E>(expr: E) -> impl Parser<SpannedToken, Spanned<Stmt>, Error = ParserError> + Clone
where
    E: Parser<SpannedToken, Spanned<Expr>, Error = ParserError> + Clone + 'static,
{
    simple_ident()
        .then_ignore(token(Token::Var))
        .then(expr)
        .map_with_span(|(name, value), span| Spanned::new(Stmt::Assign { name, value }, span.into()))
}

/// Parse a match statement: match expr { cases }
fn match_stmt<E, S>(expr: E, stmt: S) -> impl Parser<SpannedToken, Spanned<Stmt>, Error = ParserError> + Clone
where
    E: Parser<SpannedToken, Spanned<Expr>, Error = ParserError> + Clone + 'static,
    S: Parser<SpannedToken, Spanned<Stmt>, Error = ParserError> + Clone + 'static,
{
    token(Token::Match)
        .ignore_then(expr)
        .then_ignore(newline().repeated().or_not())
        .then_ignore(indent())
        .then(match_case(stmt).padded_by(newline().repeated().or_not()).repeated())
        .then_ignore(newline().repeated().or_not())
        .then_ignore(dedent())
        .map_with_span(|(value, cases), span| Spanned::new(Stmt::Match { value, cases }, span.into()))
}

/// Parse a single match case: pattern: body
fn match_case<S>(stmt: S) -> impl Parser<SpannedToken, MatchCase, Error = ParserError> + Clone
where
    S: Parser<SpannedToken, Spanned<Stmt>, Error = ParserError> + Clone + 'static,
{
    pattern()
        .then_ignore(token(Token::Colon))
        .then_ignore(newline().repeated().or_not())
        .then_ignore(indent())
        .then(stmt.padded_by(newline().repeated().or_not()).repeated())
        .then_ignore(newline().repeated().or_not())
        .then_ignore(dedent())
        .map(|(pattern, body)| MatchCase { pattern, body })
}

/// Parse a loop statement: loop body
fn loop_stmt<S>(stmt: S) -> impl Parser<SpannedToken, Spanned<Stmt>, Error = ParserError> + Clone
where
    S: Parser<SpannedToken, Spanned<Stmt>, Error = ParserError> + Clone + 'static,
{
    token(Token::Loop)
        .ignore_then(newline().repeated().or_not())
        .ignore_then(indent())
        .ignore_then(
            stmt.padded_by(newline().repeated().or_not())
                .repeated()
                .map(|body| {
                    log::debug!("loop_stmt: matched body with {} statements", body.len());
                    body
                })
        )
        .then_ignore(newline().repeated().or_not())
        .then_ignore(dedent())
        .map_with_span(|body, span| {
            log::debug!("loop_stmt: fully matched");
            Spanned::new(Stmt::Loop { body }, span.into())
        })
}

/// Parse a for statement: for var in iterable body
fn for_stmt<E, S>(expr: E, stmt: S) -> impl Parser<SpannedToken, Spanned<Stmt>, Error = ParserError> + Clone
where
    E: Parser<SpannedToken, Spanned<Expr>, Error = ParserError> + Clone + 'static,
    S: Parser<SpannedToken, Spanned<Stmt>, Error = ParserError> + Clone + 'static,
{
    token(Token::For)
        .ignore_then(simple_ident())
        .then_ignore(token(Token::In))
        .then(expr)
        .then_ignore(newline().repeated().or_not())
        .then_ignore(indent())
        .then(stmt.padded_by(newline().repeated().or_not()).repeated())
        .then_ignore(newline().repeated().or_not())
        .then_ignore(dedent())
        .map_with_span(|((var, iterable), body), span| Spanned::new(Stmt::For { var, iterable, body }, span.into()))
}

/// Parse a select statement: select! { cases }
/// Uses expression() internally to avoid stack overflow
fn select_stmt<S>(stmt: S) -> impl Parser<SpannedToken, Spanned<Stmt>, Error = ParserError> + Clone
where
    S: Parser<SpannedToken, Spanned<Stmt>, Error = ParserError> + Clone + 'static,
{
    // Import expression parser here to avoid parameter passing
    use super::expr::expression;
    let expr = expression();

    token(Token::Select)
        .ignore_then(newline().repeated().or_not())
        .ignore_then(indent())
        .ignore_then(
            select_case(expr.clone(), stmt.clone())
                .padded_by(newline().repeated().or_not())
                .repeated()
        )
        .then(
            select_timeout(expr, stmt)
                .padded_by(newline().repeated().or_not())
                .or_not()
        )
        .then_ignore(newline().repeated().or_not())
        .then_ignore(dedent())
        .map_with_span(|(cases, timeout), span| {
            log::debug!("select_stmt matched with {} cases and timeout={}", cases.len(), timeout.is_some());
            Spanned::new(Stmt::Select { cases, timeout }, span.into())
        })
}

/// Parse a select case: case operation -> var: body
fn select_case<E, S>(expr: E, stmt: S) -> impl Parser<SpannedToken, SelectCase, Error = ParserError> + Clone
where
    E: Parser<SpannedToken, Spanned<Expr>, Error = ParserError> + Clone + 'static,
    S: Parser<SpannedToken, Spanned<Stmt>, Error = ParserError> + Clone + 'static,
{
    token(Token::Case)
        .ignore_then(choice((
            between(token(Token::LParen), token(Token::RParen), channel_op(expr.clone())),
            channel_op(expr.clone()),
        )))
        .then(
            token(Token::SimpleArrow)
                .ignore_then(simple_ident())
                .or_not()
        )
        .then_ignore(token(Token::Colon))
        .then_ignore(newline().repeated().or_not())
        .then_ignore(indent())
        .then(stmt.padded_by(newline().repeated().or_not()).repeated())
        .then_ignore(newline().repeated().or_not())
        .then_ignore(dedent())
        .map(|((operation, binding), body)| SelectCase { operation, binding, body })
}

/// Parse a channel operation
fn channel_op<E>(expr: E) -> impl Parser<SpannedToken, ChannelOp, Error = ParserError> + Clone
where
    E: Parser<SpannedToken, Spanned<Expr>, Error = ParserError> + Clone + 'static,
{
    choice((
        // Receive: <! channel or <!? channel (non-blocking)
        token(Token::Receive)
            .ignore_then(expr.clone())
            .map(|channel| ChannelOp::Receive { channel, non_blocking: false }),

        token(Token::ReceiveNonBlocking)
            .ignore_then(expr.clone())
            .map(|channel| ChannelOp::Receive { channel, non_blocking: true }),

        // Send: value !> channel
        expr.clone()
            .then_ignore(token(Token::Send))
            .then(expr)
            .map(|(value, channel)| ChannelOp::Send { value, channel }),
    ))
}

/// Parse a timeout: timeout! duration: body
fn select_timeout<E, S>(expr: E, stmt: S) -> impl Parser<SpannedToken, SelectTimeout, Error = ParserError> + Clone
where
    E: Parser<SpannedToken, Spanned<Expr>, Error = ParserError> + Clone + 'static,
    S: Parser<SpannedToken, Spanned<Stmt>, Error = ParserError> + Clone + 'static,
{
    token(Token::Timeout)
        .ignore_then(expr)
        .then_ignore(token(Token::Colon))
        .then_ignore(newline().repeated().or_not())
        .then_ignore(indent())
        .then(stmt.padded_by(newline().repeated().or_not()).repeated())
        .then_ignore(newline().repeated().or_not())
        .then_ignore(dedent())
        .map(|(duration, body)| SelectTimeout { duration, body })
}

/// Parse a return statement
fn return_stmt<E>(expr: E) -> impl Parser<SpannedToken, Spanned<Stmt>, Error = ParserError> + Clone
where
    E: Parser<SpannedToken, Spanned<Expr>, Error = ParserError> + Clone + 'static,
{
    ident_named("return")
        .ignore_then(expr)
        .map_with_span(|e, span| Spanned::new(Stmt::Return(e), span.into()))
}

/// Parse a panic statement
fn panic_stmt<E>(expr: E) -> impl Parser<SpannedToken, Spanned<Stmt>, Error = ParserError> + Clone
where
    E: Parser<SpannedToken, Spanned<Expr>, Error = ParserError> + Clone + 'static,
{
    ident_named("panic!")
        .ignore_then(expr)
        .map_with_span(|message, span| Spanned::new(Stmt::Panic { message }, span.into()))
}

/// Parse an asset statement
fn assert_stmt<E>(expr: E) -> impl Parser<SpannedToken, Spanned<Stmt>, Error = ParserError> + Clone
where
    E: Parser<SpannedToken, Spanned<Expr>, Error = ParserError> + Clone + 'static,
{
    ident_named("assert!")
        .ignore_then(expr.clone())
        .then(expr)
        .map_with_span(|(condition, message), span| Spanned::new(Stmt::Assert { condition, message }, span.into()))
}

// Helper for simple identifier
fn simple_ident() -> impl Parser<SpannedToken, Ident, Error = ParserError> + Clone {
    pure_ident().map(Ident::new)
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

