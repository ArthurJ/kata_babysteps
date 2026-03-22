//! Unit tests for AST types
//!
//! This test module verifies the correctness of AST structures.

use crate::ast::*;
use crate::ast::id::Literal;

mod id_tests {
    use super::*;

    #[test]
    fn test_ident_creation() {
        let ident = Ident::new("minha_funcao");
        assert_eq!(ident.0, "minha_funcao");
        assert_eq!(ident.to_string(), "minha_funcao");
    }

    #[test]
    fn test_ident_is_operator() {
        assert!(Ident::new("+").is_operator());
        assert!(Ident::new("-").is_operator());
        assert!(Ident::new("*").is_operator());
        assert!(Ident::new("/").is_operator());
        assert!(Ident::new("<=").is_operator());
        assert!(Ident::new("==").is_operator());
        assert!(!Ident::new("soma").is_operator());
        assert!(!Ident::new("x").is_operator());
    }

    #[test]
    fn test_ident_is_action() {
        assert!(Ident::new("echo!").is_action());
        assert!(Ident::new("main!").is_action());
        assert!(!Ident::new("soma").is_action());
        assert!(!Ident::new("x").is_action());
    }

    #[test]
    fn test_qualified_ident_simple() {
        let ident = QualifiedIdent::simple("Int");
        assert!(ident.is_simple());
        assert_eq!(ident.to_string(), "Int");
    }

    #[test]
    fn test_qualified_ident_qualified() {
        let ident = QualifiedIdent::qualified("types", "NUM");
        assert!(!ident.is_simple());
        assert_eq!(ident.to_string(), "types::NUM");
    }

    #[test]
    fn test_qualified_ident_from_ident() {
        let ident = Ident::new("List");
        let qualified = QualifiedIdent::from(ident);
        assert!(qualified.is_simple());
        assert_eq!(qualified.to_string(), "List");
    }

    #[test]
    fn test_literal_int() {
        let lit = Literal::int("42");
        assert_eq!(lit.to_string(), "42");

        let hex = Literal::int("0xFF");
        assert_eq!(hex.to_string(), "0xFF");
    }

    #[test]
    fn test_literal_float() {
        let lit = Literal::float("3.14");
        assert_eq!(lit.to_string(), "3.14");

        let nan = Literal::float("nan");
        assert_eq!(nan.to_string(), "nan");
    }

    #[test]
    fn test_literal_string() {
        let lit = Literal::string("hello world");
        assert_eq!(lit.to_string(), "\"hello world\"");
    }

    #[test]
    fn test_literal_bool() {
        assert_eq!(Literal::bool(true).to_string(), "True");
        assert_eq!(Literal::bool(false).to_string(), "False");
    }

    #[test]
    fn test_literal_unit() {
        assert_eq!(Literal::unit().to_string(), "()");
    }

    #[test]
    fn test_directive_test() {
        let dir = Directive::Test { description: "test description".to_string() };
        assert_eq!(dir.to_string(), "@test(\"test description\")");
    }

    #[test]
    fn test_directive_parallel() {
        let dir = Directive::Parallel;
        assert_eq!(dir.to_string(), "@parallel");
    }

    #[test]
    fn test_directive_ffi() {
        let dir = Directive::Ffi { symbol: "kata_rt_add".to_string() };
        assert_eq!(dir.to_string(), "@ffi(\"kata_rt_add\")");
    }
}