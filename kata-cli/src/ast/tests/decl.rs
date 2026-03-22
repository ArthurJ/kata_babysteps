use crate::ast::*;

fn span(start: usize, end: usize) -> LexerSpan {
    LexerSpan { start, end }
}

fn spanned<T>(node: T, start: usize, end: usize) -> Spanned<T> {
    Spanned::new(node, span(start, end))
}

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
        .with_declarations(vec![
            spanned(TopLevel::Function(
                FunctionDef::new("main", FunctionSig::nullary(Type::named("Unit")))
            ), 0, 50)
        ])
        .with_exports(vec![Ident::new("main")]);
    assert_eq!(module.name, "main");
    assert_eq!(module.imports.len(), 1);
    assert_eq!(module.declarations.len(), 1);
    assert_eq!(module.exports.len(), 1);
}
