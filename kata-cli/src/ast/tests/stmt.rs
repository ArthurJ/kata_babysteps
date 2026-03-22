use crate::ast::*;
use crate::ast::id::Literal;

fn span(start: usize, end: usize) -> LexerSpan {
    LexerSpan { start, end }
}

fn spanned<T>(node: T, start: usize, end: usize) -> Spanned<T> {
    Spanned::new(node, span(start, end))
}

#[test]
fn test_stmt_let() {
    let s = Stmt::Let {
        pattern: spanned(Pattern::var("x"), 4, 5),
        value: spanned(Expr::literal(Literal::int("42")), 6, 8),
    };
    assert_eq!(s.to_string(), "let x 42");
}

#[test]
fn test_stmt_var() {
    let s = Stmt::Var {
        pattern: spanned(Pattern::var("counter"), 4, 11),
        value: spanned(Expr::literal(Literal::int("0")), 12, 13),
    };
    assert_eq!(s.to_string(), "var counter 0");
}

#[test]
fn test_stmt_assign() {
    let s = Stmt::Assign {
        name: Ident::new("x"),
        value: spanned(Expr::literal(Literal::int("1")), 6, 7),
    };
    assert_eq!(s.to_string(), "var x 1");
}

#[test]
fn test_stmt_break() {
    let s = Stmt::Break;
    assert_eq!(s.to_string(), "break");
    assert!(s.is_control_flow());
}

#[test]
fn test_stmt_continue() {
    let s = Stmt::Continue;
    assert_eq!(s.to_string(), "continue");
    assert!(s.is_control_flow());
}

#[test]
fn test_stmt_return() {
    let s = Stmt::Return(spanned(Expr::literal(Literal::int("42")), 7, 9));
    assert_eq!(s.to_string(), "return 42");
    assert!(s.is_control_flow());
}

#[test]
fn test_stmt_loop() {
    let s = Stmt::Loop {
        body: vec![
            spanned(Stmt::Let {
                pattern: spanned(Pattern::var("x"), 13, 14),
                value: spanned(Expr::literal(Literal::int("1")), 15, 16),
            }, 9, 16)
        ],
    };
    assert!(s.to_string().contains("loop"));
}

#[test]
fn test_stmt_for() {
    let s = Stmt::For {
        var: Ident::new("item"),
        iterable: spanned(Expr::Var { name: Ident::new("lista"), type_ascription: None }, 12, 17),
        body: vec![
            spanned(Stmt::Expr(
                spanned(Expr::Var { name: Ident::new("item"), type_ascription: None }, 22, 26)
            ), 22, 26)
        ],
    };
    let output = s.to_string();
    assert!(output.contains("for item in"));
    assert!(output.contains("lista"));
}

#[test]
fn test_match_case() {
    let case = MatchCase::new(
        spanned(Pattern::Variant {
            name: Ident::new("Ok"),
            args: vec![spanned(Pattern::var("value"), 15, 20)]
        }, 12, 21),
        vec![
            spanned(Stmt::Expr(
                spanned(Expr::Var { name: Ident::new("value"), type_ascription: None }, 24, 29)
            ), 24, 29)
        ],
    );
    assert!(case.to_string().contains("Ok(value)"));
}

#[test]
fn test_channel_op_display() {
    let recv = ChannelOp::Receive {
        channel: spanned(Expr::Var { name: Ident::new("rx"), type_ascription: None }, 3, 5),
        non_blocking: false,
    };
    assert_eq!(recv.to_string(), "<! rx");

    let recv_nb = ChannelOp::Receive {
        channel: spanned(Expr::Var { name: Ident::new("rx"), type_ascription: None }, 4, 6),
        non_blocking: true,
    };
    assert_eq!(recv_nb.to_string(), "<!? rx");

    let send = ChannelOp::Send {
        value: spanned(Expr::literal(Literal::string("hello")), 0, 7),
        channel: spanned(Expr::Var { name: Ident::new("tx"), type_ascription: None }, 11, 13),
    };
    assert_eq!(send.to_string(), "\"hello\" !> tx");
}

#[test]
fn test_error_propagation() {
    let ep = ErrorPropagation {
        expr: spanned(Expr::Var { name: Ident::new("result"), type_ascription: None }, 0, 6)
    };
    assert_eq!(ep.to_string(), "result?");
}
