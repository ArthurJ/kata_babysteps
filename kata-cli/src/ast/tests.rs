//! Unit tests for AST types
//!
//! This test module verifies the correctness of AST structures.

use crate::ast::*;
use crate::ast::id::Literal;

// =============================================================================
// ID MODULE TESTS
// =============================================================================

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

// =============================================================================
// TYPES MODULE TESTS
// =============================================================================

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

// =============================================================================
// PATTERN MODULE TESTS
// =============================================================================

mod pattern_tests {
    use super::*;

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
        let p = Pattern::tuple(vec![
            Pattern::var("a"),
            Pattern::var("b"),
            Pattern::wildcard(),
        ]);
        assert!(p.captures_variables());
        assert_eq!(p.captured_variables().len(), 2);
        assert_eq!(p.to_string(), "(a b _)");
    }

    #[test]
    fn test_pattern_variant() {
        let p = Pattern::variant("Ok", vec![Pattern::var("value")]);
        assert_eq!(p.to_string(), "Ok(value)");
        assert!(p.captures_variables());
    }

    #[test]
    fn test_pattern_variant_unit() {
        let p = Pattern::variant("None", vec![]);
        assert_eq!(p.to_string(), "None");
        assert!(!p.captures_variables());
    }

    #[test]
    fn test_pattern_or() {
        let p = Pattern::Or(vec![
            Pattern::literal(Literal::int("0")),
            Pattern::literal(Literal::int("1")),
        ]);
        assert_eq!(p.to_string(), "0 | 1");
    }

    #[test]
    fn test_pattern_empty_list() {
        let p = Pattern::empty_list();
        assert_eq!(p.to_string(), "[]");
        assert!(!p.captures_variables());
    }

    #[test]
    fn test_pattern_list_with_rest() {
        let p = Pattern::List {
            elements: vec![Pattern::var("head")],
            rest: Some(Box::new(Pattern::var("tail"))),
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
}

// =============================================================================
// EXPR MODULE TESTS
// =============================================================================

mod expr_tests {
    use super::*;

    #[test]
    fn test_expr_literal() {
        let e = Expr::literal(Literal::int("42"));
        assert!(e.is_literal());
        assert!(!e.is_var());
        assert_eq!(e.to_string(), "42");
    }

    #[test]
    fn test_expr_var() {
        let e = Expr::var("x");
        assert!(e.is_var());
        assert!(!e.is_literal());
        assert_eq!(e.to_string(), "x");
    }

    #[test]
    fn test_expr_hole() {
        let e = Expr::hole();
        assert!(e.is_hole());
        assert_eq!(e.to_string(), "_");
    }

    #[test]
    fn test_expr_tuple() {
        let e = Expr::tuple(vec![
            Expr::literal(Literal::int("1")),
            Expr::literal(Literal::int("2")),
        ]);
        assert_eq!(e.to_string(), "(1 2)");
    }

    #[test]
    fn test_expr_list() {
        let e = Expr::list(vec![
            Expr::literal(Literal::int("1")),
            Expr::literal(Literal::int("2")),
            Expr::literal(Literal::int("3")),
        ]);
        assert_eq!(e.to_string(), "[1 2 3]");
    }

    #[test]
    fn test_expr_array() {
        let e = Expr::array(vec![
            Expr::literal(Literal::int("1")),
            Expr::literal(Literal::int("2")),
        ]);
        assert_eq!(e.to_string(), "{1 2}");
    }

    #[test]
    fn test_expr_apply() {
        let e = Expr::apply(
            Expr::var("+"),
            vec![Expr::literal(Literal::int("1")), Expr::literal(Literal::int("2"))],
        );
        assert_eq!(e.to_string(), "+ 1 2");
    }

    #[test]
    fn test_expr_pipeline() {
        let e = Expr::pipeline(
            Expr::list(vec![Expr::literal(Literal::int("1"))]),
            Expr::var("map"),
        );
        assert_eq!(e.to_string(), "[1] |> map");
    }

    #[test]
    fn test_lambda_clause() {
        let clause = LambdaClause::new(
            vec![Pattern::var("x"), Pattern::var("y")],
            Expr::apply(Expr::var("+"), vec![Expr::var("x"), Expr::var("y")]),
        );
        assert_eq!(clause.to_string(), "(x y): + x y");
    }
}

// =============================================================================
// STMT MODULE TESTS
// =============================================================================

mod stmt_tests {
    use super::*;

    #[test]
    fn test_stmt_let() {
        let s = Stmt::let_binding("x", Expr::literal(Literal::int("42")));
        assert_eq!(s.to_string(), "let x 42");
    }

    #[test]
    fn test_stmt_var() {
        let s = Stmt::var_binding("counter", Expr::literal(Literal::int("0")));
        assert_eq!(s.to_string(), "var counter 0");
    }

    #[test]
    fn test_stmt_assign() {
        let s = Stmt::assign("x", Expr::literal(Literal::int("1")));
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
        let s = Stmt::Return(Expr::literal(Literal::int("42")));
        assert_eq!(s.to_string(), "return 42");
        assert!(s.is_control_flow());
    }

    #[test]
    fn test_stmt_loop() {
        let s = Stmt::loop_stmt(vec![
            Stmt::let_binding("x", Expr::literal(Literal::int("1"))),
        ]);
        assert!(s.to_string().contains("loop"));
    }

    #[test]
    fn test_stmt_for() {
        let s = Stmt::for_stmt(
            "item",
            Expr::var("lista"),
            vec![Stmt::expr(Expr::var("item"))],
        );
        let output = s.to_string();
        assert!(output.contains("for item in"));
        assert!(output.contains("lista"));
    }

    #[test]
    fn test_match_case() {
        let case = MatchCase::single(
            Pattern::variant("Ok", vec![Pattern::var("value")]),
            Stmt::expr(Expr::var("value")),
        );
        assert!(case.to_string().contains("Ok(value)"));
    }

    #[test]
    fn test_channel_op_display() {
        let recv = ChannelOp::Receive {
            channel: Expr::var("rx"),
            non_blocking: false,
        };
        assert_eq!(recv.to_string(), "<! rx");

        let recv_nb = ChannelOp::Receive {
            channel: Expr::var("rx"),
            non_blocking: true,
        };
        assert_eq!(recv_nb.to_string(), "<!? rx");

        let send = ChannelOp::Send {
            value: Expr::literal(Literal::string("hello")),
            channel: Expr::var("tx"),
        };
        assert_eq!(send.to_string(), "\"hello\" !> tx");
    }

    #[test]
    fn test_error_propagation() {
        let ep = ErrorPropagation { expr: Expr::var("result") };
        assert_eq!(ep.to_string(), "result?");
    }
}

// =============================================================================
// DECL MODULE TESTS
// =============================================================================

mod decl_tests {
    use super::*;

    #[test]
    fn test_function_def() {
        let func = FunctionDef::new(
            "soma",
            FunctionSig::binary(Type::named("Int"), Type::named("Int"), Type::named("Int")),
        );
        assert_eq!(func.name.to_string(), "soma");
        assert_eq!(func.arity, 2);
        assert!(!func.is_ffi());
    }

    #[test]
    fn test_function_def_ffi() {
        let func = FunctionDef::new(
            "+",
            FunctionSig::binary(Type::named("Int"), Type::named("Int"), Type::named("Int")),
        )
        .with_directives(vec![Directive::Ffi { symbol: "kata_rt_add".to_string() }]);
        assert!(func.is_ffi());
    }

    #[test]
    fn test_action_def() {
        let action = ActionDef::new("main")
            .with_params(vec![Ident::new("args")]);
        assert_eq!(action.name.to_string(), "main");
        assert_eq!(action.params.len(), 1);
        assert!(!action.is_parallel());
    }

    #[test]
    fn test_action_def_parallel() {
        let action = ActionDef::new("worker")
            .with_directives(vec![Directive::Parallel]);
        assert!(action.is_parallel());
    }
    #[test]
    fn test_data_def() {
        let data = DataDef::new("Vec2")
            .add_field("x", Some(Type::named("Float")))
            .add_field("y", Some(Type::named("Float")));
        if let DataKind::Product(fields) = &data.kind {
            assert_eq!(fields.len(), 2);
        } else {
            panic!("Expected Product kind");
        }
        assert_eq!(data.to_string(), "data Vec2 (x::Float y::Float)");
    }

    #[test]
    fn test_data_def_generic() {
        let data = DataDef::new("Caixa")
            .with_type_params(vec![Ident::new("T")])
            .add_field("conteudo", Some(Type::var("T")))
            .add_field("peso", Some(Type::named("Int")));
        assert_eq!(data.type_params.len(), 1);
    }

    #[test]
    fn test_enum_def() {
        let enum_def = EnumDef::new("Bool")
            .add_unit_variant("True")
            .add_unit_variant("False");
        assert_eq!(enum_def.variants.len(), 2);
    }

    #[test]
    fn test_enum_def_generic() {
        let enum_def = EnumDef::new("Result")
            .with_type_params(vec![Ident::new("T"), Ident::new("E")])
            .add_typed_variant("Ok", Type::var("T"))
            .add_typed_variant("Err", Type::var("E"));
        assert_eq!(enum_def.type_params.len(), 2);
        assert_eq!(enum_def.variants.len(), 2);
    }

    #[test]
    fn test_interface_def() {
        let interface = InterfaceDef::new("EQ")
            .with_extends(vec![Ident::new("HASH")]);
        assert_eq!(interface.name.to_string(), "EQ");
        assert_eq!(interface.extends.len(), 1);
    }

    #[test]
    fn test_impl_def() {
        let impl_def = ImplDef::new(
            QualifiedIdent::simple("Int"),
            "NUM",
        );
        assert_eq!(impl_def.type_name.to_string(), "Int");
        assert_eq!(impl_def.interface.to_string(), "NUM");
    }

    #[test]
    fn test_alias_def() {
        let alias = AliasDef::new("NonZero", Type::refined(
            Type::named("Int"),
            Predicate::Comparison {
                op: CompareOp::Neq,
                value: LiteralValue::Int(0),
            },
        ));
        assert_eq!(alias.name.to_string(), "NonZero");
    }

    #[test]
    fn test_import_namespace() {
        let import = Import::Namespace { module: "types".to_string() };
        assert_eq!(import.to_string(), "import types");
    }

    #[test]
    fn test_import_item() {
        let import = Import::Item {
            module: "types".to_string(),
            item: "NUM".to_string(),
        };
        assert_eq!(import.to_string(), "import types.NUM");
    }

    #[test]
    fn test_import_items() {
        let import = Import::Items {
            module: "types".to_string(),
            items: vec!["NUM".to_string(), "ORD".to_string()],
        };
        assert_eq!(import.to_string(), "import types.(NUM ORD)");
    }

    #[test]
    fn test_export() {
        let export = Export::new(vec![Ident::new("soma"), Ident::new("subtrai")]);
        assert_eq!(export.items.len(), 2);
        assert_eq!(export.to_string(), "export soma subtrai");
    }

    #[test]
    fn test_module() {
        let module = Module::new("main")
            .with_imports(vec![Import::Namespace { module: "types".to_string() }])
            .with_declarations(vec![TopLevel::Function(
                FunctionDef::new("main", FunctionSig::nullary(Type::named("Unit")))
            )])
            .with_exports(vec![Ident::new("main")]);
        assert_eq!(module.name, "main");
        assert_eq!(module.imports.len(), 1);
        assert_eq!(module.declarations.len(), 1);
        assert_eq!(module.exports.len(), 1);
    }
}