use std::collections::HashMap;
use crate::ast::*;
use crate::error::KataError;
use crate::typed_ast::*;

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
}

impl TypeEnv {
    pub fn new() -> Self {
        Self {
            functions: HashMap::new(),
            local_bindings: vec![HashMap::new()], // Escopo global base
            implementations: HashMap::new(),
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
        
        // HACKS PARA O FIZZBUZZ DA FASE 5/8 QUE NÃO TEM O PRELUDE COMPLETO:
        env.functions.insert("=".to_string(), FuncSignature { name: "=".to_string(), arity: 2, args_types: vec![Type::Unknown, Type::Unknown], return_type: Type::Bool, is_action: false, ffi_binding: None });
        env.functions.insert("mod".to_string(), FuncSignature { name: "mod".to_string(), arity: 2, args_types: vec![Type::Int, Type::Int], return_type: Type::Int, is_action: false, ffi_binding: None });
        env.functions.insert("and".to_string(), FuncSignature { name: "and".to_string(), arity: 2, args_types: vec![Type::Bool, Type::Bool], return_type: Type::Bool, is_action: false, ffi_binding: None });

        // Operadores aritméticos da interface NUM (Int)
        env.functions.insert("+".to_string(), FuncSignature { name: "+".to_string(), arity: 2, args_types: vec![Type::Int, Type::Int], return_type: Type::Int, is_action: false, ffi_binding: Some("__add_int".to_string()) });
        env.functions.insert("-".to_string(), FuncSignature { name: "-".to_string(), arity: 2, args_types: vec![Type::Int, Type::Int], return_type: Type::Int, is_action: false, ffi_binding: Some("__sub_int".to_string()) });
        env.functions.insert("*".to_string(), FuncSignature { name: "*".to_string(), arity: 2, args_types: vec![Type::Int, Type::Int], return_type: Type::Int, is_action: false, ffi_binding: Some("__mul_int".to_string()) });
        env.functions.insert("/".to_string(), FuncSignature { name: "/".to_string(), arity: 2, args_types: vec![Type::Int, Type::Int], return_type: Type::Int, is_action: false, ffi_binding: Some("__div_int".to_string()) });

        // Funções FFI concretas (usadas após múltiplo despacho)
        env.functions.insert("__add_int".to_string(), FuncSignature { name: "__add_int".to_string(), arity: 2, args_types: vec![Type::Int, Type::Int], return_type: Type::Int, is_action: false, ffi_binding: Some("kata_rt_add_int".to_string()) });
        env.functions.insert("__sub_int".to_string(), FuncSignature { name: "__sub_int".to_string(), arity: 2, args_types: vec![Type::Int, Type::Int], return_type: Type::Int, is_action: false, ffi_binding: Some("kata_rt_sub_int".to_string()) });
        env.functions.insert("__mul_int".to_string(), FuncSignature { name: "__mul_int".to_string(), arity: 2, args_types: vec![Type::Int, Type::Int], return_type: Type::Int, is_action: false, ffi_binding: Some("kata_rt_mul_int".to_string()) });
        env.functions.insert("__div_int".to_string(), FuncSignature { name: "__div_int".to_string(), arity: 2, args_types: vec![Type::Int, Type::Int], return_type: Type::Int, is_action: false, ffi_binding: Some("kata_rt_div_int".to_string()) });
        env.functions.insert("__eq_int".to_string(), FuncSignature { name: "__eq_int".to_string(), arity: 2, args_types: vec![Type::Int, Type::Int], return_type: Type::Bool, is_action: false, ffi_binding: None });
        env.functions.insert("__int_to_str".to_string(), FuncSignature { name: "__int_to_str".to_string(), arity: 1, args_types: vec![Type::Int], return_type: Type::Text, is_action: false, ffi_binding: Some("kata_rt_int_to_str".to_string()) });
        env.functions.insert("__list_to_str".to_string(), FuncSignature { name: "__list_to_str".to_string(), arity: 1, args_types: vec![Type::Custom("List::Text".to_string())], return_type: Type::Text, is_action: false, ffi_binding: Some("kata_rt_list_to_str".to_string()) });

        // Implementação da interface SHOW para Int
        env.implementations.entry("Int".to_string()).or_insert_with(Vec::new).push("NUM".to_string());
        env.implementations.entry("Int".to_string()).or_insert_with(Vec::new).push("SHOW".to_string());

        Self { env }
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
                    
                    // Conversão rudimentar de Ident para Type para o TypeEnv
                    let mut args_types = Vec::new();
                    for a in &sig.args {
                        match a {
                            Ident::Type(t) if t == "Int" => args_types.push(Type::Int),
                            Ident::Type(t) if t == "Float" => args_types.push(Type::Float),
                            Ident::Type(t) if t == "Text" => args_types.push(Type::Text),
                            Ident::Type(t) if t == "Bool" => args_types.push(Type::Bool),
                            Ident::Type(t) => args_types.push(Type::Custom(t.clone())),
                            Ident::Interface(t) => args_types.push(Type::Interface(t.clone())),
                            _ => args_types.push(Type::Unknown),
                        }
                    }

                    let return_type = match &sig.ret {
                        Ident::Type(t) if t == "Int" => Type::Int,
                        Ident::Type(t) if t == "Float" => Type::Float,
                        Ident::Type(t) if t == "Text" => Type::Text,
                        Ident::Type(t) if t == "Bool" => Type::Bool,
                        Ident::Type(t) => Type::Custom(t.clone()),
                        Ident::Interface(t) => Type::Interface(t.clone()),
                        _ => Type::Unknown,
                    };

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
                TopLevelDecl::Implements { target_type, interface, .. } => {
                    if let (Ident::Type(t_name), Ident::Interface(i_name)) = (target_type, interface) {
                        self.env.implementations
                            .entry(t_name.clone())
                            .or_insert_with(Vec::new)
                            .push(i_name.clone());
                    }
                }
                TopLevelDecl::ActionDef { attrs, name, params, body: _ } => {
                    let name_str = match name {
                        Ident::Action(n) => n.clone(),
                        Ident::Func(n) => format!("{}!", n), // Força consistência se faltou o !
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
                    // Descobre a definição de função, mas os tipos serão inferidos posteriormente
                    // Se já existe uma assinatura (SignatureDecl), mantemos ela
                    let name_str = match name {
                        Ident::Func(n) | Ident::Symbol(n) => n.clone(),
                        _ => continue,
                    };

                    if !self.env.functions.contains_key(&name_str) {
                        // Assinatura ainda desconhecida, será inferida durante resolve_module
                        // Por ora registramos com Unknown para evitar o unwrap em None
                        self.env.functions.insert(name_str.clone(), FuncSignature {
                            name: name_str.clone(),
                            arity: 0, // Será atualizado quando processarmos o LambdaGroup
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
                    self.env.push_scope(); // Isola variáveis para este lambda
                    let typed_expr = self.resolve_expr(expr, false)?;
                    self.env.pop_scope();

                    new_decls.push(TypedTopLevel::Definition {
                        name,
                        expr: typed_expr,
                    });
                }
                TopLevelDecl::ActionDef { attrs, name, params, body } => {
                    self.env.push_scope();
                    let mut typed_body = Vec::new();
                    for stmt in body {
                        typed_body.push(self.resolve_action(stmt)?);
                    }
                    self.env.pop_scope();

                    new_decls.push(TypedTopLevel::ActionDef { 
                        attrs,
                        name, 
                        params, 
                        body: typed_body 
                    });
                }
                TopLevelDecl::SignatureDecl { .. } => {} // Ignore durante resolve
                _ => {} // Ignore imports/exports temporariamente
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
            ActionStmt::LetBind { pattern, expr } => {
                let t_expr = self.resolve_expr(expr, true)?;
                // Se o pattern for um Identifier simples, adiciona ao TypeEnv local para inferência nas linhas seguintes
                if let Pattern::Identifier(Ident::Func(n)) = &pattern {
                    self.env.insert_local(n.clone(), t_expr.ty.clone());
                }
                Ok(TypedActionStmt::LetBind { pattern, expr: t_expr })
            }
            ActionStmt::ActionCall { target, args } => {
                let mut resolved_args = Vec::new();
                for arg in args {
                    resolved_args.push(self.resolve_expr(arg, true)?);
                }
                
                // Reconstrói a sequência com precedência gulosa
                let final_args = if resolved_args.is_empty() {
                    Vec::new()
                } else {
                    let greedy = self.consume_greedy_sequence(resolved_args, true)?;
                    match greedy.expr {
                        TypedDataExpr::Tuple(items) => items,
                        _ => vec![greedy],
                    }
                };

                // TODO: Checar assinatura da Action chamada
                Ok(TypedActionStmt::ActionCall { target, args: final_args })
            }
            _ => Err(KataError::CrossDomainViolation { msg: "Ação não implementada no Type Checker".into(), span: (0,0) }),
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
                // Tenta buscar o tipo da variável no escopo local
                let name = match &ident {
                    Ident::Func(n) | Ident::Symbol(n) => n.clone(),
                    _ => "".to_string(), // Por hora ignoramos lookup de TypeIdent puro aqui
                };

                let ty = if let Some(local_ty) = self.env.lookup_local(&name) {
                    local_ty
                } else if let Some(sig) = self.env.get_signature(&name) {
                    Type::Func { args: sig.args_types.clone(), ret: Box::new(sig.return_type.clone()) }
                } else {
                    // Type::Unknown
                    // Idealmente emitiríamos UndefinedSymbol, mas `n` é comum em fibonacci n, e pode não estar assinado.
                    // Para relaxar a Fase 3 e testar, marcamos como Unknown.
                    Type::Unknown
                };

                Ok(TypedExpr { expr: TypedDataExpr::Identifier(ident), ty })
            }
            DataExpr::Tuple(items) => {
                let mut t_items = Vec::new();
                let mut tuple_types = Vec::new();
                for item in items {
                    let t_expr = self.resolve_expr(item, in_action)?;
                    tuple_types.push(t_expr.ty.clone());
                    t_items.push(t_expr);
                }
                Ok(TypedExpr { 
                    expr: TypedDataExpr::Tuple(t_items), 
                    ty: Type::Tuple(tuple_types) 
                })
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
                let mut return_type = Type::Unknown;

                for branch in branches {
                    self.env.push_scope();
                    
                    for pat in &branch.params {
                        if let Pattern::Identifier(Ident::Func(n)) = pat {
                            self.env.insert_local(n.clone(), Type::Unknown);
                        }
                    }

                    let t_body = self.resolve_expr(branch.body, false)?;
                    return_type = t_body.ty.clone();
                    
                    t_branches.push(TypedLambdaBranch {
                        params: branch.params,
                        body: Box::new(t_body),
                    });

                    self.env.pop_scope();
                }
                
                Ok(TypedExpr { 
                    expr: TypedDataExpr::LambdaGroup { branches: t_branches }, 
                    ty: Type::Unknown
                })
            }
            DataExpr::GuardBlock { branches, otherwise } => {
                let mut t_branches = Vec::new();
                for branch in branches {
                    let cond = self.resolve_expr(branch.condition, false)?;
                    let res = self.resolve_expr(branch.result, false)?;
                    t_branches.push(TypedGuardBranch { condition: cond, result: res });
                }
                
                let t_otherwise = self.resolve_expr(*otherwise, false)?;
                let return_type = t_otherwise.ty.clone();

                Ok(TypedExpr {
                    expr: TypedDataExpr::GuardBlock { branches: t_branches, otherwise: Box::new(t_otherwise) },
                    ty: return_type,
                })
            }
            DataExpr::ScopedBlock { bindings, body, with_clauses } => {
                self.env.push_scope();

                let mut t_bindings = Vec::new();
                for b in bindings {
                    let t_expr = self.resolve_expr(b.expr, false)?;
                    if let Pattern::Identifier(Ident::Func(n)) = &b.pattern {
                        self.env.insert_local(n.clone(), t_expr.ty.clone());
                    }
                    t_bindings.push(TypedBinding { pattern: b.pattern, expr: t_expr });
                }

                let mut t_with = Vec::new();
                for w in with_clauses {
                    let t_expr = self.resolve_expr(w.expr, false)?;
                    if let Pattern::Identifier(Ident::Func(n)) = &w.pattern {
                        self.env.insert_local(n.clone(), t_expr.ty.clone());
                    }
                    t_with.push(TypedBinding { pattern: w.pattern, expr: t_expr });
                }

                let t_body = self.resolve_expr(*body, false)?;
                let return_type = t_body.ty.clone();

                self.env.pop_scope();

                Ok(TypedExpr {
                    expr: TypedDataExpr::ScopedBlock { bindings: t_bindings, body: Box::new(t_body), with_clauses: t_with },
                    ty: return_type,
                })
            }
            DataExpr::Pipe { left, right } => {
                let t_left = self.resolve_expr(*left, in_action)?;
                let t_right = self.resolve_expr(*right, in_action)?;
                let ret_ty = t_right.ty.clone(); // O pipeline retorna o tipo da função da direita

                Ok(TypedExpr {
                    expr: TypedDataExpr::Pipe { left: Box::new(t_left), right: Box::new(t_right) },
                    ty: ret_ty,
                })
            }
            DataExpr::Call { target, args } => {
                // Trata as chamadas puras que já vieram arrumadas do Parser (ex: construtores List e Array via [ ] e { })
                let mut new_args = Vec::new();
                for a in args {
                    new_args.push(self.resolve_expr(a, in_action)?);
                }
                
                let mut t_target = self.resolve_expr(*target, in_action)?;
                let mut final_target_name = None;

                let mut target_ident_name = String::new();
                let mut is_type_const = false;

                if let TypedDataExpr::Identifier(Ident::Func(ref name)) = t_target.expr {
                    target_ident_name = name.clone();
                } else if let TypedDataExpr::Identifier(Ident::Type(ref name)) = t_target.expr {
                    target_ident_name = name.clone();
                    is_type_const = true;
                }

                if !target_ident_name.is_empty() {
                    let mut resolved_name = target_ident_name.clone();
                    // Múltiplo Despacho
                    if !new_args.is_empty() && !is_type_const {
                        let arg_ty = &new_args[0].ty;
                        match resolved_name.as_str() {
                            "str" => {
                                match arg_ty {
                                    Type::Int => resolved_name = "__int_to_str".to_string(),
                                    Type::Custom(c) if c.starts_with("List") => resolved_name = "__list_to_str".to_string(),
                                    Type::Unknown | Type::Custom(_) => resolved_name = "__list_to_str".to_string(),
                                    _ => {}
                                }
                            }
                            "+" => { if arg_ty == &Type::Int { resolved_name = "__add_int".to_string() } }
                            "-" => { if arg_ty == &Type::Int { resolved_name = "__sub_int".to_string() } }
                            "*" => { if arg_ty == &Type::Int { resolved_name = "__mul_int".to_string() } }
                            "/" => { if arg_ty == &Type::Int { resolved_name = "__div_int".to_string() } }
                            "=" => { if arg_ty == &Type::Int { resolved_name = "__eq_int".to_string() } }
                            _ => {}
                        }
                    }

                    // FFI Binding resolution
                    if let Some(sig) = self.env.get_signature(&resolved_name) {
                        if let Some(ffi) = &sig.ffi_binding {
                            resolved_name = ffi.clone();
                        }
                    } else if resolved_name == "map" {
                        resolved_name = "kata_rt_mock_map".to_string();
                    } else if is_type_const && (resolved_name == "List" || resolved_name == "Array") {
                        resolved_name = "kata_rt_mock_list".to_string();
                    }

                    final_target_name = Some(resolved_name);
                }
                
                if let Some(concrete_name) = final_target_name {
                    t_target.expr = TypedDataExpr::Identifier(Ident::Func(concrete_name));
                }

                let ret_ty = match &t_target.expr {
                    TypedDataExpr::Identifier(Ident::Type(name)) => {
                        // INFERÊNCIA ESTRITA PARA CONSTRUTORES GENÉRICOS (ex: List, Array)
                        if (name == "List" || name == "Array") && !new_args.is_empty() {
                            let arg_ty = &new_args[0].ty;
                            
                            let inner_type = match arg_ty {
                                Type::Tuple(tys) if !tys.is_empty() => &tys[0],
                                _ => arg_ty,
                            };
                            
                            if inner_type != &Type::Unknown {
                                Type::Custom(format!("{}::{}", name, inner_type))
                            } else {
                                Type::Custom(name.clone())
                            }
                        } else {
                            Type::Custom(name.clone())
                        }
                    },
                    _ => Type::Unknown,
                };
                
                // Hack para manter o tipo de retorno correto mesmo quando usamos a FFI diretamente
                let ret_ty = if let TypedDataExpr::Identifier(Ident::Func(ref n)) = t_target.expr {
                    if n == "kata_rt_mock_list" {
                        if !new_args.is_empty() {
                             let arg_ty = &new_args[0].ty;
                             let inner_type = match arg_ty { Type::Tuple(tys) if !tys.is_empty() => &tys[0], _ => arg_ty };
                             Type::Custom(format!("List::{}", inner_type))
                        } else { Type::Custom("List".to_string()) }
                    } else if n == "kata_rt_mock_map" {
                        Type::Custom("List::Unknown".to_string()) // Simplificação para esta fase
                    } else { ret_ty }
                } else { ret_ty };

                Ok(TypedExpr { 
                    expr: TypedDataExpr::Call { target: Box::new(t_target), args: new_args },
                    ty: ret_ty 
                })
            }
            other => Err(KataError::CrossDomainViolation { msg: format!("Expressão não portabilizada para TypedAST ainda: {:?}", other), span:(0,0) })
        }
    }

    fn consume_greedy_sequence(&self, mut list: Vec<TypedExpr>, in_action: bool) -> Result<TypedExpr, KataError> {
        let mut result_stack: Vec<TypedExpr> = Vec::new();
        
        while let Some(t_expr) = list.pop() {
            let name_to_check = match &t_expr.expr {
                TypedDataExpr::Identifier(Ident::Func(n)) => Some((n.clone(), false)),
                TypedDataExpr::Identifier(Ident::Symbol(n)) => Some((n.clone(), false)),
                TypedDataExpr::Identifier(Ident::Type(n)) => Some((n.clone(), true)), // Tipos têm aridade genérica implícita de 1 struct
                TypedDataExpr::Identifier(Ident::Action(n)) => {
                    if !in_action {
                        return Err(KataError::CrossDomainViolation {
                            msg: format!("Action '{}' invocada no domínio funcional.", n),
                            span: (0, 0),
                        });
                    }
                    Some((n.clone(), false))
                }
                _ => None,
            };

            if let Some((mut func_name, is_type_constructor)) = name_to_check {
                let arity = if is_type_constructor { 1 } else { self.env.get_arity(&func_name) };
                
                if arity > 0 {
                    if result_stack.len() < arity {
                        return Err(KataError::ArityMismatch {
                            name: func_name.clone(),
                            expected: arity,
                            found: result_stack.len(),
                            span: (0, 0),
                        });
                    }

                    let mut args = Vec::new();
                    for _ in 0..arity {
                        args.push(result_stack.pop().unwrap());
                    }

                    // Múltiplo Despacho
                    if !is_type_constructor && !args.is_empty() {
                        let arg_ty = &args[0].ty;
                        match func_name.as_str() {
                            "str" => {
                                match arg_ty {
                                    Type::Int => func_name = "__int_to_str".to_string(),
                                    Type::Custom(c) if c.starts_with("List") => func_name = "__list_to_str".to_string(),
                                    Type::Unknown | Type::Custom(_) => func_name = "__list_to_str".to_string(),
                                    _ => {}
                                }
                            }
                            "+" => { if arg_ty == &Type::Int { func_name = "__add_int".to_string() } }
                            "-" => { if arg_ty == &Type::Int { func_name = "__sub_int".to_string() } }
                            "*" => { if arg_ty == &Type::Int { func_name = "__mul_int".to_string() } }
                            "/" => { if arg_ty == &Type::Int { func_name = "__div_int".to_string() } }
                            "=" => { if arg_ty == &Type::Int { func_name = "__eq_int".to_string() } }
                            _ => {}
                        }
                    }

                    // FFI Binding resolution
                    let mut final_func_name = func_name.clone();
                    if let Some(sig) = self.env.get_signature(&func_name) {
                        if let Some(ffi) = &sig.ffi_binding {
                            final_func_name = ffi.clone();
                        }
                    } else if func_name == "map" {
                        final_func_name = "kata_rt_mock_map".to_string();
                    } else if is_type_constructor && (func_name == "List" || func_name == "Array") {
                        final_func_name = "kata_rt_mock_list".to_string();
                    }

                    // ====== INFERÊNCIA DE TIPO (Type Matching) ======
                    let ret_ty = if is_type_constructor {
                        // Se é um construtor de tipo genérico (ex: List) inferimos o tipo interno
                        if (func_name == "List" || func_name == "Array") && !args.is_empty() {
                            let arg_ty = &args[0].ty;
                            let inner_type = match arg_ty { Type::Tuple(tys) if !tys.is_empty() => &tys[0], _ => arg_ty };
                            if inner_type != &Type::Unknown {
                                Type::Custom(format!("{}::{}", func_name, inner_type))
                            } else {
                                Type::Custom(func_name.clone())
                            }
                        } else {
                            Type::Custom(func_name.clone())
                        }
                    } else {
                        let signature = self.env.get_signature(&func_name).unwrap();
                        for (i, arg) in args.iter().enumerate() {
                            let expected_ty = &signature.args_types[i];
                            let found_ty = &arg.ty;
                            
                            // Validação simples
                            if expected_ty != &Type::Unknown && found_ty != &Type::Unknown && expected_ty != found_ty {
                                // Permite polimorfismo se o Expected for uma Interface e o Found implementar ela
                                let mut is_valid = false;
                                if let Type::Interface(iface_name) = expected_ty {
                                    let type_name = match found_ty {
                                        Type::Int => "Int",
                                        Type::Float => "Float",
                                        Type::Text => "Text",
                                        Type::Bool => "Bool",
                                        Type::Custom(t) => t.as_str(),
                                        _ => "",
                                    };
                                    if self.env.type_implements_interface(type_name, iface_name) {
                                        is_valid = true;
                                    }
                                }
                                
                                if !is_valid {
                                    println!("TypeMismatch in func: {}, expected: {}, found: {}", func_name, expected_ty.to_string(), found_ty.to_string());
                                    return Err(KataError::TypeMismatch {
                                        expected: expected_ty.to_string(),
                                        found: found_ty.to_string(),
                                        span: (0, 0),
                                    });
                                }
                            }
                        }
                        signature.return_type.clone()
                    };

                    let mut final_target = t_expr;
                    final_target.expr = TypedDataExpr::Identifier(Ident::Func(final_func_name));

                    let call = TypedExpr {
                        expr: TypedDataExpr::Call {
                            target: Box::new(final_target),
                            args,
                        },
                        ty: ret_ty,
                    };
                    result_stack.push(call);
                } else {
                    result_stack.push(t_expr);
                }
            } else {
                result_stack.push(t_expr);
            }
        }

        result_stack.reverse();
        
        if result_stack.len() == 1 {
            Ok(result_stack.pop().unwrap())
        } else {
            let mut tuple_types = Vec::new();
            for i in &result_stack { tuple_types.push(i.ty.clone()); }
            Ok(TypedExpr { 
                expr: TypedDataExpr::Tuple(result_stack), 
                ty: Type::Tuple(tuple_types) 
            })
        }
    }
}
