#[derive(Debug, Clone, PartialEq)]
pub enum Literal {
    Int(i64),
    Float(f64),
    String(String),
}

/// Identificadores classificados rigorosamente pelo Lexer.
#[derive(Debug, Clone, PartialEq)]
pub enum Ident {
    Interface(String), // ALL_CAPS
    Type(String),      // CamelCase
    Func(String),      // snake_case
    Action(String),    // snake_case!
    Symbol(String),    // Operadores arbitrários (+, *, <=, ++>)
}

/// O Domínio Puro de Dados e Computação. 
/// Uma `DataExpr` nunca altera estado, apenas o lê e computa novos valores.
#[derive(Debug, Clone, PartialEq)]
pub enum DataExpr {
    Literal(Literal),
    Identifier(Ident),
    
    /// A "Teoria Unificada" da Kata-Lang (Pós-Type Check).
    /// Pode ser uma tupla de dados `(1 2 3)`, ou uma aplicação de função resolvida.
    Tuple(Vec<DataExpr>),
    
    /// Uma sequência bruta de expressões na mesma linha ou agrupadas por delimitadores.
    /// Ex: `+ 1 1 / 8 2` vira `Seq([Ident("+"), Int(1), Int(1), Ident("/"), Int(8), Int(2)])`.
    /// O TypeChecker consumirá isso de forma "Gulosa" (Greedy) baseado nas aridades para formar os `Call` e `Tuple` reais.
    Seq(Vec<DataExpr>),
    
    /// Aplicação de Função (Açúcar semântico após a validação da Teoria Unificada pelo Type Checker).
    Call {
        target: Box<DataExpr>, // A função sendo invocada (pode ser um Ident ou um Lambda anônimo retornado)
        args: Vec<DataExpr>,
    },
    
    /// Operador Pipe implícito `|>` (Açúcar sintático para encadeamento de chamadas).
    Pipe {
        left: Box<DataExpr>,
        right: Box<DataExpr>,
    },
    
    /// Lambda (Função Pura).
    /// Pode ter múltiplos corpos de Pattern Matching.
    /// Ex: `lambda (0) 0 \n lambda (1) 1 \n lambda (n) + ...`
    LambdaGroup {
        branches: Vec<LambdaBranch>,
    },

    /// Expressão Condicional Pura baseada em Guards.
    /// `condicao: resultado \n otherwise: fallback`
    GuardBlock {
        branches: Vec<GuardBranch>,
        otherwise: Box<DataExpr>,
    },
    
    /// Bloco de Escopo Funcional (let / as) e restrições (with).
    /// Avaliação Top-Down (`let`) ou Bottom-Up (`with`).
    ScopedBlock {
        bindings: Vec<Binding>,
        body: Box<DataExpr>,
        with_clauses: Vec<Binding>,
    },
}

#[derive(Debug, Clone, PartialEq)]
pub struct LambdaBranch {
    pub params: Vec<Pattern>,
    pub body: DataExpr,
}

#[derive(Debug, Clone, PartialEq)]
pub struct GuardBranch {
    pub condition: DataExpr,
    pub result: DataExpr,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Binding {
    pub pattern: Pattern,
    pub expr: DataExpr,
}

/// Padrões para Desestruturação e Pattern Matching em Lambdas e Matches.
#[derive(Debug, Clone, PartialEq)]
pub enum Pattern {
    Wildcard,           // _ (Ignora o valor)
    Literal(Literal),   // Match exato (ex: `0` no fibonacci)
    Identifier(Ident),  // Captura variável (ex: `n`)
    Tuple(Vec<Pattern>),// Desestruturação Posicional (ex: `(x y)`)
    ListCons {          // Desestruturação de Lista (ex: `(x:xs)`)
        head: Box<Pattern>,
        tail: Box<Pattern>,
    },
}

/// O Domínio Impuro. 
/// Actions manipulam o estado da máquina, canais (CSP) e I/O.
/// É expressamente proibido conter recursão.
#[derive(Debug, Clone, PartialEq)]
pub enum ActionStmt {
    /// Avaliação cega de um dado puro (ex: Instanciar um tensor que não é salvo).
    Expr(DataExpr),
    
    /// Invocação de uma Action variádica ou canais (ex: `echo! "Olá"`, `>! tx valor`).
    ActionCall {
        target: Ident,
        args: Vec<DataExpr>,
    },
    
    /// Associações Locais de Escopo da Action.
    LetBind {
        pattern: Pattern,
        expr: DataExpr,
    },
    VarBind {
        name: Ident, // `var` não suporta destructuring complexo diretamente, apenas rebinding de nome
        expr: DataExpr,
    },
    
    /// Mutação direta de uma variável pré-existente (`var x 10`).
    Assign {
        name: Ident,
        expr: DataExpr,
    },
    
    /// Estruturas de Repetição Imperativas.
    Loop(Vec<ActionStmt>),
    For {
        item: Ident,
        collection: DataExpr,
        body: Vec<ActionStmt>,
    },
    
    /// Desvios de Fluxo em Loops.
    Break,
    Continue,
    
    /// Controle de Fluxo Condicional (Exaustivo).
    Match {
        target: DataExpr,
        arms: Vec<MatchArm>,
    },
    
    /// Retorno de Ação (Implicitamente o último comando de um bloco de Action, ou forçado por `?`).
    Return(DataExpr),
}

#[derive(Debug, Clone, PartialEq)]
pub struct MatchArm {
    pub pattern: Pattern,
    pub block: Vec<ActionStmt>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct TopLevelAttr {
    pub name: String,
    pub args: Vec<String>,
}

/// Declarações Top-Level (Módulo).
#[derive(Debug, Clone, PartialEq)]
pub enum TopLevelDecl {
    /// `import pacote::modulo` ou `import modulo as alias`
    Import {
        path: Vec<Ident>,
        alias: Option<Ident>,
    },
    
    /// `export func_a func_b`
    Export(Vec<Ident>),
    
    /// Definição de Tipos e Interfaces
    DataDef {
        name: Ident,
        fields: Vec<Ident>, // Produto (AND)
    },
    EnumDef {
        name: Ident,
        variants: Vec<Ident>, // Soma (OR) - Versão simplificada da AST
    },
    InterfaceDef {
        name: Ident,
        supertraits: Vec<Ident>,
        signatures: Vec<TypeSignature>,
    },
    
    /// Assinatura de Tipo (ex: `soma :: Int Int => Int`)
    SignatureDecl {
        attrs: Vec<TopLevelAttr>,
        name: Ident,
        sig: TypeSignature,
    },
    
    /// Implementação de Bloco de Módulo (Polimorfismo Top-Level)
    Implements {
        target_type: Ident,
        interface: Ident,
        methods: Vec<TopLevelDecl>, // Declarações recursivas para as funções associadas
    },
    
    /// Associação Constante Top-Level (Pura).
    /// Como funções são identificadores atrelados a Lambdas, elas caem aqui:
    /// `let fibonacci (lambda (n) ...)` -> Ou apenas `fibonacci (lambda ...)`
    Definition {
        name: Ident,
        expr: DataExpr,
    },
    
    /// Definição de uma Action (Impura, Entrypoint do código de I/O)
    /// `action main \n ...`
    ActionDef {
        attrs: Vec<TopLevelAttr>,
        name: Ident,
        params: Vec<Pattern>,
        body: Vec<ActionStmt>,
    },
}

#[derive(Debug, Clone, PartialEq)]
pub struct TypeSignature {
    pub args: Vec<Ident>, // Tipos de Entrada
    pub ret: Ident,       // Tipo de Retorno
}

/// O Nó Raiz da Árvore (O Arquivo .kata)
#[derive(Debug, Clone, PartialEq)]
pub struct ModuleAST {
    pub declarations: Vec<TopLevelDecl>,
}
