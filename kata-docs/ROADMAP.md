# Roadmap de Implementação: Kata-Lang

Este roadmap detalha as fases de desenvolvimento do compilador, tooling e runtime da linguagem Kata, com base nas especificações arquiteturais.

## Fase 1: Tooling Base, CLI e REPL (Fundação)
**Repositório:** `kata-front`

1. **Interface de Linha de Comando (CLI):**
   - Criação do executável base `kata` (comandos `run`, `build`, `test`, `repl`).
2. **Read-Eval-Print Loop (REPL) - Estágio 1:**
   - Loop interativo no terminal (leitura de input contínua, histórico de comandos).
   - No início, funcionará como um "Ecoe" e visualizador de AST para testar as Fases 2 e 3.
3. **Gestão de Workspace:**
   - Resolução de caminhos, leitura de arquivos `.kata` e estruturação básica de logs de compilação.

## Fase 2: Lexer e Parser (Análise Léxica e Sintática)
**Repositório:** `kata-front`

1. **Lexer Base (Tokenização):**
   - Reconhecimento de capitalização e *Significant Whitespace* (tokens `INDENT`/`DEDENT`).
   - Isolamento de *Strings* literais estritas.
   - Suporte a expressões multilinhas dentro de agrupadores `()`, `{}`, `[]`.
2. **Parser de Notação Prefixa e "Teoria Unificada":**
   - Parsing determinístico de argumentos sem precedência.
   - Transformação de `(...)`: tupla, aplicação de função ou currying implícito.
3. **Integração com o REPL:**
   - O REPL passa a imprimir a representação da Árvore Sintática Abstrata (AST).

## Fase 3: Type Checker e Early Checking (Análise Semântica)
**Repositório:** `kata-front`

1. **Ambiente de Tipos (TypeEnv) e Inferência:** Algoritmo de unificação e resolução estrita de domínios (Function vs Action).
2. **Interfaces e Coerência:** Implementações no *top-level*, Herança, Multiple Dispatch e validação da *Orphan Rule*.
3. **Tipos Refinados:** Avaliação estática de construtores e degradação de tipos.
4. **Integração com o REPL:** O REPL passa a validar os tipos em tempo real antes de aceitar a instrução.

## Fase 4: IR, Otimização e REPL JIT
**Repositório:** `kata-front/src/optimizer` e `kata-front/src/codegen`

1. **Grafo Acíclico Dirigido (DAG) e Representação Intermediária (IR):**
   - Conversão da AST tipada em IR.
2. **Otimizações (Zero-Cost):** *Stream Fusion*, *Constant Folding* e avaliação da diretiva `@comptime`.
3. **Integração REPL JIT (Just-In-Time):**
   - Uso do Cranelift JIT para compilar e executar blocos puros da IR em memória, tornando o REPL 100% funcional matematicamente.

## Fase 5: Backend AOT e Geração de Binário
**Repositório:** `kata-front/src/codegen`

1. **Codegen AOT (Ahead-of-Time):**
   - Compilação do domínio funcional para código de máquina na *stack* nativa.
   - Compilação de *Actions* em Máquinas de Estado.
2. **Tree-Shaking:** Eliminação de código inalcançável e blocos `@test` em produção.

## Fase 6: O Runtime Base e Topologia de Memória
**Repositório:** `kata-runtime`

1. **Topologia de Memória Local:** Implementação das *Arenas Zero-Cost* (Bump allocators por Action).
2. **Topologia de Memória Global:** Implementação da *Global Heap* e mecanismo ARC (Atomic Reference Counting) para promoção de variáveis.
3. **Linkagem Estática:** Infraestrutura C/Rust para acoplar as primitivas de memória ao binário gerado na Fase 5.

## Fase 7: Escalonador M:N e Modelo CSP
**Repositório:** `kata-runtime`

1. **Work-Stealing Scheduler:** Motor assíncrono cooperativo (*Green Threads*) para gerenciar a execução de *Actions*.
2. **Modelo CSP (Canais):** Implementação de `channel!` (Rendezvous), `queue!` (Buffer), `broadcast!` (Drop-oldest).
3. **Controle de Fluxo e Resiliência:** Primitiva `select!` (multiplexador) e loop de supervisão para diretivas `@restart`.

## Fase 8: Standard Library e FFI
**Repositórios:** `kata-front` / `kata-runtime`

1. **Estruturas Primitivas:** Tuplas, Lists, Arrays, Tensores.
2. **Bindings FFI (`@ffi`):** Ligações nativas para I/O básico e chamadas de sistema (Sistema de Arquivos, Console).

## Fase 9: Tooling Avançado (LSP e Editores)
**Repositórios:** `kata-lsp` / `kata-vscode`

1. **Language Server Protocol (LSP):**
   - Reutilização do *Parser* e *TypeEnv* para fornecer autocompletar, diagnóstico de erros em tempo real e *hover* de tipos.
2. **Extensão VSCode:** Syntax highlighting oficial e integração com o LSP.