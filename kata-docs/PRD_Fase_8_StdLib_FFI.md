# PRD: Fase 8 - Standard Library (StdLib) e Ligações FFI no Frontend

## 1. Objetivo da Fase
Atar os nós entre o Compilador Frontend (`kata-cli`) e o Runtime Backend (`kata-runtime`). Esta fase preencherá a Biblioteca Padrão (StdLib) com as funções nativas (`+`, `echo!`, `channel!`, `fork!`) ensinando o Type Checker e o Gerador de Código (Cranelift) a injetarem e conectarem as declarações C (FFI) ao binário final.

## 2. Escopo

**Dentro do Escopo:**
- **Diretiva @ffi:**
  - Suporte sintático e semântico para a declaração `@ffi("nome_em_c")` na Kata-Lang.
  - O Type Checker usará a assinatura do `.kata` para garantir o *Early Checking* funcional, e o Codegen usará a string do `@ffi` para mapear a instrução `Call` do Cranelift para a função nativa do Linker do SO.
- **Bootstrapping da StdLib:**
  - O `TypeEnv` será inicializado lendo uma "Biblioteca Padrão Embutida" (virtual ou de arquivo `.kata` fixo na distribuição) contendo as assinaturas nativas e matemáticas, para que o compilador não tenha *hardcodes* de `+` espalhados pelo seu motor.
- **Linkagem Cruzada (Cross-Linking):**
  - Ao executar `kata build main.kata`, a CLI orquestrará a chamada final ao compilador de C/Linker (ex: `gcc` ou `clang`) instruindo-o a mesclar o `main.o` (gerado na Fase 5) com a biblioteca estática `libkata_runtime.a` (gerada na Fase 6/7) para produzir o executável unificado e independente.

**Fora do Escopo:**
- **Coleções Complexas Nativas:** Estruturas avançadas (HAMT para Dicionários) não serão implementadas nesta fase fundacional. Focaremos no tráfego de Ints, Tuplas e Bumps puros na Arena via FFI.

## 3. Requisitos Técnicos

### 3.1. A Diretiva @ffi no Parser
O `Parser` deve suportar *decoradores/anotações* acima das declarações *Top-Level*.
Exemplo do que a StdLib conterá:
```kata
@ffi("kata_rt_fork")
fork! :: Action => ()

@ffi("kata_rt_print_int")
echo! :: Int => ()

@ffi("kata_rt_add_int")
+ :: Int Int => Int
```

### 3.2. Linker Invocation
O método `AOTCompiler::finish()` produz o byte array da arquitetura. O CLI (`main.rs`) passará a ter um processo dependente no SO (usando `std::process::Command` do Rust) que evocará `cc -o binario_final output.o -L[caminho_do_runtime] -lkata_runtime -lpthread -ldl -lm`.

## 4. Estruturas de Dados Principais

A grande alteração é permitir que a AST suporte Decoradores (Atributos):

```rust
// ast.rs
#[derive(Debug, Clone, PartialEq)]
pub struct TopLevelAttr {
    pub name: String,
    pub args: Vec<String>, // ex: ["kata_rt_add_int"]
}

pub enum TopLevelDecl {
    SignatureDecl {
        attrs: Vec<TopLevelAttr>, // Opcional, guarda os @ffi
        name: Ident,
        sig: TypeSignature,
    },
}

// type_checker.rs
pub struct FuncSignature {
    // ...
    pub ffi_binding: Option<String>,
}
```

## 5. Critérios de Aceite

1. **Parsing de Diretivas:** O código `.kata` contendo anotações `@ffi` deve ser parseado perfeitamente, registrando o *binding* correto no `FuncSignature` do `TypeEnv`.
2. **O Fim dos Mocks Mágicos:** Remover do `TypeEnv::new()` as linhas de "hardcode" que criavam o `+` artificialmente. O ambiente deve ser populado lendo um arquivo fonte embutido contendo a StdLib real.
3. **Cranelift FFI Translation:** No `aot_compiler.rs`, se o *target* do *Call* tiver o campo `ffi_binding` populado, o *ObjectBuilder* usará o `Linkage::Import` estrito atrelando aquele nó Cranelift à string "kata_rt_..." exigida, aguardando que o linker do SO junte as pontas no fim do build.
4. **Executável Funcional:** Ao executar `kata build examples/test_concurrency.kata`, o processo não deve parar na emissão do arquivo `.o`. Deve chamar o `gcc` (ou `clang`) da máquina de testes, acoplar o `libkata_runtime.a` e gerar um executável nativo que, ao ser rodado via `./test_concurrency`, não produza Segfault e emita o log das *Green Threads*.