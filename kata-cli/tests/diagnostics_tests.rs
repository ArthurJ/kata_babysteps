//! Integration tests for Type Checker Diagnostics and Spans

use kata::lexer::KataLexer;
use kata::parser::decl::module;
use kata::type_checker::checker::Checker;
use kata::type_checker::error::TypeError;
use chumsky::Parser;

fn check_code(source: &str) -> Result<(), Vec<TypeError>> {
    let tokens = KataLexer::lex_with_indent(source).expect("Lexing failed");
    let ast_module = module().parse(tokens).expect("Parsing failed");
    
    let mut checker = Checker::new();
    checker.check_module(ast_module.declarations).map(|_| ())
        .map_err(|e| vec![e])
}

#[test]
fn test_type_mismatch_span() {
    // Intentional type mismatch: adding Int and Text
    let source = "action main\n    let x (+ 1 \"erro\")";
    let result = check_code(source);
    
    assert!(result.is_err());
    let errors = result.unwrap_err();
    
    // We expect a type mismatch or unbound variable depending on how '+' is resolved
    // In this case, since "+" is multiple-dispatch, it will likely fail dispatch
    let found_expected_error = errors.iter().any(|e| {
        match e {
            TypeError::TypeMismatch { span, .. } | TypeError::UnboundVariable { span, .. } => {
                // The span should point to "erro" or the whole application
                // For now, just check that it's NOT dummy (0,0)
                span.start > 0 || span.end > 0
            }
            _ => false
        }
    });
    
    assert!(found_expected_error, "Expected a semantic error with a valid span, got {:?}", errors);
}

#[test]
fn test_unbound_variable_span() {
    let source = "action main\n    let x unknown_var";
    let result = check_code(source);
    
    assert!(result.is_err());
    let errors = result.unwrap_err();
    
    if let Some(TypeError::UnboundVariable { name, span }) = errors.first() {
        assert_eq!(name, "unknown_var");
        // "unknown_var" is at the end of the second line
        assert!(span.start > 0);
    } else {
        panic!("Expected UnboundVariable error, got {:?}", errors);
    }
}
