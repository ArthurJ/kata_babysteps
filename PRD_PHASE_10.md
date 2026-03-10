# Kata Compiler - Product Requirements Document (PRD)
## Fase 10: Runtime Integration, Concurrency & Semantic Completeness

### 1. Visão Geral
A Fase 9 consolidou o "Frontend" do compilador Kata. O Parser, a análise de tipos (Type Checker), a AST e a Representação Intermediária (IR) estão gerando grafos limpos, com otimizações estáticas ativas (Tree Shaking, Heurística de Caching). 
O objetivo da **Fase 10** é consolidar o "Backend" e o "Runtime", substituindo todos os *mocks*, *stubs* em C e soluções de Prova de Conceito (PoC) por implementações nativas e robustas em Rust, além de finalizar as pendências semânticas da linguagem.

### 2. Objetivos Principais

#### 2.1. Concorrência Nativa e Work-Stealing Scheduler
*   **Problema:** Atualmente, as funções de concorrência (`spawn!`, `channel!`, `queue!`, `broadcast!`) são apenas assinaturas aprovadas pelo Type Checker que chamam *stubs* C vazios no Linker.
*   **Requisito:** Implementar a ponte real entre o Cranelift (AOT) e o `kata-runtime/src/scheduler.rs`.
*   **Tarefas:**
    *   Substituir os stubs em `kata_rt_stubs.c` por exportações reais (FFI) do Rust em `kata-runtime/src/ffi.rs`.
    *   Fazer o Cranelift alocar e despachar ponteiros de função para as filas do `WorkStealingQueue`.
    *   Implementar a máquina de estados para corrotinas/yields estruturais gerados pelo backend (transição do contexto de memória opaca para execução paralela multithread).

#### 2.2. Consolidação do Motor de Caching (Memoizer LRU/LFU)
*   **Problema:** O Auto-Caching funciona sintaticamente e injeta blocos condicionalmente, mas a persistência usa um array global `int64_t fib_cache[1000]` embutido via C como *hack*.
*   **Requisito:** Ligar o nó Cranelift de Cache diretamente ao `Memoizer<K, V>` protegido por Mutex em `kata-runtime/src/cache.rs`.
*   **Tarefas:**
    *   Exportar `kata_rt_cache_get` e `kata_rt_cache_put` via Rust FFI utilizando estruturas reais (`Arc`, `Mutex`).
    *   Garantir a retenção de memória em *Heap* (através do já existente `KataRc` para suportar tipos complexos, e não apenas primitivos emulados como ponteiros).

#### 2.3. Resolução Completa da Stream Fusion
*   **Problema:** O otimizador identifica `map |> filter` e solicita a função embutida `fused_filter_map`, que hoje reside como um mock inútil no C.
*   **Requisito:** Dinamismo na geração de IR. Em vez de depender do linker para implementar funções mágicas compostas, o compilador deve *in-lining* (embutir) a lógica.
*   **Tarefas:**
    *   Modificar o `stream_fusion.rs` para alterar o Grafo da IR de fato: gerar um bloco `Loop` (similar ao TCO) que intercala as instruções de iteração original em um único loop em Assembly, em vez de depender de funções intrínsecas "falsas".

#### 2.4. Refinement Types Dependentes (Verificação em Runtime)
*   **Problema:** Declarações como `data Peso as (Float, > _ 0.0)` passam na compilação, mas a validação dinâmica `> 0.0` nunca ocorre.
*   **Requisito:** A injeção automática de validadores invisíveis sempre que um construtor de tipo dependente é chamado.
*   **Tarefas:**
    *   No Backend/IR, ao processar `AllocADT` para um tipo com `Refinement`, gerar um branch (Select/If) invisível de checagem.
    *   Se a condição falhar, despachar uma chamada para *Runtime Panic* nativa do Kata, garantindo que objetos de dados inválidos nunca cheguem a existir na Heap.

#### 2.5. Tuplas e List Ranges de Verdade
*   **Problema:** `[1..15]` (Ranges) e `(A, B)` (Tuplas literais) disparam comentários de `// PoC: Just a dummy array`.
*   **Requisito:** Geração exata da coleção.
*   **Tarefas:**
    *   Para Ranges: Fazer o lowering gerar um loop sequencial para popular o `AllocArray` primitivo em tempo de execução, ou um Lazy Iterator.
    *   Para Tuplas: Eliminar o `KataType::Var("Tupla_Pendente")` no Type Checker; compilar a tupla num Heap Struct anônimo que será destruído ou extraído pelo `GetElem`.

#### 2.6. Generalização Arquitetural do Cranelift Backend (32/64 bits)
*   **Problema:** O arquivo `cranelift_backend.rs` chuta `types::I64` para ponteiros e flags Booleanos, presumindo plataforma e gerando *warnings*.
*   **Requisito:** Adotar a abstração robusta do `isa` e suporte cross-platform em conformidade com as Flags de Alvo (Target Triple).
*   **Tarefas:**
    *   Tornar a atribuição de tamanho de ponteiro condicional ao contexto do compilador.
    *   Substituir respostas padrão falhas (`_ => self.backend.emit_int_const(0)`) por `unreachable!` rígidos ou retornos em Result Seguros, blindando o backend contra ASTs parcialmente otimizadas.

### 3. Entregáveis Esperados no Final da Fase 10
1. O arquivo binário `./test_concurrency` deve ativar 4 threads nativas de SO e transferir valores em tempo real entre a `main` e a `tarefa_pesada` usando filas de roubo de trabalho.
2. O arquivo `./test_fibonacci` não deve depender do compilador GCC para nada relacionado a Cache; tudo ocorrerá nas primitivas do `kata-runtime`.
3. Iniciar instâncias de `Peso` com valor negativo explodirão em RunTime panics documentadas pelo código.
4. Total erradicação das strings "PoC", "dummy", "mock" em operações centrais do compilador (`src/codegen`, `src/optimizer`, `src/type_checker`).

### 4. Ordem Recomendada de Execução
*   **Sprint 1:** Motor de Cache Rust-FFI (Troca do array C pelo Hashmap Rust existente).
*   **Sprint 2:** Geração em IR de Loops (Stream Fusion, List Ranges) e Tuplas verdadeiras no Heap.
*   **Sprint 3:** Injeção Condicional no AllocADT para Refinement Types.
*   **Sprint 4:** A Ponte de Ouro: Scheduler WorkStealing de Concorrência x FFI.