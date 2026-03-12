# PRD: Fase 2 - Lexer e Parser (Análise Léxica e Sintática)

## 1. Objetivo da Fase
Construir a primeira metade do *front-end* do compilador Kata: o Analisador Léxico (Lexer) e o Analisador Sintático (Parser). Esta fase processará arquivos de texto bruto (`.kata`) e os transformará em uma Árvore Sintática Abstrata (AST) estruturada em memória. As regras idiomáticas estritas da Kata-Lang ("Teoria Unificada", *Significant Whitespace*, capitalização rígida) serão aplicadas aqui.

## 2. Escopo

**Dentro do Escopo:**
- **Lexer (Tokenização):**
  - Implementação de um Lexer iterável e não-bloqueante (idealmente usando a biblioteca `logos` para máxima performance).
  - Emissão de tokens sintéticos de bloco: `INDENT` e `DEDENT` controlados por quebra de linha `\n` e contagem de espaços/tabs.
  - Regra de controle multilinha: Desativar a emissão de terminadores de linha/indentação se houverem `(`, `[`, ou `{` abertos.
  - Validação estrita de Identificadores Baseada em Capitalização:
    - Tipos Customizados: `CamelCase` (ex: `Tensor`, `List`).
    - Interfaces: `ALL_CAPS` (ex: `NUM`, `SHOW`).
    - Funções/Actions/Var/Let: `snake_case` (ex: `soma_valores`, `main!`).
  - Identificação de modificadores puros e impuros (sufixo `!`).
  - Extração de textos puros "cegos" (`StringLiteral`).
  
- **Parser (Árvore Sintática):**
  - Definição estrutural da Árvore Sintática Abstrata (AST) em Rust (`enum Expr`, `enum Stmt`).
  - Parsing estrito sem precedência de operadores (notação prefixa). Todo comando inicia-se por um identificador invocável ou primitiva de bloco (`let`, `var`, `loop`, `if` abolido, uso de `match`).
  - Implementação da **"Teoria Unificada"**: Um nó genérico `Tuple(...)` que será classificado no AST como Aplicação de Função, *Currying* implícito ou Tupla literal baseando-se em ser invocado.
  - Parsing de tipos refinados e *pattern matching* destrutivo em Lambdas.

- **Integração REPL/CLI:**
  - O REPL passará a invocar o Lexer e o Parser no input. Se houver erro de sintaxe, exibirá a falha com o fragmento do código (usando `miette` para sublinhar o erro visualmente).
  - Se a sintaxe for válida, o REPL imprimirá a estrutura da AST no terminal (formatação `#![derive(Debug)]`).

**Fora do Escopo:**
- Resolução de Tipos (O Parser apenas empilha nós; ele não sabe se `+ "A" 1` é inválido; isso é papel da Fase 3: Type Checker).
- Execução/Interpretação da AST gerada.

## 3. Requisitos Técnicos

### 3.1. Gestão de Erros (*Diagnostics*)
Todos os erros emitidos nesta fase devem interromper a compilação de forma elegante, retornando a linha e coluna exatas do erro.
- **LexicalError:** Caractere não reconhecido, indentação inconsistente (ex: misturar Tabs e Espaços no mesmo bloco).
- **ParseError:** Fim de arquivo inesperado (EOF), violação da Teoria Unificada (falta de parênteses), anotação de domínio inválida (ex: faltar `!` ao tentar evocar `echo`).

### 3.2. Lexer (Significant Whitespace)
- O Lexer manterá uma pilha (`Stack`) inteira dos níveis de indentação.
- Sempre que a linha começar com mais espaços do que o topo da pilha, emite-se um token `INDENT`.
- Sempre que a linha começar com menos espaços, desempilha-se e emite-se `DEDENT` para cada nível que voltar, finalizando o bloco atual na AST.

### 3.3. Árvore Sintática (AST Base)
O Parser dividirá o mundo em duas raízes hierárquicas claras na AST:
1. `Domain::Function` (Puro, nós associados a `lambda` e expressões funcionais).
2. `Domain::Action` (Impuro, nós associados a `var`, mutação de estado e loops).

## 4. Estruturas de Dados Principais

```rust
// Tokens gerados pelo Lexer
pub enum Token {
    Indent,
    Dedent,
    Newline,
    
    // Identificadores Estritos
    InterfaceIdent(String), // ALL_CAPS
    TypeIdent(String),      // CamelCase
    FuncIdent(String),      // snake_case
    ActionIdent(String),    // snake_case!
    
    // Primitivas Sintáticas
    StringLiteral(String),
    IntLiteral(i64),
    FloatLiteral(f64),
    
    // Símbolos
    LParen, RParen,   // ( )
    LBracket, RBracket, // [ ]
    LBrace, RBrace,   // { }
    Pipe,             // |>
    Hole,             // _
}

// Representação Primária da AST (Omitida a riqueza completa)
pub enum Expr {
    Literal(LiteralValue),
    Identifier(String),
    Tuple(Vec<Expr>),
    Application { target: Box<Expr>, args: Vec<Expr> },
    Lambda { patterns: Vec<Pattern>, body: Box<Expr> },
    ActionBlock(Vec<Stmt>),
}
```

## 5. Critérios de Aceite

1. **Testes do Lexer (Tokenização Simples):**
   - Dada a string `+ 1 1`, emite `[FuncIdent("+"), IntLiteral(1), IntLiteral(1)]`.
   - Rejeição na Tokenização: Identificadores que fujam da regra de capitalização (ex: `minhaFuncao` invés de `minha_funcao`) falham com *LexicalError*.
2. **Testes do Lexer (Significant Whitespace):**
   - Blocos aninhados (`lambda (x)\n    + x 1`) devem emitir o token sintético `INDENT` antes do `+` e um `DEDENT` ao fim da leitura.
   - Parênteses abertos anulam o `INDENT` em linhas subsequentes até seu fechamento.
3. **Testes do Parser:**
   - O *Parser* deve construir a AST de um arquivo `test_fibonacci.kata` completamente (com *Pattern Matching* múltiplo) sem causar *panic*.
   - Invocação variádica de *Action* sem parênteses (ex: `echo! "A" "B"`) emite `ParseError` imediato (exigência da Filosofia 3.3).
4. **Integração REPL (Echo Visual):**
   - Ao executar `kata repl`, digitar `+ 1 1` e dar Enter resultará na impressão em tela: `Application { target: Identifier("+"), args: [Literal(Int(1)), Literal(Int(1))] }`.