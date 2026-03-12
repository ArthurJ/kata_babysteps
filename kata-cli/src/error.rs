use miette::Diagnostic;
use thiserror::Error;

#[derive(Error, Debug, Diagnostic, Clone)]
pub enum KataError {
    #[error("O identificador `{ident}` viola a regra de capitalização da Kata-Lang.")]
    #[diagnostic(
        code(kata::lexical::capitalization),
        help("Para funções, ações e var/let use 'snake_case'.\nPara tipos estruturais (Data/Enum) use 'CamelCase'.\nPara interfaces use 'ALL_CAPS'.")
    )]
    CapitalizationViolation {
        ident: String,
        #[label("Identificador mal formatado aqui")]
        span: (usize, usize),
    },

    #[error("Caractere não reconhecido `{c}`.")]
    #[diagnostic(code(kata::lexical::unrecognized))]
    UnrecognizedToken {
        c: char,
        #[label("Símbolo inválido encontrado")]
        span: (usize, usize),
    },

    #[error("Mistura inconsistente de Tabs e Espaços na indentação.")]
    #[diagnostic(
        code(kata::lexical::indentation),
        help("A Kata-Lang exige o uso estritamente uniforme de Espaços OU Tabs num mesmo bloco. Não misture.")
    )]
    MixedIndentation {
        #[label("Inconsistência de indentação aqui")]
        span: (usize, usize),
    },

    #[error("Desalinhamento de indentação.")]
    #[diagnostic(
        code(kata::lexical::misaligned_indentation),
        help("A indentação recuou para um nível que não corresponde a nenhum bloco aberto anteriormente.")
    )]
    MisalignedIndentation {
        #[label("Recuo inválido aqui")]
        span: (usize, usize),
    },

    #[error("Invocação inesperada de Action (`{ident}!`) fora de parênteses.")]
    #[diagnostic(
        code(kata::syntax::action_variadic),
        help("Actions variádicas exigem encapsulamento em parênteses. Ex: (echo! \"A\" \"B\")")
    )]
    ActionWithoutParens {
        ident: String,
        #[label("A chamada a action precisa de parênteses")]
        span: (usize, usize),
    },
    
    #[error("Parêntese de abertura não possui fechamento correspondente.")]
    #[diagnostic(code(kata::syntax::unclosed_paren))]
    UnclosedParen {
        #[label("Parêntese aberto aqui e nunca fechado")]
        span: (usize, usize),
    },

    #[error("Fim Inesperado do Arquivo (EOF)")]
    #[diagnostic(code(kata::syntax::eof))]
    UnexpectedEOF,

    #[error("Token inesperado: {msg}")]
    #[diagnostic(code(kata::syntax::unexpected_token))]
    UnexpectedToken {
        msg: String,
        #[label("Token inválido encontrado aqui")]
        span: (usize, usize),
    },

    // ==========================================
    // ERROS SEMÂNTICOS E DE TIPAGEM (Fase 3)
    // ==========================================

    #[error("Incompatibilidade de Tipo. Esperava `{expected}`, mas encontrou `{found}`.")]
    #[diagnostic(code(kata::type_check::mismatch))]
    TypeMismatch {
        expected: String,
        found: String,
        // (Opcional por agora até termos spans na AST)
        #[label("Erro de tipagem aqui")]
        span: (usize, usize),
    },

    #[error("Símbolo indefinido: `{name}` não foi encontrado no escopo atual.")]
    #[diagnostic(
        code(kata::type_check::undefined_symbol),
        help("Verifique se você importou o módulo ou digitou o nome corretamente.")
    )]
    UndefinedSymbol {
        name: String,
        #[label("Uso de variável/função não declarada")]
        span: (usize, usize),
    },

    #[error("Incompatibilidade de Aridade. A função `{name}` exige {expected} argumentos, mas recebeu {found}.")]
    #[diagnostic(code(kata::type_check::arity_mismatch))]
    ArityMismatch {
        name: String,
        expected: usize,
        found: usize,
        #[label("Quantidade incorreta de argumentos")]
        span: (usize, usize),
    },

    #[error("Violação de Domínio Cruzado (Cross-Domain): {msg}")]
    #[diagnostic(
        code(kata::type_check::cross_domain),
        help("A Kata-Lang proíbe estritamente a chamada de Actions (I/O, mutação) dentro de funções/lambdas puros.")
    )]
    CrossDomainViolation {
        msg: String,
        #[label("Impureza detectada no domínio funcional")]
        span: (usize, usize),
    },

    // ---- Warnings ----
    #[error("Uso de operador simbólico customizado `{ident}`.")]
    #[diagnostic(
        severity(warning),
        code(kata::style::obscure_symbol),
        help("A Kata-Lang permite símbolos arbitrários como funções, mas isso pode prejudicar a legibilidade do código.\nConsidere usar um nome descritivo em 'snake_case' (ex: 'concatenar' em vez de '{ident}').")
    )]
    ObscureSymbolWarning {
        ident: String,
        #[label("Símbolo customizado detectado aqui")]
        span: (usize, usize),
    },
}
