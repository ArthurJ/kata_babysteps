# PRD: Fase 4 - Representação Intermediária (IR), Otimização e REPL JIT

## 1. Objetivo da Fase
Elevar a performance do código e pavimentar o caminho para o Backend e para a execução real dentro do REPL. Esta fase consome a *Typed AST* gerada na Fase 3 e a converte numa Representação Intermediária (IR) Linear baseada num Grafo Acíclico Dirigido (DAG). O principal objetivo é executar as Otimizações de Custo Zero (Zero-Cost Abstractions) prometidas na filosofia da Kata-Lang antes da geração do código de máquina. Adicionalmente, ativaremos um motor de compilação JIT para que o REPL consiga *executar* a matemática inserida, saindo do estado de apenas exibir a AST.

## 2. Escopo

**Dentro do Escopo:**
- **Construção do Módulo de IR (`kata-front/src/ir.rs`):**
  - Conversão de *TypedExpr* para um formato plano, linear e baseado em alocação de valores estáticos (SSA - *Static Single Assignment*).
- **Otimização de Alto Nível (Middle-End):**
  - **Constant Folding:** Avaliação estática de expressões puras compostas exclusivamente de literais (ex: transformar `+ 10 5` num literal `15` em tempo de compilação).
  - **Identificação de Pureza:** Verificação rigorosa para garantir que os *Constant Foldings* só operem no Domínio Funcional puro, não cortando código com possíveis *side effects*.
  - Eliminação de Ramos Mortos Puros (ex: *Guards* cujo `condition` já foi otimizado para `True` estaticamente).
- **REPL Iterativo Executável (Motor JIT Experimental):**
  - Implementação da biblioteca `cranelift-jit` exclusivamente para injetar as funções da `IR` na memória local do processo REPL e colher o resultado nativamente.
  - A execução JIT focará apenas em **Cálculo Puro** (Domínio de Funções). Executar *Actions* (I/O) no JIT do REPL é complexo e deixaremos temporariamente restrito ao comando de CLI `kata run`.

**Fora do Escopo:**
- **Stream Fusion Completo:** O encadeamento de otimizações de listas em memória (fundir mapas e loops dinâmicos) não será feito nesta fase, pois exige o Runtime da Fase 6 e as `Arenas` funcionais para poder medir os buffers.
- **Backend AOT (Ahead-of-Time):** A geração do binário final (`.o` e ELF/MachO) via `cranelift-object` ficará restrita à Fase 5. Esta fase gera apenas o modelo matemático (Middle-end) e o Roda em RAM (JIT para REPL).

## 3. Requisitos Técnicos

### 3.1. Constant Folding e SSA
- A Kata-Lang preza por não causar overhead. Durante o mapeamento da *Typed AST* para IR, qualquer invocação `Call` cujo target seja uma operação intrínseca conhecida (ex: FFI `+`, `-`, `*`) e cujos argumentos sejam nós estritamente literais deve ser resolvida pela máquina do compilador e substituída por um único nó literal equivalente.

### 3.2. REPL JIT Workflow
O fluxo de um comando inserido no REPL mudará drasticamente:
1. Usuário digita: `+ 10 20`
2. Lexer: Tokeniza
3. Parser: `Seq[+, 10, 20]`
4. Type Checker: `Call(+, [10, 20]) -> Type::Int`
5. IR Builder + Optimizer: Aplica o Constant Folding (Resultado: `IR::Literal(Int(30))`).
6. Cranelift JIT: Caso a otimização não consiga resolver estaticamente e reste cálculo matemático dinâmico, o JIT compilará a `IR` para *Assembly* em memória, invocará a função nativa na máquina e resgatará a *struct* binária devolvida na RAM.
7. Print do REPL: Retornará `30 :: Int`.

## 4. Estruturas de Dados Principais

A IR é radicalmente mais simples que a AST. É uma lista de instruções planas focada em registradores temporários.

```rust
// Uma chave referenciando um valor no DAG
pub type ValueId = usize;

#[derive(Debug, Clone)]
pub enum IRInst {
    LoadInt(i64),
    LoadFloat(f64),
    LoadString(String),
    Call {
        target: String, // Nome da função ffi ou compilada
        args: Vec<ValueId>, 
    },
    // Controle de Fluxo reduzido a blocos (CondJump / Jump)
    Branch {
        condition: ValueId,
        true_block: usize,
        false_block: usize,
    }
}

pub struct IRBlock {
    pub instructions: Vec<IRInst>,
}

pub struct IRFunction {
    pub name: String,
    pub return_type: crate::type_checker::Type,
    pub blocks: Vec<IRBlock>,
}
```

## 5. Critérios de Aceite

1. **Construção da IR Linear:** A *TypedAST* de Fibonacci deve ser traduzida numa `IRFunction` sem pânicos, produzindo uma lista linear de `Call`s e `Branch`es.
2. **Otimização Constant Folding:** A *Typed AST* da string `(+ 10 (* 2 5))` deve gerar uma IR contendo estritamente uma única instrução: `LoadInt(20)`. Nenhum *Call* deve chegar à IR final nesse cenário.
3. **Execução Mágica no REPL:**
   - Ao teclar `+ 100 200` no REPL, ele não deve apenas exibir a AST ou IR, ele deve responder com `=> 300`.
   - Se uma função pura com laços não-otimizáveis for escrita no REPL, ela deve ser perfeitamente executada nativamente pelo `cranelift-jit` antes de cuspir o resultado.