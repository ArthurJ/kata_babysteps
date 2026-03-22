//! Unit tests for AST types
//!
//! This test module verifies the correctness of AST structures.

use crate::ast::*;
use crate::ast::id::Literal;

mod types_tests {
    use super::*;

    #[test]
    fn test_type_named() {
        let t = Type::named("Int");
        assert!(t.is_simple());
        assert_eq!(t.to_string(), "Int");
    }

    #[test]
    fn test_type_generic() {
        let t = Type::generic("Optional", vec![Type::named("Int")]);
        assert_eq!(t.to_string(), "Optional::Int");

        let t2 = Type::generic("Result", vec![Type::named("Int"), Type::var("E")]);
        assert_eq!(t2.to_string(), "Result::Int::E");
    }

    #[test]
    fn test_type_var() {
        let t = Type::var("T");
        assert!(t.is_var());
        assert_eq!(t.to_string(), "T");
    }

    #[test]
    fn test_type_tuple() {
        let t = Type::tuple(vec![Type::named("Int"), Type::named("Float")]);
        assert_eq!(t.to_string(), "(Int Float)");
    }

    #[test]
    fn test_type_function() {
        let t = Type::function(vec![Type::named("Int"), Type::named("Int")], Type::named("Int"));
        assert_eq!(t.to_string(), "(Int Int) -> Int");
    }

    #[test]
    fn test_type_refined() {
        let t = Type::refined(
            Type::named("Int"),
            Predicate::Comparison {
                op: CompareOp::Gt,
                value: LiteralValue::Int(0),
            },
        );
        assert_eq!(t.to_string(), "(Int, > _ 0)");
    }

    #[test]
    fn test_function_sig() {
        let sig = FunctionSig::binary(Type::named("Int"), Type::named("Int"), Type::named("Int"));
        assert_eq!(sig.arity(), 2);
        assert_eq!(sig.to_string(), "Int Int => Int");
    }

    #[test]
    fn test_function_sig_unary() {
        let sig = FunctionSig::unary(Type::named("Int"), Type::named("Bool"));
        assert_eq!(sig.arity(), 1);
        assert_eq!(sig.to_string(), "Int => Bool");
    }

    #[test]
    fn test_function_sig_nullary() {
        let sig = FunctionSig::nullary(Type::named("Unit"));
        assert_eq!(sig.arity(), 0);
        assert_eq!(sig.to_string(), "Unit");
    }

    #[test]
    fn test_predicate_comparison() {
        let p = Predicate::Comparison {
            op: CompareOp::Lt,
            value: LiteralValue::Int(10),
        };
        assert_eq!(p.to_string(), "< _ 10");
    }

    #[test]
    fn test_predicate_except() {
        let p = Predicate::Except(QualifiedIdent::simple("Complex"));
        assert_eq!(p.to_string(), "except Complex");
    }

    #[test]
    fn test_compare_op_display() {
        assert_eq!(CompareOp::Gt.to_string(), ">");
        assert_eq!(CompareOp::Gte.to_string(), ">=");
        assert_eq!(CompareOp::Lt.to_string(), "<");
        assert_eq!(CompareOp::Lte.to_string(), "<=");
        assert_eq!(CompareOp::Eq.to_string(), "=");
        assert_eq!(CompareOp::Neq.to_string(), "!=");
    }
}