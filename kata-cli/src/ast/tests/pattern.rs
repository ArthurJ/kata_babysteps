use crate::ast::*;
use crate::ast::id::Literal;

fn span(start: usize, end: usize) -> LexerSpan {
    LexerSpan { start, end }
}

fn spanned<T>(node: T, start: usize, end: usize) -> Spanned<T> {
    Spanned::new(node, span(start, end))
}

#[test]
fn test_pattern_literal() {
    let p = Pattern::literal(Literal::int("42"));
    assert!(!p.captures_variables());
    assert_eq!(p.to_string(), "42");
}

#[test]
fn test_pattern_var() {
    let p = Pattern::var("x");
    assert!(p.captures_variables());
    assert_eq!(p.captured_variables().len(), 1);
    assert_eq!(p.to_string(), "x");
}

#[test]
fn test_pattern_wildcard() {
    let p = Pattern::wildcard();
    assert!(!p.captures_variables());
    assert_eq!(p.to_string(), "_");
}

#[test]
fn test_pattern_tuple() {
    let p = Pattern::Tuple(vec![
        spanned(Pattern::var("a"), 1, 2),
        spanned(Pattern::var("b"), 3, 4),
        spanned(Pattern::wildcard(), 5, 6),
    ]);
    assert!(p.captures_variables());
    assert_eq!(p.captured_variables().len(), 2);
    assert_eq!(p.to_string(), "(a b _)");
}

#[test]
fn test_pattern_variant() {
    let p = Pattern::Variant {
        name: Ident::new("Ok"),
        args: vec![spanned(Pattern::var("value"), 3, 8)],
    };
    assert_eq!(p.to_string(), "Ok(value)");
    assert!(p.captures_variables());
}

#[test]
fn test_pattern_variant_unit() {
    let p = Pattern::Variant {
        name: Ident::new("None"),
        args: vec![],
    };
    assert_eq!(p.to_string(), "None");
    assert!(!p.captures_variables());
}

#[test]
fn test_pattern_or() {
    let p = Pattern::Or(vec![
        spanned(Pattern::literal(Literal::int("0")), 0, 1),
        spanned(Pattern::literal(Literal::int("1")), 4, 5),
    ]);
    assert_eq!(p.to_string(), "0 | 1");
}

#[test]
fn test_pattern_empty_list() {
    let p = Pattern::List { elements: vec![], rest: None };
    assert_eq!(p.to_string(), "[]");
    assert!(!p.captures_variables());
}

#[test]
fn test_pattern_list_with_rest() {
    let p = Pattern::List {
        elements: vec![spanned(Pattern::var("head"), 1, 5)],
        rest: Some(Box::new(spanned(Pattern::var("tail"), 9, 13))),
    };
    assert!(p.captures_variables());
    assert_eq!(p.to_string(), "[head ...tail]");
}

#[test]
fn test_pattern_range() {
    let p = Pattern::Range {
        start: Literal::int("1"),
        end: Literal::int("10"),
        inclusive: false,
    };
    assert_eq!(p.to_string(), "[1..10]");
}

#[test]
fn test_pattern_range_inclusive() {
    let p = Pattern::Range {
        start: Literal::int("1"),
        end: Literal::int("10"),
        inclusive: true,
    };
    assert_eq!(p.to_string(), "[1..=10]");
}

#[test]
fn test_guard_display() {
    assert_eq!(Guard::Otherwise.to_string(), "otherwise");
}
