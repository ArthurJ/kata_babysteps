# Product Requirements Document (PRD) - Kata Compiler Phase 3: The DAG & Pure Function Optimizer

## 1. Overview
A Fase 3 do compilador Kata faz a transição do "Front-end" (Sintaxe e Tipos) para o "Back-end" (Análise e Otimização). O objetivo principal desta fase é transformar as funções puras validadas na Fase 2 em um Grafo Acíclico Dirigido (DAG) - uma Representação Intermediária (IR) - e aplicar as otimizações matemáticas fundamentais que tornam a Kata rápida por design.

Esta fase lida estritamente com lógica pura. Side-effects (Actions) não serão otimizados aqui. O foco é garantir que o encadeamento funcional (pipelines, currying) se torne a estrutura de execução mais eficiente possível antes da geração de código de máquina (que ocorrerá na Fase 5).

## 2. Goals & Objectives
*   **IR Design (DAG representation):** Projetar e implementar uma estrutura de Representação Intermediária na memória baseada em grafos, adequada para manipulações funcionais.
*   **Constant Folding & Propagation:** Resolver estaticamente operações matemáticas e lógicas cujo resultado é previsível em tempo de compilação.
*   **Stream Fusion (Deforestation):** Detectar encadeamentos de operações sobre coleções (ex: `map`, `filter`, `reduce` no `Pipe`) e fundi-los em loops únicos para evitar alocação de coleções intermediárias.
*   **Tail Call Optimization (TCO):** Analisar e garantir TCO para funções recursivas puras, transformando-as em iterações no nível da IR para prevenir *Stack Overflow*.
*   **CLI Integration:** Adicionar o comando `kata opt` (ou estender `kata check` com flags) para visualizar a IR otimizada gerada.

## 3. Scope & Requirements

### 3.1. Representação Intermediária (Kata IR - DAG)
A AST gerada pelo Front-end é hierárquica e boa para inferência, mas ruim para otimização de fluxo de dados.
*   **Req 3.1.1:** O compilador deve possuir um módulo de conversão (Lowering) que transforma as `Decl::Function` da AST em um formato IR.
*   **Req 3.1.2:** A IR deve ser estruturada como um Grafo Acíclico Dirigido (DAG), onde os nós representam operações/valores e as arestas representam o fluxo de dependência de dados.
*   **Req 3.1.3:** Variáveis locais (`With`, `Let`) devem ser convertidas para formato SSA (Static Single Assignment) ou equivalência em grafos para simplificar o rastreamento de tempo de vida.

### 3.2. Constant Folding (Resolução Estática)
A Kata incentiva o uso de currying e pipelines. Muitas vezes isso resulta em expressões que podem ser resolvidas antes do runtime.
*   **Req 3.2.1:** A IR deve analisar nós literais e primitivos (ex: `add 5 10`).
*   **Req 3.2.2:** Se todos os dependentes de um nó puro são constantes literais, o nó deve ser avaliado em tempo de compilação e substituído por um único nó literal com o resultado (ex: substituir pela constante `15`).
*   **Req 3.2.3:** Propagação de Constantes: O novo nó literal deve alimentar os nós subsequentes no DAG, repetindo o ciclo até que nenhuma otimização estática seja possível.

### 3.3. Stream Fusion (Deforestation Analyzer)
Evitar "Thunks" e alocações de listas intermediárias quando encadeamos funções de alta ordem sobre iteráveis.
*   **Req 3.3.1:** Identificar padrões estruturais na IR de funções da (futura) Standard Library, especificamente encadeamentos de `map`, `filter` e `fold`/`reduce`.
*   *(Exemplo conceitual de entrada):* `lista |> map (* 2) |> filter (> 5)`
*   **Req 3.3.2:** O otimizador deve fundir operações contíguas de map/filter em um único nó de "Stream/Iteração", que processará os elementos em uma única passagem (sem alocar a lista intermediária resultante do `map`).
*   **Req 3.3.3:** *Constraints:* A fusão deve ser interrompida em operações que dependem de estado global da lista (ex: `sort` ou `reverse`), particionando a stream.

### 3.4. Tail Call Optimization (TCO)
Em uma linguagem funcional onde a iteração primária é a recursão, TCO não é uma "feature", é um requisito de segurança.
*   **Req 3.4.1:** O analisador de IR deve identificar nós de "Chamada de Função" (`Call` ou `App`) que ocorrem na posição de retorno final de uma função (Tail Position).
*   **Req 3.4.2:** Se a chamada final for recursiva (chamando a própria função pai), o compilador deve sinalizar este nó especificamente como um `TailCall` na IR.
*   **Req 3.4.3:** O gerador de IR deve transformar logicamente esse `TailCall` em um nó de "Loop/Jump" (saltando de volta para o topo da função reescrevendo os argumentos), garantindo `O(1)` de espaço na call stack durante a Fase 5.

## 4. Architecture & Implementation Guidelines

*   **Arquitetura de Passes (Passes Architecture):** A otimização não acontece de uma vez. O módulo de otimização deve ser projetado como um "Pipeline de Passes".
    1.  Passo 1: AST -> IR (Lowering).
    2.  Passo 2: Constant Folding Pass.
    3.  Passo 3: TCO Analysis Pass.
    4.  Passo 4: Stream Fusion Pass.
*   **Representação Visual:** Fornecer meios de imprimir a IR gerada. Implementar a trait `Display` para a IR de forma que o desenvolvedor possa ver os resultados de otimização (ex: formato s-expression plano ou pseudo-assembly).
*   **Testabilidade:** O framework de testes do Rust (`cargo test`) deve validar individualmente cada passe de otimização. Por exemplo, escrever testes que verificam se uma AST contendo `add 2 3` resulta em uma IR contendo apenas o literal `5`.

## 5. Out of Scope for Phase 3
*   Geração de código LLVM ou Assembly (Fase 5).
*   Escalonamento de Goroutines/Green Threads e runtime (Fase 4).
*   Otimização de `Actions` (Side-effects ou concorrência) além da inferência básica.
*   Garbage Collection / ARC handling (isso será anexado no runtime/backend).
