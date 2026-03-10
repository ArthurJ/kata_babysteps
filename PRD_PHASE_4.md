# Product Requirements Document (PRD) - Kata Compiler Phase 4: Embedded Runtime & Memory Management

## 1. Overview
A Fase 4 marca o início do desenvolvimento do "Motor de Execução" (Runtime Embutido) da Kata. Diferente das linguagens com Virtual Machine (JVM/CLR), a Kata compila para um binário nativo estático (Ahead-of-Time). No entanto, para suportar concorrência massiva segura (via Actors/Actions), passagem de mensagens imutável e gerenciamento de memória automático sem *Stop-The-World*, precisamos de um mini-S.O. injetado no binário gerado.

O Runtime da Kata será responsável por orquestrar *Green Threads*, gerenciar a memória estritamente imutável (via ARC customizado) e fornecer os canais isolados para a comunicação assíncrona. Esta fase não trata de geração de código de máquina (isso é na Fase 5), mas sim de construir as bibliotecas de sistema C/Rust que a Fase 5 irá linkar.

## 2. Goals & Objectives
*   **Immutability First Memory Management:** Implementar um coletor de lixo não-bloqueante baseado em contagem de referência atômica (ARC - Atomic Reference Counting), focado no ciclo de vida de estruturas de dados puras.
*   **Action Scheduler (Green Threads):** Desenvolver um escalonador M:N cooperativo que executará as `Actions` isoladamente.
*   **Channels & Mailboxes:** Implementar a infraestrutura de fila de mensagens sem *locks* clássicos, baseada no modelo Actor, garantindo que as `Actions` só se comuniquem enviando "Cópias Virtuais" seguras.
*   **Memoization Cache (LRU):** Construir a fundação de estado interno do Runtime responsável pelo *cache* de funções puras (`@cache_strategy`).

## 3. Scope & Requirements

### 3.1. Kata Memory Manager (KMM) - ARC
Como o Tipo de Dado em Kata é garantidamente Acíclico e Imutável (imposto pela Fase 2), evitamos a necessidade de um *Tracer/Mark-and-Sweep*.
*   **Req 3.1.1:** Desenvolver a estrutura base `KataRc<T>`, um ponteiro de contagem de referência thread-safe (similar ao `std::sync::Arc` do Rust).
*   **Req 3.1.2:** Implementar *Copy-on-Write* (CoW) oculto. Quando uma mutação local simulada (`With`) precisar alterar uma "lista", o runtime deve tentar reciclar o buffer se o contador de referência for 1, ou alocar uma nova cópia se > 1.
*   **Req 3.1.3:** O Memory Manager não deve usar Global Allocators genéricos para tudo. Deve ser utilizado um "Arena Allocator" atrelado ao ciclo de vida de uma Action. Quando a Action morre, seu sub-heap inteiro é destruído instantaneamente.

### 3.2. Kata Scheduler (Actor System)
As `Actions` da linguagem rodam assincronamente como *Goroutines*.
*   **Req 3.2.1:** Construir a abstração de `Task` (Corrotina de baixo nível) que armazena a stack de execução local, o ID da Action e o ponteiro da próxima instrução.
*   **Req 3.2.2:** Escalonador M:N: O Runtime iniciará com *N* Threads do S.O. (Work-stealing thread pool) e escalonará *M* Actions sobre essas threads nativas.
*   **Req 3.2.3:** O escalonador deve forçar um ponto de preempção cooperativo ("yield") em loops ou em operações I/O pesadas das Actions para evitar que uma Action trave a thread do SO.

### 3.3. Isolation Channels (Passagem de Mensagens)
O único meio de comunicação entre as Actions com estado isolado.
*   **Req 3.3.1:** Implementar um *Lock-Free Multi-Producer Single-Consumer (mpsc)* Queue para servir de Mailbox para cada Action.
*   **Req 3.3.2:** A passagem de mensagem (`send`) deve apenas transferir a posse (transferência do ponteiro `KataRc`) do dado imutável entre os Heaps das Actions. Se o dado não for estruturalmente puro (checado na Fase 2), a Fase 4 apenas consome o binário seguro. Nenhuma cópia profunda (`Deep Copy`) de megabytes deve ser feita no Runtime se os dados são intrinsecamente imutáveis.

### 3.4. Cache Automático e Diretivas de Runtime
*   **Req 3.4.1:** Implementar um sistema de Cache LRU Global de Thread-Segura (com shards para evitar contenção de trava).
*   **Req 3.4.2:** Prover ganchos (`extern "C"`) que a Fase 5 usará para injetar argumentos em funções decoradas com `@cache_strategy`, consultando o Cache LRU do Runtime antes de reexecutar a Função Pura.

## 4. Architecture & Implementation Guidelines
*   **Linguagem de Implementação:** O Runtime será escrito em **Rust**, utilizando `no_std` em partes críticas para reduzir a *footprint* do binário (ou std focado em performance via `tokio`/`crossbeam` para o escalonador).
*   **ABI Isolada (Pluggability Absoluta):** O Runtime deve expor **estritamente uma API C ABI (`extern "C"`)** para o código gerado. O compilador (Fase 5) nunca deve assumir detalhes de implementação interna do Runtime (como structs de Rust ou layouts de memória). O compilador apenas emitirá chamadas de sistema virtuais (ex: `kata_rt_spawn_action()`, `kata_rt_send_channel()`, `kata_rt_alloc_rc()`). Isso garante que o Runtime oficial possa ser completamente substituído no futuro (ex: por uma versão mínima para IoT em C puro, ou uma versão rodando dentro do WASM) sem alterar o compilador.
*   **Compilação Estática:** O Runtime deve ser compilado como uma biblioteca estática (`libkataruntime.a` ou `kataruntime.rlib`), que a Fase 5 fará o link (*ld* / LLVM lld) com o código de máquina do usuário.
*   **Observabilidade Oculta:** O Runtime deve possuir contadores atômicos em *Release mode* para que o comando futuro `kata top` possa inspecionar ativamente as filas do escalonador.

## 5. Out of Scope for Phase 4
*   Otimizações matemáticas ou deflorestamento de listas (Feito na Fase 3).
*   Geração de Bytecode ou Código LLVM a partir da IR (Feito na Fase 5).
*   Parser ou resolução de Tipos (Feito na Fase 1 e 2).