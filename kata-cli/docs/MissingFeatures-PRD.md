# PRD: Funcionalidades Faltantes e Refinamentos (Fase 3 / Fase 4)

Este documento descreve as funcionalidades pendentes para a conclusão da Fase 3 (Análise Semântica) e transição para a Fase 4 (Geração de Código) da linguagem Kata.

*Nota: Os problemas anteriores (Literais de Coleção, Ambiguidade de Dois-Pontos e Upcasting de Interfaces/Multiple Dispatch) foram concluídos com sucesso e removidos deste backlog.*

## 1. Concorrência CSP no Type Checker
**Contexto:** O *Parser* já reconhece perfeitamente a sintaxe de canais (`!>`, `<!`, `<!?`) e o bloco multiplexador `select`. No entanto, o *Type Checker* ainda não compreende a semântica de concorrência.
**O que precisa ser feito:**
- **Inferência de Canais:** Ensinar o Type Checker a inferir e validar os tipos `Sender::T` e `Receiver::T`.
- **Validação de Operadores Direcionais:** Garantir que um `!> tx valor` valide se `valor` unifica com o `T` de `Sender::T`. Garantir que `<! rx` retorne o tipo `T` inferido de `Receiver::T`.
- **Bloco `select`:** Tipar os ramos de um `select`, garantindo que todos os fluxos convirjam para um tipo de retorno consistente ou que lidem com os dados extraídos de forma segura.

## 2. Tipos Refinados Avançados e Enums Predicados
**Contexto:** A estrutura base de Tipos Refinados existe na AST, mas a mecânica matemática real que bloqueia estados inválidos na Kata-Lang ainda precisa ser construída no motor de inferência.
**O que precisa ser feito:**
- **Construtores Inteligentes (Smart Constructors):** Garantir que instanciar um Tipo Refinado dinamicamente sempre force o retorno de um tipo `Result::T::Err`.
- **Alarme de Custo via DAG:** Rastrear o *Call Graph* dos predicados. Se o compilador detectar um ciclo recursivo (custo superior a O(1)), ele deve bloquear a compilação a menos que o tipo seja explicitamente anotado com a diretiva `@heavy_predicate`, tornando a degradação de performance visível no código.
- **Degradação e Fallbacks:** Implementar as regras onde a interação matemática entre um Tipo Refinado e um Comum (ex: `PositiveInt - Int`) degrada o resultado para o tipo base (`Int`), exigindo nova validação ou prova lógica.
- **Análise de Inalcançabilidade (Unreachable Pattern):** O compilador deve avaliar a lógica de `enum` predicados (ex: `Magreza(< _ 18.5)`) e emitir erro se uma variante inferior for impossível de ser alcançada devido a um predicado superior mais abrangente.

## 3. Preparação para a Fase 4 (Monomorfização e Codegen)
**Contexto:** O Type Checker atual valida tipos genéricos (ex: `A => A`), mas a geração de binário de custo zero requer a criação de funções concretas especializadas.
**O que precisa ser feito:**
- Criar a infraestrutura de Monomorfização na saída da TAST: o compilador deve identificar todas as instâncias concretas chamadas de uma função genérica (ex: `id_Int`, `id_Float`) e gerar os nós da árvore correspondentes antes de enviar o código para o backend (Cranelift).
