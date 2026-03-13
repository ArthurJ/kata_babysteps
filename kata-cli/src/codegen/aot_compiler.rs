use crate::codegen::shared_codegen::{compile_normal_function, compile_tail_recursive_function};
use crate::ir::IRFunction;
use crate::type_checker::Type;
use cranelift_codegen::ir::{AbiParam, InstBuilder, Signature};
use cranelift_codegen::settings;
use cranelift_frontend::{FunctionBuilder, FunctionBuilderContext};
use cranelift_module::{Linkage, Module};
use cranelift_object::{ObjectBuilder, ObjectModule};
use std::collections::HashMap;

pub struct AOTCompiler {
    module: ObjectModule,
    ctx: cranelift_codegen::Context,
    compiled_signatures: HashMap<String, Signature>,
}

impl AOTCompiler {
    pub fn new(module_name: &str) -> Self {
        let flag_builder = settings::builder();
        let isa_builder = cranelift_native::builder().unwrap_or_else(|msg| {
            panic!("host machine is not supported: {}", msg);
        });
        let isa = isa_builder.finish(settings::Flags::new(flag_builder)).unwrap();
        let builder = ObjectBuilder::new(
            isa,
            module_name,
            cranelift_module::default_libcall_names(),
        ).unwrap();
        let module = ObjectModule::new(builder);

        Self {
            module,
            ctx: cranelift_codegen::Context::new(),
            compiled_signatures: HashMap::new(),
        }
    }

    pub fn compile_function(&mut self, ir_func: &IRFunction) -> Result<(), String> {
        self.module.clear_context(&mut self.ctx);

        let int_type = cranelift_codegen::ir::types::I64;
        let ptr_type = self.module.target_config().pointer_type();

        let mut sig = Signature::new(self.module.target_config().default_call_conv);
        for t in &ir_func.sig.args_types {
            let cranelift_ty = match t {
                Type::Int | Type::Bool => int_type,
                Type::Float => cranelift_codegen::ir::types::F64,
                _ => ptr_type,
            };
            sig.params.push(AbiParam::new(cranelift_ty));
        }

        let ret_cranelift_ty = match &ir_func.sig.return_type {
            Type::Int | Type::Bool => int_type,
            Type::Float => cranelift_codegen::ir::types::F64,
            _ => ptr_type,
        };
        sig.returns.push(AbiParam::new(ret_cranelift_ty));

        self.ctx.func.signature = sig.clone();

        let mut builder_ctx = FunctionBuilderContext::new();
        let mut builder = FunctionBuilder::new(&mut self.ctx.func, &mut builder_ctx);

        // Compila função tail-recursive ou normal
        if ir_func.is_tail_recursive {
            compile_tail_recursive_function(ir_func, &mut builder, &mut self.module, int_type, ptr_type)?;
        } else {
            compile_normal_function(ir_func, &mut builder, &mut self.module, int_type, ptr_type)?;
        }

        builder.finalize();

        // Exporta a função com seu nome original
        let id = self.module
            .declare_function(&ir_func.name, Linkage::Export, &self.ctx.func.signature)
            .map_err(|e| e.to_string())?;

        self.module
            .define_function(id, &mut self.ctx)
            .map_err(|e| {
                log::error!("CRANELIFT VERIFIER ERROR in {}: {:?}", ir_func.name, e);
                e.to_string()
            })?;

        // Salva a assinatura desta função para uso posterior no wrapper main
        self.compiled_signatures.insert(ir_func.name.clone(), self.ctx.func.signature.clone());

        Ok(())
    }

    pub fn compile_system_main(&mut self, root_actions: Vec<String>) -> Result<(), String> {
        self.module.clear_context(&mut self.ctx);

        let int_type = cranelift_codegen::ir::types::I32;
        let mut sig = Signature::new(self.module.target_config().default_call_conv);
        sig.params.push(AbiParam::new(int_type)); // argc
        sig.params.push(AbiParam::new(self.module.target_config().pointer_type())); // argv
        sig.returns.push(AbiParam::new(int_type));

        self.ctx.func.signature = sig.clone();

        let mut builder_ctx = FunctionBuilderContext::new();
        let mut builder = FunctionBuilder::new(&mut self.ctx.func, &mut builder_ctx);

        // Cria o bloco de entrada com os parâmetros da assinatura (argc, argv)
        let block = builder.create_block();
        for param in &sig.params {
            builder.append_block_param(block, param.value_type);
        }
        builder.switch_to_block(block);
        builder.seal_block(block);

        // Chama a action main/kata_main (único entrypoint padrão)
        for action in root_actions {
            let lookup_names = if action == "main" {
                vec!["kata_main".to_string(), "main!".to_string(), "main".to_string()]
            } else {
                vec![action.clone()]
            };

            let (func_name, action_sig) = lookup_names.iter()
                .find_map(|name| {
                    self.compiled_signatures.get(name).map(|sig| (name.clone(), sig.clone()))
                })
                .unwrap_or_else(|| {
                    log::warn!("Assinatura não encontrada para {}, usando padrão", action);
                    (action.clone(), Signature::new(self.module.target_config().default_call_conv))
                });

            let func_id = self.module.declare_function(&func_name, Linkage::Import, &action_sig)
                .map_err(|e| format!("Erro ao declarar action {}: {}", func_name, e))?;
            let local_func = self.module.declare_func_in_func(func_id, &mut builder.func);
            builder.ins().call(local_func, &[]);
        }

        let ret_val = builder.ins().iconst(int_type, 0);
        builder.ins().return_(&[ret_val]);
        builder.finalize();

        let id = self.module.declare_function("main", Linkage::Export, &self.ctx.func.signature).unwrap();
        self.module.define_function(id, &mut self.ctx).map_err(|e| {
            log::error!("CRANELIFT VERIFIER ERROR in wrapper main: {:?}", e);
            e.to_string()
        })?;
        Ok(())
    }

    pub fn finish(self) -> Vec<u8> {
        let product = self.module.finish();
        product.emit().unwrap()
    }
}
