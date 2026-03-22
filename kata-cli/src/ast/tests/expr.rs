use crate::ast::*;
use crate::ast::id::Literal;

fn span(start: usize, end: usize) -> LexerSpan {
    LexerSpan { start, end }
}

fn spanned<T>(node: T, start: usize, end: usize) -> Spanned<T> {
    Spanned::new(node, span(start, end))
}

#[test]
fn test_expr_literal() {
    let e = Expr::literal(Literal::int("42"));
    assert!(e.is_literal());
    assert!(!e.is_var());
    assert_eq!(e.to_string(), "42");
}

#[test]
fn test_expr_var() {
    let e = Expr::Var { name: Ident::new("x"), type_ascription: None };
    assert!(e.is_var());
    assert!(!e.is_literal());
    assert_eq!(e.to_string(), "x");
}

#[test]
fn test_expr_hole() {
    let e = Expr::Hole;
    assert!(e.is_hole());
    assert_eq!(e.to_string(), "_");
}

#[test]
fn test_expr_tuple() {
    let e = Expr::Tuple(vec![
        spanned(Expr::literal(Literal::int("1")), 1, 2),
        spanned(Expr::literal(Literal::int("2")), 3, 4),
    ]);
    assert_eq!(e.to_string(), "(1 2)");
}

#[test]
fn test_expr_list() {
    let e = Expr::List(vec![
        spanned(Expr::literal(Literal::int("1")), 1, 2),
        spanned(Expr::literal(Literal::int("2")), 3, 4),
        spanned(Expr::literal(Literal::int("3")), 5, 6),
    ]);
    assert_eq!(e.to_string(), "[1 2 3]");
}

#[test]
fn test_expr_array() {
    let e = Expr::Array(vec![
        spanned(Expr::literal(Literal::int("1")), 1, 2),
        spanned(Expr::literal(Literal::int("2")), 3, 4),
    ]);
    assert_eq!(e.to_string(), "{1 2}");
}

#[test]
fn test_expr_apply() {
    let e = Expr::Apply {
        func: Box::new(spanned(Expr::Var { name: Ident::new("+"), type_ascription: None }, 0, 1)),
        args: vec![
            spanned(Expr::literal(Literal::int("1")), 2, 3),
            spanned(Expr::literal(Literal::int("2")), 4, 5),
        ],
    };
    assert_eq!(e.to_string(), "+ 1 2");
}

#[test]
fn test_expr_pipeline() {
    let e = Expr::Pipeline {
        value: Box::new(spanned(Expr::List(vec![
            spanned(Expr::literal(Literal::int("1")), 1, 2)
        ]), 0, 3)),
        func: Box::new(spanned(Expr::Var { name: Ident::new("map"), type_ascription: None }, 7, 10)),
    };
    assert_eq!(e.to_string(), "[1] |> map");
}

#[test]
fn test_lambda_clause() {
    let clause = LambdaClause::new(
        vec![
            spanned(Pattern::var("x"), 3, 4),
            spanned(Pattern::var("y"), 5, 6),
        ],
        spanned(Expr::Apply {
            func: Box::new(spanned(Expr::Var { name: Ident::new("+"), type_ascription: None }, 9, 10)),
            args: vec![
                spanned(Expr::Var { name: Ident::new("x"), type_ascription: None }, 11, 12),
                spanned(Expr::Var { name: Ident::new("y"), type_ascription: None }, 13, 14),
            ]
        }, 9, 14),
    );
    assert_eq!(clause.to_string(), "x y: + x y");
}
