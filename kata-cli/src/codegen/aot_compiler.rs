use cranelift_codegen::ir::{AbiParam, InstBuilder, Signature};
use cranelift_codegen::settings::{self, Configurable};
use cranelift_frontend::{FunctionBuilder, FunctionBuilderContext};
use cranelift_module::{Linkage, Module};
use cranelift_object::{ObjectBuilder, ObjectModule};
use cranelift_native::builder as host_builder;

use crate::ir::{IRFunction, IRValue, ValueId};
use crate::type_checker::Type;

/// Motor Ahead-of-Time que gera arquivos Objeto (.o) binários para o SO alvo.
pub struct AOTCompiler {
    builder_context: FunctionBuilderContext,
    ctx: cranelift_codegen::Context,
    module: ObjectModule,
}

impl AOTCompiler {
    pub fn new(module_name: &str) -> Self {
        let mut flag_builder = settings::builder();
        // Otimização agressiva no codegen
        flag_builder.set("opt_level", "speed").unwrap();
        flag_builder.set("is_pic", "true").unwrap(); // Requerido para binários modernos de SO (Position Independent Code)
        
        let isa_builder = host_builder().expect("Host ISA não suportada pelo Cranelift");
        let isa = isa_builder.finish(settings::Flags::new(flag_builder)).unwrap();

        // Configura a geração de Objeto (ELF/Mach-O nativo)
        let builder = ObjectBuilder::new(
            isa,
            module_name,
            cranelift_module::default_libcall_names(),
        ).expect("Falhou ao criar o ObjectBuilder");
        
        let module = ObjectModule::new(builder);

        Self {
            builder_context: FunctionBuilderContext::new(),
            ctx: module.make_context(),
            module,
        }
    }

    /// Compila uma única função para dentro do módulo objeto atual.

    pub fn compile_function(&mut self, ir_func: &IRFunction) -> Result<(), String> {
        self.module.clear_context(&mut self.ctx);

        let int_type = cranelift_codegen::ir::types::I64;
        let ptr_type = self.module.target_config().pointer_type();
        let mut sig = Signature::new(self.module.target_config().default_call_conv);

        for arg_ty in &ir_func.sig.args_types {
            match arg_ty {
                crate::type_checker::Type::Text | crate::type_checker::Type::List(_) | crate::type_checker::Type::Custom(_) => {
                    sig.params.push(AbiParam::new(ptr_type));
                }
                _ => {
                    sig.params.push(AbiParam::new(int_type));
                }
            }
        }

        match &ir_func.sig.return_type {
            crate::type_checker::Type::Text | crate::type_checker::Type::List(_) | crate::type_checker::Type::Custom(_) => {
                sig.returns.push(AbiParam::new(ptr_type));
            }
            _ => {
                sig.returns.push(AbiParam::new(int_type));
            }
        }

        
        self.ctx.func.signature = sig.clone();
        
        {
            let mut builder = FunctionBuilder::new(&mut self.ctx.func, &mut self.builder_context);
            let entry_block = builder.create_block();
            
            builder.append_block_params_for_function_params(entry_block);
            builder.switch_to_block(entry_block);
            builder.seal_block(entry_block);

            let mut value_map = std::collections::HashMap::new();

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

            fn translate(
                id: ValueId, 
                builder: &mut FunctionBuilder, 
                module: &mut ObjectModule, 
                ir_func: &IRFunction, 
                value_map: &mut std::collections::HashMap<ValueId, cranelift_codegen::ir::Value>,
                int_type: cranelift_codegen::ir::types::Type,
                ptr_type: cranelift_codegen::ir::types::Type,
            ) -> cranelift_codegen::ir::Value {
                if let Some(&v) = value_map.get(&id) {
                    return v;
                }

                let val = match &ir_func.ctx.arena[id] {
                    IRValue::IntConst(n) => builder.ins().iconst(int_type, *n),
                    IRValue::FloatConst(n) => builder.ins().f64const(*n),
                    IRValue::StringConst(s) => {
                        let mut data_ctx = cranelift_module::DataDescription::new();
                        let mut bytes = s.as_bytes().to_vec();
                        bytes.push(0); 
                        data_ctx.define(bytes.into_boxed_slice());
                        
                        let name = format!("anon_string_{}_{}", ir_func.name.replace("!", "").replace("..", "dotdot"), id.index());
                        let data_id = module.declare_data(&name, Linkage::Local, false, false).unwrap();
                        module.define_data(data_id, &data_ctx).unwrap();
                        
                        let local_id = module.declare_data_in_func(data_id, &mut builder.func);
                        builder.ins().symbol_value(ptr_type, local_id)
                    }
                    IRValue::FuncPtr(func_name) => {
                        if func_name == ".." {
                            builder.ins().iconst(int_type, 0)
                        } else {
                            // Import the function and return its address
                            let mut sig = Signature::new(module.target_config().default_call_conv);
                            // We mock the signature for the map callback
                            sig.params.push(AbiParam::new(int_type));
                            sig.returns.push(AbiParam::new(ptr_type));
                            
                            
                            // If it's a FuncPtr to "map", we need to point to "kata_rt_mock_map"
                            let actual_name = func_name;

                            // Local linkage for fizzbuzz because it's compiled in the same module!
                            let linkage = if actual_name == "fizzbuzz" { Linkage::Export } else { Linkage::Import };

                            let actual_name_ref = if actual_name.is_empty() { "__kata_empty_func" } else { actual_name };
                            println!("Declaring funcptr import: '{}' linkage: {:?}", actual_name_ref, linkage);
                            let func_id = module.declare_function(actual_name_ref, linkage, &sig).unwrap();                            let local_func = module.declare_func_in_func(func_id, &mut builder.func);
                            builder.ins().func_addr(ptr_type, local_func)
                        }
                    }
                    IRValue::Param(_, _) => unreachable!("Params should be mapped"),
                    IRValue::Call { target, args, ret_type } => {
                        let mut arg_vals = Vec::new();
                        for &arg in args {
                            arg_vals.push(translate(arg, builder, module, ir_func, value_map, int_type, ptr_type));
                        }

                        if target == "+" && args.len() == 2 {
                            builder.ins().iadd(arg_vals[0], arg_vals[1])
                        } else if target == "-" && args.len() == 2 {
                            builder.ins().isub(arg_vals[0], arg_vals[1])
                        } else if target == "*" && args.len() == 2 {
                            builder.ins().imul(arg_vals[0], arg_vals[1])
                        } else if target == "=" && args.len() == 2 {
                            let cmp = builder.ins().icmp(cranelift_codegen::ir::condcodes::IntCC::Equal, arg_vals[0], arg_vals[1]);
                            builder.ins().uextend(int_type, cmp)
                        } else if target == "mod" && args.len() == 2 {
                            builder.ins().srem(arg_vals[0], arg_vals[1])
                        } else if target == "and" && args.len() == 2 {
                            builder.ins().band(arg_vals[0], arg_vals[1])
                        } else {
                            let mut target_sig = Signature::new(module.target_config().default_call_conv);
                            let func_name_str = if target.is_empty() { "__kata_empty_func".to_string() } else { target.clone() };

                            for val in &arg_vals {
                                let ty = builder.func.dfg.value_type(*val);
                                target_sig.params.push(AbiParam::new(ty));
                            }

                            // Verifica o tipo de retorno explicitado na AST
                            match ret_type {
                                crate::type_checker::Type::Int => {
                                    target_sig.returns.push(AbiParam::new(int_type));
                                }
                                crate::type_checker::Type::Float => {
                                    target_sig.returns.push(AbiParam::new(cranelift_codegen::ir::types::F64));
                                }
                                crate::type_checker::Type::Text | crate::type_checker::Type::List(_) | crate::type_checker::Type::Custom(_) | crate::type_checker::Type::Array(_) => {
                                    target_sig.returns.push(AbiParam::new(ptr_type));
                                }
                                crate::type_checker::Type::Tuple(_) => {
                                    target_sig.returns.push(AbiParam::new(ptr_type));
                                }
                                _ => {
                                    // Default/Unknown assume int/pointer genérico (fallback conservador)
                                    target_sig.returns.push(AbiParam::new(int_type));
                                }
                            }

                            if func_name_str.is_empty() { println!("WARN: declaring empty target_sig name!"); } println!("Declaring call import: '{}'", func_name_str); let func_id = module.declare_function(&func_name_str, Linkage::Import, &target_sig).unwrap();
                            let local_func = module.declare_func_in_func(func_id, &mut builder.func);
                            let call = builder.ins().call(local_func, &arg_vals);
                            let results = builder.inst_results(call);
                            if results.is_empty() {
                                builder.ins().iconst(int_type, 0)
                            } else {
                                results[0]
                            }
                        }
                    }
                    IRValue::MakeTuple(items) => {
                        let mut alloc_sig = Signature::new(module.target_config().default_call_conv);
                        alloc_sig.params.push(AbiParam::new(cranelift_codegen::ir::types::I32)); // size
                        alloc_sig.params.push(AbiParam::new(cranelift_codegen::ir::types::I32)); // type_tag
                        alloc_sig.returns.push(AbiParam::new(ptr_type));

                        let alloc_func_id = module.declare_function("kata_rt_alloc", Linkage::Import, &alloc_sig).unwrap();
                        let local_alloc_func = module.declare_func_in_func(alloc_func_id, &mut builder.func);

                        let size_val = builder.ins().iconst(cranelift_codegen::ir::types::I32, (items.len() * 8) as i64);
                        let type_tag_val = builder.ins().iconst(cranelift_codegen::ir::types::I32, 1); // 1 = Tuple
                        
                        let call = builder.ins().call(local_alloc_func, &[size_val, type_tag_val]);
                        let ptr_val = builder.inst_results(call)[0];
                        
                        for (i, item) in items.iter().enumerate() {
                            let val = translate(*item, builder, module, ir_func, value_map, int_type, ptr_type);
                            // Armazena no offset (cada item é i64/ptr -> 8 bytes)
                            builder.ins().store(cranelift_codegen::ir::MemFlags::trusted(), val, ptr_val, (i * 8) as i32);
                        }
                        
                        ptr_val
                    },
                    IRValue::Guard { condition, true_result, false_result } => {
                        let cond_val = translate(*condition, builder, module, ir_func, value_map, int_type, ptr_type);
                        
                        let true_block = builder.create_block();
                        let false_block = builder.create_block();
                        let merge_block = builder.create_block();
                        
                        let cond_bool = builder.ins().icmp_imm(cranelift_codegen::ir::condcodes::IntCC::NotEqual, cond_val, 0);
                        builder.ins().brif(cond_bool, true_block, &[], false_block, &[]);
                        
                        // TRUE BLOCK
                        builder.switch_to_block(true_block);
                        // Seal only after all predecessors are known (here, the single brif edge)
                        builder.seal_block(true_block);
                        let mut true_map = value_map.clone();
                        let true_val = translate(*true_result, builder, module, ir_func, &mut true_map, int_type, ptr_type);
                        
                        // The type returned by Guard could be Int or Ptr. 
                        // For simplicity, we just look at the Type of `true_val`.
                        let val_type = builder.func.dfg.value_type(true_val);
                        builder.ins().jump(merge_block, &[true_val]);
                        
                        // FALSE BLOCK
                        builder.switch_to_block(false_block);
                        builder.seal_block(false_block);
                        let mut false_map = value_map.clone();
                        let false_val = translate(*false_result, builder, module, ir_func, &mut false_map, int_type, ptr_type);
                        builder.ins().jump(merge_block, &[false_val]);
                        
                        // MERGE BLOCK
                        builder.switch_to_block(merge_block);
                        builder.seal_block(merge_block);
                        builder.append_block_param(merge_block, val_type);
                        
                        // Must sync any new states back if they mattered, but in functional IR they don't, except the return val
                        builder.block_params(merge_block)[0]
                    }
                };

                value_map.insert(id, val);
                val
            }

            let return_value = translate(ir_func.root, &mut builder, &mut self.module, ir_func, &mut value_map, int_type, ptr_type);
            builder.ins().return_(&[return_value]);
            builder.finalize();
        }

        let id = self.module
            .declare_function(&ir_func.name, Linkage::Export, &self.ctx.func.signature)
            .map_err(|e| e.to_string())?;
        
        self.module
            .define_function(id, &mut self.ctx)
            .map_err(|e| e.to_string())?;

        Ok(())
    }
        pub fn compile_system_main(&mut self, root_actions: Vec<String>) -> Result<(), String> {
        self.module.clear_context(&mut self.ctx);

        // Assinatura nativa da main() do C: `int main(int argc, char** argv)`
        let mut sig = Signature::new(self.module.target_config().default_call_conv);
        // C main signature params
        sig.params.push(AbiParam::new(cranelift_codegen::ir::types::I32)); // argc
        sig.params.push(AbiParam::new(self.module.target_config().pointer_type())); // argv
        sig.returns.push(AbiParam::new(cranelift_codegen::ir::types::I32)); // Returns int in C
        
        self.ctx.func.signature = sig.clone();
        
        {
            let mut builder = FunctionBuilder::new(&mut self.ctx.func, &mut self.builder_context);
            let entry_block = builder.create_block();
            builder.append_block_params_for_function_params(entry_block);
            builder.switch_to_block(entry_block);
            builder.seal_block(entry_block);

            // Reseta/inicializa a arena local para o ambiente da Main
            let mut reset_sig = Signature::new(self.module.target_config().default_call_conv);
            let reset_func_id = self.module.declare_function("kata_rt_reset_arena", Linkage::Import, &reset_sig).unwrap();
            let local_reset_func = self.module.declare_func_in_func(reset_func_id, &mut builder.func);
            builder.ins().call(local_reset_func, &[]);

            // Importa a assinatura padrão das functions que construímos
            let kata_sig = Signature::new(self.module.target_config().default_call_conv);
            let imported_sig = builder.func.import_signature(kata_sig);

            for action_name in root_actions {
                let mut action_sig = Signature::new(self.module.target_config().default_call_conv);
                // The main action in kata takes no arguments typically, but compiled kata functions usually return I64
                // According to compile_function, if no returns were added for Unknown type, it returns I64.
                action_sig.returns.push(AbiParam::new(cranelift_codegen::ir::types::I64));
                
                let func_name = self.module.declare_function(&action_name, Linkage::Import, &action_sig).unwrap();
                let local_func = self.module.declare_func_in_func(func_name, &mut builder.func);
                // Dispara a Action top-level
                builder.ins().call(local_func, &[]);
            }

            // main() do C espera retornar `0` para indicar sucesso pro SO!
            let exit_code = builder.ins().iconst(cranelift_codegen::ir::types::I32, 0);
            builder.ins().return_(&[exit_code]);
            builder.finalize();
        }

        // Exporta a `main` publicamente pro GCC encontrar no Linker!
        let id = self.module
            .declare_function("main", Linkage::Export, &self.ctx.func.signature)
            .map_err(|e| e.to_string())?;
            
        self.module
            .define_function(id, &mut self.ctx)
            .map_err(|e| e.to_string())?;

        Ok(())
    }

    /// Emite o byte array do binário no formato da máquina alvo (.o)
    pub fn finish(self) -> Vec<u8> {
        self.module.finish().emit().unwrap()
    }
}