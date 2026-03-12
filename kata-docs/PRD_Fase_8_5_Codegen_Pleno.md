# PRD: Fase 8.5 - Otimização de Backend, Branching e Alocação Nativa (Codegen Pleno)

## 1. Status Atual do Compilador (O que alcançamos até a Fase 8)
A Kata-Lang atualmente possui uma infraestrutura de **Frontend inabalável**. O compilador escrito em Rust (`kata-cli`) consegue:
- **Lexer e Parser:** Ler código limpo e idiomático (sem parênteses para aridade fixa), reconhecer *Significant Whitespace*, escopos `with`, list destructuring (`x:xs`) e diretivas `@ffi`.
- **Type Checker:** Executar "Gulosidade" baseada na aridade (o motor lê os blocos da esquerda pra direita formando a árvore matemática sem precisar de Pratt Parsing) e inferência de tipos através das lógicas de Hindley-Milner, garantindo Polimorfismo e interfaces `implements`.
- **Middle-End:** A infraestrutura de Representação Intermediária (IR) gera Grafos de bloco estático (SSA), permitindo otimizações poderosas (Constant Folding).
- **Runtime:** A biblioteca em C/Rust (`kata-runtime`) está pronta, abrigando *Zero-Cost ThreadArenas*, Promoção Global (ARC), Canais e Green Threads (via Tokio).
- **Codegen/AOT:** O Cranelift e o GCC foram costurados. A linguagem produz artefatos `.o` e compila binários executáveis em menos de 50 milissegundos.

**O Problema (Por que o FizzBuzz falha em Execução):**
O *Backend* (AOT e Cranelift) na Fase 5 foi mockado para a prova de viabilidade. Ele sabe traduzir expressões planas (`+ 10 5`) para `iadd` em Assembly, mas **não sabe traduzir a lógica de fluxo** (Condicionais, Variáveis Locais, e Alocação na Memória). Quando tentamos rodar o arquivo compilado do `fizzbuzz.kata` nativamente, o SO joga uma `Segmentation Fault` (Crash) porque as instruções aninhadas não emitem o Assembly de controle de pilha ou as invocações de alocação corretas.

## 2. Objetivo da Fase 8.5
Completar a engenharia do `aot_compiler.rs` e do construtor de IR. Ensinaremos ao Cranelift como gerar blocos de pulo (Jump/Brz) para os condicionales Kata (*GuardBlocks*), como invocar as FFIs do runtime para gerenciar ponteiros dinâmicos (Listas e Strings na *ThreadArena*), e como gerenciar escopo léxico na stack nativa da máquina. O sucesso desta fase é ver os testes `fizzbuzz.kata` e `fibonacci.kata` rodando de ponta a ponta nativamente no terminal, sem *crashes*.

## 3. Escopo

**Dentro do Escopo:**
- **Atualização do `src/ir.rs` (Construção do DAG Pleno):**
  - Mapear e implementar os nós `TypedDataExpr::GuardBlock` e `ScopedBlock` dentro da IR. A IR deixará de ser um mock `IntConst(0)` para blocos complexos e criará a malha de Jumps condicionados entre *Basic Blocks*.
  - Garantir o mapeamento do uso das variáveis puras a seus registradores de alocação inicial (`ValueId`).
- **Ampliação do Motor Cranelift (`src/codegen/aot_compiler.rs`):**
  - Ensinar o Cranelift a emitir instruções `brif` e `jump` baseado na árvore de controle de fluxo de blocos gerada pelo *IR Builder*.
  - Ensinar a máquina a inicializar o Ponteiro de Arena Local (`kata_rt_alloc`) na raiz da invocação da `main`, para permitir construtores dinâmicos na memória sem corromper a stack.
  - Implementar o disparo de FFI customizado. No momento, o AOT tenta converter tudo pra I64. O Cranelift deve checar a string FFI e invocar a conversão da convenção C apropriadamente (como I64 para ponteiros `c_char` na string `SHOW str`).

**Fora do Escopo:**
- Otimização extrema de loops vetorizados (SIMD).
- Compilação em JIT (Apenas consolidaremos no motor AOT para produção do binário).

## 4. Requisitos Técnicos e Estrutura de Atuação

Abaixo estão os módulos cruciais onde a nova sessão deverá focar sua implementação e debugar os testes E2E do Cranelift.

### 4.1. Refatoração de Controle de Fluxo (Branching)
Na Kata-Lang, não existe `if/else`, apenas `GuardBlocks`. O Guard se traduz diretamente para a primitiva `brz` (Branch if Zero) no assembly.
```rust
// Modificação em aot_compiler.rs
let condition_val = resolve_val(cond_id, &mut builder);
let true_block = builder.create_block();
let false_block = builder.create_block();

builder.ins().brnz(condition_val, true_block, &[]);
builder.ins().jump(false_block, &[]);
```

### 4.2. Manipulação de Memory Pointers
Strings nativas na Kata são ponteiros puros para bytes UTF-8 em C. O Cranelift deve alocá-las no `.rodata` do arquivo Objeto e passar o ponteiro físico para FFIs como `echo!`.
```rust
// Instanciação de StringLiteral no ObjectBuilder
let data_ctx = DataContext::new();
data_ctx.define(string_bytes.into_boxed_slice());
let data_id = module.declare_data("anon_string", Linkage::Local, false, false)?;
module.define_data(data_id, &data_ctx)?;
```

### 4.3. Call Frame de Actions
Como a `Action` opera uma máquina de estado (ou sequências imperativas cruas), ela exige a construção da sua própria Arena (da Fase 6) se possuir invocações à lista/arrays dinâmicos. Como para o FizzBuzz os ranges são finitos, passaremos por uma conversão de Tupla na Heap antes de instanciar a ThreadArena estrita.

## 5. Critérios de Aceite para a Próxima Sessão

1. A IR `ir.rs` é capaz de representar o Branch de um `Guard` de Fibonacci (n==0, n==1, n) através de Blocos e Jumps, e o Otimizador `Constant Folding` sabe descartar os ramos inacessíveis puros.
2. O Motor do Cranelift `aot_compiler.rs` produz o binário do arquivo `./examples/test_fibonacci.kata` sem erros. Ao executar no terminal, ele não produz Segfault e imprime o número correto no STDOUT nativo.
3. O Motor do Cranelift permite a compilação do `./examples/test_fizzbuzz.kata` com chamadas às FFIs de strings, executando nativamente a lógica de modulo `mod` sem estourar alocação de memória no SO.