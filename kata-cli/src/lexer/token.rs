//! Token definitions for Kata Language
//!
//! Tokens are produced by the lexer and consumed by the parser.
//! Operators like +, -, *, / are NOT tokens - they're identifiers
//! defined via interfaces (NUM, ORD, EQ) in the stdlib.

use std::fmt;

/// All tokens in the Kata language
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum Token {
    // === Literals ===
    /// Identifier (includes function names, operators like +, -, etc.)
    /// Action identifiers include trailing ! (e.g., "echo!", "main!")
    Ident(String),

    /// Integer literal as string for later parsing
    /// Supports: decimal ("42"), hex ("0xFF"), binary ("0b1010")
    Int(String),

    /// Float literal as string for later parsing
    /// Supports: decimal ("3.14"), special ("nan", "inf")
    Float(String),

    /// String literal content (without quotes)
    String(String),

    /// Bytes literal content (without b prefix and quotes)
    Bytes(String),

    /// Hole operator for partial application: _
    Hole,

    // === Keywords ===
    // Declarations
    Lambda,      // lambda (also accepts λ)
    Action,      // action
    Data,        // data
    Enum,        // enum
    Interface,   // interface
    Implements,  // implements
    Alias,       // alias (nominal type creation)

    // Module system
    Import,      // import
    Export,      // export

    // Bindings
    Let,         // let
    Var,         // var

    // Control flow (imperative - actions only)
    Match,       // match
    For,         // for
    In,          // in
    Loop,        // loop
    Break,       // break
    Continue,    // continue
    Case,        // case (in select!)
    Select,      // select!
    Timeout,     // timeout!

    // Control flow (functional - pattern matching)
    With,        // with
    Otherwise,   // otherwise
    As,          // as (guards and destructuring)

    // Type system
    Unit,        // Unit (empty type)
    Except,      // except (refined types)

    // Concurrency (CSP)
    Channel,     // channel!
    Queue,       // queue!
    Broadcast,   // broadcast!

    // Indexing
    At,          // at (indexing operator)

    // === Indentation (layout-based syntax) ===
    Indent,      // Increase in indentation
    Dedent,      // Decrease in indentation
    Newline,     // End of line

    // === Operators and Symbols ===
    Semicolon,       // ; (tensor dimension separator)
    Colon,           // :
    DoubleColon,     // ::
    SimpleArrow,     // ->
    Arrow,           // =>
    LParen,          // (
    RParen,          // )
    LBracket,        // [
    RBracket,        // ]
    LBrace,          // {
    RBrace,          // }
    Comma,           // ,
    Dot,             // .
    DotDot,          // ..
    DotDotEqual,     // ..=
    DotDotDot,       // ...
    Pipe,            // |
    Pipeline,        // |>
    Send,            // !>
    Receive,         // <!
    ReceiveNonBlocking, // <!?
    AtSymbol,        // @ (directive)
    Backslash,       // \ (line continuation)
    Question,        // ? (error propagation in actions)
    Dollar,          // $ (explicit application)

    // === End of file ===
    Eof,
}

impl fmt::Display for Token {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Token::Ident(s) => write!(f, "Ident({})", s),
            Token::Int(s) => write!(f, "Int({})", s),
            Token::Float(s) => write!(f, "Float({})", s),
            Token::String(s) => write!(f, "String({:?})", s),
            Token::Bytes(s) => write!(f, "Bytes({:?})", s),
            Token::Hole => write!(f, "_"),
            Token::Lambda => write!(f, "lambda"),
            Token::Action => write!(f, "action"),
            Token::Data => write!(f, "data"),
            Token::Enum => write!(f, "enum"),
            Token::Interface => write!(f, "interface"),
            Token::Implements => write!(f, "implements"),
            Token::Alias => write!(f, "alias"),
            Token::Import => write!(f, "import"),
            Token::Export => write!(f, "export"),
            Token::Let => write!(f, "let"),
            Token::Var => write!(f, "var"),
            Token::Match => write!(f, "match"),
            Token::For => write!(f, "for"),
            Token::In => write!(f, "in"),
            Token::Loop => write!(f, "loop"),
            Token::Break => write!(f, "break"),
            Token::Continue => write!(f, "continue"),
            Token::Case => write!(f, "case"),
            Token::Select => write!(f, "select!"),
            Token::Timeout => write!(f, "timeout!"),
            Token::With => write!(f, "with"),
            Token::Otherwise => write!(f, "otherwise"),
            Token::As => write!(f, "as"),
            Token::Unit => write!(f, "Unit"),
            Token::Except => write!(f, "except"),
            Token::Channel => write!(f, "channel!"),
            Token::Queue => write!(f, "queue!"),
            Token::Broadcast => write!(f, "broadcast!"),
            Token::At => write!(f, "at"),
            Token::Indent => write!(f, "INDENT"),
            Token::Dedent => write!(f, "DEDENT"),
            Token::Newline => write!(f, "\\n"),
            Token::Semicolon => write!(f, ";"),
            Token::Colon => write!(f, ":"),
            Token::DoubleColon => write!(f, "::"),
            Token::SimpleArrow => write!(f, "->"),
            Token::Arrow => write!(f, "=>"),
            Token::LParen => write!(f, "("),
            Token::RParen => write!(f, ")"),
            Token::LBracket => write!(f, "["),
            Token::RBracket => write!(f, "]"),
            Token::LBrace => write!(f, "{{"),
            Token::RBrace => write!(f, "}}"),
            Token::Comma => write!(f, ","),
            Token::Dot => write!(f, "."),
            Token::DotDot => write!(f, ".."),
            Token::DotDotEqual => write!(f, "..="),
            Token::DotDotDot => write!(f, "..."),
            Token::Pipe => write!(f, "|"),
            Token::Pipeline => write!(f, "|>"),
            Token::Send => write!(f, "!>"),
            Token::Receive => write!(f, "<!"),
            Token::ReceiveNonBlocking => write!(f, "<!?"),
            Token::AtSymbol => write!(f, "@"),
            Token::Backslash => write!(f, "\\"),
            Token::Question => write!(f, "?"),
            Token::Dollar => write!(f, "$"),
            Token::Eof => write!(f, "EOF"),
        }
    }
}

/// Span information for a token
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Span {
    pub start: usize,
    pub end: usize,
}

impl Span {
    pub fn new(start: usize, end: usize) -> Self {
        Self { start, end }
    }
}

impl fmt::Display for Span {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}..{}", self.start, self.end)
    }
}

impl From<std::ops::Range<usize>> for Span {
    fn from(range: std::ops::Range<usize>) -> Self {
        Span::new(range.start, range.end)
    }
}

impl From<Span> for std::ops::Range<usize> {
    fn from(span: Span) -> Self {
        span.start..span.end
    }
}

/// A token with its span in the source
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct SpannedToken {
    pub token: Token,
    pub span: Span,
}