# PRD: Fase 8.6 - Limpeza do IR, Fim dos Mockups e Monomorfização Real

## 1. O Problema (A Dívida Técnica Acumulada)
Durante as Fases 4 até a 8.5, o processo de tradução da Árvore Sintática Abstrata (AST) para Assembly (Cranelift/AOT) foi alicerçado sobre atalhos severos (Mocks e Hacks). Para fazer a prova de viabilidade dos binários no menor tempo possível, nós empurramos a responsabilidade da resolução de múltiplos despachos de funções e do gerenciamento de contexto para o final do compilador (Codegen) ou para funções C mockadas. 

Esses atalhos corromperam as tabelas de símbolos (Object files malformados) em cenários reais como o Fibonacci e inviabilizam a generalização para novas estruturas de dados, porque a árvore perde a estabilidade de nomes. Precisamos pagar essa dívida.

## 2. Objetivo da Fase 8.6
Estabelecer um motor de "Monomorfização" real no `Type Checker` e purificar o gerador de Representação Intermediária (`ir.rs`). O compilador deve deixar de usar strings mágicas vazias (`""`) para camuflar nós e deve mapear todas as chamadas de FFI e Polimorfismo sem `if/else` engessados no Cranelift.

O sucesso desta fase é a compilação fluida sem warnings de objetos corrompidos pelo Linker, e a substituição da `kata_rt_mock_list` por uma infraestrutura genérica ou corretamente assinada.

## 3. Escopo

**Dentro do Escopo:**
- **Purificação do IR Builder (`src/ir.rs`):**
  - Remover os blocos de HACK (ex: if manual verificando se a string da call é `"str"` ou `"__list_to_str"`).
  - Parar de instanciar `IRValue::FuncPtr` com strings vazias para Identificadores genéricos de Tipo (`Ident::Type`).
  - Completar a engine matemática do *Constant Folding* (implementar os branches e operadores faltantes como subtração e divisão `TODO: -, / e afins`).
  - Lidar com o binding das variáveis do Pattern Matching de escopos que foi ignorado.

- **Expansão Dinâmica do AOT (`src/codegen/aot_compiler.rs`):**
  - Substituir as injeções manuais de FFI (`if target == "List" { "kata_rt_mock_list" }`) por uma tabela de Importação Baseada na AST. A IR deve carregar a assinatura (O que veio do `@ffi(...)` do `types.kata`) e o Cranelift apenas confia na árvore em vez de ter o roteamento no motor.

- **Refino no Type Checker (`src/type_checker.rs`):**
  - Assumir a responsabilidade pelo "Múltiplo Despacho". Ao invés do `ir.rs` decidir qual FFI C chamar com base num If, o Type Checker detecta o tipo invocado em uma interface e anota na AST a chamada final correta.

**Fora do Escopo:**
- Implementação de um Garbage Collector ou varredura de memórias orfãs (Promotions - reservado para a Fase 9).
- Coleções expansíveis em tempo de execução na RAM, além das imutáveis suportadas nativamente.

## 4. Requisitos Técnicos e Atuação

### 4.1. Limpeza de Identificadores (IR)
Um nó Identificador de Tipo (ex: ao ler a assinatura da `List`) não deve emitir uma constante de ponteiro que não existe, pois isso aciona falha no _ld linker_.
```rust
// Ação necessária no `lower_expr` do ir.rs
TypedDataExpr::Identifier(ident) => {
    // Retornar nós Opacos ou mapear para Tipagem estrutural na IR, em vez de `FuncPtr("")`.
}
```

### 4.2. Eliminação de Mock-Collections no Runtime
As funções `kata_rt_mock_list` e `kata_rt_mock_map` dentro de `collections.rs` quebram para argumentos indefinidos. A `mock_list` assume a injeção exclusiva de Ranges/Tuples. A `mock_map` executa Function Pointers C puros sem controle de ambiente/escopo (Closures) e falha se a função mapeada exigir capturas. 

Essas estruturas passarão por refino para:
1. Obedecer o novo `MakeTuple` funcional ou uma FFI alocadora genérica (`List`).
2. Implementar a passagem correta do bloco de ambiente (Closure Environment) para o iterador de mapeamento (`map`), substituindo a gambiarra do ponteiro estrito.

## 5. Critérios de Aceite para a Próxima Sessão
1. Compilação e linkagem (`cc`) de todos os exemplos (`test_fibonacci.kata` e `test_fizzbuzz.kata`) executada com sucesso absoluto (Exit 0) e sem a corrupção da Tabela de Símbolos do SO.
2. A análise de `grep -r "hack" src/` aponta a remoção e refatoração dos nós de múltiplo despacho artificial do código fonte no `ir.rs`.
3. Os binários resultantes executam a sequência estrita programada sem Crash C (Segmentation Faults) advindos de assinaturas corrompidas.