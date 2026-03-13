use crate::ir::{IRFunction, IRValue};
use crate::type_checker::Type;
use cranelift_codegen::ir::{AbiParam, Block, Signature, Value, InstBuilder};
use cranelift_codegen::settings::{self, Configurable};
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

/// Compila uma função tail-recursive com estrutura de loop
fn compile_tail_recursive_function(
    ir_func: &IRFunction,
    builder: &mut FunctionBuilder,
    module: &mut ObjectModule,
    int_type: cranelift_codegen::ir::types::Type,
    ptr_type: cranelift_codegen::ir::types::Type,
) -> Result<(), String> {
    // Cria blocos: entry -> loop_block
    let entry_block = builder.create_block();
    let loop_block = builder.create_block();

    // Entry block: apenas passa os parâmetros iniciais para o loop
    builder.switch_to_block(entry_block);
    builder.append_block_params_for_function_params(entry_block);

    // Passa os parâmetros do entry para o loop_block
    let entry_params: Vec<Value> = (0..ir_func.sig.args_types.len())
        .map(|i| builder.block_params(entry_block)[i])
        .collect();
    builder.ins().jump(loop_block, &entry_params);
    builder.seal_block(entry_block);

    // Loop block: onde a lógica da função é executada
    builder.switch_to_block(loop_block);
    for t in &ir_func.sig.args_types {
        let cranelift_ty = match t {
            Type::Int | Type::Bool => int_type,
            Type::Float => cranelift_codegen::ir::types::F64,
            _ => ptr_type,
        };
        builder.append_block_param(loop_block, cranelift_ty);
    }

    // Mapeia parâmetros do loop_block
    let mut value_map: HashMap<crate::ir::ValueId, Value> = HashMap::new();
    for (i, _) in ir_func.sig.args_types.iter().enumerate() {
        let val = builder.block_params(loop_block)[i];
        for (id, ir_val) in ir_func.ctx.arena.iter() {
            if let IRValue::Param(idx, _) = ir_val {
                if *idx == i {
                    value_map.insert(id, val);
                }
            }
        }
    }

    // Traduz a raiz da função
    let return_value = translate(
        ir_func.root,
        builder,
        module,
        ir_func,
        &mut value_map,
        int_type,
        ptr_type,
        loop_block,
    );

    // Adiciona return apenas se o bloco não foi terminado por TailRecurse
    if let Some(val) = return_value {
        builder.ins().return_(&[val]);
    }

    builder.seal_block(loop_block);

    Ok(())
}

/// Compila uma função normal (não tail-recursive)
fn compile_normal_function(
    ir_func: &IRFunction,
    builder: &mut FunctionBuilder,
    module: &mut ObjectModule,
    int_type: cranelift_codegen::ir::types::Type,
    ptr_type: cranelift_codegen::ir::types::Type,
) -> Result<(), String> {
    // Cria o bloco de entrada
    let entry_block = builder.create_block();
    builder.switch_to_block(entry_block);
    builder.append_block_params_for_function_params(entry_block);

    // Mapeia parâmetros IR para valores Cranelift
    let mut value_map: HashMap<crate::ir::ValueId, Value> = HashMap::new();
    for (i, _) in ir_func.sig.args_types.iter().enumerate() {
        let val = builder.block_params(entry_block)[i];
        for (id, ir_val) in ir_func.ctx.arena.iter() {
            if let IRValue::Param(idx, _) = ir_val {
                if *idx == i {
                    value_map.insert(id, val);
                }
            }
        }
    }

    // Traduz o corpo da função
    let return_value = translate(
        ir_func.root,
        builder,
        module,
        ir_func,
        &mut value_map,
        int_type,
        ptr_type,
        entry_block,
    );

    // Adiciona return apenas se o bloco não foi terminado
    if let Some(val) = return_value {
        builder.ins().return_(&[val]);
    }
    builder.seal_block(entry_block);

    Ok(())
}

/// Traduz um nó IR para instruções Cranelift
/// Retorna Some(Value) ou None se o bloco foi terminado (por TailRecurse)
fn translate(
    id: crate::ir::ValueId,
    builder: &mut FunctionBuilder,
    module: &mut ObjectModule,
    ir_func: &IRFunction,
    value_map: &mut HashMap<crate::ir::ValueId, Value>,
    int_type: cranelift_codegen::ir::types::Type,
    ptr_type: cranelift_codegen::ir::types::Type,
    loop_block: Block,
) -> Option<Value> {
    if let Some(&v) = value_map.get(&id) {
        return Some(v);
    }

    let val: Value = match &ir_func.ctx.arena[id] {
        IRValue::IntConst(n) => builder.ins().iconst(int_type, *n),
        IRValue::FloatConst(n) => builder.ins().f64const(*n),
        IRValue::StringConst(s) => {
            let data_id = module.declare_data(&format!("str_{}", id.index()), Linkage::Export, false, false).unwrap();
            let mut desc = cranelift_module::DataDescription::new();
            desc.define(s.as_bytes().to_vec().into_boxed_slice());
            module.define_data(data_id, &desc).unwrap();
            let local_data = module.declare_data_in_func(data_id, &mut builder.func);
            builder.ins().symbol_value(ptr_type, local_data)
        }
        IRValue::FuncPtr(name) => {
            if name == "kata_main" {
                builder.ins().iconst(ptr_type, 0)
            } else {
                let mut sig = Signature::new(module.target_config().default_call_conv);
                sig.params.push(AbiParam::new(int_type));
                sig.returns.push(AbiParam::new(ptr_type));

                let linkage = if name == "kata_main" || name.starts_with("impl_") { Linkage::Export } else { Linkage::Import };
                let func_id = module.declare_function(name, linkage, &sig).unwrap();
                let local_func = module.declare_func_in_func(func_id, &mut builder.func);
                builder.ins().func_addr(ptr_type, local_func)
            }
        }
        IRValue::Param(_, _) => unreachable!("Params should be mapped"),
        IRValue::Call { target, args, ret_type, is_tail_call: _ } => {
            let mut arg_vals = Vec::new();
            for &arg in args {
                if let Some(v) = translate(arg, builder, module, ir_func, value_map, int_type, ptr_type, loop_block) {
                    arg_vals.push(v);
                }
            }

            let mut target_sig = Signature::new(module.target_config().default_call_conv);
            for val in &arg_vals {
                let ty = builder.func.dfg.value_type(*val);
                target_sig.params.push(AbiParam::new(ty));
            }

            let ret_cranelift_ty = match ret_type {
                Type::Int | Type::Bool => int_type,
                Type::Float => cranelift_codegen::ir::types::F64,
                _ => ptr_type,
            };
            target_sig.returns.push(AbiParam::new(ret_cranelift_ty));

            let func_id = module.declare_function(target, Linkage::Import, &target_sig).unwrap();
            let local_func = module.declare_func_in_func(func_id, &mut builder.func);
            let call = builder.ins().call(local_func, &arg_vals);
            builder.inst_results(call)[0]
        }
        IRValue::TailRecurse { args } => {
            // TCO: Atualiza os parâmetros do loop e faz jump
            let mut arg_vals = Vec::new();
            for &arg in args {
                if let Some(v) = translate(arg, builder, module, ir_func, value_map, int_type, ptr_type, loop_block) {
                    arg_vals.push(v);
                }
            }

            log::debug!("Aplicando TCO (TailRecurse)");
            builder.ins().jump(loop_block, &arg_vals);

            // Bloco terminado - retorna None
            return None;
        }
        IRValue::MakeTuple(items) => {
            let mut item_vals = Vec::new();
            for &i in items {
                if let Some(v) = translate(i, builder, module, ir_func, value_map, int_type, ptr_type, loop_block) {
                    item_vals.push(v);
                }
            }
            if item_vals.is_empty() { builder.ins().iconst(int_type, 0) }
            else { item_vals[0] }
        }
        IRValue::MakeClosure { code_ptr, env_captures: _ } => {
            let mut sig = Signature::new(module.target_config().default_call_conv);
            sig.params.push(AbiParam::new(ptr_type));
            sig.params.push(AbiParam::new(int_type));
            sig.returns.push(AbiParam::new(ptr_type));

            let linkage = if code_ptr.starts_with("impl_") { Linkage::Export } else { Linkage::Import };
            let func_id = module.declare_function(code_ptr, linkage, &sig).unwrap();
            let local_func = module.declare_func_in_func(func_id, &mut builder.func);
            builder.ins().func_addr(ptr_type, local_func)
        }
        IRValue::Guard { condition, true_result, false_result } => {
            let cond_val = translate(*condition, builder, module, ir_func, value_map, int_type, ptr_type, loop_block)?;

            let true_block = builder.create_block();
            let false_block = builder.create_block();
            let merge_block = builder.create_block();

            builder.ins().brif(cond_val, true_block, &[], false_block, &[]);

            // True branch
            builder.switch_to_block(true_block);
            builder.seal_block(true_block);
            let mut true_map = value_map.clone();
            let true_val = translate(*true_result, builder, module, ir_func, &mut true_map, int_type, ptr_type, loop_block);

            // Verifica se o true branch terminou
            let true_terminated = true_val.is_none();
            let val_type = if let Some(v) = true_val {
                let t = builder.func.dfg.value_type(v);
                builder.ins().jump(merge_block, &[v]);
                t
            } else {
                int_type
            };

            // False branch
            builder.switch_to_block(false_block);
            builder.seal_block(false_block);
            let mut false_map = value_map.clone();
            let false_val = translate(*false_result, builder, module, ir_func, &mut false_map, int_type, ptr_type, loop_block);

            let false_terminated = false_val.is_none();
            if let Some(v) = false_val {
                builder.ins().jump(merge_block, &[v]);
            }

            // Merge block
            if true_terminated && false_terminated {
                // Ambos terminaram - isso é incomum mas pode acontecer
                // Retornamos um valor dummy
                builder.ins().iconst(int_type, 0)
            } else {
                builder.switch_to_block(merge_block);
                builder.seal_block(merge_block);
                builder.append_block_param(merge_block, val_type);
                builder.block_params(merge_block)[0]
            }
        }
    };

    value_map.insert(id, val);
    Some(val)
}
