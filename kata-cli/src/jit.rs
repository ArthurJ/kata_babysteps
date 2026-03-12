use cranelift_codegen::ir::{AbiParam, InstBuilder, Signature};
use cranelift_codegen::settings::{self, Configurable};
use cranelift_frontend::{FunctionBuilder, FunctionBuilderContext};
use cranelift_jit::{JITBuilder, JITModule};
use cranelift_module::{Linkage, Module};
use cranelift_native::builder as host_builder;

use crate::ir::{IRFunction, IRValue, ValueId};
use crate::type_checker::Type;

/// O Motor JIT que compila e executa código IR dinamicamente no processo atual.
pub struct JITEngine {
    builder_context: FunctionBuilderContext,
    ctx: cranelift_codegen::Context,
    module: JITModule,
}

impl JITEngine {
    pub fn new() -> Self {
        // Configura o Cranelift para focar em velocidade de compilação
        let mut flag_builder = settings::builder();
        flag_builder.set("use_colocated_libcalls", "false").unwrap();
        flag_builder.set("is_pic", "false").unwrap();
        
        let isa_builder = host_builder().expect("Host ISA não suportada");
        let isa = isa_builder.finish(settings::Flags::new(flag_builder)).unwrap();

        // Configura o JIT Module
        let builder = JITBuilder::with_isa(isa, cranelift_module::default_libcall_names());
        let module = JITModule::new(builder);

        Self {
            builder_context: FunctionBuilderContext::new(),
            ctx: module.make_context(),
            module,
        }
    }

    /// Compila uma IRFunction na RAM, gerando um ponteiro executável.
    /// Retorna a representação avaliada. Para o REPL, estamos restringindo
    /// o suporte atual ao retorno de números inteiros de 64 bits.
    pub fn compile_and_run(&mut self, ir_func: &IRFunction) -> Result<i64, String> {
        // 1. Limpa o contexto anterior
        self.module.clear_context(&mut self.ctx);

        // 2. Prepara a assinatura baseada no tipo de retorno da IR
        let int_type = cranelift_codegen::ir::types::I64;
        let mut sig = Signature::new(self.module.target_config().default_call_conv);
        
        match ir_func.sig.return_type {
            Type::Int => sig.returns.push(AbiParam::new(int_type)),
            Type::Unknown => sig.returns.push(AbiParam::new(int_type)), // Fallback experimental
            _ => return Err("JIT experimental suporta apenas retornos numéricos estritos (Int)".into()),
        }
        
        self.ctx.func.signature = sig.clone();
        
        // 3. Monta a função Cranelift
        {
            let mut builder = FunctionBuilder::new(&mut self.ctx.func, &mut self.builder_context);
            let entry_block = builder.create_block();
            
            builder.append_block_params_for_function_params(entry_block);
            builder.switch_to_block(entry_block);
            builder.seal_block(entry_block);

            // Mapeamento de `ValueId` da nossa IR para os `Value`s do Cranelift
            let mut value_map = std::collections::HashMap::new();

            // Uma helper para extrair o valor Cranelift ou emitir as constantes lazily
            let mut resolve_val = |id: ValueId, b: &mut FunctionBuilder| -> cranelift_codegen::ir::Value {
                if let Some(&v) = value_map.get(&id) {
                    return v;
                }
                
                let cranelift_val = match &ir_func.ctx.arena[id] {
                    IRValue::IntConst(n) => b.ins().iconst(int_type, *n),
                    _ => unimplemented!("Valor não suportado no JIT Builder ainda"),
                };
                
                value_map.insert(id, cranelift_val);
                cranelift_val
            };

            // Percorre as definições (Neste caso simples, vamos direto no root)
            // Em uma implementação AOT completa da Fase 5, percorreria os blocos do DAG
            let root_ir = &ir_func.ctx.arena[ir_func.root];
            
            let return_value = match root_ir {
                IRValue::IntConst(n) => builder.ins().iconst(int_type, *n),
                IRValue::Call { target, args, .. } => {
                    // Matematica JIT pura
                    if target == "+" && args.len() == 2 {
                        let arg0 = resolve_val(args[0], &mut builder);
                        let arg1 = resolve_val(args[1], &mut builder);
                        builder.ins().iadd(arg0, arg1)
                    } else if target == "*" && args.len() == 2 {
                        let arg0 = resolve_val(args[0], &mut builder);
                        let arg1 = resolve_val(args[1], &mut builder);
                        builder.ins().imul(arg0, arg1)
                    } else if target == "-" && args.len() == 2 {
                        let arg0 = resolve_val(args[0], &mut builder);
                        let arg1 = resolve_val(args[1], &mut builder);
                        builder.ins().isub(arg0, arg1)
                    } else if target == "/" && args.len() == 2 {
                        let arg0 = resolve_val(args[0], &mut builder);
                        let arg1 = resolve_val(args[1], &mut builder);
                        builder.ins().sdiv(arg0, arg1)
                    } else {
                        return Err(format!("Função JIT '{}' não suportada/reconhecida", target));
                    }
                }
                _ => return Err("Nó raiz da IR incompatível com o JIT básico".into()),
            };

            builder.ins().return_(&[return_value]);
            builder.finalize();
        }

        // 4. Declara a função para o módulo e Compila
        let id = self.module
            .declare_function(&ir_func.name, Linkage::Export, &self.ctx.func.signature)
            .map_err(|e| e.to_string())?;
        
        self.module
            .define_function(id, &mut self.ctx)
            .map_err(|e| e.to_string())?;

        // 5. Linka tudo na RAM e extrai o ponteiro executável nativo da CPU!
        self.module.clear_context(&mut self.ctx);
        self.module.finalize_definitions();
        
        let code_ptr = self.module.get_finalized_function(id);
        
        // Transmuta o ponteiro bruto num ponteiro de função Rust e RODA nativamente!
        let func: fn() -> i64 = unsafe { std::mem::transmute(code_ptr) };
        
        Ok(func())
    }
}