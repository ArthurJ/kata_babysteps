use kata::ast::id::{Ident, QualifiedIdent};
use kata::ast::types::{Type, FunctionSig};
use kata::ast::decl::{TopLevel, ImplDef, FunctionDef};
use kata::ast::expr::LambdaClause;
use kata::ast::pattern::Pattern;
use kata::type_checker::checker::Checker;
use kata::type_checker::environment::InterfaceInfo;
use std::collections::HashMap;

#[test]
fn test_valid_implementation() {
    let mut checker = Checker::new();
    
    // Define a local interface: interface NUM { + :: NUM NUM => NUM }
    let mut members = HashMap::new();
    let num_sig = FunctionSig::binary(Type::named("NUM"), Type::named("NUM"), Type::named("NUM"));
    members.insert("+".to_string(), num_sig.clone());
    
    let interface_info = InterfaceInfo {
        name: "NUM".to_string(),
        extends: vec![],
        members,
    };
    checker.env.register_interface(interface_info, true); // Local interface
    
    // Define a local type: data Int
    checker.env.register_type("Int", Type::named("Int"), true); // Local type
    
    // Create an implementation: Int implements NUM { + :: Int Int => Int ... }
    // Note: The implementation signatures must match exactly for now
    let impl_func = FunctionDef::new("+", num_sig)
        .add_clause(LambdaClause {
            patterns: vec![
                Pattern::Var(Ident::new("x")),
                Pattern::Var(Ident::new("y")),
            ],
            guards: vec![],
            body: None,
            with: vec![],
        });
        
    let impl_def = ImplDef::new(QualifiedIdent::simple("Int"), "NUM")
        .add_implementation(impl_func);
        
    let result = checker.check_module(vec![TopLevel::Implements(impl_def)]);
    assert!(result.is_ok(), "Expected valid implementation to pass, got {:?}", result.err());
}

#[test]
fn test_orphan_rule_violation() {
    let mut checker = Checker::new();
    
    // Define a FOREIGN interface
    let interface_info = InterfaceInfo {
        name: "FOREIGN_IFACE".to_string(),
        extends: vec![],
        members: HashMap::new(),
    };
    checker.env.register_interface(interface_info, false); // NOT local
    
    // Define a FOREIGN type
    checker.env.register_type("ForeignType", Type::named("ForeignType"), false); // NOT local
    
    // Create an implementation: ForeignType implements FOREIGN_IFACE
    let impl_def = ImplDef::new(QualifiedIdent::simple("ForeignType"), "FOREIGN_IFACE");
        
    let result = checker.check_module(vec![TopLevel::Implements(impl_def)]);
    
    assert!(result.is_err(), "Expected Orphan Rule violation to fail");
    let err = result.unwrap_err();
    assert!(format!("{}", err).contains("Orphan Rule Violation"));
}

#[test]
fn test_missing_member() {
    let mut checker = Checker::new();
    
    // Define a local interface with a member
    let mut members = HashMap::new();
    members.insert("must_have".to_string(), FunctionSig::nullary(Type::named("Unit")));
    
    let interface_info = InterfaceInfo {
        name: "IFACE".to_string(),
        extends: vec![],
        members,
    };
    checker.env.register_interface(interface_info, true);
    
    // Define a local type
    checker.env.register_type("MyType", Type::named("MyType"), true);
    
    // Empty implementation (missing 'must_have')
    let impl_def = ImplDef::new(QualifiedIdent::simple("MyType"), "IFACE");
        
    let result = checker.check_module(vec![TopLevel::Implements(impl_def)]);
    
    assert!(result.is_err(), "Expected failure due to missing member");
    let err = result.unwrap_err();
    assert!(format!("{}", err).contains("Unbound Variable"));
    assert!(format!("{}", err).contains("must_have"));
}

#[test]
fn test_signature_mismatch() {
    let mut checker = Checker::new();
    
    // Define a local interface
    let mut members = HashMap::new();
    let expected_sig = FunctionSig::nullary(Type::named("Int"));
    members.insert("foo".to_string(), expected_sig);
    
    let interface_info = InterfaceInfo {
        name: "IFACE".to_string(),
        extends: vec![],
        members,
    };
    checker.env.register_interface(interface_info, true);
    
    checker.env.register_type("MyType", Type::named("MyType"), true);
    
    // Implementation with WRONG signature (returns Float instead of Int)
    let wrong_sig = FunctionSig::nullary(Type::named("Float"));
    let impl_func = FunctionDef::new("foo", wrong_sig)
        .add_clause(LambdaClause { 
            patterns: vec![], 
            guards: vec![], 
            body: None, 
            with: vec![] 
        });
        
    let impl_def = ImplDef::new(QualifiedIdent::simple("MyType"), "IFACE")
        .add_implementation(impl_func);
        
    let result = checker.check_module(vec![TopLevel::Implements(impl_def)]);
    
    assert!(result.is_err(), "Expected signature mismatch to fail");
    let err = result.unwrap_err();
    assert!(format!("{}", err).contains("Type Mismatch"));
}
