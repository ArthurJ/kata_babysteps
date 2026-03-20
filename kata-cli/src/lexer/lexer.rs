//! Lexer implementation for Kata Language
//!
//! Uses chumsky for tokenization with support for:
//! - Significant indentation (INDENT/DEDENT) - processed separately
//! - Operators as identifiers (+, -, etc.)
//! - Multiple string types (text, bytes)
//! - Numeric literals (int, float, hex, binary)

use chumsky::prelude::*;

use super::token::{Token, Span, SpannedToken};
use super::error::LexerError;

// ============================================================================
// NUMERIC LITERALS
// ============================================================================

fn decimal_digits() -> impl Parser<char, String, Error = Simple<char>> + Clone {
    filter(|c: &char| c.is_ascii_digit())
        .chain(filter(|c: &char| c.is_ascii_digit() || *c == '_').repeated())
        .collect::<String>()
}

/// Parse the value of an integer (without sign), returns String
fn int_value() -> impl Parser<char, String, Error = Simple<char>> + Clone {
    // Hexadecimal: 0x[0-9a-fA-F_]+
    let hex_int = just("0x")
        .ignore_then(
            filter(|c: &char| c.is_ascii_hexdigit())
                .chain(filter(|c: &char| c.is_ascii_hexdigit() || *c == '_').repeated())
                .collect::<String>()
        )
        .map(|s: String| format!("0x{}", s.replace('_', "")));

    // Octal: 0o[0-7_]+
    let oct_int = just("0o")
        .ignore_then(
            filter(|c: &char| matches!(c, '0'..='7'))
                .chain(filter(|c: &char| matches!(c, '0'..='7') || *c == '_').repeated())
                .collect::<String>()
        )
        .map(|s: String| format!("0o{}", s.replace('_', "")));

    // Binary: 0b[01_]+
    let bin_int = just("0b")
        .ignore_then(
            filter(|c: &char| matches!(c, '0' | '1'))
                .chain(filter(|c: &char| matches!(c, '0' | '1') || *c == '_').repeated())
                .collect::<String>()
        )
        .map(|s: String| format!("0b{}", s.replace('_', "")));

    // Decimal: [0-9][0-9_]*
    let dec_int = decimal_digits()
        .map(|s: String| s.replace('_', ""));

    choice((hex_int, oct_int, bin_int, dec_int))
}

/// Parse integer literals (positive integers)
fn int_literal() -> impl Parser<char, Token, Error = Simple<char>> + Clone {
    int_value().map(Token::Int)
}

/// Parse negative integers: -[0-9]... (without space)
fn negative_int_literal() -> impl Parser<char, Token, Error = Simple<char>> + Clone {
    just('-')
        .ignore_then(int_value())
        .map(|s: String| Token::Int(format!("-{}", s)))
}

/// Parse float value (without sign), returns String
fn float_value() -> impl Parser<char, String, Error = Simple<char>> + Clone {
    // Normal float: [0-9]+\.[0-9]+
    decimal_digits()
        .then_ignore(just('.'))
        .then(decimal_digits())
        .map(|(whole, frac): (String, String)| {
            format!("{}.{}", whole.replace('_', ""), frac.replace('_', ""))
        })
}

/// Parse float literals (positive floats and special values)
fn float_literal() -> impl Parser<char, Token, Error = Simple<char>> + Clone {
    // Normal float
    let float_normal = float_value().map(Token::Float);

    // Special: nan, inf
    let float_nan = just("nan").to(Token::Float("nan".to_string()));
    let float_inf = just("inf").to(Token::Float("inf".to_string()));
    let float_neg_inf = just("-inf").to(Token::Float("-inf".to_string()));

    choice((float_neg_inf, float_normal, float_nan, float_inf))
}

/// Parse negative floats: -[0-9]... (without space)
fn negative_float_literal() -> impl Parser<char, Token, Error = Simple<char>> + Clone {
    just('-')
        .ignore_then(float_value())
        .map(|s: String| Token::Float(format!("-{}", s)))
}

/// Combined parser for all signed numeric literals (negative numbers)
/// Must be tried BEFORE identifier() to prevent '-' from being captured as an operator
fn signed_number_literal() -> impl Parser<char, Token, Error = Simple<char>> + Clone {
    choice((negative_float_literal(), negative_int_literal()))
}

// ============================================================================
// STRING AND BYTES LITERALS
// ============================================================================

fn string_literal() -> impl Parser<char, Token, Error = Simple<char>> + Clone {
    // Escape sequences
    let escape = just('\\')
        .ignore_then(choice((
            just('\\').to('\\'),
            just('n').to('\n'),
            just('t').to('\t'),
            just('r').to('\r'),
            just('"').to('"'),
            just('\'').to('\''),
            just('0').to('\0'),
            // Unicode escape: \u{XXXX}
            just('u').ignore_then(
                just('{')
                    .ignore_then(
                        filter(|c: &char| c.is_ascii_hexdigit())
                            .repeated()
                            .at_least(1)
                            .at_most(6)
                            .collect::<String>()
                    )
                    .then_ignore(just('}'))
                    .map(|hex: String| {
                        u32::from_str_radix(&hex, 16)
                            .ok()
                            .and_then(|code| char::from_u32(code))
                            .unwrap_or('\0')
                    })
            ),
        )));

    // Double-quoted strings
    let double_quoted = just('"')
        .ignore_then(
            escape.clone()
                .or(none_of("\\\""))
                .repeated()
                .collect::<String>()
        )
        .then_ignore(just('"'))
        .map(Token::String);

    // Single-quoted strings (also support escapes)
    let single_quoted = just('\'')
        .ignore_then(
            escape
                .or(none_of("\\'"))
                .repeated()
                .collect::<String>()
        )
        .then_ignore(just('\''))
        .map(Token::String);

    choice((double_quoted, single_quoted))
}

fn bytes_literal() -> impl Parser<char, Token, Error = Simple<char>> + Clone {
    just("b\"")
        .ignore_then(
            none_of("\\\"")
                .repeated()
                .collect::<String>()
        )
        .then_ignore(just('"'))
        .map(Token::Bytes)
}

// ============================================================================
// IDENTIFIERS AND KEYWORDS
// ============================================================================

/// Check if character can start an identifier
fn is_ident_start(c: char) -> bool {
    c.is_ascii_lowercase() || c == '_'
}

/// Check if character can continue an identifier
fn is_ident_char(c: char) -> bool {
    c.is_ascii_alphanumeric() || c == '_'
}

/// Check if character is an operator symbol
fn is_operator_char(c: char) -> bool {
    matches!(c, '+' | '-' | '*' | '/' | '\\' | '=' | '!' | '<' | '>' | '?' | '|' | '&' | '^' | '@' | '$' | '%')
}

/// Convert string to keyword token if applicable
fn str_to_keyword(s: &str) -> Option<Token> {
    match s {
        "lambda" | "λ" => Some(Token::Lambda),
        "action" => Some(Token::Action),
        "data" => Some(Token::Data),
        "enum" => Some(Token::Enum),
        "interface" => Some(Token::Interface),
        "implements" => Some(Token::Implements),
        "alias" => Some(Token::Alias),
        "import" => Some(Token::Import),
        "export" => Some(Token::Export),
        "let" => Some(Token::Let),
        "var" => Some(Token::Var),
        "match" => Some(Token::Match),
        "for" => Some(Token::For),
        "in" => Some(Token::In),
        "loop" => Some(Token::Loop),
        "break" => Some(Token::Break),
        "continue" => Some(Token::Continue),
        "case" => Some(Token::Case),
        "select" => Some(Token::Select),
        "timeout" => Some(Token::Timeout),
        "with" => Some(Token::With),
        "otherwise" => Some(Token::Otherwise),
        "as" => Some(Token::As),
        "Unit" => Some(Token::Unit),
        "except" => Some(Token::Except),
        "channel!" => Some(Token::Channel),
        "queue!" => Some(Token::Queue),
        "broadcast!" => Some(Token::Broadcast),
        "at" => Some(Token::At),
        _ => None,
    }
}

fn identifier() -> impl Parser<char, Token, Error = Simple<char>> + Clone {
    // Regular identifier: [a-z_][a-z0-9_]*!?
    let regular = filter(|c: &char| is_ident_start(*c))
        .chain(filter(|c: &char| is_ident_char(*c)).repeated())
        .collect::<String>()
        .then(just('!').or_not())
        .map(|(base, bang): (String, Option<char>)| {
            match bang {
                Some(_) => format!("{}!", base),
                None => base,
            }
        });

    // Type name: [A-Z][a-zA-Z0-9_]*
    let type_name = filter(|c: &char| c.is_ascii_uppercase())
        .chain(filter(|c: &char| c.is_ascii_alphanumeric() || *c == '_').repeated())
        .collect::<String>();

    // Operator as identifier: [+\\-*/<>=!|&^@#$%]+
    let operator = filter(|c: &char| is_operator_char(*c))
        .repeated()
        .at_least(1)
        .collect::<String>();

    // Lambda alternative: λ (unicode) - treated as identifier that maps to Lambda token
    let lambda_unicode = just('λ')
        .map(|_| "lambda".to_string());

    choice((lambda_unicode, regular, type_name, operator))
        .map(|s: String| {
            // Single underscore '_' is a Hole, everything else is an identifier
            if s == "_" {
                Token::Hole
            } else {
                str_to_keyword(&s).unwrap_or_else(|| Token::Ident(s))
            }
        })
}

// ============================================================================
// STRUCTURAL TOKENS
// ============================================================================

fn structural_tokens() -> impl Parser<char, Token, Error = Simple<char>> + Clone {
    // Multi-character tokens (longer first)
    let multi_char = choice((
        just("::").to(Token::DoubleColon),
        just("->").to(Token::SimpleArrow),
        just("=>").to(Token::Arrow),
        just("<!?").to(Token::ReceiveNonBlocking),
        just("<!").to(Token::Receive),
        just("!>").to(Token::Send),
        just("|>").to(Token::Pipeline),
        just("..=").to(Token::DotDotEqual),
        just("...").to(Token::DotDotDot),
        just("..").to(Token::DotDot),
    ));

    // Single character tokens
    let single_char = choice((
        just(';').to(Token::Semicolon),
        just(':').to(Token::Colon),
        just('(').to(Token::LParen),
        just(')').to(Token::RParen),
        just('[').to(Token::LBracket),
        just(']').to(Token::RBracket),
        just('{').to(Token::LBrace),
        just('}').to(Token::RBrace),
        just(',').to(Token::Comma),
        just('.').to(Token::Dot),
        just('|').to(Token::Pipe),
        just('@').to(Token::AtSymbol),
        just('\\').to(Token::Backslash),
        just('?').to(Token::Question),
        just('$').to(Token::Dollar),
        just('\n').to(Token::Newline),
    ));

    choice((multi_char, single_char))
}

// ============================================================================
// WHITESPACE AND COMMENTS
// ============================================================================

fn whitespace_and_comments() -> impl Parser<char, (), Error = Simple<char>> + Clone {
    // Line comment: #[^\n]*
    let comment = just('#')
        .ignore_then(filter(|c: &char| *c != '\n').repeated())
        .ignored();

    // Horizontal whitespace: [ \t\r\f]
    let whitespace = filter(|c: &char| matches!(*c, ' ' | '\t' | '\r')).ignored();

    choice((comment, whitespace))
        .repeated()
        .ignored()
}

// ============================================================================
// MAIN LEXER
// ============================================================================

/// Lexer for Kata language
pub struct KataLexer;

impl KataLexer {
    /// Create the lexer parser
    fn parser() -> impl Parser<char, Vec<(Token, std::ops::Range<usize>)>, Error = Simple<char>> + Clone {
        let token_parser = choice((
            // Literals (try longer/ambiguous first)
            float_literal(),
            signed_number_literal(),
            int_literal(),
            string_literal(),
            bytes_literal(),

            // Structural tokens
            structural_tokens(),

            // Identifiers and keywords
            identifier(),
        ))
            .map_with_span(|token, span| Some((token, span)));

        // Horizontal whitespace: just skip it but keep track of it if needed
        // For indentation, we need the NEXT token's span.start to be AFTER the spaces.
        // If I use .padded_by(whitespace_without_newline()), chumsky's padded_by
        // will consume them and the span.start of the inner parser will NOT include them.
        // THIS IS THE PROBLEM.
        
        let horizontal_whitespace = filter(|c: &char| matches!(*c, ' ' | '\t' | '\r')).repeated().at_least(1).ignored();
        let comment = just('#').ignore_then(filter(|c: &char| *c != '\n').repeated()).ignored();

        choice((
            token_parser,
            // Skip comments and horizontal whitespace
            comment.map_with_span(|_, _| None),
            horizontal_whitespace.map_with_span(|_, _| None),
        ))
        .repeated()
        .map(|v| v.into_iter().flatten().collect())
        .then_ignore(end())
    }

    /// Tokenize source code into a list of spanned tokens
    pub fn lex(source: &str) -> Result<Vec<SpannedToken>, Vec<LexerError>> {
        let parser = Self::parser();

        match parser.parse(source) {
            Ok(tokens) => {
                let mut spanned: Vec<SpannedToken> = tokens
                    .into_iter()
                    .map(|(token, span)| {
                        SpannedToken {
                            token,
                            span: Span::new(span.start, span.end),
                        }
                    })
                    .collect();

                // Add EOF token at the end
                let last_end = spanned.last().map(|t| t.span.end).unwrap_or(0);
                spanned.push(SpannedToken {
                    token: Token::Eof,
                    span: Span::new(last_end, last_end),
                });

                Ok(spanned)
            }
            Err(errors) => {
                let lexer_errors: Vec<LexerError> = errors
                    .into_iter()
                    .map(|e| {
                        let span = Span::new(e.span().start, e.span().end);
                        LexerError::ParseError {
                            message: e.to_string(),
                            span,
                        }
                    })
                    .collect();
                Err(lexer_errors)
            }
        }
    }

    /// Tokenize source code with indentation processing
    ///
    /// This produces INDENT/DEDENT tokens for layout-based syntax.
    /// Uses significant whitespace (Python-style).
    pub fn lex_with_indent(source: &str) -> Result<Vec<SpannedToken>, Vec<LexerError>> {
        // First pass: get raw tokens
        let raw_tokens = Self::lex(source)?;

        // Second pass: process indentation based on source
        Ok(Self::process_indentation(source, &raw_tokens))
    }

    /// Process indentation in token stream based on source positions
    ///
    /// Rules:
    /// - Track indentation levels in a stack
    /// - After newline, count leading whitespace from start of line
    /// - Emit INDENT if indentation increases
    /// - Emit DEDENT(s) if indentation decreases
    /// - Error on inconsistent use of tabs/spaces within same block
    fn process_indentation(source: &str, tokens: &[SpannedToken]) -> Vec<SpannedToken> {
        let mut result = Vec::new();
        let mut indent_stack: Vec<usize> = vec![0]; // Stack of indentation levels (in spaces)
        let mut pending_newline: Option<SpannedToken> = None;
        let mut indent_type: Option<char> = None; // Track ' ' or '\t' consistency
        let mut at_line_start = true;
        let mut last_newline_pos: usize = 0;

        // Helper to count indentation from start of line
        let count_indent_from_line_start = |source: &str, token_start: usize, line_start: usize| -> (usize, Option<char>) {
            let mut spaces = 0;
            let mut found_type: Option<char> = None;

            // Using char indices to avoid UTF-8 byte boundary panics
            for ch in source.chars().skip(line_start).take(token_start.saturating_sub(line_start)) {
                match ch {
                    ' ' => {
                        spaces += 1;
                        if found_type.is_none() {
                            found_type = Some(' ');
                        }
                    }
                    '\t' => {
                        // Tab = 4 spaces
                        spaces += 4;
                        if found_type.is_none() {
                            found_type = Some('\t');
                        }
                    }
                    _ => {
                        // Non-whitespace found, stop counting
                        break;
                    }
                }
            }
            (spaces, found_type)
        };

        for i in 0..tokens.len() {
            let spanned = &tokens[i];

            match &spanned.token {
                Token::Newline => {
                    // Record the position of this newline for indent calculation
                    last_newline_pos = spanned.span.end; // Position after the newline
                    if pending_newline.is_none() {
                        pending_newline = Some(spanned.clone());
                    }
                    at_line_start = true;
                }

                Token::Eof => {
                    // Emit any pending newline
                    if let Some(newline) = pending_newline.take() {
                        result.push(newline);
                    }
                    // Emit DEDENTs for all remaining indentation levels
                    while indent_stack.len() > 1 {
                        indent_stack.pop();
                        result.push(SpannedToken {
                            token: Token::Dedent,
                            span: spanned.span,
                        });
                    }
                    result.push(spanned.clone());
                }

                _ => {
                    if at_line_start {
                        // Calculate indentation from line start position
                        let (current_indent, current_type) = count_indent_from_line_start(
                            source,
                            spanned.span.start,
                            last_newline_pos
                        );
                        
                        log::debug!("Indent check at pos {}: current={}, last_newline={}", spanned.span.start, current_indent, last_newline_pos);

                        // Check consistency of indentation type
                        if let Some(prev_type) = indent_type {
                            if current_type.is_some() && current_type != Some(prev_type) {
                                // Mixed tabs and spaces - could emit error
                                // For now, just continue with the new type
                            }
                        } else if current_type.is_some() {
                            indent_type = current_type;
                        }

                        // Compare with current indentation level
                        let top_indent = *indent_stack.last().unwrap_or(&0);

                        if current_indent > top_indent {
                            // Indentation increased - emit INDENT
                            indent_stack.push(current_indent);

                            // Emit pending newline first
                            if let Some(newline) = pending_newline.take() {
                                result.push(newline);
                            }

                            result.push(SpannedToken {
                                token: Token::Indent,
                                span: spanned.span,
                            });
                        } else if current_indent < top_indent {
                            // Indentation decreased - emit DEDENT(s)
                            // Emit pending newline first
                            if let Some(newline) = pending_newline.take() {
                                result.push(newline);
                            }

                            while let Some(&top) = indent_stack.last() {
                                if top > current_indent {
                                    indent_stack.pop();
                                    result.push(SpannedToken {
                                        token: Token::Dedent,
                                        span: spanned.span,
                                    });
                                } else {
                                    break;
                                }
                            }
                        } else {
                            // Same indentation - just emit pending newline
                            if let Some(newline) = pending_newline.take() {
                                result.push(newline);
                            }
                        }

                        at_line_start = false;
                    } else {
                        // Not at line start - emit pending newline if any
                        if let Some(newline) = pending_newline.take() {
                            result.push(newline);
                        }
                    }

                    result.push(spanned.clone());
                }
            }
        }

        result
    }
}