# Product Requirements Document (PRD) - Kata Compiler Phase 6: Ecosystem, Tooling & Native StdLib

## 1. Overview
A Fase 6 marca a transição da linguagem Kata de um compilador funcional para um ecossistema de desenvolvimento produtivo. O foco desta fase é a **Experiência do Desenvolvedor (DX)**, fornecendo ferramentas de IDE maduras (Syntax Highlighting, LSP) e a implementação real em memória (Runtime/Cranelift) das estruturas e funções avançadas da Biblioteca Padrão mapeadas na Fase 5.

## 2. Decisão de Arquitetura de Tooling
Seguindo os padrões do Rust (`rust-analyzer`) e TypeScript, o tooling da Kata será dividido em:
1.  **Extensão Leve (VS Code):** Responsável apenas por Syntax Highlighting via TextMate e injeção do LSP.
2.  **Language Server Pesado (`kata-lsp`):** Um binário nativo em Rust, reaproveitando o `kata-front`, responsável por análise semântica, erros em tempo real e autocomplete.

## 3. Goals & Objectives

### 3.1. Suporte à IDE (Syntax Highlighting e LSP)
*   **Kata VS Code Extension:** Criar uma extensão `.vsix` com a gramática TextMate (`kata.tmLanguage.json`) para colorização estática imediata (Keywords, Strings, Identifiers).
*   **Language Server Protocol (LSP):** 
    *   Implementar o crate `kata-lsp` (usando `tower-lsp`).
    *   **Diagnostics:** Reportar os erros de Parse e Type Mismatch (`❌ Erro de Sintaxe`) diretamente como "squiggly lines" (linhas vermelhas) no editor.
    *   **Semantic Tokens:** Substituir a cor básica da IDE pelas cores reais do TypeChecker (diferenciando `Data`, `Action` e `Function`).
    *   **Hover & Go-To:** Mostrar o tipo inferido (`KataType`) ao passar o mouse sobre uma variável.

### 3.2. Implementação da Native Standard Library (Runtime)
Na Fase 5, nós mapeamos as coleções (`List`, `Dict`, `Set`, `TupleRange`) na AST e no TypeChecker. Na Fase 6, precisamos fazer o Backend e o Runtime saberem alocar isso na memória.
*   **Memória Base:** Expandir o Memory Manager (`kata-runtime/src/memory.rs`) para suportar a alocação de blocos (Heaps) para `Dict` (HashMaps) e `List`.
*   **Funções Intrínsecas (FFI):** Escrever as funções nativas em Rust dentro do `kata-runtime` para a matemática (`sin`, `cos`, `tan`, `abs`) e mapeá-las no gerador de código para que o Cranelift saiba chamar o código de máquina do Rust quando o usuário compilar o Kata.
*   **Stream Fusion (Otimizador Avançado):** Processar as chamadas iterativas de coleções da AST (`map`, `filter`) no motor de execução.

### 3.3. O Formatter (`kata fmt`)
*   Como a Kata é baseada em indentação rígida, criar o comando `kata fmt` integrado no CLI.
*   Ele lerá a AST e fará o "un-parse", reescrevendo o arquivo com espaçamentos padronizados.

## 4. Scope & Requirements Breakdown

### Req 1: Editor Extension (VS Code)
*   **1.1:** Arquivo `package.json` definindo a linguagem `kata` e a extensão `.kata`.
*   **1.2:** Gramática cobrindo números flexíveis (`1_000`), ranges (`1..5`), strings interpoladas e keywords.

### Req 2: Kata Language Server (`kata-lsp`)
*   **2.1:** Ciclo de vida: Inicializa quando o VS Code abre, mantém o `TypeEnv` em memória.
*   **2.2:** Sincronização: Escutar mudanças no documento (`textDocument/didChange`), re-rodar o `Parser` e emitir `textDocument/publishDiagnostics`.

### Req 3: Native Runtime Bridge
*   **3.1:** Implementar FFI no Cranelift (`cranelift_backend.rs`) para alocar Coleções Complexas chamando métodos do Runtime.
*   **3.2:** Conectar as funções matemáticas do Rust como chamadas externas válidas no IR da Kata.

## 5. Out of Scope for Phase 6
*   Linguagens de Frontend da Web (WASM).
*   Package Manager Externo.
