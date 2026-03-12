# PRD: Fase 5 - Backend AOT, Tree-Shaking e Geração de Binário

## 1. Objetivo da Fase
Implementar o motor de compilação Ahead-of-Time (AOT) da Kata-Lang. Esta fase recebe as representações `IRFunction` geradas pelo IR Builder, elimina o código morto (Tree-Shaking Perfeito), traduz a IR para Assembly nativo através do Cranelift e gera um arquivo objeto (`.o`) ou um binário executável independente (`ELF`/`Mach-O`) que pode ser rodado no sistema operacional sem a presença do compilador ou do REPL.

## 2. Escopo

**Dentro do Escopo:**
- **Grafo de Chamadas (Call Graph) e Tree-Shaking:**
  - Varredura de alcançabilidade partindo de múltiplas raízes: **todas as Actions invocadas no Top-Level do módulo principal**.
  - Remoção de qualquer `IRFunction` que não seja atingível por essas chamadas raiz.
  - Eliminação agressiva de funções de teste (anotadas estaticamente com `@test`) caso o ambiente seja produção (`kata build`).
  
- **Geração de Objeto AOT (Cranelift Object):**
  - Integração da biblioteca `cranelift-object` para transformar o contexto do Cranelift em arquivos binários estáticos compatíveis com o *linker* do sistema (ex: `gcc` / `ld`).
  - Geração de uma **função de sistema `main` sintética**: Como Sistemas Operacionais exigem o símbolo `main` nativo, o compilador criará essa função internamente e empilhará a invocação em sequência de todas as Actions chamadas no Top-Level do `.kata`.
  
- **Suporte ao Domínio Impuro (Actions) na Geração de Código:**
  - Tradução dos nós `TypedActionStmt` para o formato nativo.
  - Inicialização das estruturas de controle de estado para permitir loops e reatribuições nativas na stack.

- **Atualização da CLI (`kata build` e `kata run`):**
  - Fazer os comandos de CLI funcionarem de verdade, consumindo arquivos `.kata` reais, disparando o pipeline inteiro e produzindo um artefato invocável no disco.

**Fora do Escopo:**
- Otimizações intrincadas de alocação de memória na *Heap* Global ou canais concorrentes (CSP). Isso requer suporte de código C/Rust e é o coração absoluto da **Fase 6: Runtime**. Nesta Fase 5, a alocação de structs continuará operando em stubs genéricos e cálculos puros na *stack*.
- Emissão de *Debug Info* (`gimli`/DWARF). Focaremos apenas no *Release* Assembly puro e direto.

## 3. Requisitos Técnicos

### 3.1. Alcançabilidade Perfeita (Tree-Shaking Perfeito)
O motor de alcançabilidade será construído no arquivo `src/codegen/tree_shaker.rs`. 
O algoritmo:
1. O compilador varre a AST do arquivo principal procurando por **invocações imperativas no nível superior** (ex: `echo! "A"`, `processar!`).
2. Adiciona o nome de todas essas funções nativas ou de usuário à lista de `visitados` (Raízes). Se não houver chamadas top-level, e a compilação for um executável autônomo, ele não fará nada ou emitirá um aviso.
3. Para cada instrução `Call` dentro do DAG da IR das raízes mapeadas, o destino da *Call* é adicionado à lista `pendente`.
4. Repete o passo 3 iterativamente consumindo a lista `pendente` até esvaziar.
5. Todas as `IRFunction`s que não estiverem na lista `visitados` são imediatamente destruídas, não sendo enviadas ao Codegen do Cranelift.

### 3.2. Diferença JIT vs AOT (O Módulo Cranelift)
Diferente da Fase 4 que usava `JITBuilder` e rodava os ponteiros diretamente em RAM, a Fase 5 usará o `ObjectBuilder`.
A interface do `cranelift-object` constrói as instruções exatamente iguais ao JIT, mas em vez de dar `finalize_definitions` na memória local, ele exige que o programador chame `module.finish().emit()` gravando o stream de bytes para o disco (`output.o`).

## 4. Estruturas de Dados Principais

A arquitetura do codegen refletirá um despachante.

```rust
// Mapeamento e gestão do AOT Backend
pub struct AOTCompiler {
    module: ObjectModule,
    builder_context: FunctionBuilderContext,
    ctx: codegen::Context,
}

impl AOTCompiler {
    pub fn compile_module(&mut self, functions: Vec<IRFunction>) -> Result<Vec<u8>, String>;
}

// O analisador de árvore morta
pub struct TreeShaker {
    entrypoint: String,
}

impl TreeShaker {
    /// Filtra a lista mantendo apenas o código vivo.
    pub fn shake(&self, all_funcs: Vec<IRFunction>) -> Vec<IRFunction>;
}
```

## 5. Critérios de Aceite

1. **Tree-Shaking Eficaz:** Um arquivo `.kata` contendo uma função `soma` não usada e a função `main!` que imprime um texto não deve conter os rastros em Assembly de `soma` no output final.
2. **Objeto Nativo:** Executar `kata build test.kata` deve produzir um arquivo `test.o` real no diretório local.
3. **CLI Real:** Ao executar `kata run mock_math.kata`, o CLI deve ler o arquivo (se estiver vazio deve falhar), varrer o Lexer, Parser, TypeChecker, construir IR, agitar a árvore, montar o AOT e compilar. (Opcional caso ainda não liguemos ao GCC para rodar nativamente, a validação de que o pipeline encerrou com sucesso já será suficiente na Fase 5).