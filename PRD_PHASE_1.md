# Product Requirements Document (PRD)
**Projeto:** Compilador da Linguagem Kata
**Fase 1:** Frontend (Sintaxe, Lexer, Parser e Validação Básica)
**Data:** Fevereiro de 2026

---

## 1. Visão Geral e Objetivos
A Fase 1 estabelece as fundações do compilador da linguagem Kata. O objetivo exclusivo desta etapa é ler o código-fonte em texto plano (`.kata`), transformá-lo em uma Árvore Sintática Abstrata (AST) bem tipada em memória, e garantir que as regras estruturais e semânticas básicas (como pureza de escopo e nomenclatura) sejam validadas.

**Fora de Escopo nesta Fase:** Geração de código (LLVM/Inkwell), verificação de tipos profunda (Type Checking de Generics/Interfaces), e execução (Runtime).

## 2. Arquitetura e Stack Tecnológica
*   **Linguagem de Desenvolvimento:** Rust.
*   **Lexer (Tokenizer):** `logos` (Geração de autômato finito via macros para alta performance).
*   **Parser:** Construído à mão (Hand-written Recursive Descent Parser). Necessário para controle absoluto sobre mensagens de erro e implementação da "Lei da Aridade".
*   **Tratamento de Erros (Opcional recomendado):** `miette` ou `ariadne` para exibir diagnósticos visuais no terminal (apontando linha/coluna com trechos de código).

---

## 3. Especificação dos Módulos

### 3.1. Módulo Lexer (`lexer.rs`)
Responsável por converter texto bruto num fluxo (stream) de `Tokens`. O Lexer será um `enum` anotado com macros do `logos`.

**Regras de Reconhecimento Rigorosas (Regex no Logos):**
*   **Tipos de Dados (CamelCase):** `^[A-Z][a-zA-Z0-9]*$` -> `Token::TypeIdent(String)`
*   **Interfaces (ALL_CAPS):** `^[A-Z_][A-Z0-9_]*$` -> `Token::InterfaceIdent(String)`
*   **Variáveis/Funções (snake_case):** `^[a-z_][a-z0-9_]*$` -> `Token::Ident(String)`
*   **Ações (snake_case!):** `^[a-z_][a-z0-9_]*!$` -> `Token::ActionIdent(String)`
*   **Operadores e Símbolos:** `_` (Hole), `|>` (Pipe), `=` (Igualdade Estrutural), `(`, `)`.
*   **Palavras-chave:** `lambda` (ou `λ`), `data`, `action`, `let`, `var`, `with`, `as`.
*   **Diretivas:** `@[a-z_]+` -> `Token::Directive(String)` (O Lexer pega apenas o nome da diretiva. Os argumentos, se houver, são parseados normalmente como tokens de parênteses e strings).
*   **Controle de Fluxo Baseado em Indentação:** Para suportar escopo em blocos, o Lexer não pode descartar as quebras de linha e o nível de alinhamento visual do código (Similar ao Python). O Lexer emitirá tokens `Token::Indent`, `Token::Dedent` e `Token::Newline` com base na consistência de espaços/tabs em cada nova linha.
*   **Descarte:** Espaços em branco inline e comentários (`# ...`).

### 3.2. Módulo AST (`ast.rs`)
Definição das estruturas de dados que representarão o programa na memória. Deve ser projetada utilizando `enums` do Rust.

*Exemplo da topologia esperada:*
```rust
pub enum Expr {
    Literal(LiteralValue),
    Identifier(String),
    Application { func: Box<Expr>, args: Vec<Expr> },
    Lambda { params: Vec<String>, body: Box<Expr>, guards: Option<Vec<Guard>> },
    ImplicitCurry { target: Box<Expr>, provided_args: Vec<Expr>, missing: usize },
    Pipe { left: Box<Expr>, right: Box<Expr> }
}

pub struct DirectiveDef {
    pub name: String,
    pub args: Option<Vec<Expr>>, // Ex: para o ('none') do @cache_strategy
}

pub struct ActionDef {
    pub name: String,
    pub params: Vec<String>, // Ex: (rx_canal id)
    pub body: Vec<Expr>,
    pub directives: Vec<DirectiveDef>,
}
```

### 3.3. Módulo Parser (`parser.rs`)
O coração da Fase 1. Um parser descendente recursivo que consome o iterador de Tokens.

**Desafios Técnicos e Requisitos Críticos:**
1.  **Tabela de Aridade (Arity Registry):** Como a Kata usa notação prefixa pura, o parser precisa saber quantos argumentos uma função consome para construir a AST corretamente. O Parser deve manter um registro em memória (`HashMap<String, usize>`) atualizado à medida que lê as definições de `lambda` e funções nativas (ex: `+` = 2, `map` = 2), para saber quando parar de ser guloso.
2.  **Isolamento por Parênteses (A Lei da Aridade):**
    *   Sempre que o parser encontrar um `(`, ele abre um novo contexto isolado.
    *   Se a função exigir 2 argumentos, mas encontrar um `)` após consumir apenas 1 argumento, o parser **deve** gerar um nó `Expr::ImplicitCurry` na AST.
3.  **Validação do Hole (`_`):**
    *   Se faltarem 2 ou mais argumentos para satisfazer a aridade de uma função dentro de um parêntese e os Holes `_` não estiverem explicitamente desenhados, o parser deve disparar um **Erro de Sintaxe** fatal.
4.  **Desugaring do Pipe (`|>`):** 
    *   Sintaticamente, `a |> b _` deve ser transformado pelo parser diretamente no nó equivalente a `(b a)`. O Pipe não precisa existir como um nó complexo na AST final, ele é apenas "açúcar sintático" para aplicação de função.
5.  **Parsing de Diretivas e Actions:** 
    *   O parser deve permitir que uma declaração `action` seja precedida por múltiplas `DirectiveDef` (ex: `@parallel`).
    *   Diferente da `lambda`, uma `action` define seus argumentos imediatamente após o nome, obrigatoriamente englobados por parênteses: `action worker (rx_canal id)`. O parser deve consumir esse grupo de parênteses e atribuí-los ao campo `params` da `ActionDef`.

### 3.4. Módulo de Validação Semântica Básica (`validator.rs`)
Uma passagem (Visitor) pela AST gerada para aplicar as restrições da especificação que não são puramente sintáticas.

**Regras de Validação Pass-through:**
1.  **Pureza de State:** Um nó `var` (declaração mutável) causará erro de compilação se sua raiz ancestral não for um nó `ActionDef`.
2.  **Pureza de I/O:** Qualquer nó `Application` apontando para um `ActionIdent` (sufixo `!`) causará erro se estiver dentro de uma definição `lambda`.
3.  **Ações Anônimas:** É impossível declarar uma action sem nome (validação de estrutura).
4.  **Diretivas Inválidas:** Aplicar `@cache_strategy` em uma `ActionDef` causará um aviso/erro (só é válido em Funções).

---

## 4. Entregáveis e Critérios de Aceite

### Artefatos
1.  Pacote Rust principal (`kata-front`).
2.  CLI utilitária que permite executar: `kata parse arquivo.kata`.

### Critérios de Aceite (Testes Obrigatórios)
A suíte de testes (`cargo test`) deve cobrir e aprovar os seguintes cenários:
*   [ ] **Sucesso:** Parsing de uma declaração completa de `data` com tipos refinados.
*   [ ] **Sucesso:** Parsing de `lambda` com `Guards` e `with` aninhados.
*   [ ] **Sucesso:** Parsing da expressão `lista |> filter (is_pair) _ |> sum _` resolvendo o currying implícito no `is_pair` com perfeição.
*   [ ] **Falha (Erro Limpo):** Uso de `var` dentro de `lambda`.
*   [ ] **Falha (Erro Limpo):** Chamada de função com múltiplos parâmetros omitidos sem o uso do operador `_`.
*   [ ] **Falha (Erro Limpo):** Variáveis declaradas em CamelCase ou Tipos em snake_case.

## 5. Cronograma Sugerido de Execução
*   **Semana 1:** Configuração do projeto, implementação do Lexer (`logos`) e testes unitários de tokenização.
*   **Semana 2:** Estruturação da AST (`enums` e `structs`) e início do Parser (lógica base e tabela de aridade).
*   **Semana 3:** Implementação avançada do Parser (Lei da Aridade, Parênteses, Currying e Pipe).
*   **Semana 4:** Validação semântica, integração da CLI e polimento das mensagens de erro.
