# Especificação Técnica: AST Span Refactoring & Precision Diagnostics

## Status Atual (21/03/2026)
- **Infraestrutura**: `Spanned<T>` implementado em `src/ast/mod.rs`.
- **AST**: `Expr`, `Stmt`, `Pattern`, `TopLevel` e `Type` agora são recursivamente "spanned".
- **Parser**: Todos os módulos em `src/parser/*.rs` foram atualizados para emitir metadados de localização.
- **Type Checker**: Primeira fase de integração concluída; `dummy_span` removido em favor de `node.span`.

## 1. Estrutura de Dados (AST Wrapper)
Utilizamos um invólucro genérico `Spanned<T>` em `src/ast/mod.rs`.

```rust
pub struct Spanned<T> {
    pub node: T,
    pub span: Span,
}
```

**Observação:** Descobrimos que `Pattern` e `Type` também precisam ser `Spanned` para permitir diagnósticos cirúrgicos em desestruturação e assinaturas de tipos.

## 2. Refatoração do Parser
Concluída. O parser agora utiliza `.map_with_span(Spanned::new)` em todas as construções.
- **Desafio**: Manter a legibilidade do código com o aumento da complexidade dos tipos de retorno do `chumsky`.

## 3. Integração com o Type Checker
**Em andamento.**
- [x] Remoção do `dummy_span` em `check_expr` e `check_action_stmt`.
- [x] Propagação de spans reais para `unify`.
- [ ] **Pendente**: Restaurar a lógica completa de `check_lambda` e `resolve_prefix_apply` que foi simplificada durante a migração.
- [ ] **Pendente**: Corrigir chamadas de `instantiate` e unificação de tipos genéricos.

## 4. Estratégia de Estabilização (Novo)
Dada a quebra de 140+ testes, o novo plano é:
1.  **Compilação da Lib**: Garantir que `cargo check --lib` passe sem erros (ajustando `checker.rs` e `inference.rs`).
2.  **Testes de Diagnóstico**: Validar a propagação de spans via `tests/diagnostics_tests.rs`.
3.  **Reparo de Testes Legados**: Atualizar sistematicamente `src/ast/tests.rs` e `src/parser/tests/` usando helpers para criar spans automáticos nos testes.

## Critérios de Conclusão (Revisados)
1. `cargo check --lib` limpo.
2. `tests/diagnostics_tests.rs` passando e comprovando que erros de tipo apontam para o offset correto (não 0:0).
3. Pelo menos 80% dos testes unitários originais restaurados e passando.
