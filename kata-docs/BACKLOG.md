# Kata-Lang Backlog

Este documento rastreia recursos, discussões pendentes e futuras melhorias para a arquitetura da linguagem Kata.

## Sistema de Tipos e Coleções

- **Coleções Heterogêneas de Interface (Dynamic Dispatch):**
  Atualmente, o sistema de tipos suporta apenas monomorfização completa (Early Checking). Isso significa que coleções como `List::NUM` onde cada elemento poderia ser `Int`, `Float` ou `Vec2` (todos implementando `NUM`) são impossíveis de representar.

  O enum `Type` atual possui `Interface(String)` que representa a interface em si, mas não um "tipo dinâmico que implementa esta interface". Para suportar isso, seria necessário adicionar algo como `Type::DynamicImpl(String)` ou `Type::TraitObject(String)`.

  **Implicações de Performance:**
  - Monomorfização (atual): Zero overhead, chamadas diretas, inlineável
  - Despacho dinâmico (necessário): Overhead de 1-2 ciclos por chamada, indireção via vtable, impossibilita inlining

  **Decisão Pendente:** Avaliar se o trade-off de performance vale a pena para casos de uso como plugins dinâmicos ou containers heterogêneos. Alternativa: manter o sistema puramente monomorfizado e documentar a limitação.

  **Código de exemplo que falharia hoje:**
  ```kata
  # Impossível: lista com Int e Float juntos
  let valores [1, 2.5, 3] :: List::NUM  # Cada elemento teria vtable diferente
  map (x => + x 1) valores             # Despacho dinâmico necessário
  ```

## Ferramentas e Infraestrutura

- **Integração do Cranelift JIT:**
  O Cranelift possui capacidades JIT (Just-In-Time) que poderiam ser integradas ao runtime para permitir:
  - REPL verdadeiro com compilação incremental
  - Compilação sob demanda de caminhos quentes (tiered compilation)
  - Carregamento dinâmico de plugins com otimização em runtime

  Isso permitiria otimizar automaticamente código dinâmico de volta para forma monomorfizada quando tipos se estabilizam em runtime.
