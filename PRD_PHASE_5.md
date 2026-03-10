# Product Requirements Document (PRD) - Kata Compiler Phase 5: Geração de Código & Linkagem Nativa (Backend II)

## 1. Overview
A Fase 5 é o cume do desenvolvimento do Compilador Kata. O objetivo principal desta fase é transformar o Grafo de Representação Intermediária (IR/DAG), previamente otimizado na Fase 3, em código de máquina executável nativo. Além disso, o código gerado deve ser "costurado" (linkado) ao Runtime Embutido (desenvolvido na Fase 4) para fornecer o gerenciamento de memória (ARC) e concorrência (Actor Scheduler) de forma transparente ao usuário.

Esta fase estabelece o pipeline final de build: `Código Kata -> AST -> IR Otimizada -> Código de Máquina -> Binário Final`.

## 2. Decisão de Arquitetura do Backend (Desacoplamento e Múltiplos Motores)
Para garantir que o compilador não fique refém de uma única tecnologia e para contornar problemas de setup em ambientes não-padronizados, a Kata adotará uma **Arquitetura de Geração de Código Desacoplada (Pluggable Codegen)** baseada em Traits (`trait KataCodegenBackend`).

*   **O Motor Definitivo (LLVM):** O objetivo de longo prazo da linguagem é usar o LLVM (via `inkwell`) devido aos seus otimizadores de vetorização (SIMD) agressivos e integração madura.
*   **O Motor Intermediário (Cranelift):** Como o Cranelift é puramente nativo em Rust, compila instantaneamente e não exige bibliotecas C++ pré-instaladas no Sistema Operacional (ex: `llvm-dev`), ele será implementado como o **Backend Primário Inicial** para a Prova de Conceito.
*   **A Abstração:** O módulo da Fase 5 não falará com o LLVM ou Cranelift diretamente no loop principal. Ele definirá uma Interface (ex: `fn emit_add(&mut self, a, b)`) que motores específicos implementarão. Quando mudarmos para LLVM no futuro, o "core" da Fase 5 não será alterado, apenas um novo arquivo `llvm_backend.rs` será adicionado e instanciado.

## 3. Goals & Objectives
*   **Geração de LLVM IR:** Mapear cada `IrNode` do DAG da Kata para uma ou mais instruções na Representação Intermediária do LLVM.
*   **Tradução do Memory Manager (KMM):** Implementar o mapeamento seguro da passagem por empréstimo (ARC Elision), garantindo que os blocos estruturais do usuário tenham seu ciclo de vida gerenciado pelo Runtime apenas ao cruzar fronteiras críticas de ciclo de vida (ex: concorrência ou retorno impuro).
*   **Tradução de Actions para Corrotinas:** Compilar os blocos `action` em funções no formato esperado pelo escalonador C-ABI do Runtime (`KataActionFunc`) e emitir chamadas para `kata_rt_spawn_action`.
*   **Heurística de Auto-Caching:** Conforme definido no Roadmap atualizado, analisar o custo do sub-grafo de funções pesadas no momento da geração e envelopá-las automaticamente com chamadas de memoização do Runtime (`LRU`).
*   **Pipeline de Linkagem (Object File Generation):** Configurar o LLVM para emitir um arquivo objeto (`.o` / `.obj`) nativo da plataforma alvo e invocar o *Linker* do sistema (ex: `lld`, `gcc` ou `msvc`) para anexar a `libkataruntime.a` estaticamente.

## 4. Scope & Requirements

### 4.1. Mapeamento de Tipos e Estruturas (Type Lowering)
O Backend LLVM precisa saber o tamanho e o formato (layout) dos tipos puros aprovados pela Fase 2 (Type Checker).
*   **Req 4.1.1:** Mapear os tipos primitivos da Kata (`Int` -> `i64`, `Float` -> `double`, `Bool` -> `i1`).
*   **Req 4.1.2:** Mapear Sum Types (ADTs) e Product Types para structs C-compatíveis no LLVM (ex: uma struct contendo uma *Tag* de identificação e uma *Union* com os dados da variante).
*   **Req 4.1.3:** Todos os tipos complexos (listas, ADTs grandes, strings) devem ser alocados no Heap gerenciado do Runtime, e a LLVM IR deve transitar apenas com ponteiros (`i8*`).

### 4.2. Tradução do DAG de Expressões Puras (IR para LLVM IR)
Percorrer a `KataGraph` (produzida na Fase 3) emitindo código.
*   **Req 4.2.1:** Converter operações matemáticas básicas e literais para `add`, `fmul`, `icmp`, etc. do LLVM.
*   **Req 4.2.2 (TCO):** Traduzir o `IrNode::TcoLoop` para blocos básicos LLVM (`BasicBlock`) interconectados por saltos incondicionais (`br`), atualizando os valores dos argumentos via nós `phi` em vez de criar novos frames de pilha (Stack Frames).
*   **Req 4.2.3 (Short-Circuit Evaluation):** Garantir que ramificações (como pattern matching ou booleanos lógicos) utilizem blocos de controle de fluxo condicional (`cond_br`) para evitar processar o ramo falso.

### 4.3. Interface com o Runtime (The C-ABI Bridge)
O compilador deve emitir as chamadas FFI no momento exato em que a lógica de negócio do usuário exige infraestrutura de sistema.
*   **Req 4.3.1 (Concorrência):** Quando a Kata encontrar `spawn nome_action()`, o LLVM deve construir um struct de contexto com as variáveis capturadas (`Closure Environment`), convertê-lo para `void*`, e chamar `kata_rt_spawn_action(@funcao_compilada, %contexto)`.
*   **Req 4.3.2 (Auto-Caching):** O gerador LLVM deve avaliar a profundidade da AST da função. Se ultrapassar o limiar heurístico, ele deve criar um corpo de função LLVM "wrapper" que busca a chave num Hash e, em caso de erro (Cache Miss), chama a função real pesada e injeta o resultado com `put()`.

### 4.4. O Processo de Build Estático
*   **Req 4.4.1:** O compilador (`kata build`) deve ser capaz de produzir um arquivo objeto (`.o`).
*   **Req 4.4.2:** O compilador deve invocar automaticamente o Linker nativo da máquina do usuário (usando `std::process::Command`), unindo o arquivo `.o` do usuário com o arquivo compilado da Fase 4 (`kata-runtime.rlib` ou `.a`).
*   **Req 4.4.3:** O binário gerado não deve ter dependências dinâmicas além da libc padrão, facilitando a distribuição (Static Linking).

## 5. Architecture & Implementation Guidelines
*   **Crate `inkwell`:** Será utilizado no repositório `kata-front` para envelopar o LLVM-C de forma idiomática em Rust.
*   **Design Pass-by-Pass:** O módulo `codegen.rs` deve consumir o Grafo Otimizado e possuir um contexto interno (`CodegenContext`) que rastreia os "LLVM Values" gerados para cada "Node ID" do Grafo da Kata.
*   **Debug Information (DWARF):** (Meta Opcional) O compilador LLVM tentará emitir informações de debug atreladas às linhas do arquivo fonte `.kata` original para que depuradores como `gdb` ou `lldb` funcionem nativamente com a Kata.

## 6. Out of Scope for Phase 5
*   Bibliotecas da Standard Library de alto nível (Listas complexas, HTTP, I/O Avançado) (Feito na Fase 6).
*   LSP (Language Server Protocol) e Ferramentas (Feito na Fase 6).
*   Parser, Tipagem ou Otimização Matemática (Feitos nas Fases 1 a 3).