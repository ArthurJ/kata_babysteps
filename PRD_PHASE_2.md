# Product Requirements Document (PRD)
**Projeto:** Compilador da Linguagem Kata
**Fase 2:** Analisador Semântico e Sistema de Tipos (Type Checker)
**Data:** Fevereiro de 2026

---

## 1. Visão Geral e Objetivos
A Fase 2 expande o Front-end construído na Fase 1. Enquanto a Fase 1 garantiu que o código fonte estivesse sintaticamente correto e formasse uma AST válida, a Fase 2 garantirá a **segurança matemática e estrutural do programa** antes que qualquer otimização ou geração de código (Fase 3) ocorra.

O Type Checker será o módulo mais rigoroso do compilador, atuando como o grande guardião da arquitetura Kata (Data vs Functions vs Actions). Ele deve provar que todas as operações são válidas, que os tipos estruturais encaixam, e que a pureza não está sendo violada.

**Fora de Escopo nesta Fase:** Geração de código (IR/LLVM), Otimizações em AST (Constant Folding, Stream Fusion), Runtime e Execução.

---

## 2. Arquitetura e Stack Tecnológica
*   **Linguagem:** Rust.
*   **Módulo Base:** `type_checker` (interagindo diretamente com a AST da Fase 1).
*   **Algoritmo Principal:** Variação de **Hindley-Milner** misturado com **Type-Checking Bidirecional**, dado que Kata suporta inferência pesada, mas possui anotações de tipo opcionais e refinamentos.
*   **Representação em Memória:** Tabelas de Símbolos (*Symbol Tables* / Ambientes de Tipagem) estruturadas em escopos aninhados, gerenciando variáveis, funções, *Actions*, interfaces e *Data*.

---

## 3. Especificação dos Módulos

### 3.1 Tabela de Símbolos e Ambiente (Environment)
Uma estrutura (`TypeEnv`) capaz de registrar os identificadores e seus tipos descobertos/declarados.
*   **Escopos Aninhados:** Deve suportar entrada e saída de escopos (para `lambda`, blocos `with` e `actions`).
*   **Registro de *Data* e *Interfaces*:** Deve armazenar as assinaturas estruturais dos tipos de dados declarados para validar contratos.

### 3.2 Motor de Inferência (Type Inference Engine)
*   Se o usuário não declarou explicitamente a assinatura de tipo de uma função/variável, o compilador deve deduzir usando as operações internas.
*   Exemplo: Na função `lambda (x y) + x y`, se `+` for restrito a `Int Int -> Int`, inferir que `x` e `y` são `Int`.

### 3.3 Verificação Estrutural e Polimorfismo
*   **Data (Tipos Estruturais):** Não haverá classes, apenas verificação de que um registro contém os campos necessários. O type checker deve garantir que a instanciação ou acesso a campos de um tipo `CamelCase` obedeça à definição original.
*   **Generics (Polimorfismo Paramétrico):** Suportar variáveis de tipo (como `T` em `List::T`). O Type Checker precisará de um mecanismo de unificação/substituição para instanciar funções genéricas quando chamadas.
*   **Interfaces (Polimorfismo Ad-Hoc):** Validar se um tipo genérico obedece às restrições. Se uma função pede `T as |T implements ORD|`, o Type Checker deve garantir que o tipo passado no local de chamada possua as funções exigidas pela interface `ORD`. O motor deve suportar:
    *   **Herança de Interfaces (Super-Traits):** Permitir que uma interface exija outra (ex: `NUM` exige `ADDABLE`), facilitando a inferência de limites inferiores.
    *   **Interfaces Multiparamétricas (Tipos Associados):** Permitir contratos que envolvam múltiplos atores para resolver **sobrecarga de operadores**. Ex: Uma interface `ADD_BEHAVIOR(L, R, O)` permite que o compilador registre e infira que `Int + Float = Float` e `Text + Text = Text`, mas rejeite combinações não registradas sem recorrer à coerção implícita cega.
*   **Tipos de Soma (Algebraic Data Types - ADTs):** Suportar a validação de Enums com carga de dados (ex: `| Falha(Text Int)`). O Type Checker deve realizar **Exhaustiveness Checking** rigoroso, garantindo que as definições de `lambda` via Pattern Matching cubram 100% das variantes do tipo, travando a compilação caso o desenvolvedor esqueça algum caso de borda.

### 3.4 Validador de Tipos Refinados
A linguagem Kata possui tipos refinados utilizando o operador Hole (`_`), ex: `data EstritoPositivo as |Int, > _ 0|`.
*   O Type Checker deve representar esses tipos como um superconjunto lógico: `RefinedType(BaseType, PredicateExpr)`.
*   Nesta fase de compilação estática, ele deve tentar provar refinamentos simples (ex: constante passando para a função). Se o valor vier de *runtime* (ex: I/O), o checker deverá injetar um nó de asserção na AST (para falhar em runtime caso seja inválido) ou exigir que o programador utilize o tipo união `Result`.

### 3.5 Barreira Estrita de Pureza (Purity Checker)
Embora a Fase 1 tenha implementado um bloqueio sintático básico, o Type Checker deve formalizar a **Mônada "Action" (Impureza)** no sistema de tipos.
*   **Functions (Puras):** O tipo inferido nunca pode conter traços de *Actions*.
*   **Concorrência Segura (Canais):** Garantir que canais de concorrência (`channel!`, `<!`, `>!`) manipulem apenas tipos de dados com propriedade (*Ownership*) ou imutáveis puros para que os Heaps de `Actions` diferentes permaneçam isolados.

---

## 4. Estratégia de Testes

Os testes desta fase serão cruciais e deverão validar a rejeição de programas perigosos tanto quanto a aceitação dos corretos.

### 4.1 Testes Unitários de Inferência
*   [ ] **Inferência Direta:** Inferir `Int` de operações aritméticas, `Text` de concatenações ou `str`.
*   [ ] **Tipagem de Lambdas:** Inferir `(A, B) -> A` numa função que retorna o primeiro argumento.
*   [ ] **Resolvendo Generics:** Testar unificação de tipos quando passamos um `Int` para uma função que aceita `T`.

### 4.2 Testes de Verificação Estrutural e Contratos
*   [ ] **Sucesso:** Instanciar e passar um tipo *Data* (ex: `Complexo`) para uma função que exige campos `real` e `imag`.
*   [ ] **Falha de Tipo:** Passar um `Float` onde é esperado um `Int`.
*   [ ] **Falha Estrutural:** Tentativa de acessar um campo que não existe em um *Data*.
*   [ ] **Contrato de Interface:** Chamar função que exige interface `ORD` passando um tipo que não possui o operador `>`.

### 4.3 Testes de Tipos Refinados
*   [ ] **Sucesso Estático:** Passar o literal `10` para uma função que pede `|Int, > _ 0|`.
*   [ ] **Falha Estática:** O Type Checker deve rejeitar no ato da compilação se passarmos literal `-5` para `|Int, > _ 0|`.

### 4.4 Testes de Fronteira de Pureza
*   [ ] **Transbordamento Mutável:** Tentar atribuir um tipo impuro (que só deveria existir em `var` ou `action`) a um tipo puro (`let` de um Data imutável).
*   [ ] **Vazamento de Referência de Canal:** Garantir que o tipo `Canal::T` não possa ser vazado acidentalmente num retorno puro sem ser mapeado em um tipo união seguro.

---

## 5. Cronograma Sugerido de Execução
*   **Epoch 1:** Implementação do `TypeEnv` (Ambiente e Escopos) e infraestrutura base dos tipos na AST. Início do algoritmo de Hindley-Milner.
*   **Epoch 2:** Inferência de tipos de literais, primitivos e unificação de funções base.
*   **Epoch 3:** Implementação de Tipos Estruturais (Data), Generics e Interfaces.
*   **Epoch 4:** Tipos Refinados, segregação avançada de impurezas, e integração do Type Checker ao CLI principal (`kata check`).
