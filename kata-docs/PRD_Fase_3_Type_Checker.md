# PRD: Fase 3 - Type Checker e Early Checking (Análise Semântica)

## 1. Objetivo da Fase
Garantir a integridade matemática e lógica do programa Kata-Lang antes da geração de código. O *Type Checker* (Analisador Semântico) transformará a AST bruta (`DataExpr::Seq`) gerada pela Fase 2 em uma Árvore Tipada (Typed-AST) onde todas as chamadas de função (`Call`), restrições de interfaces (`implements`) e tipos refinados (`except`) estarão validados. Aplicaremos rigorosamente a regra de *Early Checking* em funções genéricas.

## 2. Escopo

**Dentro do Escopo:**
- **Construção do Ambiente de Tipos (TypeEnv):**
  - Registro de todas as declarações Top-Level (`data`, `enum`, `interface`, `action`, assinaturas `::`).
  - Mapeamento das primitivas base (*built-ins*): `Int`, `Float`, `Text`, `List`, `Array`, `Tensor`.
  
- **Mecanismo de "Gulosidade" (Arity-Based Parser):**
  - Transformação do `DataExpr::Seq` (AST Plana) em `DataExpr::Call` e `DataExpr::Tuple` baseando-se estritamente na aridade das funções registradas no TypeEnv.
  - Implementação do Currying Explícito (`_` / Hole) gerando *Closures* de aridade parcial.
  
- **Interfaces e Despacho Múltiplo (Multiple Dispatch):**
  - Validação da declaração `implements` no *Top-Level* (garantindo que todas as assinaturas exigidas pela interface foram preenchidas).
  - Verificação da *Orphan Rule* (Pelo menos o Tipo ou a Interface devem ser declarados no módulo atual).
  - Resolução estática das rotas de despacho (*Monomorfização* teórica).

- **Tipos Refinados e Early Checking:**
  - Garantir a "degradação matemática" (Ex: `PositiveInt` somado a `PositiveInt` resulta em `Int`).
  - Funções genéricas só podem ser processadas se as restrições `with` provarem a existência dos métodos invocados antes da instanciação (*Early Checking*).

- **Integração FFI:**
  - Cadastramento das primitivas matemáticas (como `+` e `-`) como ligações `@ffi` com aridade 2 no TypeEnv inicial.

**Fora do Escopo:**
- Otimização do Grafo de Chamadas (DAG) ou *Stream Fusion* (Fase 4).
- Geração de Código Cranelift/Assembly (Fase 5).
- Avaliação estática `@comptime` de matrizes e literais puros.

## 3. Requisitos Técnicos

### 3.1. O Processo de Checagem (Duas Passagens)
A Kata-Lang requer uma avaliação estrita em duas etapas devido ao seu escopo isolado:
1. **Passagem de Registro (Discovery Pass):** Varre a AST inteira registrando os nomes, assinaturas (`TypeSignature`), `data`, `enum` e `interface` no TypeEnv. Isso permite o uso de funções antes de suas declarações no arquivo.
2. **Passagem de Unificação (Resolution Pass):** Percorre o corpo de cada `action` e `lambda`, convertendo os `Seq` em árvores de chamadas reais e validando a compatibilidade de tipos Hindley-Milner.

### 3.2. A "Teoria Unificada" (Tupla vs Aplicação)
- Quando o *Resolution Pass* encontrar um `Tuple(Vec<Expr>)`, ele avaliará o primeiro item.
- Se o item 0 for um identificador de Função (`FuncIdent`) ou uma closure avaliável, o compilador exige que o número de argumentos a seguir corresponda à aridade dessa função. Se sim, converte-se em `Call`.
- Se sobrarem argumentos não consumidos dentro dos parênteses, é um Erro de Sintaxe (*Arity Mismatch*).
- Se faltarem argumentos, é ativado o *Currying Implícito* (retorna uma nova `Lambda` encapsulando os fornecidos).

## 4. Estruturas de Dados Principais

```rust
// A Tabela Global de Tipos de um Módulo
pub struct TypeEnv {
    pub types: HashMap<String, TypeInfo>,
    pub functions: HashMap<String, FuncSignature>,
    pub interfaces: HashMap<String, InterfaceDef>,
    pub implementations: Vec<TraitImpl>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum Type {
    Int,
    Float,
    Text,
    Bool,
    List(Box<Type>),
    Tensor(Box<Type>, Vec<usize>),
    Func { args: Vec<Type>, ret: Box<Type> },
    Custom(String), // Tipos de Dados / Enums
    Interface(String),
}

#[derive(Debug, Clone)]
pub struct FuncSignature {
    pub name: String,
    pub arity: usize,
    pub args_types: Vec<Type>,
    pub return_type: Type,
    pub is_action: bool,
}
```

## 5. Critérios de Aceite

1. **A Passagem de Aridade (Gulosidade):**
   - Dada a sequência `+ 1 1 / 8 2` gerada na Fase 2, o *Type Checker* deve consultar o TypeEnv (sabendo que `+` e `/` possuem aridade 2) e reestruturar a AST para um bloco contendo duas chamadas isoladas: `Call(+, [1, 1])` e `Call(/, [8, 2])`.
2. **Erro de Incompatibilidade de Tipo (Type Mismatch):**
   - Tentativas de compilar `+ "A" 1` devem ser interceptadas pelo TypeEnv e rejeitadas, emitindo um diagnóstico rico via `miette`: *"Função '+' espera argumentos (NUM, NUM) mas recebeu (Text, Int)."*
3. **Erro de Domínio Cruzado:**
   - Detectar e abortar a compilação caso o programador invoque uma `Action` (`echo!`) dentro de um bloco funcional (`lambda`).
4. **Validação FFI:**
   - As funções nativas devem ser expostas simulando o registro global `@ffi("kata_add_int")` antes da execução dos testes.
5. **Teste E2E do Type Checker:** O comando `kata test` será capaz de processar `examples/test_fibonacci.kata`, passando do Parsing para o Checking e reportando `"Type Check: OK"`.