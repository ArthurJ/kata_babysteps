use logos::{Lexer, Logos, Span};
use crate::error::KataError;
use std::collections::VecDeque;

#[derive(Logos, Debug, PartialEq, Clone)]
pub enum Token {
    // ---- Tokens Sintéticos de Estrutura ----
    Indent,
    Dedent,

    // ---- Keywords e Operadores Estritos ----
    #[token("data")]
    DataKw,
    #[token("enum")]
    EnumKw,
    #[token("interface")]
    InterfaceKw,
    #[token("implements")]
    ImplementsKw,
    #[token("action")]
    ActionKw,
    #[token("lambda")]
    #[token("λ")] // Alias universal
    LambdaKw,
    #[token("let")]
    LetKw,
    #[token("var")]
    VarKw,
    #[token("with")]
    WithKw,
    #[token("as")]
    AsKw,
    #[token("loop")]
    LoopKw,
    #[token("for")]
    ForKw,
    #[token("match")]
    MatchKw,
    #[token("select!")]
    SelectKw,
    #[token("otherwise:")]
    OtherwiseKw,
    #[token("export")]
    ExportKw,
    #[token("import")]
    ImportKw,

    // ---- Pontuação e Organizadores (Teoria Unificada) ----
    #[token("(")]
    LParen,
    #[token(")")]
    RParen,
    #[token("[")]
    LBracket,
    #[token("]")]
    RBracket,
    #[token("{")]
    LBrace,
    #[token("}")]
    RBrace,
    #[token(";")] // Modificador de Dimensão (Tensores)
    Semicolon,
    #[token("::")] // Assinatura de Tipos
    DoubleColon,
    #[token(":")] // Guardas e List Destructuring (x:xs)
    Colon,
    #[token(",")] // Separador opcional de tuplas
    Comma,
    #[token(".")] // Dot notation / Import
    Dot,
    
    // ---- Operadores Matemáticos, Lógicos e Estruturais ----
    #[token("|>")]
    Pipe,
    #[token("_", priority = 3)]
    Hole,
    #[token("=>")]
    FatArrow,
    #[token("->")]
    Arrow,

    #[token("..")]
    DoubleDot,

    // Operadores do Modelo CSP (Concorrência)
    #[token(">!")] // Send
    SendChan,
    #[token("<!")] // Receive Bloqueante
    RecvChan,
    #[token("<!?")] // Receive Não-Bloqueante
    TryRecvChan,

    // ---- Literals Primários ----
    // Floats requerem '.' e podem conter '_' como separador
    #[regex(r"-?[0-9][0-9_]*\.[0-9][0-9_]*", |lex| {
        let s = lex.slice().replace("_", "");
        s.parse().ok()
    })]
    FloatLiteral(f64),
    
    // Inteiros não têm '.' e podem conter '_' como separador
    #[regex(r"-?[0-9][0-9_]*", |lex| {
        let s = lex.slice().replace("_", "");
        s.parse().ok()
    })]
    IntLiteral(i64),

    // Strings Cegos (Puros)
    #[regex(r#""[^"]*""#, |lex| lex.slice()[1..lex.slice().len()-1].to_string())]
    StringLiteral(String),

    // ---- Annotations (Diretivas) ----
    #[regex(r"@[a-z_]+", |lex| lex.slice()[1..].to_string())]
    Annotation(String),

    // ---- A Regra de Nomenclatura Estrita da Kata-Lang ----
    
    // 1. Interfaces (ALL_CAPS) - Sem minúsculas e deve conter algo, permite sublinhado
    #[regex(r"[A-Z_][A-Z0-9_]*", |lex| lex.slice().to_string())]
    InterfaceIdent(String),
    
    // 2. Tipos de Dados / Enum (CamelCase) - Inicia com Maiúscula e DEVE conter letras minúsculas em algum momento para diferenciar do ALL_CAPS
    #[regex(r"[A-Z][a-zA-Z0-9]*[a-z][a-zA-Z0-9]*", |lex| lex.slice().to_string())]
    TypeIdent(String),
    
    // 3. Funções, Actions, Variáveis (snake_case) - Inicia minúsculo. Se for seguido de `!`, o Regex captura como ActionIdent (abaixo)
    #[regex(r"[a-z][a-z0-9_]*", |lex| lex.slice().to_string())]
    FuncIdent(String),

    // 4. Actions invocáveis (`snake_case!`)
    #[regex(r"[a-z][a-z0-9_]*!", |lex| lex.slice().to_string())]
    ActionIdent(String),

    // 5. Funções Simbólicas e Matemática Arbitrária (SymbolIdent)
    // Permite símbolos como +, -, *, /, =, <, >, !, ?, @, $, %, ^, &, |, ~, etc.
    // Structural tokens (como `=>`, `|>` ou `>!`) processados pelo #[token] tomam precedência e NÃO caem aqui, o que é perfeito!
    #[regex(r"[+\-*/=<>!@$%^&|~?]+", |lex| lex.slice().to_string())]
    SymbolIdent(String),

    // Tratamento de Erros e Linhas
    #[regex(r"\r?\n")]
    Newline,

    #[regex(r"[ \t]+")]
    Whitespace,

    // Comentários do Kata Lang iniciam com #
    #[regex(r"#[^\n]*", logos::skip)]
    Comment,
}

pub struct KataLexer<'a> {
    inner: Lexer<'a, Token>,
    indent_stack: Vec<usize>,
    paren_count: usize,
    is_start_of_line: bool,
    pending_tokens: VecDeque<Result<(Token, Span), KataError>>,
    eof_reached: bool,
}

impl<'a> KataLexer<'a> {
    pub fn new(source: &'a str) -> Self {
        Self {
            inner: Token::lexer(source),
            indent_stack: vec![0],
            paren_count: 0,
            is_start_of_line: true,
            pending_tokens: VecDeque::new(),
            eof_reached: false,
        }
    }

    fn handle_indentation(&mut self, text: &str, span: &Span) -> Result<(), KataError> {
        if text.contains(' ') && text.contains('\t') {
            return Err(KataError::MixedIndentation { span: (span.start, span.end) });
        }
        
        // Count characters for absolute depth
        let current_indent = text.chars().count();
        let last_indent = *self.indent_stack.last().unwrap();

        if current_indent > last_indent {
            self.indent_stack.push(current_indent);
            self.pending_tokens.push_back(Ok((Token::Indent, span.clone())));
        } else if current_indent < last_indent {
            while let Some(&top) = self.indent_stack.last() {
                if top > current_indent {
                    self.indent_stack.pop();
                    self.pending_tokens.push_back(Ok((Token::Dedent, span.clone())));
                } else if top == current_indent {
                    break;
                } else {
                    return Err(KataError::MisalignedIndentation { span: (span.start, span.end) });
                }
            }
        }
        Ok(())
    }
}

impl<'a> Iterator for KataLexer<'a> {
    type Item = Result<(Token, Span), KataError>;

    fn next(&mut self) -> Option<Self::Item> {
        loop {
            // Se temos tokens enfileirados sinteticamente (INDENT, DEDENT) retorne imediatamente
            if let Some(token) = self.pending_tokens.pop_front() {
                return Some(token);
            }

            if self.eof_reached {
                return None;
            }

            let token_res = self.inner.next();
            let span = self.inner.span();

            match token_res {
                Some(Ok(token)) => {
                    // Update Parentehsises tracker
                    match token {
                        Token::LParen | Token::LBracket | Token::LBrace => self.paren_count += 1,
                        Token::RParen | Token::RBracket | Token::RBrace => self.paren_count = self.paren_count.saturating_sub(1),
                        _ => {}
                    }

                    if self.paren_count > 0 {
                        // Quando os parênteses estão abertos ignoramos completamente Newlines e Whitespaces
                        match token {
                            Token::Newline | Token::Whitespace => continue,
                            _ => {
                                self.is_start_of_line = false;
                                return Some(Ok((token, span)));
                            }
                        }
                    }

                    if self.is_start_of_line {
                        match token {
                            Token::Whitespace => {
                                if let Err(e) = self.handle_indentation(self.inner.slice(), &span) {
                                    return Some(Err(e));
                                }
                                self.is_start_of_line = false;
                                // Ignore o Whitespace bruto e processe o próximo (o handle_indentation enfileirou INDENT/DEDENT se necessário)
                                continue;
                            }
                            Token::Newline => {
                                // Linha vazia apenas com newline, ignora e mantém start_of_line
                                continue;
                            }
                            _ => {
                                // Linha começou logo com código (indent = 0)
                                if let Err(e) = self.handle_indentation("", &span) {
                                    return Some(Err(e));
                                }
                                self.is_start_of_line = false;
                                // Enfileira o token atual atrás dos possíveis DEDENTs
                                self.pending_tokens.push_back(Ok((token, span)));
                                continue;
                            }
                        }
                    } else {
                        // Meio da linha
                        match token {
                            Token::Whitespace => continue, // Ignore inline spaces
                            Token::Newline => {
                                self.is_start_of_line = true;
                                return Some(Ok((token, span)));
                            }
                            _ => return Some(Ok((token, span))),
                        }
                    }
                }
                Some(Err(_)) => {
                    let text = self.inner.slice();
                    let c = text.chars().next().unwrap_or('?');
                    return Some(Err(KataError::UnrecognizedToken { c, span: (span.start, span.end) }));
                }
                None => {
                    self.eof_reached = true;
                    // Ao atingir o final, limpe a pilha de indentação inteira
                    let end_span = span.end..span.end;
                    while self.indent_stack.len() > 1 {
                        self.indent_stack.pop();
                        self.pending_tokens.push_back(Ok((Token::Dedent, end_span.clone())));
                    }
                    if let Some(token) = self.pending_tokens.pop_front() {
                        return Some(token);
                    } else {
                        return None;
                    }
                }
            }
        }
    }
}
