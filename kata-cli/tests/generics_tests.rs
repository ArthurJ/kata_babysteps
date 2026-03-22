use kata::ast::decl::{FunctionDef, TopLevel};
use kata::ast::expr::{Expr, LambdaClause, WithBinding};
use kata::ast::id::Ident;
use kata::ast::types::{Type, FunctionSig};
use kata::ast::pattern::Pattern;
use kata::ast::Spanned;
use kata::lexer::Span;
use kata::type_checker::checker::Checker;

fn span() -> Span {
    Span { start: 0, end: 0 }
}

fn spanned<T>(node: T) -> Spanned<T> {
    Spanned::new(node, span())
}

#[test]
fn test_generic_constraint_satisfied() {
    let mut checker = Checker::new();
    
    // Register NUM interface and Int implementation
    let mut members = std::collections::HashMap::new();
    members.insert("+".to_string(), FunctionSig::binary(Type::var("A"), Type::var("A"), Type::var("A")));
    let num_iface = kata::type_checker::environment::InterfaceInfo {
        name: "NUM".to_string(),
        extends: vec![],
        members,
    };
    checker.env.register_interface(num_iface, true);
    checker.env.register_type("Int", Type::named("Int"), true);
    checker.env.register_dispatch("+", FunctionSig::binary(Type::named("Int"), Type::named("Int"), Type::named("Int")));
    
    // Function: soma_generica :: A A => A
    // lambda (x y): + x y
    // with + :: A A => A
    let sig = FunctionSig::binary(Type::var("A"), Type::var("A"), Type::var("A"));
    let mut f = FunctionDef::new("soma_generica", sig);
    
    let clause = LambdaClause {
        patterns: vec![
            spanned(Pattern::Var(Ident::new("x"))),
            spanned(Pattern::Var(Ident::new("y"))),
        ],
        guards: vec![],
        body: Some(spanned(Expr::Apply {
            func: Box::new(spanned(Expr::Var { name: Ident::new("+"), type_ascription: None })),
            args: vec![
                spanned(Expr::Var { name: Ident::new("x"), type_ascription: None }),
                spanned(Expr::Var { name: Ident::new("y"), type_ascription: None })
            ]
        })),
        with: vec![
            WithBinding::Signature {
                name: Ident::new("+"),
                sig: FunctionSig::binary(Type::var("A"), Type::var("A"), Type::var("A")),
            }
        ],
    };
    f.clauses.push(clause);
    
    let result = checker.check_module(vec![spanned(TopLevel::Function(f))]);
    assert!(result.is_ok(), "Expected generic constraint to be satisfied: {:?}", result.err());
}

#[test]
fn test_generic_constraint_unsatisfied() {
    let mut checker = Checker::new();
    
    // NO implementation of "+" for Text
    checker.env.register_type("Text", Type::named("Text"), true);
    
    // Function: falha :: A A => A
    // lambda (x y): + x y
    // with + :: A A => A
    let sig = FunctionSig::binary(Type::var("A"), Type::var("A"), Type::var("A"));
    let mut f = FunctionDef::new("falha", sig);
    
    let clause = LambdaClause {
        patterns: vec![
            spanned(Pattern::Var(Ident::new("x"))),
            spanned(Pattern::Var(Ident::new("y"))),
        ],
        guards: vec![],
        body: Some(spanned(Expr::Apply {
            func: Box::new(spanned(Expr::Var { name: Ident::new("+"), type_ascription: None })),
            args: vec![
                spanned(Expr::Var { name: Ident::new("x"), type_ascription: None }),
                spanned(Expr::Var { name: Ident::new("y"), type_ascription: None })
            ]
        })),
        with: vec![
            WithBinding::Signature {
                name: Ident::new("+"),
                sig: FunctionSig::binary(Type::var("A"), Type::var("A"), Type::var("A")),
            }
        ],
    };
    f.clauses.push(clause);
    
    let result = checker.check_module(vec![spanned(TopLevel::Function(f))]);
    assert!(result.is_err(), "Expected generic constraint to fail because no implementation of + exists");
}

#[test]
fn test_interface_constraint_satisfied() {
    let mut checker = Checker::new();
    
    // NUM interface and Int type
    let mut members = std::collections::HashMap::new();
    members.insert("+".to_string(), FunctionSig::binary(Type::var("A"), Type::var("A"), Type::var("A")));
    let num_iface = kata::type_checker::environment::InterfaceInfo {
        name: "NUM".to_string(),
        extends: vec![],
        members,
    };
    checker.env.register_interface(num_iface, true);
    checker.env.register_type("Int", Type::named("Int"), true);
    checker.env.register_dispatch("+", FunctionSig::binary(Type::named("Int"), Type::named("Int"), Type::named("Int")));
    
    // A implements NUM
    let sig = FunctionSig::binary(Type::var("A"), Type::var("A"), Type::var("A"));
    let mut f = FunctionDef::new("foo", sig);
    let clause = LambdaClause {
        patterns: vec![
            spanned(Pattern::Wildcard),
            spanned(Pattern::Wildcard)
        ],
        guards: vec![],
        body: Some(spanned(Expr::literal(kata::ast::id::Literal::int("0")))), // Dummy body
        with: vec![
            WithBinding::Interface {
                typ: Type::var("A"),
                interface: Ident::new("NUM"),
            }
        ],
    };
    f.clauses.push(clause);
    
    let result = checker.check_module(vec![spanned(TopLevel::Function(f))]);
    assert!(result.is_ok(), "Expected Interface constraint to be satisfied: {:?}", result.err());
}
