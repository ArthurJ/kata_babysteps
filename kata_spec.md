# Especificação da Linguagem Kata (Draft v2.0)

## 1. Filosofia e Paradigma Arquitetural

A linguagem Kata é projetada para o desenvolvimento de sistemas de alta performance, seguros e concorrentes. O núcleo do design da Kata é a segregação estrita do programa em três conceitos fundamentais:

*   **Data (Dados):** Estruturas puras, imutáveis e **serializáveis por padrão**. Um `Data` é garantidamente uma árvore matemática livre de efeitos (pode ser enviada num JSON ou em cabos de rede). Não possuem comportamento (métodos) atrelados.
*   **Functions (Cálculos):** Transformações puras, determinísticas e referencialmente transparentes. Recebem `Data` e retornam `Data`. Não possuem efeitos colaterais.
*   **Actions (Ações):** O "mundo real" imperativo e a **cápsula de isolamento do Sistema Operacional**. Qualquer *Handle* ou recurso que não seja serializável (ex: Arquivo Aberto, Socket TCP, Mutex, Corrotina) nunca é instanciado como `Data`; ele só existe e trafega preso dentro da Mônada `Action`. Responsáveis por I/O, concorrência, e onde a mutabilidade local explícita é permitida.

## 2. Padrões de Nomenclatura e Sintaxe Base

A Kata impõe padrões estritos de nomenclatura no nível do Lexer/Parser:
*   `ALL_CAPS`: Utilizado exclusivamente para **Interfaces**.
*   `CamelCase`: Utilizado exclusivamente para **Tipos (Data)**. Ex: `Complexo`, `List`, `Array`.
*   `snake_case`: Utilizado para **variáveis, funções e actions**. Ex: `map`, `echo!`.

### 2.1 O Tipo `Unit` e a "Ausência de Valor"
Na matemática estrita da Kata, não existe o conceito de `void` (C/Java). Toda função tem uma entrada e uma saída. Quando não há dados significativos para enviar ou receber, utiliza-se o tipo `Unit`, representado pelo literal tupla vazia `()`.
Ele é vital para assinar *Actions* de efeito colateral puro (ex: `echo! :: Text -> Action::Unit`) ou geradores (ex: `random :: Unit -> Float`).

## 3. Controle de Estado e Execução

A Kata não usa chaves `{}` ou palavras como `begin/end` para blocos lógicos genéricos. O escopo é estritamente ditado pelo **Nível de Indentação Visual** do código. Chaves e colchetes são exclusivos para topologias de memória.

*   `let`: Cria uma ligação de variável **imutável**.
*   `var`: Cria uma variável **mutável**. Restrita ao interior de blocos `action`.
*   `=`: **Não é um operador de atribuição**. É a função pura baseada na Interface `EQ` utilizada para avaliar igualdade estrutural profunda (`= 10 10`).

### 3.1 Filosofia de Execução: Laziness de Compilação vs Eagerness de Runtime
A Kata possui execução **Estrita (Eager)**. Isso garante controle absoluto de uso de memória e CPU `O(N)`, sem pausas do *Garbage Collector*.
No entanto, graças ao **Otimizador DAG**, o compilador aplica *Stream Fusion*. Múltiplos encadeamentos de coleções (`filter` aninhado a `map`) são fundidos antes do programa rodar, eliminando as alocações intermediárias no Heap e gerando um único *for-loop* em assembly nativo.

## 4. O Sistema de Tipos e Coleções (O Layout da Memória)

O Polimorfismo é Paramétrico (Generics) ou Ad-Hoc (Interfaces).
A Kata utiliza limitadores sintáticos nativos (`[ ]` e `{ }`) para explicitar a topologia exata de alocação de memória das coleções no nível de compilação.

### 4.1 Coleções Persistentes (Linked Lists / Heap)
Otimizadas para algoritmos recursivos puramente funcionais, Pattern Matching de cabeças e caudas e imutabilidade de custo zero (Nodes isolados).
*   **Sintaxe:** Colchetes `[ ]`.
*   **Acesso:** Encadeado (`head:tail` via construtor `Cons`).
*   **Tipo Genérico:** `List::T`

```kata
let lista_recursiva [1 2 3]
let nova_lista (Cons 0 lista_recursiva)
```

### 4.2 Arrays Dinâmicos (Memória Contígua / Cache Friendly)
Otimizados para I/O dinâmico, iterações rápidas (`map`/`fold`) e interoperabilidade com sistemas que despejam bytes C-style. Seu tamanho é variável e desconhecido em tempo de compilação.
*   **Sintaxe:** Chaves `{ }` **sem** ponto-e-vírgula de quebra.
*   **Tipo Genérico:** `Array::T`
*   **Restrição:** Arrays são "genéricos", logo, não recebem interfaces de matemática acelerada (Álgebra Linear).

```kata
# Array de tamanho desconhecido lido de um banco
let idades::Array::Int {10 20 30} 
```

### 4.3 Tensores Estáticos N-Dimensionais (SIMD / Álgebra Linear)
Quando as dimensões de um bloco de memória contígua são conhecidas estaticamente, a coleção sobe para o domínio da Família Matemática (`Tensor`). Esse tipo é mapeado diretamente para instruções SIMD (ex: AVX512) no Cranelift/LLVM, sem overhead de verificação no runtime.
*   **Sintaxe:** Chaves `{ }` contendo um promotor **ponto-e-vírgula** `;` para isolar dimensões ou encerrar a linha.
*   **Tipo Genérico:** `Tensor::T::(Shape...)` (Usa Const Generics).

```kata
# 1D: Vetor Linha (Shape 1x3). O ';' final promove de Array para Tensor
let v_linha {1 2 3 ;} 

# 1D: Vetor Coluna (Shape 3x1). Um elemento por linha.
let v_coluna {1 ; 2 ; 3}

# 2D: Matriz Estática (Shape 2x2)
let matriz_identidade::Tensor::Float::(2 2) {
    1.0  0.0 ;
    0.0  1.0
}
```

### 4.4 Coerção de Fronteira Dinâmica para Estática (`Result`)
Para usar os operadores matemáticos (SIMD) em um dado lido dinamicamente de I/O, o desenvolvedor precisa cruzar a "Fronteira Segura". Isso é feito usando o Tipo do Tensor como um construtor de validação que sempre retorna um `Result::T::E`.

```kata
let dados_dinamicos (ler_csv!) # Array::Float

# Valida em runtime se o array tem tamanho 3. Retorna um Result.
let tentativa_segura (Tensor::Float::(3) dados_dinamicos)

lambda (tentativa_segura)
    (Ok tensor): echo! (str (+ tensor tensor)) # Matemática SIMD liberada!
    (Err motivo): echo! "As dimensões do banco falharam."
```

### 4.5 Extração Posicional Randômica (`at`)
Em vez de arrays[0], a Kata utiliza a função nativa `at`, que lida com acessos de forma segura (Boundary Check) retornando sempre `Result`.
*   Acesso 1D: `(at 0 meu_vetor)`
*   Acesso ND: `(at (1 0) minha_matriz)` (Usando Tupla para as coordenadas).

## 5. Interfaces Nativas (Traits e Contratos)

As Interfaces permitem sobrecarga segura. O compilador só aprova a compilação se a operação solicitada estiver garantida por um contrato no `TypeEnv`.
*(A maioria delas já vêm implementadas nativamente no motor de Rust e na StdLib Kata).*

*   **Matemáticas (Operadores Acelerados SIMD):**
    *   `ADD_BEHAVIOR(L, R, Out)`: Destrava operador `+`.
    *   `MUL_BEHAVIOR(L, R, Out)`: Destrava operador `*`. Ex: Multiplicar Tensor(2x3) por Tensor(3x2).
    *   `DOT_BEHAVIOR(L, R, Out)`: Destrava o Produto Escalar `dot`. Exclusivo para Tensores.
*   **Lógicas:**
    *   `EQ(L, R)`: Determina igualdade (`=`).
    *   `ORD(L, R)`: Determina ordenação (`>`, `<`).
*   **Utilitárias de StdLib:**
    *   `SHOW`: Define que o tipo `Data` pode ser stringficado via função `str`.
    *   `ITERABLE`: Contrato que Array, Tensor e List implementam internamente para o encadeamento de fluxos lógicos (`map`, `filter`).

## 6. Actions e Concorrência de Mensagens

As `Actions` dominam os limites imperativos do código. Corrotinas (`spawn!`) e Canais (`channel!`) criam *Green Threads* M:N, e o controle da memória é feito via passagem de mensagens síncronas/assíncronas (Zero-Copy com `KataRc`).

### 6.1 Os Handles Protegidos e a Serialização
Os Tipos Customizados normais da Kata (como `data` e `enum`) não têm permissão para segurar ponteiros de Rede, SO, Threads, ou Mutexes.

Qualquer recurso que pertença ao kernel do sistema **não é retornável como um "Data" no estado global**. Ele vive aprisionado dentro da Mônada *Action*. Isso garante que todas as instâncias de `Data` do programa possam sofrer serialização orgânica (via Interface JSON nativa no futuro) sem vazamentos perigosos e erros de ponteiro morto de C/C++.

```kata
# O FileHandle só existe transitando no mundo Action.
action script_principal
    let handle (open_file! "log.txt") # Retorna um descritor de OS encapsulado
    write! handle "Dado purificado"
```

## 7. Estruturação de Módulos (Visibilidade)

Elementos declarados na Kata são privados (acessíveis apenas no próprio escopo léxico/arquivo) por padrão.
Para exportá-los, a linguagem adota uma abordagem de agregação de fim-de-arquivo, permitindo uma lista de exportações explícita separada das definições.

```kata
# utils/math.kata
data Complex
    real: Float
    imag: Float

soma_complexa :: Complex Complex -> Complex
lambda (a b) ...

# O Export deve ser listado explicitamente no final ou inicio para isolar assinatura
export
    Complex
    soma_complexa
```
*Importando:* `import utils/math as M`.

## 8. Diretivas e Empréstimos Implícitos

As Diretivas (prefixadas com `@`) ditam ordens ao compilador (LLVM/Runtime).

*   `@cache_strategy('lru')`: Aplica Memoização Automática na `Function` do DAG.
*   `@parallel`: Força o Runtime a ignorar o Actor Scheduler leve (Green Threads) e criar uma Thread pesada de Kernel para a rotina `spawn!`.
*   **ARC Elision:** A passagem de tipos por referência funcional em Lambdas **NÃO** dispara incrementos/decrementos atômicos no contador de referências. O ciclo de vida do dado cru é atrelado ao chamador para maximizar a performance, ativando o RC estrito (Heap) apenas na migração em canais ou encerramento de escopo.
