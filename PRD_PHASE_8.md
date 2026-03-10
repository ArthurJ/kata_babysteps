# Product Requirements Document (PRD)
**Projeto:** Compilador da Linguagem Kata
**Fase 8:** A Standard Library Escrita em Kata e Motor de Interfaces
**Data:** Março de 2026

---

## 1. Visão Geral e Objetivos
A Fase 8 marca o *bootstrapping* funcional da linguagem Kata: a fundação da Standard Library abandona as simulações e hardcodes escritos em Rust dentro do compilador e passa a ser escrita na própria linguagem Kata. 

O compilador (`kata-front`) carregará um conjunto de arquivos nativos (`core/*.kata`) para inicializar o ambiente de tipos (`TypeEnv`) antes de analisar o programa do usuário.

### 1.1 Impacto Arquitetural no Código Rust
A implementação desta fase **reduzirá drasticamente a complexidade do compilador Rust**. 
Atualmente, o arquivo `std_lib.rs` gasta centenas de linhas montando laboriosamente árvores sintáticas da AST em Rust (ex: `KataType::ParameterizedCustom(...)`) para tentar simular como um Tipo ou uma Lista deveria se comportar.
Com a Fase 8, o `std_lib.rs` será apagado quase que inteiramente. O Rust não precisará mais "saber" o que é um `List`, um `Result` ou um `map`; o Parser da Kata lerá isso de arquivos limpos e fará o trabalho duro usando a sua própria inteligência de Fase 1 e 2. O Rust se tornará um motor estrito que processa as regras universais de inferência, sem viés de negócio.

## 2. Escopo Funcional

### 2.1 O Motor de Importação da StdLib Embutida
*   **Req 1.1 (Virtual File System):** O compilador deverá empacotar (via macro `include_str!` do Rust) a pasta `core/` no binário final.
*   **Req 1.2 (Bootstrap do TypeEnv):** Ao iniciar, o `TypeChecker` fará o parse de `core/types.kata` e `core/math.kata` de forma invisível, populando o escopo global. 
*   **Req 1.3 (Limpeza do Rust):** Deleção completa da criação manual de enums e tipos estruturais do crate `kata-front/src/std_lib.rs`.

### 2.2 Estruturas Nativas Escritas em Kata
O módulo `core/types.kata` implementará puramente os contratos que antes assombravam o Backend.
*   **Req 2.1 (A Coleção List):**
    ```kata
    # Dentro de core/types.kata
    export enum List::T
        | Cons(T List::T)
        | Nil
        
    export map :: (A -> B) List::A -> List::B
    lambda (_ Nil) Nil
    lambda (f (Cons x xs)) (Cons (f x) (map f xs))
    ```
*   **Req 2.2 (Utilitários Universais):** O mesmo processo será feito para `Optional::T` e `Result::T::E`. A partir de agora, falhas da Fase 7 (como a coerção de Tensores) retornarão este `Result` exato escrito em Kata.

### 2.3 Registro de Interfaces (Traits)
A StdLib utilizará a sintaxe de associação de Traits para destrancar a mágica do Backend.
*   **Req 3.1 (`ADD_BEHAVIOR` e `MUL_BEHAVIOR`):** O operador `+` será destrancado globalmente.
    ```kata
    # Dentro de core/math.kata
    export
        (Int, Int, Int) implements ADD_BEHAVIOR
        (Float, Float, Float) implements ADD_BEHAVIOR
        (Tensor::T::(M N), Tensor::T::(N P), Tensor::T::(M P)) implements MUL_BEHAVIOR
    ```

### 2.4 As Novas Interfaces Lógicas do Backend
Essas interfaces existem como "selos de garantia" escritos em Kata, mas a execução profunda dependerá de instruções nativas otimizadas do Cranelift no Rust.
*   **Req 4.1 (`EQ` e `ORD`):** 
    As funções base `= a b` e `< a b` não podem ser recursivas infinitas por razões de performance e *Stack Overflow*. O contrato expõe a assinatura `(= :: L R -> Bool with (L, R) implements EQ)`. O Cranelift no Rust interpretará o `EQ` injetando uma inspeção SIMD de memória ponteiro a ponteiro de alta velocidade.
*   **Req 4.2 (`SHOW` e Stringificação Serializável):** 
    Uma vez que todo `Data` é serializável (livre de I/O), criamos a interface abstrata `SHOW`. O contrato expõe `str :: T -> Text with T implements SHOW`. Quando invocado para *Records* ou Coleções persistentes complexas, o gerador de código desce a árvore serializando em formato de log/JSON, sem que o usuário tenha escrito uma única linha de conversão.

### 2.5 A Válvula de Escape FFI (Foreign Function Interface)
*   **Req 5.1:** Para declarar *Actions* impuras que a matemática da Kata não suporta implementar (acesso de hardware, Kernel, ou indexação O(1) não segura), a `core.kata` permitirá declarar a assinatura base, seguida de uma diretiva de injeção FFI.
    ```kata
    # Dentro de core/io.kata
    @ffi('kata_rt_print')
    export echo! :: Text -> Action::Unit
    ```
    Isso avisa ao `TypeChecker` (Rust) que a função existe e qual é sua pureza, mas joga a responsabilidade de execução para a tabela FFI do compilador gerador de objeto (Fase 5), permitindo que todo o sistema valide o programa limpo sem conhecer o código C/Rust subjacente.

## 3. Fora de Escopo
*   **Serialização de Rede Direta:** O transporte desses dados serializados via TCP/Channels complexos será na Fase 9. Aqui entregamos apenas o mecanismo de representação textual da memória estática via `SHOW`.
*   **Aceleração de GPU Paralela explícita:** O foco do Tensor agora é travar a segurança de dimensões. O mapeamento SIMD de hardware de baixo nível para matrizes gigantes será um *opt-in* de otimização em fases posteriores.
