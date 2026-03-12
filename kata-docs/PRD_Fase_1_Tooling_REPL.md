# PRD: Fase 1 - Tooling Base, CLI e REPL (Fundação)

## 1. Objetivo da Fase
Estabelecer a base de infraestrutura do compilador em Rust (`kata-front`). O foco é criar um CLI robusto que servirá como ponto de entrada para todas as futuras funcionalidades do compilador (compilação, execução, testes) e um REPL (Read-Eval-Print Loop) primário para facilitar o ciclo de feedback rápido durante as Fases 2 (Lexer/Parser) e 3 (Type Checker).

## 2. Escopo

**Dentro do Escopo:**
- Configuração do projeto Rust (`kata-front`) com organização de módulos limpa.
- CLI utilizando a biblioteca `clap` para manipulação de argumentos.
- Comandos CLI stubbed (vazios, mas roteados corretamente): `build`, `run`, `test`.
- Comando CLI `repl`: Um shell interativo (utilizando `rustyline` para histórico e navegação).
- Infraestrutura básica de leitura de arquivos `.kata` a partir de um path.
- Tratamento primário de erros de CLI (arquivos não encontrados, argumentos inválidos).
- **Testes de Integração (E2E):** O comando `cargo test` no Rust executará o comando CLI para ler e validar todos os arquivos dentro do diretório `examples/`.

**Decisões Arquiteturais Definidas nesta Fase:**
- **Matemática FFI:** A aritmética básica (como `+`) **não** será processada como mágica no compilador. A StdLib usará a diretiva `@ffi("kata_add_int")` para mapear os operadores aos binários nativos escritos em Rust.

**Fora do Escopo:**
- Qualquer tokenização (Lexer) ou parsing (AST) real de código `.kata`.
- Geração de código ou interpretação matemática.
- O REPL nesta fase atua apenas como um "echo" (imprime de volta o que foi digitado) ou valida comandos internos (como `.exit`).

## 3. Requisitos Técnicos

### 3.1. Estrutura do CLI (`kata-front/src/bin/kata.rs` ou similar)
O binário principal deverá aceitar os seguintes subcomandos:
- `kata build <ENTRY_FILE>`: Analisará o arquivo e suas dependências e (futuramente) gerará um binário nativo.
- `kata run <ENTRY_FILE>`: Atalho para construir e executar o binário imediatamente.
- `kata test [DIRECTORY/FILE]`: Varre o caminho em busca de funções anotadas com `@test` e as executa num ambiente isolado.
- `kata repl`: Inicia o ambiente interativo.

**Profiling Temporal:** Após a execução de qualquer comando CLI (build, run, test), o CLI deve imprimir o tempo total decorrido na saída (ex: `Execução finalizada em 14.5ms`).

### 3.2. Estrutura do REPL (`kata-front/src/repl.rs`)
- Prompt obrigatório: `#--> `.
- Suporte a histórico de comandos (setas para cima/baixo) armazenado temporariamente na sessão.
- Suporte a comandos *meta*:
  - `.exit` ou `.quit`: Encerra o REPL graciosamente.
  - `Ctrl+C` / `Ctrl+D`: Tratamento seguro para interromper a instrução ou sair.
- **Ciclo Básico Inicial:** Lê a entrada do usuário -> [FUTURO: Lexer -> Parser -> TypeCheck] -> [FUTURO: Print AST/Validação] -> Loop.
- **Profiling no REPL:** Assim como o CLI, cada comando digitado e finalizado no REPL deve ser sucedido pelo tempo decorrido da avaliação antes de renderizar o próximo prompt.

## 4. Estruturas de Dados Principais

A Fase 1 foca primariamente em roteamento imperativo, não demandando estruturas de AST completas.

```rust
// Exemplo conceitual para o CLI
enum Command {
    Build { entry: PathBuf },
    Run { entry: PathBuf },
    Test { target: Option<PathBuf> },
    Repl,
}
```

## 5. Critérios de Aceite

1. **Compilação Rust:** O projeto `kata-front` compila sem *warnings* no `cargo build`.
2. **Help do CLI:** A execução de `cargo run -- --help` exibe a descrição clara dos comandos `build`, `run`, `test` e `repl`.
3. **Erros de Path:** O comando `kata build arquivo_inexistente.kata` não deve causar um *panic* do Rust (`unwrap` brusco), mas exibir um erro formatado e amigável ao usuário.
4. **REPL Interativo:** 
   - Ao executar `kata repl`, o prompt `#--> ` é exibido.
   - Digitar qualquer texto e apertar Enter imprime a mensagem "Entrada recebida: ..." sucedida do tempo decorrido (ex: `[0.15ms]`).
   - Digitar `.exit` encerra o processo com código `0`.
5. **Integração de Testes:** A suíte de testes do Rust (em `tests/`) iterará sobre os arquivos em `examples/` invocando a CLI e garantindo que pelo menos o arquivo pode ser lido e aberto em memória antes de relatar sucesso/falha temporal.