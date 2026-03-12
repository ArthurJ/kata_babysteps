use crate::ir::{IRFunction, IRValue};
use std::collections::{HashMap, HashSet};

/// Responsável por percorrer o Grafo de Chamadas (Call Graph) a partir de um conjunto
/// de "Raízes" (Entrypoints Top-Level) e podar todas as funções mortas ou bibliotecas não utilizadas.
pub struct TreeShaker {
    // Mapa para busca rápida de funções por nome
    available_funcs: HashMap<String, IRFunction>,
    // Lista dos nomes das funções/actions que o Top-Level invocou ativamente
    roots: Vec<String>,
}

impl TreeShaker {
    pub fn new(all_functions: Vec<IRFunction>, roots: Vec<String>) -> Self {
        let mut available_funcs = HashMap::new();
        for f in all_functions {
            available_funcs.insert(f.name.clone(), f);
        }

        Self {
            available_funcs,
            roots,
        }
    }

    /// Executa o algoritmo de Alcançabilidade Perfeita
    pub fn shake(mut self) -> Vec<IRFunction> {
        let mut visited = HashSet::new();
        let mut queue: Vec<String> = self.roots.clone();

        // Operadores intrínsecos e FFI que nunca devem ser chacoalhados
        let instrinsics = vec!["+", "-", "*", "/", "echo!"];
        for op in instrinsics {
            visited.insert(op.to_string());
        }

        while let Some(current_func_name) = queue.pop() {
            // Se já processamos, pule
            if !visited.insert(current_func_name.clone()) {
                continue;
            }

            // Puxa a IRFunction se ela existir no módulo (pode ser intrínseca/FFI e não estar na lista)
            if let Some(func) = self.available_funcs.get(&current_func_name) {
                // Varre TODOS os blocos e instruções do DAG dessa função buscando novas chamadas
                // Por ora o IRBuilder simples tem só o nó root alocado. Numa IR real, iteraremos arena.iter().
                for (_id, val) in func.ctx.arena.iter() {
                    match val {
                        IRValue::Call { target, .. } => {
                            if !visited.contains(target) {
                                queue.push(target.clone());
                            }
                        }
                        _ => {}
                    }
                }
            }
        }

        // Filtra o mapa original retendo apenas as que foram visitadas
        let mut shaken_functions = Vec::new();
        for (name, func) in self.available_funcs.into_iter() {
            if visited.contains(&name) {
                shaken_functions.push(func);
            }
        }

        shaken_functions
    }
}
