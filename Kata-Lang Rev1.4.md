# Especificação da Linguagem Kata (Draft v1.4)

## 1. Filosofia e Paradigma Arquitetural

A linguagem Kata é projetada para o desenvolvimento de sistemas de alta performance, seguros e concorrentes. O núcleo do design da Kata é a segregação estrita do programa em três conceitos fundamentais:

*   **Data (Dados):** Estruturas puras, imutáveis e serializáveis por padrão. Não possuem comportamento (métodos) atrelados.
*   **Functions (Cálculos):** Transformações puras, determinísticas e referencialmente transparentes. Não possuem efeitos colaterais.
*   **Actions (Ações):** O "mundo real" imperativo. Responsáveis por I/O, concorrência, e onde a mutabilidade local explícita é permitida.

## 2. Padrões de Nomenclatura

A Kata impõe padrões estritos de nomenclatura no nível do compilador:
*   `ALL_CAPS`: Utilizado exclusivamente para **Interfaces**.
*   `CamelCase`: Utilizado exclusivamente para **Tipos (Data)**.
*   `snake_case`: Utilizado para **variáveis, funções e actions**.

## 3. Controle de Estado e Igualdade

Para garantir clareza visual, a Kata reserva palavras e símbolos específicos:

*   `let`: Cria uma ligação de variável **imutável**.
*   `var`: Cria uma variável **mutável**. É estritamente restrita ao interior de blocos `action`.
*   `=`: **Não é um operador de atribuição**. É uma função comum de aridade 2 utilizada para avaliar igualdade estrutural (ex: `= 10 10` retorna `True`). 

## 4. O Sistema de Tipos e Interfaces

A Kata possui Tipos Estruturais e Interfaces para contratos. O Polimorfismo é Paramétrico (Generics) ou Ad-Hoc (Interfaces). Não há herança por subtipagem.

### 4.1 Tipos Básicos (Standard Library)
A Kata separa estritamente coleções persistentes de arrays de memória contígua para permitir otimizações de hardware (SIMD) no DAG.

*   **Primitivos:** `Int`, `Float`, `Byte`, `Bool`, `Unit`, `Text`.
    -   O tipo `Text` é UTF-8 nativo e suporta interpolação (ex: `str "Olá #{nome}#"`, com `#{ }#` sendo o local de interpolação).
*   **Coleções Persistentes:** Otimizadas para imutabilidade e compartilhamento estrutural.
    *   `List::T`: Encadeada, otimizada para operações *head/tail*.
    *   `Map::K::V`, `Set::T`.
*   **Arrays Numéricos (Memória Contígua):** Otimizados para SIMD e acesso linear rápido.
    *   `Vector::T` e `Matrix::T`.
*   **Uniões (Tratamento de Erros/Ausência):** Substituem exceções (`try/catch`).
    *   `Optional::T` (Representa presença/ausência).
    *   `Result::T::E` (Representa Sucesso `Ok` ou Falha `Err`).

### 4.2 Definição de Tipos e Interfaces

```kata
# Definição de Data (Estrutural)
data Complexo
  real: Float
  imag: Float

# Tipos Refinados com o Operador Hole (?)
data EstritoPositivo as |Int, > ? 0|

# Generics Inline e Restrições (Interfaces)
max :: T T -> T
lambda (x y)
    > x y: x
    otherwise: y
    with T as |T implements ORD|
```

## 5. Sintaxe Base e Expressividade Funcional

*   **Notação Polonesa (Prefixa):** Funções e operadores precedem os argumentos (ex: `* x y`).
*   **A Palavra Reservada `lambda` (ou `λ`):** É a declaração base de uma função anônima (matemática pura). **Uma função nomeada na Kata é conceitualmente composta por uma ou mais lambdas** associadas a um identificador.
*   **O Operador Hole (`?`):** Utilizado para Currying (aplicação parcial) explícito. Ex: `let add10 (+ 10 ?)`
*   **Pipe Operator (`|>`):** Injeta o valor da esquerda na posição do Hole `?` da expressão à direita.

### 5.1 Pattern Matching vs. Guards

O fluxo de controle na Kata é feito exclusivamente por esses dois mecanismos:

*   **Pattern Matching (O Despacho):** Não realiza computação. Tenta encaixar o dado de entrada em padrões pré-definidos para escolher qual corpo de função (lambda) será executado.
*   **Guards (A Lógica):** Semelhantes a um *switch-case*, operam através de expressões lógicas para escolher a linha a ser executada **dentro de um único lambda**.

```kata
# Exemplo combinando Pattern Matching (múltiplas lambdas)
saudacao :: Text -> Text
lambda ("Alice") "Olá, Chefe!"  # Pattern Match exato
lambda (nome)    concat "Olá " nome # Pattern Match genérico

# Exemplo de Guards (dentro de uma única lambda)
fizzbuzz :: Int Int Int -> Text
lambda (fizz_num buzz_num x) 
  both: "FizzBuzz"           # Guards (avaliados de cima para baixo)
  fizz: "Fizz"
  buzz: "Buzz"
  otherwise: str x
  with                       
    fizz as = (mod x fizz_num) 0
    buzz as = (mod x buzz_num) 0
    both as (and fizz buzz)
```

### 5.2 Closures, Motor de Execução e TCO
*   **Topologia DAG:** A Kata compila Functions em um Grafo Acíclico Dirigido, evitando a criação de *thunks* em tempo de execução.
*   **TCO Estrito:** A Kata **proíbe** chamadas recursivas que não estejam na posição de cauda, garantindo segurança de memória na pilha.
*   **Closures de Valores Imutáveis:** Lambdas anônimas locais podem capturar valores imutáveis (`let`), usando *Reference Counting* (RC/Arc) interno sem criar Garbage Collection. É proibido capturar variáveis mutáveis (`var`).

## 6. Actions: O Mundo Imperativo e Concorrência

As `Actions` são o ponto de orquestração. Qualquer efeito colateral ou I/O **deve** usar o sufixo `!`.
*   **Ausência de Actions Anônimas:** Pela necessidade de rastreabilidade rígida (Logs/Stack Traces) e para evitar *Callback Hell*, a Kata **proíbe Actions anônimas**. Toda Action deve ser nomeada.
*   **Parâmetros de Action:** A passagem de parâmetros para o mundo impuro é feita diretamente na declaração do nome da Action. O uso de `lambda` é restrito à pureza matemática.

```kata
# Mutabilidade e laços de repetição ocorrem apenas dentro de Actions
action servidor_contador
    var conexoes 0               
    while (< conexoes 10)
        let nova_req (listen! 8080) 
        var conexoes (+ conexoes 1)
        processar! nova_req
```

### 6.1 Concorrência Nativa (Canais Isolados)

Heaps são isolados por Action. A comunicação ocorre exclusivamente por canais, transferindo a posse (Ownership) dos dados de forma segura e sem pausas de Garbage Collector.

```kata
# Action recebendo parâmetros de forma direta (Sem uso de lambda)
action worker (rx_canal id)
  let dado (<! rx_canal) # Operador direcional de recebimento
  echo! str "Worker {id} recebeu: {dado}"

action main
  let (tx rx) (channel!)   
  
  # O ambiente impuro encapsulado numa chamada limpa
  spawn! (worker! rx 42)
  
  >! tx "Mensagem Segura"  # Operador direcional de envio
```

## 7. Módulos, Arquivos

*   **Visibilidade:** Elementos são privados por padrão. Elementos públicos devem ser explicitados num bloco `export` isolado.
*   **Importação:** Sintaxe `import Modulo as M`. Ao importar um Tipo de Dado, **todas as interfaces que ele implementa são reconhecidas implicitamente pelo compilador**.
*   **Módulos de Diretório:** O arquivo `mod.kata` atua como o agregador de um diretório.
*   **Entry Point:** Chamadas de Actions soltas na raiz do arquivo só executam se o arquivo for o principal (`__main__`).
