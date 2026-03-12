use crate::typed_ast::*;
use crate::type_checker::Type;
use id_arena::{Arena, Id};
use std::collections::HashMap;

/// Um "Ponteiro" leve e seguro para um valor computado na nossa IR.
/// Usando `id_arena`, garantimos alta performance na alocação da IR.
pub type ValueId = Id<IRValue>;

/// O Grafo Acíclico Dirigido (DAG) da nossa Representação Intermediária.
/// Todos os valores da função vivem aqui de forma plana.
#[derive(Debug, Clone)]
pub struct IRContext {
    pub arena: Arena<IRValue>,
}

impl IRContext {
    pub fn new() -> Self {
        Self {
            arena: Arena::new(),
        }
    }
}

/// Um valor dentro da Representação Intermediária.
/// O formato SSA dita que cada variável/instrução é escrita apenas uma vez.
#[derive(Debug, Clone, PartialEq)]
pub enum IRValue {
    // ---- Constantes e Literais ----
    IntConst(i64),
    FloatConst(f64),
    StringConst(String),
    FuncPtr(String),
    
    // ---- Argumentos de Entrada da Função ----
    Param(usize, Type),
    
    // ---- Operações Funcionais ----
    /// Invocação genérica de função nativa ou do próprio módulo
    Call {
        target: String,
        args: Vec<ValueId>,
        ret_type: Type,
    },
    
    /// Construtor bruto de tuplas
    MakeTuple(Vec<ValueId>),
    
    // ---- Controle de Fluxo ----
    /// Representa um fluxo de desvio (if/else aninhado)
    Guard {
        condition: ValueId,
        true_result: ValueId,
        false_result: ValueId,
    },
}

/// Uma Função compilada em formato de Representação Intermediária.
/// Ela abriga o DAG completo (o Contexto) e a instrução que gera o retorno (Root).
#[derive(Debug)]
pub struct IRFunction {
    pub name: String,
    pub sig: crate::type_checker::FuncSignature,
    pub ctx: IRContext,
    
    /// O nó final do DAG que representa a resposta do fluxo de controle principal da função.
    pub root: ValueId, 
}

/// O construtor que converte TypedAST -> IR.
pub struct IRBuilder {
    ctx: IRContext,
    // (Opcional) Ambiente local temporário mapeando variáveis a ValueIds
    env: HashMap<String, ValueId>,
}

impl IRBuilder {
    pub fn new() -> Self {
        Self {
            ctx: IRContext::new(),
            env: HashMap::new(),
        }
    }

    /// Método de entrada principal. Recebe uma declaração de Lambda e extrai a IR.
    pub fn build_function(&mut self, name: &str, sig: crate::type_checker::FuncSignature, expr: &TypedExpr) -> IRFunction {
        // Zera o contexto para esta função
        self.ctx = IRContext::new();
        self.env.clear();

        // Cadastra os parâmetros baseados na assinatura como nós folha na Arena
        for (i, t) in sig.args_types.iter().enumerate() {
            let param_id = self.ctx.arena.alloc(IRValue::Param(i, t.clone()));
            // Simulação de nomes para parâmetros
            // Na Kata-Lang os nomes vem do pattern matching `lambda (x)`.
            // Para a fase 4 minimalista de Constant Folding, não estamos bindando 
            // os nomes do pattern matching ainda, apenas transcrevendo Call e Literal.
        }

        let root_id = self.lower_expr(expr);

        // ==== PASSAGEM DE OTIMIZAÇÃO (Middle-End) ====
        let optimized_root = self.optimize_constant_folding(root_id);

        IRFunction {
            name: name.to_string(),
            sig,
            ctx: self.ctx.clone(),
            root: optimized_root,
        }
    }


    pub fn build_action(&mut self, name: &str, sig: crate::type_checker::FuncSignature, body: &[TypedActionStmt]) -> IRFunction {
        self.ctx = IRContext::new();
        self.env.clear();

        for (i, t) in sig.args_types.iter().enumerate() {
            let param_id = self.ctx.arena.alloc(IRValue::Param(i, t.clone()));
        }

        let mut last_id = self.ctx.arena.alloc(IRValue::IntConst(0));

        for stmt in body {
            match stmt {
                TypedActionStmt::Expr(expr) => {
                    last_id = self.lower_expr(expr);
                }
                TypedActionStmt::LetBind { pattern, expr } => {
                    let val_id = self.lower_expr(expr);
                    if let crate::ast::Pattern::Identifier(crate::ast::Ident::Func(n)) = pattern {
                        self.env.insert(n.clone(), val_id);
                    }
                    last_id = val_id;
                }
                TypedActionStmt::ActionCall { target, args } => {
                    let mut arg_ids = Vec::new();
                    for a in args {
                        arg_ids.push(self.lower_expr(a));
                    }
                    
                    let target_name = match target {
                        crate::ast::Ident::Action(n) | crate::ast::Ident::Func(n) | crate::ast::Ident::Symbol(n) => n.clone(),
                        _ => "unknown".to_string(),
                    };

                    last_id = self.ctx.arena.alloc(IRValue::Call {
                        target: target_name,
                        args: arg_ids,
                        ret_type: crate::type_checker::Type::Unknown, // Actions costumam não retornar no mock atual
                    });
                }
                _ => {}
            }
        }

        let optimized_root = self.optimize_constant_folding(last_id);

        IRFunction {
            name: name.to_string(),
            sig,
            ctx: self.ctx.clone(),
            root: optimized_root,
        }
    }


    /// Caminha a AST gerando o DAG
    fn lower_expr(&mut self, expr: &TypedExpr) -> ValueId {
        match &expr.expr {
            TypedDataExpr::Literal(crate::ast::Literal::Int(n)) => {
                self.ctx.arena.alloc(IRValue::IntConst(*n))
            }
            TypedDataExpr::Literal(crate::ast::Literal::Float(n)) => {
                self.ctx.arena.alloc(IRValue::FloatConst(*n))
            }
            TypedDataExpr::Literal(crate::ast::Literal::String(s)) => {
                self.ctx.arena.alloc(IRValue::StringConst(s.clone()))
            }
            TypedDataExpr::Identifier(ident) => {
                let name = match ident {
                    crate::ast::Ident::Func(n) | crate::ast::Ident::Symbol(n) => n.clone(),
                    _ => return self.ctx.arena.alloc(IRValue::IntConst(0)), // Avoid breaking Linker with empty FuncPtr
                };
                if let Some(&id) = self.env.get(&name) {
                    id
                } else {
                    self.ctx.arena.alloc(IRValue::FuncPtr(name.clone()))
                }
            }
            TypedDataExpr::Call { target, args } => {
                let target_name = if let TypedDataExpr::Identifier(crate::ast::Ident::Symbol(n) | crate::ast::Ident::Func(n) | crate::ast::Ident::Type(n)) = &target.expr {
                    n.clone()
                } else {
                    "unknown_call".to_string()
                };

                let mut arg_ids = Vec::new();
                for a in args {
                    arg_ids.push(self.lower_expr(a));
                }

                self.ctx.arena.alloc(IRValue::Call {
                    target: target_name,
                    args: arg_ids,
                    ret_type: expr.ty.clone(),
                })
            }
            TypedDataExpr::Pipe { left, right } => {
                let left_id = self.lower_expr(left);
                let old_hole = self.env.get("_").copied();
                self.env.insert("_".to_string(), left_id);
                let result_id = self.lower_expr(right);
                if let Some(old) = old_hole {
                    self.env.insert("_".to_string(), old);
                } else {
                    self.env.remove("_");
                }
                result_id
            }
            TypedDataExpr::Tuple(items) => {
                let mut item_ids = Vec::new();
                for i in items {
                    item_ids.push(self.lower_expr(i));
                }
                self.ctx.arena.alloc(IRValue::MakeTuple(item_ids))
            }
            TypedDataExpr::ScopedBlock { bindings, body, with_clauses } => {
                for b in bindings {
                    let val_id = self.lower_expr(&b.expr);
                    if let crate::ast::Pattern::Identifier(crate::ast::Ident::Func(n)) = &b.pattern {
                        self.env.insert(n.clone(), val_id);
                    }
                }
                for w in with_clauses {
                    let val_id = self.lower_expr(&w.expr);
                    if let crate::ast::Pattern::Identifier(crate::ast::Ident::Func(n)) = &w.pattern {
                        self.env.insert(n.clone(), val_id);
                    }
                }
                self.lower_expr(body)
            }
            TypedDataExpr::GuardBlock { branches, otherwise } => {
                let mut current_otherwise = self.lower_expr(otherwise);
                
                for branch in branches.iter().rev() {
                    let cond_id = self.lower_expr(&branch.condition);
                    let res_id = self.lower_expr(&branch.result);
                    
                    current_otherwise = self.ctx.arena.alloc(IRValue::Guard {
                        condition: cond_id,
                        true_result: res_id,
                        false_result: current_otherwise,
                    });
                }
                current_otherwise
            }
            TypedDataExpr::LambdaGroup { branches } => {
                let mut current_otherwise = self.ctx.arena.alloc(IRValue::IntConst(0));
                
                for branch in branches.iter().rev() {
                    let mut cond_id = None;
                    let mut has_unconditional_match = true;

                    for (i, pat) in branch.params.iter().enumerate() {
                        let param_id = self.ctx.arena.alloc(IRValue::Param(i, Type::Unknown));
                        
                        match pat {
                            crate::ast::Pattern::Literal(crate::ast::Literal::Int(n)) => {
                                has_unconditional_match = false;
                                let lit_id = self.ctx.arena.alloc(IRValue::IntConst(*n));
                                let eq_id = self.ctx.arena.alloc(IRValue::Call {
                                    target: "=".to_string(),
                                    args: vec![param_id, lit_id],
                                    ret_type: Type::Bool,
                                });
                                cond_id = Some(if let Some(c) = cond_id {
                                    self.ctx.arena.alloc(IRValue::Call {
                                        target: "and".to_string(),
                                        args: vec![c, eq_id],
                                        ret_type: Type::Bool,
                                    })
                                } else {
                                    eq_id
                                });
                            }
                            crate::ast::Pattern::Identifier(crate::ast::Ident::Func(n)) => {
                                self.env.insert(n.clone(), param_id);
                            }
                            _ => {}
                        }
                    }

                    let res_id = self.lower_expr(&branch.body);

                    if has_unconditional_match {
                        current_otherwise = res_id;
                    } else if let Some(c) = cond_id {
                        current_otherwise = self.ctx.arena.alloc(IRValue::Guard {
                            condition: c,
                            true_result: res_id,
                            false_result: current_otherwise,
                        });
                    }
                }
                current_otherwise
            }
            _ => {
                self.ctx.arena.alloc(IRValue::IntConst(0))
            }
        }
    }

    // ==========================================
    // OTIMIZADOR (Constant Folding)
    // ==========================================

    /// Percorre o DAG. Se encontrar uma instrução `Call` matemática
    /// cujos argumentos já sejam constantes estritas, realiza a matemática
    /// no compilador e substitui o nó por uma `IRValue::IntConst`.
    fn optimize_constant_folding(&mut self, id: ValueId) -> ValueId {
        // Passo 1: Avalia as ramificações de baixo para cima
        let val = self.ctx.arena[id].clone();
        
        if let IRValue::Call { target, args, .. } = &val {
            // Otimiza recursivamente os argumentos primeiro
            let mut opt_args = Vec::new();
            let mut all_constants = true;
            
            for &a in args {
                let o_a = self.optimize_constant_folding(a);
                opt_args.push(o_a);
                
                // Checa se o argumento resultante é uma constante matemática
                match &self.ctx.arena[o_a] {
                    IRValue::IntConst(_) | IRValue::FloatConst(_) => {} // Passou
                    _ => all_constants = false, // Impede o dobramento
                }
            }

            // Se os operandos são puramente constantes e a função é segura
            if all_constants {
                // Lógica Mágica do Compilador: Executa a matemática em Rust!
                if target == "+" {
                    let mut sum = 0;
                    for a in &opt_args {
                        if let IRValue::IntConst(n) = self.ctx.arena[*a] {
                            sum += n;
                        }
                    }
                    // Retorna um nó NOVO otimizado, abandonando a Call velha no DAG
                    return self.ctx.arena.alloc(IRValue::IntConst(sum));
                }
                if target == "*" {
                    let mut prod = 1;
                    for a in &opt_args {
                        if let IRValue::IntConst(n) = self.ctx.arena[*a] {
                            prod *= n;
                        }
                    }
                    return self.ctx.arena.alloc(IRValue::IntConst(prod));
                }
                // TODO: -, / e afins
            }
            
            // Se não deu pra dobrar constantes, mas atualizamos as folhas, 
            // atualiza o nó original (já que ele é mutável).
            if let IRValue::Call { args: old_args, .. } = &mut self.ctx.arena[id] {
                *old_args = opt_args;
            }
        }

        // Se for um nó opaco ou que já falhou na otimização, retorna ele mesmo
        id
    }
}