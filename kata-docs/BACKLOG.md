# Kata-Lang Backlog

Este documento rastreia recursos, discussões pendentes e futuras melhorias para a arquitetura da linguagem Kata.

## Sistema de Tipos e Domínio Funcional

- **Mecanismo de Propagação de Erros (Result) em Funções Puras:** 
  Atualmente, o operador `?` é exclusivo para *Actions* (domínio imperativo). Isso significa que, no domínio funcional (puro), o encadeamento de múltiplas operações que retornam `Result` (ex: cálculos matemáticos com tipos refinados que podem falhar) exige o uso extensivo de Lambdas de *Pattern Matching* explícitos. 
  **Ação Futura:** Estudar a viabilidade de introduzir um mecanismo monádico de composição (similar ao `bind` / `>>=` de Haskell) ou açúcar sintático compatível com a pureza da linguagem para tornar o encadeamento de `Results` menos verboso em pipelines puros (`|>` ).