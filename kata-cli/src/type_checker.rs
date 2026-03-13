use std::collections::HashMap;
use crate::ast::*;
use crate::error::KataError;
use crate::typed_ast::*;
use crate::recursion_analysis::{RecursionAnalyzer, RecursionAnalysis};

#[derive(Debug, Clone, PartialEq)]
pub enum Type {
    Int,
    Float,
    Text,
    Bool,
    List(Box<Type>),
    Array(Box<Type>),
    Tensor(Box<Type>, Vec<usize>),
    Tuple(Vec<Type>),
    Func { args: Vec<Type>, ret: Box<Type> },
    Custom(String),
    Interface(String),
    Unknown, // Fallback/Placeholder durante inferência
}

impl std::fmt::Display for Type {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Type::Int => write!(f, "Int"),
            Type::Float => write!(f, "Float"),
            Type::Text => write!(f, "Text"),
            Type::Bool => write!(f, "Bool"),
            Type::List(inner) => write!(f, "List::{}", inner),
            Type::Array(inner) => write!(f, "Array::{}", inner),
            Type::Tensor(inner, _shape) => write!(f, "Tensor::{}::(...)", inner),
            Type::Tuple(types) => {
                let inner: Vec<String> = types.iter().map(|t| format!("{}", t)).collect();
                write!(f, "({})", inner.join(" "))
            }
            Type::Func { args, ret } => {
                let args_str: Vec<String> = args.iter().map(|t| format!("{}", t)).collect();
                write!(f, "({}) => {}", args_str.join(" "), ret)
            }
            Type::Custom(name) => write!(f, "{}", name),
            Type::Interface(name) => write!(f, "{}", name),
            Type::Unknown => write!(f, "?"),
        }
    }
}

#[derive(Debug, Clone)]
pub struct FuncSignature {
    pub name: String,
    pub arity: usize,
    pub args_types: Vec<Type>,
    pub return_type: Type,
    pub is_action: bool,
    pub ffi_binding: Option<String>,
}

#[derive(Debug, Clone)]
pub struct TypeEnv {
    pub functions: HashMap<String, FuncSignature>,
    // Local scope for variable bindings within a block
    pub local_bindings: Vec<HashMap<String, Type>>, 
    // Maps a Type name (e.g. "Int") to a list of Interfaces it implements (e.g. "NUM")
    pub implementations: HashMap<String, Vec<String>>,
    // (TypeName, MethodName) -> ImplementationName (e.g. ("Int", "+") -> "__add_int")
    pub method_resolutions: HashMap<(String, String), String>,
    // InterfaceName -> List of Method Signatures
    pub interfaces: HashMap<String, Vec<FuncSignature>>,
}

impl TypeEnv {
    pub fn new() -> Self {
        Self {
            functions: HashMap::new(),
            local_bindings: vec![HashMap::new()], // Escopo global base
            implementations: HashMap::new(),
            method_resolutions: HashMap::new(),
            interfaces: HashMap::new(),
        }
    }

    pub fn get_arity(&self, name: &str) -> usize {
        self.functions.get(name).map(|sig| sig.arity).unwrap_or(0)
    }

    pub fn get_signature(&self, name: &str) -> Option<&FuncSignature> {
        self.functions.get(name)
    }

    pub fn push_scope(&mut self) {
        self.local_bindings.push(HashMap::new());
    }

    pub fn pop_scope(&mut self) {
        self.local_bindings.pop();
    }

    pub fn insert_local(&mut self, name: String, ty: Type) {
        if let Some(scope) = self.local_bindings.last_mut() {
            scope.insert(name, ty);
        }
    }

    pub fn lookup_local(&self, name: &str) -> Option<Type> {
        for scope in self.local_bindings.iter().rev() {
            if let Some(ty) = scope.get(name) {
                return Some(ty.clone());
            }
        }
        None
    }
    
    pub fn type_implements_interface(&self, type_name: &str, interface_name: &str) -> bool {
        if let Some(interfaces) = self.implementations.get(type_name) {
            interfaces.contains(&interface_name.to_string())
        } else {
            false
        }
    }

    pub fn resolve_method(&self, type_name: &str, method_name: &str) -> Option<String> {
        self.method_resolutions.get(&(type_name.to_string(), method_name.to_string())).cloned()
    }
}

pub struct TypeChecker {
    pub env: TypeEnv,
}

impl TypeChecker {
    pub fn new() -> Self {
        let mut env = TypeEnv::new();
        
        env.functions.insert("str".to_string(), FuncSignature {
            name: "str".to_string(),
            arity: 1,
            args_types: vec![Type::Unknown],
            return_type: Type::Text,
            is_action: false,
            ffi_binding: None,
        });
        
        // HACKS MÍNIMOS PARA REPL/BOOTSTRAP
        env.functions.insert("=".to_string(), FuncSignature { name: "=".to_string(), arity: 2, args_types: vec![Type::Unknown, Type::Unknown], return_type: Type::Bool, is_action: false, ffi_binding: None });
        env.functions.insert("mod".to_string(), FuncSignature { name: "mod".to_string(), arity: 2, args_types: vec![Type::Int, Type::Int], return_type: Type::Int, is_action: false, ffi_binding: None });
        env.functions.insert("and".to_string(), FuncSignature { name: "and".to_string(), arity: 2, args_types: vec![Type::Bool, Type::Bool], return_type: Type::Bool, is_action: false, ffi_binding: None });
        env.functions.insert("==".to_string(), FuncSignature { name: "==".to_string(), arity: 2, args_types: vec![Type::Unknown, Type::Unknown], return_type: Type::Bool, is_action: false, ffi_binding: None });

        // Funções FFI concretas de baixo nível
        env.functions.insert("__add_int".to_string(), FuncSignature { name: "__add_int".to_string(), arity: 2, args_types: vec![Type::Int, Type::Int], return_type: Type::Int, is_action: false, ffi_binding: Some("kata_rt_add_int".to_string()) });
        env.functions.insert("__sub_int".to_string(), FuncSignature { name: "__sub_int".to_string(), arity: 2, args_types: vec![Type::Int, Type::Int], return_type: Type::Int, is_action: false, ffi_binding: Some("kata_rt_sub_int".to_string()) });
        env.functions.insert("__mul_int".to_string(), FuncSignature { name: "__mul_int".to_string(), arity: 2, args_types: vec![Type::Int, Type::Int], return_type: Type::Int, is_action: false, ffi_binding: Some("kata_rt_mul_int".to_string()) });
        env.functions.insert("__div_int".to_string(), FuncSignature { name: "__div_int".to_string(), arity: 2, args_types: vec![Type::Int, Type::Int], return_type: Type::Int, is_action: false, ffi_binding: Some("kata_rt_div_int".to_string()) });
        env.functions.insert("__eq_int".to_string(), FuncSignature { name: "__eq_int".to_string(), arity: 2, args_types: vec![Type::Int, Type::Int], return_type: Type::Bool, is_action: false, ffi_binding: Some("kata_rt_eq_int".to_string()) });
        env.functions.insert("__int_to_str".to_string(), FuncSignature { name: "__int_to_str".to_string(), arity: 1, args_types: vec![Type::Int], return_type: Type::Text, is_action: false, ffi_binding: Some("kata_rt_int_to_str".to_string()) });
        env.functions.insert("__list_to_str".to_string(), FuncSignature { name: "__list_to_str".to_string(), arity: 1, args_types: vec![Type::Custom("List::Text".to_string())], return_type: Type::Text, is_action: false, ffi_binding: Some("kata_rt_list_to_str".to_string()) });

        // Novas FFI para collections com KataClosure
        env.functions.insert("kata_rt_list_from_range".to_string(), FuncSignature {
            name: "kata_rt_list_from_range".to_string(),
            arity: 5,
            args_types: vec![Type::Int, Type::Int, Type::Int, Type::Int, Type::Int],
            return_type: Type::Unknown, // Retorna ponteiro de lista
            is_action: false,
            ffi_binding: Some("kata_rt_list_from_range".to_string())
        });

        env.functions.insert("kata_rt_map".to_string(), FuncSignature {
            name: "kata_rt_map".to_string(),
            arity: 2,
            args_types: vec![Type::Unknown, Type::Unknown], // closure, list
            return_type: Type::Unknown, // Retorna nova lista
            is_action: false,
            ffi_binding: Some("kata_rt_map".to_string())
        });

        Self { env }
    }

    fn ident_to_type(&self, ident: &Ident) -> Type {
        match ident {
            Ident::Type(t) if t == "Int" => Type::Int,
            Ident::Type(t) if t == "Float" => Type::Float,
            Ident::Type(t) if t == "Text" => Type::Text,
            Ident::Type(t) if t == "Bool" => Type::Bool,
            Ident::Type(t) => {
                if t.contains("::") {
                    let mut parts = t.split("::");
                    let base = parts.next().unwrap();
                    let inner = parts.next().unwrap();
                    match base {
                        "List" => Type::List(Box::new(self.ident_to_type(&Ident::Type(inner.to_string())))),
                        "Array" => Type::Array(Box::new(self.ident_to_type(&Ident::Type(inner.to_string())))),
                        _ => Type::Custom(t.clone()),
                    }
                } else {
                    Type::Custom(t.clone())
                }
            }
            Ident::Interface(t) => Type::Interface(t.clone()),
            _ => Type::Unknown,
        }
    }

    pub fn discover(&mut self, module: &ModuleAST) -> Result<(), KataError> {
        for decl in &module.declarations {
            match decl {
                TopLevelDecl::SignatureDecl { attrs, name, sig } => {
                    let name_str = match name {
                        Ident::Func(n) | Ident::Action(n) | Ident::Symbol(n) => n.clone(),
                        _ => continue,
                    };
                    let arity = sig.args.len();
                    let args_types = sig.args.iter().map(|a| self.ident_to_type(a)).collect();
                    let return_type = self.ident_to_type(&sig.ret);
                    let mut ffi_binding = None;
                    for attr in attrs {
                        if attr.name == "ffi" && !attr.args.is_empty() {
                            ffi_binding = Some(attr.args[0].clone());
                        }
                    }
                    self.env.functions.insert(name_str.clone(), FuncSignature {
                        name: name_str.clone(),
                        arity,
                        args_types,
                        return_type,
                        is_action: name_str.ends_with('!'),
                        ffi_binding,
                    });
                }
                TopLevelDecl::InterfaceDef { name, supertraits: _, methods } => {
                    if let Ident::Interface(i_name) = name {
                        let mut sigs = Vec::new();
                        for m in methods {
                            if let TopLevelDecl::SignatureDecl { attrs: _, name: m_name, sig } = m {
                                let method_name = match m_name {
                                    Ident::Func(n) | Ident::Symbol(n) => n.clone(),
                                    _ => continue,
                                };
                                sigs.push(FuncSignature {
                                    name: method_name,
                                    arity: sig.args.len(),
                                    args_types: sig.args.iter().map(|a| self.ident_to_type(a)).collect(),
                                    return_type: self.ident_to_type(&sig.ret),
                                    is_action: false,
                                    ffi_binding: None,
                                });
                            }
                        }
                        self.env.interfaces.insert(i_name.clone(), sigs);
                    }
                }
                TopLevelDecl::Implements { target_type, interface, methods } => {
                    if let (Ident::Type(t_name), Ident::Interface(i_name)) = (target_type, interface) {
                        self.env.implementations
                            .entry(t_name.clone())
                            .or_insert_with(Vec::new)
                            .push(i_name.clone());

                        for method in methods {
                            if let TopLevelDecl::Definition { name, expr } = method {
                                let method_name = match name {
                                    Ident::Func(n) | Ident::Symbol(n) => n.clone(),
                                    _ => continue,
                                };
                                let mut resolved_to = format!("impl_{}_{}_{}", t_name, i_name, method_name);
                                if let DataExpr::LambdaGroup { branches } = expr {
                                    if branches.len() == 1 {
                                        if let DataExpr::Seq(ref items) = branches[0].body {
                                            if items.len() == 1 {
                                                if let DataExpr::Identifier(Ident::Func(ref target)) = items[0] {
                                                    resolved_to = target.clone();
                                                } else if let DataExpr::Identifier(Ident::Symbol(ref target)) = items[0] {
                                                    resolved_to = target.clone();
                                                }
                                            }
                                        }
                                    }
                                    if resolved_to.starts_with("impl_") {
                                         self.env.functions.insert(resolved_to.clone(), FuncSignature {
                                            name: resolved_to.clone(),
                                            arity: branches[0].params.len(),
                                            args_types: vec![Type::Unknown; branches[0].params.len()],
                                            return_type: Type::Unknown,
                                            is_action: false,
                                            ffi_binding: None,
                                        });
                                    }
                                }
                                self.env.method_resolutions.insert((t_name.clone(), method_name.clone()), resolved_to);
                            }
                        }
                    }
                }
                TopLevelDecl::ActionDef { attrs, name, params, body: _ } => {
                    let name_str = match name {
                        Ident::Action(n) => n.clone(),
                        Ident::Func(n) => format!("{}!", n),
                        _ => continue,
                    };
                    let arity = params.len();
                    let mut ffi_binding = None;
                    for attr in attrs {
                        if attr.name == "ffi" && !attr.args.is_empty() {
                            ffi_binding = Some(attr.args[0].clone());
                        }
                    }
                    if !self.env.functions.contains_key(&name_str) {
                        self.env.functions.insert(name_str.clone(), FuncSignature {
                            name: name_str.clone(),
                            arity,
                            args_types: vec![Type::Unknown; arity],
                            return_type: Type::Unknown,
                            is_action: true,
                            ffi_binding,
                        });
                    }
                }
                TopLevelDecl::Definition { name, .. } => {
                    let name_str = match name {
                        Ident::Func(n) | Ident::Symbol(n) => n.clone(),
                        _ => continue,
                    };
                    if !self.env.functions.contains_key(&name_str) {
                        self.env.functions.insert(name_str.clone(), FuncSignature {
                            name: name_str.clone(),
                            arity: 0,
                            args_types: vec![],
                            return_type: Type::Unknown,
                            is_action: false,
                            ffi_binding: None,
                        });
                    }
                }
                _ => {} 
            }
        }
        Ok(())
    }

    pub fn resolve_module(&mut self, module: ModuleAST) -> Result<TypedModule, KataError> {
        let mut new_decls = Vec::new();
        for decl in module.declarations {
            match decl {
                TopLevelDecl::Definition { name, expr } => {
                    // Busca assinatura da função ANTES de entrar no escopo
                    let param_types: Vec<Type> = if let DataExpr::LambdaGroup { branches } = &expr {
                        if let Some(first_branch) = branches.first() {
                            if !first_branch.params.is_empty() {
                                let func_name = match &name {
                                    Ident::Func(n) | Ident::Symbol(n) => n.clone(),
                                    _ => String::new(),
                                };
                                self.env.get_signature(&func_name)
                                    .map(|sig| sig.args_types.clone())
                                    .unwrap_or_default()
                            } else { Vec::new() }
                        } else { Vec::new() }
                    } else { Vec::new() };

                    self.env.push_scope();

                    // Análise de recursão ANTES de resolver (precisa da AST original)
                    eprintln!("DEBUG RecursionAnalysis: Processando definição de {:?}, expr tipo {:?}", name, std::mem::discriminant(&expr));

                    if let DataExpr::LambdaGroup { ref branches } = expr {
                        eprintln!("DEBUG RecursionAnalysis: É LambdaGroup com {} branches", branches.len());
                        let func_name = match &name {
                            Ident::Func(n) | Ident::Symbol(n) => n.clone(),
                            _ => String::new(),
                        };

                        eprintln!("DEBUG RecursionAnalysis: LambdaGroup encontrado para '{}' com {} branches", func_name, branches.len());

                        if !func_name.is_empty() {
                            let analyzer = RecursionAnalyzer::new();
                            let analysis = analyzer.analyze_function(&func_name, branches);

                            eprintln!("DEBUG RecursionAnalysis: Análise para '{}': {:?}", func_name, analysis);

                            // Verifica limites de profundidade
                            if let Err(e) = analyzer.check_depth_limit(&analysis) {
                                return Err(e);
                            }

                            // Emite warning para recursão não-TCO
                            eprintln!("DEBUG RecursionAnalysis: Verificando se precisa emitir warning...");
                            match &analysis {
                                RecursionAnalysis::NonTailRecursive { reason, .. } => {
                                    let reason_str = format!("{:?}", reason);
                                    eprintln!("AVISO: Função '{}' é recursiva sem TCO ({}). Considere reescrever com acumulador.",
                                             func_name, reason_str);
                                }
                                RecursionAnalysis::TailRecursive { .. } => {
                                    eprintln!("DEBUG RecursionAnalysis: Função '{}' é tail recursive - OK", func_name);
                                }
                                _ => {
                                    eprintln!("DEBUG RecursionAnalysis: Análise não requer warning: {:?}", analysis);
                                }
                            }
                        }

                        // Agora tipa os parâmetros
                        if let Some(first_branch) = branches.first() {
                            for (i, pat) in first_branch.params.iter().enumerate() {
                                if i < param_types.len() {
                                    if let Pattern::Identifier(Ident::Func(p_name)) = pat {
                                        self.env.insert_local(p_name.clone(), param_types[i].clone());
                                    }
                                }
                            }
                        }
                    }

                    let typed_expr = self.resolve_expr(expr, false)?;

                    self.env.pop_scope();
                    new_decls.push(TypedTopLevel::Definition { name, expr: typed_expr });
                }
                TopLevelDecl::ActionDef { attrs, name, params, body } => {
                    self.env.push_scope();
                    let mut typed_body = Vec::new();
                    for stmt in body {
                        typed_body.push(self.resolve_action(stmt)?);
                    }
                    self.env.pop_scope();
                    new_decls.push(TypedTopLevel::ActionDef { attrs, name, params, body: typed_body });
                }
                TopLevelDecl::Implements { target_type, interface, methods } => {
                    if let (Ident::Type(t_name), Ident::Interface(i_name)) = (&target_type, &interface) {
                        for method in methods {
                            if let TopLevelDecl::Definition { name, expr } = method {
                                let method_name = match name {
                                    Ident::Func(n) | Ident::Symbol(n) => n.clone(),
                                    _ => continue,
                                };
                                let resolved_name = self.env.resolve_method(t_name, &method_name).unwrap_or_else(|| {
                                    format!("impl_{}_{}_{}", t_name, i_name, method_name)
                                });
                                if resolved_name.starts_with("__") || resolved_name.starts_with("kata_rt") {
                                    continue;
                                }
                                self.env.push_scope();
                                if let DataExpr::LambdaGroup { ref branches } = expr {
                                    let mut sig_to_use = None;
                                    if let Some(iface_methods) = self.env.interfaces.get(i_name) {
                                        if let Some(sig) = iface_methods.iter().find(|s| s.name == method_name) {
                                            sig_to_use = Some(sig.clone());
                                        }
                                    }
                                    if let Some(sig) = sig_to_use {
                                        for branch in branches {
                                            for (i, param) in branch.params.iter().enumerate() {
                                                if i < sig.args_types.len() {
                                                    let mut ty = sig.args_types[i].clone();
                                                    if let Type::Interface(ref n) = ty {
                                                        if n == i_name { ty = self.ident_to_type(&target_type); }
                                                    }
                                                    if let Pattern::Identifier(Ident::Func(p_name)) = param {
                                                        self.env.insert_local(p_name.clone(), ty);
                                                    }
                                                }
                                            }
                                        }
                                    }
                                }
                                let typed_expr = self.resolve_expr(expr, false)?;
                                self.env.pop_scope();
                                new_decls.push(TypedTopLevel::Definition {
                                    name: Ident::Func(resolved_name),
                                    expr: typed_expr,
                                });
                            }
                        }
                    }
                }
                _ => {}
            };
        }
        Ok(TypedModule { declarations: new_decls })
    }

    fn resolve_action(&mut self, stmt: ActionStmt) -> Result<TypedActionStmt, KataError> {
        match stmt {
            ActionStmt::Expr(expr) => {
                let t_expr = self.resolve_expr(expr, true)?;
                Ok(TypedActionStmt::Expr(t_expr))
            }
            ActionStmt::LetBind { pattern, expr, type_annotation } => {
                let t_expr = self.resolve_expr(expr, true)?;
                if let Pattern::Identifier(Ident::Func(n)) = &pattern {
                    self.env.insert_local(n.clone(), t_expr.ty.clone());
                }
                Ok(TypedActionStmt::LetBind { pattern, expr: t_expr, type_annotation })
            }
            ActionStmt::ActionCall { target, args } => {
                let mut resolved_args = Vec::new();
                for arg in args {
                    resolved_args.push(self.resolve_expr(arg, true)?);
                }
                
                let mut final_target = target;
                let target_name = match &final_target {
                    Ident::Action(n) | Ident::Func(n) | Ident::Symbol(n) => n.clone(),
                    _ => "".to_string(),
                };

                if let Some(sig) = self.env.get_signature(&target_name) {
                    if let Some(ffi) = &sig.ffi_binding {
                        final_target = Ident::Action(ffi.clone());
                    }
                }

                let final_args = if resolved_args.is_empty() {
                    Vec::new()
                } else {
                    let greedy = self.consume_greedy_sequence(resolved_args, true)?;
                    match greedy.expr {
                        TypedDataExpr::Tuple(items) => items,
                        _ => vec![greedy],
                    }
                };
                Ok(TypedActionStmt::ActionCall { target: final_target, args: final_args })
            }
            _ => Err(KataError::CrossDomainViolation { msg: "Ação não implementada".into(), span: (0,0) }),
        }
    }

    fn resolve_expr(&mut self, expr: DataExpr, in_action: bool) -> Result<TypedExpr, KataError> {
        match expr {
            DataExpr::Literal(lit) => {
                let ty = match lit {
                    Literal::Int(_) => Type::Int,
                    Literal::Float(_) => Type::Float,
                    Literal::String(_) => Type::Text,
                };
                Ok(TypedExpr { expr: TypedDataExpr::Literal(lit), ty })
            }
            DataExpr::Identifier(ident) => {
                let name = match &ident {
                    Ident::Func(n) | Ident::Symbol(n) | Ident::Action(n) => n.clone(),
                    _ => "".to_string(),
                };

                let local_ty = self.env.lookup_local(&name);
                let sig = self.env.get_signature(&name);
                eprintln!("DEBUG TypeChecker: Identifier {} - local_ty: {:?}, sig: {}", name, local_ty, sig.is_some());

                let mut ty = if let Some(local_ty) = local_ty {
                    local_ty
                } else if let Some(sig) = sig {
                    if sig.arity == 1 {
                        sig.return_type.clone()
                    } else {
                        Type::Func { args: sig.args_types.clone(), ret: Box::new(sig.return_type.clone()) }
                    }
                } else {
                    Type::Unknown
                };

                let mut final_ident = ident.clone();
                let type_name = format!("{}", ty);

                // Transforma 'main!' em 'kata_main' para exportação/linkagem
                if name == "main!" {
                    final_ident = Ident::Action("kata_main".to_string());
                } else if let Some(resolved) = self.env.resolve_method(&type_name, &name) {
                    final_ident = Ident::Func(resolved);
                } else if name == "str" && type_name == "Int" {
                    final_ident = Ident::Func("impl_Int_SHOW_str".to_string());
                } else if name == "+" && type_name == "Int" {
                    final_ident = Ident::Func("impl_Int_NUM_+".to_string());
                } else if name == "-" && type_name == "Int" {
                    final_ident = Ident::Func("impl_Int_NUM_-".to_string());
                } else if name == "*" && type_name == "Int" {
                    final_ident = Ident::Func("impl_Int_NUM_*".to_string());
                } else if name == "/" && type_name == "Int" {
                    final_ident = Ident::Func("impl_Int_NUM_/".to_string());
                } else if name == "=" {
                    final_ident = Ident::Func("impl_Int_NUM_=".to_string());
                } else if name == "mod" {
                    final_ident = Ident::Func("impl_Int_NUM_mod".to_string());
                } else if name == "and" {
                    final_ident = Ident::Func("impl_Bool_AND_and".to_string());
                }

                if let Some(sig) = self.env.get_signature(&name) {
                    if let Some(ffi) = &sig.ffi_binding {
                        final_ident = Ident::Func(ffi.clone());
                    }
                }

                Ok(TypedExpr { expr: TypedDataExpr::Identifier(final_ident), ty })
            }
            DataExpr::Tuple(items) => {
                let mut t_items = Vec::new();
                let mut tuple_types = Vec::new();
                for item in items {
                    let t_expr = self.resolve_expr(item, in_action)?;
                    tuple_types.push(t_expr.ty.clone());
                    t_items.push(t_expr);
                }
                Ok(TypedExpr { expr: TypedDataExpr::Tuple(t_items), ty: Type::Tuple(tuple_types) })
            }
            DataExpr::Seq(list) => {
                if list.is_empty() {
                    return Ok(TypedExpr { expr: TypedDataExpr::Tuple(vec![]), ty: Type::Tuple(vec![]) });
                }
                let mut resolved_list = Vec::new();
                for item in list {
                    resolved_list.push(self.resolve_expr(item, in_action)?);
                }
                self.consume_greedy_sequence(resolved_list, in_action)
            }
            DataExpr::LambdaGroup { branches } => {
                let mut t_branches = Vec::new();
                for branch in branches {
                    self.env.push_scope();
                    for pat in &branch.params {
                        // Extrai o tipo do pattern e insere no escopo
                        let pat_type = self.extract_pattern_type(pat);
                        if let Pattern::Identifier(Ident::Func(n)) = pat {
                            // Só insere se ainda não existe no escopo
                            if self.env.lookup_local(n).is_none() {
                                self.env.insert_local(n.clone(), pat_type);
                            }
                        }
                    }
                    let t_body = self.resolve_expr(branch.body, false)?;
                    t_branches.push(TypedLambdaBranch {
                        params: branch.params,
                        body: Box::new(t_body),
                    });
                    self.env.pop_scope();
                }
                Ok(TypedExpr { expr: TypedDataExpr::LambdaGroup { branches: t_branches }, ty: Type::Unknown })
            }
            DataExpr::Call { target, args } => {
                let mut resolved_args = Vec::new();
                for a in args {
                    resolved_args.push(self.resolve_expr(a, in_action)?);
                }
                let t_target = self.resolve_expr(*target, in_action)?;
                let mut list = vec![t_target];
                list.extend(resolved_args);
                self.consume_greedy_sequence(list, in_action)
            }
            DataExpr::ScopedBlock { bindings, body, with_clauses } => {
                self.env.push_scope();
                for b in bindings {
                    let t_expr = self.resolve_expr(b.expr, in_action)?;
                    if let Pattern::Identifier(Ident::Func(n)) = b.pattern {
                        self.env.insert_local(n, t_expr.ty.clone());
                    }
                }
                for w in with_clauses {
                    let t_expr = self.resolve_expr(w.expr, in_action)?;
                    if let Pattern::Identifier(Ident::Func(n)) = w.pattern {
                        self.env.insert_local(n, t_expr.ty.clone());
                    }
                }
                let t_body = self.resolve_expr(*body, in_action)?;
                self.env.pop_scope();
                Ok(t_body)
            }
            DataExpr::GuardBlock { branches, otherwise, with_clauses } => {
                self.env.push_scope();
                let mut t_with = Vec::new();
                for w in with_clauses {
                    let t_expr = self.resolve_expr(w.expr, in_action)?;
                    if let Pattern::Identifier(Ident::Func(ref n)) = w.pattern {
                        self.env.insert_local(n.clone(), t_expr.ty.clone());
                    }
                    t_with.push(TypedBinding { pattern: w.pattern, expr: t_expr });
                }
                let mut t_branches = Vec::new();
                for b in branches {
                    t_branches.push(TypedGuardBranch {
                        condition: self.resolve_expr(b.condition, in_action)?,
                        result: self.resolve_expr(b.result, in_action)?,
                    });
                }
                let t_otherwise = self.resolve_expr(*otherwise, in_action)?;
                eprintln!("DEBUG TypeChecker: otherwise type = {:?}", t_otherwise.ty);
                self.env.pop_scope();
                Ok(TypedExpr { 
                    ty: t_otherwise.ty.clone(), 
                    expr: TypedDataExpr::GuardBlock { 
                        branches: t_branches, otherwise: Box::new(t_otherwise), with_clauses: t_with 
                    } 
                })
            }
            DataExpr::Range { start, end, inclusive } => {
                // Resolve os bounds do range
                let t_start = start.map(|s| self.resolve_expr(*s, in_action)).transpose()?;
                let t_end = end.map(|e| self.resolve_expr(*e, in_action)).transpose()?;

                // Verifica se os tipos são compatíveis (Int para ranges numéricos)
                let start_ty = t_start.as_ref().map(|e| e.ty.clone());
                let end_ty = t_end.as_ref().map(|e| e.ty.clone());

                // Se ambos os bounds existem, devem ser do mesmo tipo
                if let (Some(s_ty), Some(e_ty)) = (&start_ty, &end_ty) {
                    if s_ty != e_ty {
                        return Err(KataError::CrossDomainViolation {
                            msg: format!("Range com tipos incompatíveis: {:?} e {:?}", s_ty, e_ty),
                            span: (0, 0),
                        });
                    }
                }

                // Ranges são transformados em List::Int (para ranges numéricos) ou List do tipo apropriado
                let inner_ty = start_ty.or(end_ty).unwrap_or(Type::Int);

                Ok(TypedExpr {
                    expr: TypedDataExpr::Range {
                        start: t_start.map(Box::new),
                        end: t_end.map(Box::new),
                        inclusive,
                    },
                    ty: Type::List(Box::new(inner_ty)),
                })
            }
            DataExpr::FieldAccess { target, field } => {
                let t_target = self.resolve_expr(*target, in_action)?;

                // Resolve acesso a módulo: module.func
                // Na implementação atual, transformamos em uma chamada direta
                if let TypedDataExpr::Identifier(Ident::Func(module_name)) = &t_target.expr {
                    let full_name = format!("{}.{}", module_name, field);
                    eprintln!("DEBUG TypeChecker: Resolvendo FieldAccess: {}.{}", module_name, field);

                    // Verifica se a função existe no ambiente
                    if let Some(sig) = self.env.get_signature(&full_name) {
                        let ty = if sig.arity == 1 {
                            sig.return_type.clone()
                        } else {
                            Type::Func { args: sig.args_types.clone(), ret: Box::new(sig.return_type.clone()) }
                        };
                        return Ok(TypedExpr {
                            expr: TypedDataExpr::FieldAccess {
                                target: Box::new(t_target),
                                field: field.clone(),
                            },
                            ty,
                        });
                    }

                    // Se não encontrou com o nome completo, tenta só o campo como identificador
                    // (isso permite imports de sub-módulos)
                    return Ok(TypedExpr {
                        expr: TypedDataExpr::FieldAccess {
                            target: Box::new(t_target),
                            field: field.clone(),
                        },
                        ty: Type::Unknown,
                    });
                }

                // Para outros casos (acesso a campo de struct, etc.)
                Ok(TypedExpr {
                    expr: TypedDataExpr::FieldAccess {
                        target: Box::new(t_target),
                        field: field.clone(),
                    },
                    ty: Type::Unknown,
                })
            }
            DataExpr::Pipe { left, right } => {
                let t_left = self.resolve_expr(*left, in_action)?;
                // O resultado do lado esquerdo é passado para o lado direito através do hole (_)
                // O hole (_) será substituído pelo valor do lado esquerdo
                let t_right = self.resolve_expr(*right, in_action)?;

                // O tipo do pipe é o tipo do resultado da expressão direita
                let result_ty = t_right.ty.clone();
                Ok(TypedExpr {
                    expr: TypedDataExpr::Pipe {
                        left: Box::new(t_left),
                        right: Box::new(t_right),
                    },
                    ty: result_ty,
                })
            }
            DataExpr::Tensor { elements, dimensions } => {
                // Resolve todos os elementos do tensor
                let mut t_elements = Vec::new();
                for elem in elements {
                    t_elements.push(self.resolve_expr(elem, in_action)?);
                }

                // Assume que todos os elementos têm o mesmo tipo
                let inner_ty = t_elements.first().map(|e| e.ty.clone()).unwrap_or(Type::Unknown);

                // Cria o tipo Tensor
                Ok(TypedExpr {
                    expr: TypedDataExpr::Tuple(t_elements), // Por enquanto, trata como tupla
                    ty: Type::Tensor(Box::new(inner_ty), dimensions.clone()),
                })
            }
            other => Err(KataError::CrossDomainViolation { msg: format!("Expressão não portabilizada: {:?}", other), span:(0,0) })
        }
    }

    fn consume_greedy_sequence(&self, mut list: Vec<TypedExpr>, in_action: bool) -> Result<TypedExpr, KataError> {
        let mut changed = true;
        while changed {
            changed = false;
            let mut i = 0;
            while i < list.len() {
                let t_expr = list[i].clone();
                let is_func = match &t_expr.expr {
                    TypedDataExpr::Identifier(Ident::Func(n) | Ident::Symbol(n) | Ident::Action(n)) => {
                        let arity = self.env.get_arity(n);
                        arity > 0 || n == "str" || n == "echo!" || n == "+" || n == "-" || n == "*" || n == "/" || n == "==" || n == "and"
                    }
                    _ => false,
                };

                if is_func {
                    let func_name = match &t_expr.expr {
                        TypedDataExpr::Identifier(Ident::Func(n)) | TypedDataExpr::Identifier(Ident::Symbol(n)) | TypedDataExpr::Identifier(Ident::Action(n)) => n.clone(),
                        _ => unreachable!(),
                    };
                    
                    let mut arity = self.env.get_arity(&func_name);
                    if arity == 0 {
                        match func_name.as_str() {
                            "+" | "-" | "*" | "/" | "==" | "and" => arity = 2,
                            "str" | "echo!" => arity = 1,
                            _ => {}
                        }
                    }

                    if arity > 0 && i + arity < list.len() {
                        let mut args_ok = true;
                        let mut args = Vec::new();
                        for j in 1..=arity {
                            let arg = &list[i + j];
                            let arg_is_func = match &arg.expr {
                                TypedDataExpr::Identifier(Ident::Func(n) | Ident::Symbol(n) | Ident::Action(n)) => {
                                    let a = self.env.get_arity(n);
                                    a > 0 || n == "str" || n == "+" || n == "-" || n == "*" || n == "/" || n == "==" || n == "and"
                                }
                                _ => false,
                            };
                            if arg_is_func { args_ok = false; break; }
                            args.push(arg.clone());
                        }

                        if args_ok {
                            let mut resolved_name = func_name;
                            let arg_ty = &args[0].ty;
                            let type_name = format!("{}", arg_ty);
                            
                            if let Some(sig) = self.env.get_signature(&resolved_name) {
                                eprintln!("DEBUG: Assinatura encontrada para '{}': {:?}", resolved_name, sig);
                            } else {
                                eprintln!("DEBUG: Nenhuma assinatura para '{}'", resolved_name);
                            }

                            if let Some(resolved) = self.env.resolve_method(&type_name, &resolved_name) {
                                eprintln!("DEBUG: Método resolvido: {} -> {}", resolved_name, resolved);
                                resolved_name = resolved;
                            } else {
                                eprintln!("DEBUG: Tentando resolver '{}' para tipo '{}', arg_ty={:?}", resolved_name, type_name, arg_ty);
                                // Para operadores aritméticos, assume Int se o tipo for Unknown
                                let effective_type = if type_name == "?" {
                                    match resolved_name.as_str() {
                                        "+" | "-" | "*" | "/" | "mod" | "=" => "Int",
                                        _ => &type_name,
                                    }
                                } else {
                                    &type_name
                                };
                                match resolved_name.as_str() {
                                    "+" if effective_type == "Int" => resolved_name = "impl_Int_NUM_+".to_string(),
                                    "-" if effective_type == "Int" => {
                                        eprintln!("DEBUG: Resolvendo '-' para impl_Int_NUM_-, arg_ty={}", effective_type);
                                        resolved_name = "impl_Int_NUM_-".to_string();
                                    }
                                    "*" if effective_type == "Int" => {
                                        eprintln!("DEBUG: Resolvendo '*' para impl_Int_NUM_*, arg_ty={}", effective_type);
                                        resolved_name = "impl_Int_NUM_*".to_string();
                                    }
                                    "/" if effective_type == "Int" => resolved_name = "impl_Int_NUM_/".to_string(),
                                    "str" if effective_type == "Int" => resolved_name = "impl_Int_SHOW_str".to_string(),
                                    "=" => resolved_name = "impl_Int_NUM_=".to_string(),
                                    "mod" => resolved_name = "impl_Int_NUM_mod".to_string(),
                                    "and" => resolved_name = "impl_Bool_AND_and".to_string(),
                                    _ => {}
                                }
                            }

                            if let Some(sig) = self.env.get_signature(&resolved_name) {
                                if let Some(ffi) = &sig.ffi_binding { resolved_name = ffi.clone(); }
                            }

                            let mut ret_ty = Type::Unknown;
                            if let Some(sig) = self.env.get_signature(&resolved_name) {
                                ret_ty = sig.return_type.clone();
                            }

                            let mut final_target_expr = t_expr.clone();
                            final_target_expr.expr = TypedDataExpr::Identifier(Ident::Func(resolved_name.clone()));

                            let call = TypedExpr {
                                ty: ret_ty,
                                expr: TypedDataExpr::Call {
                                    target: Box::new(final_target_expr),
                                    args,
                                }
                            };

                            
                            list.drain(i..=i+arity);
                            list.insert(i, call);
                            changed = true;
                            break; 
                        }
                    }
                }
                i += 1;
            }
        }

        if list.len() == 1 {
            Ok(list.pop().unwrap())
        } else {
            let tys = list.iter().map(|e| e.ty.clone()).collect();
            Ok(TypedExpr { ty: Type::Tuple(tys), expr: TypedDataExpr::Tuple(list) })
        }
    }

    /// Extrai o tipo de um pattern para inferência de tipos em lambdas
    fn extract_pattern_type(&self, pat: &Pattern) -> Type {
        match pat {
            Pattern::Literal(Literal::Int(_)) => Type::Int,
            Pattern::Literal(Literal::Float(_)) => Type::Float,
            Pattern::Literal(Literal::String(_)) => Type::Text,
            Pattern::Identifier(_) => Type::Unknown, // Será inferido do uso
            Pattern::Wildcard => Type::Unknown,
            Pattern::Tuple(pats) => {
                let inner_types: Vec<Type> = pats.iter().map(|p| self.extract_pattern_type(p)).collect();
                Type::Tuple(inner_types)
            }
            Pattern::ListCons { head, tail: _ } => {
                let head_type = self.extract_pattern_type(head);
                Type::List(Box::new(head_type))
            }
        }
    }
}
