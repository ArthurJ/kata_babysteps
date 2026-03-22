# Fase 3: Kata-Lang Type Checker e TAST — Status Report

## Visão Geral
A Fase 3 foca na transição da AST textual para uma Árvore Sintática Tipada (TAST), garantindo a corretude lógica e a separação de domínios.

## Status de Implementação (Atualizado)

### ✅ Concluído (Done)
1. **Infraestrutura Base:** TAST, Motor HM básico (Inference), e DAG de dependências.
2. **Sistema de Interfaces (Fase Inicial):** 
   - Registro de `InterfaceDef` e `ImplDef` no `Environment`.
   - Implementação da **Orphan Rule** (Regra do Órfão).
   - Validação básica de aridade em contratos de interface.
3. **Refatoração Léxica/Parser:**
   - Suporte a múltiplas cláusulas de lambda e blocos identados.
   - Suporte a diretivas como `@ffi`, `@comutative` e `@predicate` em declarações e membros de interface.
   - Flexibilização de `otherwise:` como fallback universal.
   - Suporte a `import` e `export` em qualquer posição do módulo.
   - **Regras Estritas de Nomenclatura do Domínio Impuro (Bang `!`):** O Parser bloqueia ativamente o uso do sufixo `!` em declarações de Actions, Funções e nomes de variáveis (`let`/`var`). O uso do "bang" fica restrito exclusivamente à invocação/aplicação (ex: `echo!`) e aos operadores de concorrência (`!>`, `<!`).
4. **Generics e Polimorfismo Paramétrico:**
   - Unificação de genéricos (`is_generic_name`, `instantiate_generic`).
   - Busca transitiva em Super-Traits.
   - Testes implementados e passando (`tests/generics_tests.rs`).
5. **Diagnósticos (Spans precisos):**
   - Captura de posições de código no `TypeError` para apontamentos precisos.
   - Testes passando para exibição de variáveis soltas e mismatch de tipos (`tests/diagnostics_tests.rs`).
6. **Tipos Refinados (Fase Inicial):**
   - Inclusão do tipo `Type::Refined` na AST.
   - Regras de unificação e degradação básica já implementadas (`inference.rs` e `checker.rs`).
7. **Validação de Efeitos (Auditoria de Pureza):**
   - Implementado o módulo `effects.rs` que varre a TAST inteira após a tipagem.
   - Previne e lança o erro `ImpureCallInPureContext` caso qualquer invocação a uma função com sufixo `!` ou uso como referência ocorra dentro do corpo de uma Função pura ou lambda.

### 🚧 Em Andamento (In Progress)
1. **Concorrência CSP (Canais e Select):**
   - *Status:* O parser já consegue ler operadores direcionais (`!>`, `<!`) e a estrutura de multiplexação (`select`), mas o Checker os desconhece.
   - *Objetivo:* Ensinar o Type Checker a deduzir tipos de canais (ex: inferir que `Sender::T` restringe as operações de envio) e validar os ramos de um bloco `select`.

### 📋 Pendente (To Do)
1. **Tipos Refinados Avançados:** 
   - Validação de predicados complexos e *Flow Typing* (redução/inferência através de caminhos de controle).

## O Bloqueio da StdLib (Resolvido)
Anteriormente, o relatório apontava a falta de Generics como o impeditivo crítico para validar a biblioteca padrão (`core/types.kata`), dado que o sistema falhava ao unificar tipos de interface (ex: `NUM`) com tipos concretos (ex: `Int`).

Com a entrega do suporte a polimorfismo paramétrico e instanciação genérica no `inference.rs`, o gargalo de tipagem base foi superado. O desafio atual concentra-se estritamente na checagem dos operadores CSP.
