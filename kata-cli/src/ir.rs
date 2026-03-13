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
        /// Se true, esta é uma tail call recursiva que pode ser otimizada como jump
        is_tail_call: bool,
    },
    
    /// Construtor bruto de tuplas
    MakeTuple(Vec<ValueId>),

    /// Construtor de Closure (code_ptr, env_ptr)
    /// Usado para criar closures de primeira classe que podem ser passadas como argumentos
    MakeClosure {
        code_ptr: String,           // Nome da função compilada
        env_captures: Vec<ValueId>, // Valores capturados do ambiente
    },

    // ---- Controle de Fluxo ----
    /// Representa um fluxo de desvio (if/else aninhado)
    Guard {
        condition: ValueId,
        true_result: ValueId,
        false_result: ValueId,
    },

    // ---- Tail Call Optimization (TCO) ----
    /// Nó especial para TCO - representa "atualizar parâmetros e repetir"
    /// Usado apenas dentro de funções tail-recursive
    /// Este nó é terminal (como um return) - o bloco termina aqui
    TailRecurse {
        args: Vec<ValueId>,  // Novos valores para os parâmetros
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

    /// Se true, esta função é tail-recursive e usa TailRecurse no corpo
    pub is_tail_recursive: bool,
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

        // ==== PASSAGEM DE TCO (Tail Call Optimization) ====
        // Transforma calls recursivas em TailRecurse
        let (tco_root, is_tail_recursive) = self.mark_tail_calls(name, optimized_root);

        IRFunction {
            name: name.to_string(),
            sig,
            ctx: self.ctx.clone(),
            root: tco_root,
            is_tail_recursive,
        }
    }

    /// Transforma chamadas recursivas em TailRecurse no DAG
    /// Retorna (novo_root, true_se_houver_tail_calls)
    fn mark_tail_calls(&mut self, func_name: &str, root_id: ValueId) -> (ValueId, bool) {
        self.mark_tail_calls_recursive(func_name, root_id, true)
    }

    fn mark_tail_calls_recursive(&mut self,
        func_name: &str,
        id: ValueId,
        in_tail_position: bool
    ) -> (ValueId, bool) {
        let val = self.ctx.arena[id].clone();

        match &val {
            // Guard: processa os branches
            IRValue::Guard { condition, true_result, false_result } => {
                let (new_cond, cond_has_tc) = self.mark_tail_calls_recursive(func_name, *condition, false);
                let (new_true, true_has_tc) = self.mark_tail_calls_recursive(func_name, *true_result, in_tail_position);
                let (new_false, false_has_tc) = self.mark_tail_calls_recursive(func_name, *false_result, in_tail_position);

                let has_tail_call = cond_has_tc || true_has_tc || false_has_tc;

                if new_cond != *condition || new_true != *true_result || new_false != *false_result {
                    let new_id = self.ctx.arena.alloc(IRValue::Guard {
                        condition: new_cond,
                        true_result: new_true,
                        false_result: new_false,
                    });
                    return (new_id, has_tail_call);
                }
                return (id, has_tail_call);
            }

            // Call: verifica se é recursiva e em posição de cauda
            IRValue::Call { target, args, ret_type: _, is_tail_call: _ } => {
                let is_recursive = target == func_name;

                // Processa argumentos (nunca em posição de cauda)
                let mut new_args = Vec::new();
                let mut args_have_tc = false;
                for arg in args {
                    let (new_arg, has_tc) = self.mark_tail_calls_recursive(func_name, *arg, false);
                    new_args.push(new_arg);
                    args_have_tc = args_have_tc || has_tc;
                }

                if is_recursive && in_tail_position {
                    // Converte para TailRecurse - instrução terminal
                    eprintln!("DEBUG IRBuilder: Convertendo call recursiva para TailRecurse em '{}'", func_name);
                    let new_id = self.ctx.arena.alloc(IRValue::TailRecurse {
                        args: new_args,
                    });
                    return (new_id, true);
                } else if new_args != *args || args_have_tc {
                    // Recria com novos argumentos
                    let ret_type = if let IRValue::Call { ret_type, .. } = &self.ctx.arena[id] {
                        ret_type.clone()
                    } else {
                        Type::Unknown
                    };
                    let new_id = self.ctx.arena.alloc(IRValue::Call {
                        target: target.clone(),
                        args: new_args,
                        ret_type,
                        is_tail_call: false,
                    });
                    return (new_id, args_have_tc);
                }
            }

            // Outros casos: não precisam de processamento especial
            _ => {}
        }

        (id, false)
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
                TypedActionStmt::LetBind { pattern, expr, type_annotation: _ } => {
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
                        is_tail_call: false,
                    });
                }
                _ => {}
            }
        }

        let optimized_root = self.optimize_constant_folding(last_id);

        // Actions não são tail-recursive por definição (não se chamam recursivamente)
        IRFunction {
            name: name.to_string(),
            sig,
            ctx: self.ctx.clone(),
            root: optimized_root,
            is_tail_recursive: false,
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
                    crate::ast::Ident::Func(n) | crate::ast::Ident::Symbol(n) | crate::ast::Ident::Action(n) => n.clone(),
                    _ => "unknown_ident".to_string(),
                };

                // O Type Checker já resolveu o monomorfismo - usamos o nome diretamente
                if let Some(&id) = self.env.get(&name) {
                    id
                } else if name == "_" {
                    // Hole não vinculado - em aplicações parciais, isso é um erro
                    // Por enquanto, retornamos um placeholder que será substituído
                    eprintln!("WARNING: Hole '_' não vinculado em aplicação parcial não implementado");
                    self.ctx.arena.alloc(IRValue::IntConst(0)) // Placeholder
                } else {
                    self.ctx.arena.alloc(IRValue::FuncPtr(name.clone()))
                }
            }
            TypedDataExpr::Call { target, args } => {
                let mut target_name = match &target.expr {
                    TypedDataExpr::Identifier(crate::ast::Ident::Func(n) | crate::ast::Ident::Symbol(n) | crate::ast::Ident::Action(n)) => n.clone(),
                    _ => "unknown_call".to_string(),
                };

                // O Type Checker já resolveu o monomorfismo - usamos o nome diretamente
                let mut arg_ids = Vec::new();
                for a in args {
                    arg_ids.push(self.lower_expr(a));
                }

                self.ctx.arena.alloc(IRValue::Call {
                    target: target_name,
                    args: arg_ids,
                    ret_type: expr.ty.clone(),
                    is_tail_call: false, // Será atualizado posteriormente se for TCO
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
            TypedDataExpr::GuardBlock { branches, otherwise, with_clauses } => {
                // Processa cláusulas WITH primeiro para que os ramos as enxerguem
                eprintln!("DEBUG GuardBlock: processing {} with_clauses", with_clauses.len());
                for w in with_clauses {
                    eprintln!("DEBUG GuardBlock: with clause {:?} = {:?}", w.pattern, w.expr);
                    let val_id = self.lower_expr(&w.expr);
                    if let crate::ast::Pattern::Identifier(crate::ast::Ident::Func(n)) = &w.pattern {
                        eprintln!("DEBUG GuardBlock: inserting {} into env", n);
                        self.env.insert(n.clone(), val_id);
                    }
                }
                eprintln!("DEBUG GuardBlock: env keys: {:?}", self.env.keys().collect::<Vec<_>>());

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
                                    target: "impl_Int_NUM_=".to_string(),
                                    args: vec![param_id, lit_id],
                                    ret_type: Type::Bool,
                                    is_tail_call: false,
                                });
                                cond_id = Some(if let Some(c) = cond_id {
                                    self.ctx.arena.alloc(IRValue::Call {
                                        target: "impl_Bool_AND_and".to_string(),
                                        args: vec![c, eq_id],
                                        ret_type: Type::Bool,
                                        is_tail_call: false,
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
            TypedDataExpr::Range { start, end, inclusive } => {
                // Para ranges numéricos, geramos uma chamada à runtime
                // kata_rt_list_from_range(start, end, inclusive, step=1, type_tag)
                let start_id = match start {
                    Some(s) => self.lower_expr(s),
                    None => self.ctx.arena.alloc(IRValue::IntConst(0)), // Range aberto no início
                };
                let end_id = match end {
                    Some(e) => self.lower_expr(e),
                    None => self.ctx.arena.alloc(IRValue::IntConst(i64::MAX)), // Range aberto no fim
                };
                let inclusive_id = self.ctx.arena.alloc(IRValue::IntConst(if *inclusive { 1 } else { 0 }));
                let step_id = self.ctx.arena.alloc(IRValue::IntConst(1));
                let type_tag = self.ctx.arena.alloc(IRValue::IntConst(0)); // 0 = Int

                self.ctx.arena.alloc(IRValue::Call {
                    target: "kata_rt_list_from_range".to_string(),
                    args: vec![start_id, end_id, inclusive_id, step_id, type_tag],
                    ret_type: expr.ty.clone(),
                    is_tail_call: false,
                })
            }
            TypedDataExpr::FieldAccess { target, field } => {
                // Para acesso a módulo (module.func), o type checker já resolveu
                // o nome completo. Se o target for um Ident, combinamos com o campo.
                if let TypedDataExpr::Identifier(crate::ast::Ident::Func(module_name)) = &target.expr {
                    let full_name = format!("{}.{}", module_name, field);
                    self.ctx.arena.alloc(IRValue::FuncPtr(full_name))
                } else {
                    // Para acesso a campo de struct (futuro), precisamos de mais informação
                    eprintln!("WARNING: FieldAccess em runtime ainda não implementado para {:?}", target);
                    self.ctx.arena.alloc(IRValue::FuncPtr(field.clone()))
                }
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
                match target.as_str() {
                    "+" => {
                        let mut sum = 0;
                        for a in &opt_args {
                            if let IRValue::IntConst(n) = self.ctx.arena[*a] { sum += n; }
                        }
                        return self.ctx.arena.alloc(IRValue::IntConst(sum));
                    }
                    "-" => {
                        if opt_args.len() >= 2 {
                            if let (IRValue::IntConst(a), IRValue::IntConst(b)) = (&self.ctx.arena[opt_args[0]], &self.ctx.arena[opt_args[1]]) {
                                return self.ctx.arena.alloc(IRValue::IntConst(a - b));
                            }
                        }
                    }
                    "*" => {
                        let mut prod = 1;
                        for a in &opt_args {
                            if let IRValue::IntConst(n) = self.ctx.arena[*a] { prod *= n; }
                        }
                        return self.ctx.arena.alloc(IRValue::IntConst(prod));
                    }
                    "/" => {
                        if opt_args.len() >= 2 {
                            if let (IRValue::IntConst(a), IRValue::IntConst(b)) = (&self.ctx.arena[opt_args[0]], &self.ctx.arena[opt_args[1]]) {
                                if *b != 0 {
                                    return self.ctx.arena.alloc(IRValue::IntConst(a / b));
                                }
                            }
                        }
                    }
                    _ => {}
                }
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