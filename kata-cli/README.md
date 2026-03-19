# Kata CLI

Compilador e REPL para a linguagem Kata - uma linguagem funcional com tipagem estática, inferência de tipos e compilação nativa via Cranelift.

## Status Atual

**Fase 1: Lexer** ✅ Concluído
**Fase 2: Parser** ✅ Concluído

| Componente | Status |
|------------|--------|
| Lexer (tokenização) | ✅ Implementado |
| Indentação significativa | ✅ Implementado |
| Parser | ✅ Implementado |
| Type Checker | 📝 Planejado |
| IR | 📝 Planejado |
| Codegen (Cranelift) | 📝 Planejado |
| REPL | 📝 Planejado |

## Instalação

```bash
cd kata-cli
cargo build --release
```

O binário será gerado em `target/release/kata`.

## Uso Atual

### `--dump-tokens` - Visualizar tokens

```bash
kata --dump-tokens arquivo.kata
```

Processa o arquivo e exibe todos os tokens gerados pelo lexer.

### `--dump-ast` - Visualizar AST (Árvore Sintática)

```bash
kata --dump-ast arquivo.kata
```

Processa o arquivo através do Lexer e do Parser, exibindo a Árvore Sintática Abstrata (AST) final. Isso é excelente para visualizar como o compilador entende a precedência matemática, as estruturas de dados e os blocos de concorrência CSP.

### `--indent` - Processar indentação

```bash
kata --dump-tokens --indent arquivo.kata
```

Inclui tokens `INDENT` e `DEDENT` para linguagem com sintaxe baseada em indentação (similar a Python).

### Exemplos

```bash
# Tokenizar
cargo run -- --dump-tokens examples/test_fibonacci.kata

# Gerar AST
cargo run -- --dump-ast examples/test_fibonacci.kata

# Gerar tokens e AST
cargo run -- --dump-tokens --dump-ast examples/test_fibonacci.kata
```

## Funcionalidades do Lexer e Parser

### Tokens Suportados

| Categoria | Tokens |
|-----------|--------|
| **Identificadores** | `snake_case`, `CamelCase`, `ALL_CAPS`, operadores (`+`, `-`, `*`, etc.), actions (`echo!`) |
| **Literais** | Int (decimal, hex, binary), Float, String (`"..."`, `'...'`), Bytes (`b"..."`) |
| **Keywords** | `lambda`, `action`, `let`, `var`, `data`, `enum`, `match`, `loop`, `for`, `in`, etc. |
| **Estruturas** | `()`, `[]`, `{}`, `::`, `->`, `=>`, `|>`, `!>`, `<!`, `<!?` |
| **Indentação** | `INDENT`, `DEDENT`, `Newline` |
| **Especiais** | `_` (hole), `@` (diretivas), `?` (propagação de erro), `$` (aplicação explícita) |

### Strings

- Aspas duplas: `"hello world"`
- Aspas simples: `'também válido, "aspas" dentro'`
- Escapes: `\\`, `\n`, `\t`, `\r`, `\"`, `\'`, `\0`
- Unicode: `\u{1F600}`

### Números

- Decimal: `42`, `1_000_000`
- Hexadecimal: `0xFF`, `0xCAFE`
- Octal: `0o77`, `0o755`
- Binário: `0b1010`, `0b1100_0011`
- Float: `3.14`, `nan`, `inf`, `-inf`

### Comentários

```kata
# Comentário de linha
let x 42  # comentário inline
```

## Comandos Planejados

### `build` - Compilar um arquivo (planejado)

```bash
kata build arquivo.kata
```

**Opções de debug:**

```bash
# Imprimir tokens (saída do lexer)
kata build --dump-tokens arquivo.kata

# Imprimir AST (saída do parser) - planejado
kata build --dump-ast arquivo.kata

# Imprimir ambos
kata build --dump-tokens --dump-ast arquivo.kata
```

### `run` - Compilar e executar (planejado)

```bash
kata run arquivo.kata
```

Também suportará `--dump-tokens` e `--dump-ast`.

### `repl` - Modo interativo (planejado)

```bash
kata repl
```

**Comandos especiais no REPL:**
- `.env` - Mostra funções e variáveis definidas
- `.clear` - Limpa o ambiente
- `.help` - Mostra ajuda
- `.exit` ou `.quit` - Sai do REPL

## Logging (planejado)

Por padrão, apenas warnings serão mostrados. Para controlar o nível de log:

```bash
# Ver todos os logs de debug
RUST_LOG=debug cargo run -- build arquivo.kata

# Ver apenas info, warnings e erros
RUST_LOG=info cargo run -- build arquivo.kata

# Ver apenas warnings (padrão)
RUST_LOG=warn cargo run -- build arquivo.kata
```

## Estrutura do Projeto

```
src/
├── lib.rs              # Exports públicos
├── main.rs             # CLI entry point
├── lexer/
│   ├── mod.rs          # Exports
│   ├── token.rs        # Token enum + Span
│   ├── lexer.rs        # KataLexer implementation
│   └── error.rs        # LexerError
├── parser/             # (planejado)
├── type_checker/      # (planejado)
├── ir/                 # (planejado)
└── codegen/            # (planejado)
```

## Dependências

- Rust 1.70+
- Cranelift (backend de compilação)
- Chumsky (parser combinator)