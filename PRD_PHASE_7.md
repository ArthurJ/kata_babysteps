# Product Requirements Document (PRD)
**Projeto:** Compilador da Linguagem Kata
**Fase 7:** Álgebra Linear, SIMD e Ergonomia de Coleções
**Data:** Março de 2026

---

## 1. Visão Geral e Objetivos
A Fase 7 expande o sistema de tipos e a Standard Library da linguagem Kata para suportar operações matemáticas de alta performance (Álgebra Linear e vetorização SIMD) de forma $100\%$ segura em tempo de compilação. Além disso, resolve inconsistências de nomenclatura e melhora a ergonomia de acesso a coleções.

A filosofia central desta fase é a **Fronteira Explícita**: estruturas dinâmicas de I/O são estritamente separadas de estruturas matemáticas de tamanho estático. O desenvolvedor é responsável por realizar a conversão segura (que retorna um `Result`) antes de ter acesso aos operadores matemáticos acelerados pelo hardware.

## 2. Escopo e Requisitos Funcionais

### 2.1. Acesso Posicional Randômico (`at`)
*   **Req 2.1:** Introduzir a função pura `at` para permitir a extração posicional de elementos em coleções (`Array::T`, `Tensor::T::(Shape)` ou `List::T`), evitando a necessidade de *Pattern Matching* manual quando o índice é conhecido.
*   **Req 2.2 (Unidimensional):** A assinatura para coleções 1D deve ser `at :: Int Colecao -> Result::T::Text`.
*   **Req 2.3 (Multidimensional):** A assinatura para tensores N-dimensionais utilizará uma tupla posicional para as coordenadas: `at :: (Int Int ...) Tensor -> Result::T::Text`.
*   **Comportamento:** Se o índice estiver fora dos limites (Out of Bounds), a função deve retornar a variante `Err` do tipo `Result` contendo a mensagem de falha, garantindo a pureza e segurança sem causar Pânico no Runtime.

### 2.2. Const Generics e Sistema de Tipos
*   **Req 3.1:** O motor de inferência (`KataType` no Type Checker) deve ser expandido para suportar **Const Generics** (Parâmetros Constantes Genéricos).
*   **Req 3.2:** Tipos customizados devem poder carregar Tuplas de números inteiros em sua assinatura de compilação para representar dimensões exatas (Shape). Exemplo na AST interna: `ParameterizedCustom(Name, TypeParams, ShapeTuple)`.

### 2.3. Segregação de Domínios: Coleções Dinâmicas (Array) vs Estáticas (Tensor)
A Standard Library deve definir claramente duas famílias de estruturas de dados:

*   **Família Dinâmica (I/O e Estruturas Flexíveis):**
    *   `Array::T` (Arrays unidimensionais contíguos dinâmicos).
    *   Tamanho e *Shape* desconhecidos em tempo de compilação.
    *   **Restrição:** O Type Checker **NÃO** habilitará Traits matemáticas avançadas (`ADD_BEHAVIOR`, `MUL_BEHAVIOR`, `DOT_BEHAVIOR`) para estes tipos.
*   **Família Matemática Estática (Tensor N-Dimensional):**
    *   `Tensor::T::(Dimensões...)` (Ex: `Tensor::Int::(3)` para Vetor 3D, ou `Tensor::Float::(2 3)` para Matriz 2x3).
    *   **Aceleração:** O Type Checker validará e habilitará operações nativas (`+`, `*`, `dot`) estritamente para essas estruturas baseadas no seu *Shape*. Garantindo que dimensões incompatíveis gerem erro estático (Fase 2) e permitindo que o Backend emita instruções SIMD otimizadas (Fase 5).

### 2.4. Nova Sintaxe Literal (O Layout de Memória)
A Kata utilizará os delimitadores sintáticos para indicar explicitamente a topologia de alocação de memória da coleção:
*   **Colchetes `[ ]`:** Exclusivos para Coleções Persistentes (Linked `List::T`), otimizadas para operações `head:tail` e recursão.
*   **Chaves `{ }`:** Exclusivas para Blocos de **Memória Contígua** (Arrays e Tensores).

*   **Req 4.1 (Array Dinâmico):** Um bloco demarcado por chaves sem o uso de separadores de linha será tipado pelo compilador como um vetor contíguo dinâmico genérico. Exemplo: `{1 2 3}` infere `Array::Int`.
*   **Req 4.2 (O Promotor Tensor `;`):** A presença de um ou mais pontos e vírgulas `;` dentro de um bloco de chaves promoverá imediatamente a estrutura para a Família Matemática Estática (`Tensor`).
*   **Req 4.3 (Tensor 2D+):** O `;` atua como quebra de dimensão. Exemplo: `{1 2 ; 3 4}` informa ao parser o formato bidimensional, inferindo `Tensor::Int::(2 2)`.
*   **Req 4.4 (Vetor Linha Matemático):** Para instanciar um vetor puramente matemático $1 \times N$ (vetor linha) com literais (desbloqueando operações SIMD em tempo de compilação), o programador deve separar os elementos por espaço e incluir um `;` final vazio. Exemplo: `{1 2 3 ;}` infere `Tensor::Int::(1 3)`.
*   **Req 4.5 (Vetor Coluna Matemático):** Para instanciar um vetor puramente matemático $N \times 1$ (vetor coluna), o programador deve quebrar a linha (inserir `;`) após cada elemento isolado. Exemplo: `{1 ; 2 ; 3}` infere `Tensor::Int::(3 1)`.

### 2.5. Coerção Segura via Construtores de Tipo
*   **Req 5.1:** Para cruzar dados da Família Dinâmica (`Array::T`) para a Família Estática (`Tensor::T::(Shape)`), o desenvolvedor utilizará a sintaxe nativa de Construtor de Tipo como função de coerção.
*   **Req 5.2:** A tentativa de coerção `(Tensor::T::(Shape) array_dinamico)` deve verificar as dimensões reais em *runtime* **uma única vez**.
*   **Req 5.3 (Retorno Seguro):** A coerção deve retornar obrigatoriamente um tipo `Result`.
    *   Exemplo de Assinatura: `Tensor::T::(Shape) :: Array::T -> Result::(Tensor::T::(Shape))::Text`.
    *   Se a conversão falhar (tamanho incompatível), retorna a variante `Err` com a mensagem descritiva, forçando o programador a tratar a anomalia via Pattern Matching e evitando falhas não rastreáveis no runtime.

## 3. Interfaces (Traits) de Álgebra Linear
*   **Req 6.1:** As Interfaces existentes `ADD_BEHAVIOR` e `MUL_BEHAVIOR` devem ser registradas no Type Env para abranger unificações algébricas baseadas no *Shape*. Exemplo da regra matemática N-Dimensional: `(Tensor::T::(M N), Tensor::T::(N P)) implements MUL_BEHAVIOR(Tensor::T::(M P))`.
*   **Req 6.2:** Criar a nova interface `DOT_BEHAVIOR(L, R, O)` dedicada ao Produto Escalar (Dot Product) de Tensores 1D e instanciar a função pura `dot` na Standard Library associada a este contrato.

## 4. Exemplos de Uso Idiomático
```kata
# A. Criação Direta (Segurança de Compilação - 100% Estático e Acelerado via SIMD)
let t1 {1 2 3} # Infere: Tensor::Int::(3)
let t2 {4 5 6} # Infere: Tensor::Int::(3)
let v_soma (+ t1 t2) # Validação O(1) no Type Checker. Gera código assembly SIMD (ex: AVX).

# B. Dados Dinâmicos (Conversão Segura via Construtor)
let array_dinamico (ler_banco!) # Tipo: Array::Int
let cast_seguro (Tensor::Int::(3) array_dinamico) # Tenta coagir para um Tensor de Shape (3)

# Pattern Matching obrigatório para desempacotar o Result
lambda (cast_seguro)
    (Ok tensor_estatico): echo! (str (dot tensor_estatico t1))
    (Err erro): echo! "Dimensões incompatíveis."

# C. Acesso Posicional Unificado (Retorna Result)
let m1 {1 2 ; 3 4} # Tensor::Int::(2 2)
let valor_resultante (at (1 0) m1) # Retorna a variante (Ok 3)
```
