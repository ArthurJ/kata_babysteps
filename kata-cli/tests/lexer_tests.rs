//! Lexer tests for Kata Language

use kata::lexer::{KataLexer, Token};

// ============================================================================
// IDENTIFIER TESTS
// ============================================================================

#[test]
fn test_basic_identifiers() {
    let source = "foo bar_baz";
    let result = KataLexer::lex(source);
    assert!(result.is_ok());
    let tokens = result.unwrap();
    assert_eq!(tokens.len(), 3); // foo, bar_baz, EOF
    assert_eq!(tokens[0].token, Token::Ident("foo".to_string()));
    assert_eq!(tokens[1].token, Token::Ident("bar_baz".to_string()));
}

#[test]
fn test_action_identifiers() {
    let source = "echo! main! channel!";
    let result = KataLexer::lex(source);
    assert!(result.is_ok());
    let tokens = result.unwrap();
    assert_eq!(tokens[0].token, Token::Ident("echo!".to_string()));
    assert_eq!(tokens[1].token, Token::Ident("main!".to_string()));
    // channel! is a keyword, not identifier
    assert_eq!(tokens[2].token, Token::Channel);
}

#[test]
fn test_type_names() {
    let source = "Int List Optional";
    let result = KataLexer::lex(source);
    assert!(result.is_ok());
    let tokens = result.unwrap();
    assert_eq!(tokens[0].token, Token::Ident("Int".to_string()));
    assert_eq!(tokens[1].token, Token::Ident("List".to_string()));
    assert_eq!(tokens[2].token, Token::Ident("Optional".to_string()));
}

// ============================================================================
// KEYWORD TESTS
// ============================================================================

#[test]
fn test_keywords() {
    let source = "lambda action let var";
    let result = KataLexer::lex(source);
    assert!(result.is_ok());
    let tokens = result.unwrap();
    assert_eq!(tokens[0].token, Token::Lambda);
    assert_eq!(tokens[1].token, Token::Action);
    assert_eq!(tokens[2].token, Token::Let);
    assert_eq!(tokens[3].token, Token::Var);
}

#[test]
fn test_lambda_alternative() {
    let source = "λ";
    let result = KataLexer::lex(source);
    assert!(result.is_ok());
    let tokens = result.unwrap();
    assert_eq!(tokens[0].token, Token::Lambda);
}

#[test]
fn test_control_flow_keywords() {
    let source = "match loop for in break continue";
    let result = KataLexer::lex(source);
    assert!(result.is_ok());
    let tokens = result.unwrap();
    assert_eq!(tokens[0].token, Token::Match);
    assert_eq!(tokens[1].token, Token::Loop);
    assert_eq!(tokens[2].token, Token::For);
    assert_eq!(tokens[3].token, Token::In);
    assert_eq!(tokens[4].token, Token::Break);
    assert_eq!(tokens[5].token, Token::Continue);
}

#[test]
fn test_concurrency_keywords() {
    let source = "channel! queue! broadcast! select timeout";
    let result = KataLexer::lex(source);
    assert!(result.is_ok());
    let tokens = result.unwrap();
    assert_eq!(tokens[0].token, Token::Channel);
    assert_eq!(tokens[1].token, Token::Queue);
    assert_eq!(tokens[2].token, Token::Broadcast);
    assert_eq!(tokens[3].token, Token::Select);
    assert_eq!(tokens[4].token, Token::Timeout);
}

// ============================================================================
// OPERATOR TESTS (as identifiers)
// ============================================================================

#[test]
fn test_arithmetic_operators_as_idents() {
    let source = "+ - * /";
    let result = KataLexer::lex(source);
    assert!(result.is_ok());
    let tokens = result.unwrap();
    assert_eq!(tokens[0].token, Token::Ident("+".to_string()));
    assert_eq!(tokens[1].token, Token::Ident("-".to_string()));
    assert_eq!(tokens[2].token, Token::Ident("*".to_string()));
    assert_eq!(tokens[3].token, Token::Ident("/".to_string()));
}

#[test]
fn test_comparison_operators_as_idents() {
    let source = "= != < > <= >=";
    let result = KataLexer::lex(source);
    assert!(result.is_ok());
    let tokens = result.unwrap();
    // Note: Some of these might be multi-char operators
    // The lexer should handle them appropriately
    assert_eq!(tokens[0].token, Token::Ident("=".to_string()));
}

#[test]
fn test_channel_operators() {
    let source = "!> <! <!?";
    let result = KataLexer::lex(source);
    assert!(result.is_ok());
    let tokens = result.unwrap();
    assert_eq!(tokens[0].token, Token::Send);
    assert_eq!(tokens[1].token, Token::Receive);
    assert_eq!(tokens[2].token, Token::ReceiveNonBlocking);
}

#[test]
fn test_pipeline_operator() {
    let source = "|>";
    let result = KataLexer::lex(source);
    assert!(result.is_ok());
    let tokens = result.unwrap();
    assert_eq!(tokens[0].token, Token::Pipeline);
}

// ============================================================================
// LITERAL TESTS
// ============================================================================

#[test]
fn test_string_literal() {
    let source = r#""hello world""#;
    let result = KataLexer::lex(source);
    assert!(result.is_ok());
    let tokens = result.unwrap();
    assert_eq!(tokens[0].token, Token::String("hello world".to_string()));
}

#[test]
fn test_string_with_escape() {
    let source = r#""hello\nworld""#;
    let result = KataLexer::lex(source);
    assert!(result.is_ok());
    let tokens = result.unwrap();
    // Escapes are converted: \n becomes actual newline character
    assert_eq!(tokens[0].token, Token::String("hello\nworld".to_string()));
}

#[test]
fn test_string_single_quoted() {
    let source = r#"'isto é uma string'"#;
    let result = KataLexer::lex(source);
    assert!(result.is_ok());
    let tokens = result.unwrap();
    assert_eq!(tokens[0].token, Token::String("isto é uma string".to_string()));
}

#[test]
fn test_string_single_with_double_quotes() {
    let source = r#"'isto também, mesmo com "aspas" dentro'"#;
    let result = KataLexer::lex(source);
    assert!(result.is_ok());
    let tokens = result.unwrap();
    assert_eq!(tokens[0].token, Token::String("isto também, mesmo com \"aspas\" dentro".to_string()));
}

#[test]
fn test_string_double_with_single_quotes() {
    let source = r#""texto com 'aspas simples' dentro""#;
    let result = KataLexer::lex(source);
    assert!(result.is_ok());
    let tokens = result.unwrap();
    assert_eq!(tokens[0].token, Token::String("texto com 'aspas simples' dentro".to_string()));
}

#[test]
fn test_string_escape_unicode() {
    let source = r#""hello \u{1F600}""#;
    let result = KataLexer::lex(source);
    assert!(result.is_ok());
    let tokens = result.unwrap();
    assert_eq!(tokens[0].token, Token::String("hello 😀".to_string()));
}

#[test]
fn test_string_all_escapes() {
    let source = r#""\\ \n \t \r \" ' \0""#;
    let result = KataLexer::lex(source);
    assert!(result.is_ok());
    let tokens = result.unwrap();
    assert_eq!(tokens[0].token, Token::String("\\ \n \t \r \" ' \0".to_string()));
}

#[test]
fn test_newline_token() {
    let source = "foo\nbar";
    let result = KataLexer::lex(source);
    assert!(result.is_ok());
    let tokens = result.unwrap();
    assert_eq!(tokens[0].token, Token::Ident("foo".to_string()));
    assert_eq!(tokens[1].token, Token::Newline);
    assert_eq!(tokens[2].token, Token::Ident("bar".to_string()));
}

#[test]
fn test_int_decimal() {
    let source = "42";
    let result = KataLexer::lex(source);
    assert!(result.is_ok());
    let tokens = result.unwrap();
    assert_eq!(tokens[0].token, Token::Int("42".to_string()));
}

#[test]
fn test_int_hex() {
    let source = "0xFF";
    let result = KataLexer::lex(source);
    assert!(result.is_ok());
    let tokens = result.unwrap();
    assert_eq!(tokens[0].token, Token::Int("0xFF".to_string()));
}

#[test]
fn test_int_binary() {
    let source = "0b1010";
    let result = KataLexer::lex(source);
    assert!(result.is_ok());
    let tokens = result.unwrap();
    assert_eq!(tokens[0].token, Token::Int("0b1010".to_string()));
}

#[test]
fn test_float_literal() {
    let source = "3.14159";
    let result = KataLexer::lex(source);
    assert!(result.is_ok());
    let tokens = result.unwrap();
    assert_eq!(tokens[0].token, Token::Float("3.14159".to_string()));
}

#[test]
fn test_hole() {
    let source = "_";
    let result = KataLexer::lex(source);
    assert!(result.is_ok());
    let tokens = result.unwrap();
    assert_eq!(tokens[0].token, Token::Hole);
}

// ============================================================================
// STRUCTURAL TOKEN TESTS
// ============================================================================

#[test]
fn test_parentheses() {
    let source = "( )";
    let result = KataLexer::lex(source);
    assert!(result.is_ok());
    let tokens = result.unwrap();
    assert_eq!(tokens[0].token, Token::LParen);
    assert_eq!(tokens[1].token, Token::RParen);
}

#[test]
fn test_brackets() {
    let source = "[ ]";
    let result = KataLexer::lex(source);
    assert!(result.is_ok());
    let tokens = result.unwrap();
    assert_eq!(tokens[0].token, Token::LBracket);
    assert_eq!(tokens[1].token, Token::RBracket);
}

#[test]
fn test_braces() {
    let source = "{ }";
    let result = KataLexer::lex(source);
    assert!(result.is_ok());
    let tokens = result.unwrap();
    assert_eq!(tokens[0].token, Token::LBrace);
    assert_eq!(tokens[1].token, Token::RBrace);
}

#[test]
fn test_arrows() {
    let source = "-> =>";
    let result = KataLexer::lex(source);
    assert!(result.is_ok());
    let tokens = result.unwrap();
    assert_eq!(tokens[0].token, Token::SimpleArrow);
    assert_eq!(tokens[1].token, Token::Arrow);
}

#[test]
fn test_double_colon() {
    let source = "::";
    let result = KataLexer::lex(source);
    assert!(result.is_ok());
    let tokens = result.unwrap();
    assert_eq!(tokens[0].token, Token::DoubleColon);
}

#[test]
fn test_question_mark() {
    let source = "?";
    let result = KataLexer::lex(source);
    assert!(result.is_ok());
    let tokens = result.unwrap();
    assert_eq!(tokens[0].token, Token::Question);
}

#[test]
fn test_dollar() {
    let source = "$";
    let result = KataLexer::lex(source);
    assert!(result.is_ok());
    let tokens = result.unwrap();
    assert_eq!(tokens[0].token, Token::Dollar);
}

#[test]
fn test_at_symbol() {
    let source = "@";
    let result = KataLexer::lex(source);
    assert!(result.is_ok());
    let tokens = result.unwrap();
    assert_eq!(tokens[0].token, Token::AtSymbol);
}

// ============================================================================
// COMMENT TESTS
// ============================================================================

#[test]
fn test_line_comment() {
    let source = r#"foo # this is a comment
bar"#;
    let result = KataLexer::lex(source);
    assert!(result.is_ok());
    let tokens = result.unwrap();
    assert_eq!(tokens[0].token, Token::Ident("foo".to_string()));
    assert_eq!(tokens[1].token, Token::Newline);
    assert_eq!(tokens[2].token, Token::Ident("bar".to_string()));
}

// ============================================================================
// COMPLEX EXPRESSION TESTS
// ============================================================================

#[test]
fn test_function_signature() {
    let source = "fib :: Int => Int";
    let result = KataLexer::lex(source);
    assert!(result.is_ok());
    let tokens = result.unwrap();
    assert_eq!(tokens[0].token, Token::Ident("fib".to_string()));
    assert_eq!(tokens[1].token, Token::DoubleColon);
    assert_eq!(tokens[2].token, Token::Ident("Int".to_string()));
    assert_eq!(tokens[3].token, Token::Arrow);
    assert_eq!(tokens[4].token, Token::Ident("Int".to_string()));
}

#[test]
fn test_function_call_prefix() {
    let source = "+ (fib (- n 1)) (fib (- n 2))";
    let result = KataLexer::lex(source);
    assert!(result.is_ok());
    let tokens = result.unwrap();
    assert_eq!(tokens[0].token, Token::Ident("+".to_string()));
    assert_eq!(tokens[1].token, Token::LParen);
    assert_eq!(tokens[2].token, Token::Ident("fib".to_string()));
}

#[test]
fn test_explicit_application() {
    let source = "$(fib 40)";
    let result = KataLexer::lex(source);
    assert!(result.is_ok());
    let tokens = result.unwrap();
    assert_eq!(tokens[0].token, Token::Dollar);
    assert_eq!(tokens[1].token, Token::LParen);
    assert_eq!(tokens[2].token, Token::Ident("fib".to_string()));
    assert_eq!(tokens[3].token, Token::Int("40".to_string()));
    assert_eq!(tokens[4].token, Token::RParen);
}

#[test]
fn test_channel_creation() {
    let source = "let (tx rx) channel!()";
    let result = KataLexer::lex(source);
    assert!(result.is_ok());
    let tokens = result.unwrap();
    assert_eq!(tokens[0].token, Token::Let);
    assert_eq!(tokens[1].token, Token::LParen);
    assert_eq!(tokens[2].token, Token::Ident("tx".to_string()));
    assert_eq!(tokens[3].token, Token::Ident("rx".to_string()));
    assert_eq!(tokens[4].token, Token::RParen);
    assert_eq!(tokens[5].token, Token::Channel);
    assert_eq!(tokens[6].token, Token::LParen);
    assert_eq!(tokens[7].token, Token::RParen);
}

#[test]
fn test_import_statement() {
    let source = "import types.(SHOW HASH EQ ORD)";
    let result = KataLexer::lex(source);
    assert!(result.is_ok());
    let tokens = result.unwrap();
    assert_eq!(tokens[0].token, Token::Import);
    assert_eq!(tokens[1].token, Token::Ident("types".to_string()));
    assert_eq!(tokens[2].token, Token::Dot);
    assert_eq!(tokens[3].token, Token::LParen);
    assert_eq!(tokens[4].token, Token::Ident("SHOW".to_string()));
}

#[test]
fn test_data_declaration() {
    let source = "data Vec2 (x y)";
    let result = KataLexer::lex(source);
    assert!(result.is_ok());
    let tokens = result.unwrap();
    assert_eq!(tokens[0].token, Token::Data);
    assert_eq!(tokens[1].token, Token::Ident("Vec2".to_string()));
    assert_eq!(tokens[2].token, Token::LParen);
    assert_eq!(tokens[3].token, Token::Ident("x".to_string()));
    assert_eq!(tokens[4].token, Token::Ident("y".to_string()));
    assert_eq!(tokens[5].token, Token::RParen);
}

#[test]
fn test_enum_declaration() {
    let source = "enum Transacao | Aprovada | Recusada(Text) | Pendente";
    let result = KataLexer::lex(source);
    assert!(result.is_ok());
    let tokens = result.unwrap();
    assert_eq!(tokens[0].token, Token::Enum);
    assert_eq!(tokens[1].token, Token::Ident("Transacao".to_string()));
    assert_eq!(tokens[2].token, Token::Pipe);
    assert_eq!(tokens[3].token, Token::Ident("Aprovada".to_string()));
}

// ============================================================================
// EOF TEST
// ============================================================================

#[test]
fn test_eof_is_added() {
    let source = "foo";
    let result = KataLexer::lex(source);
    assert!(result.is_ok());
    let tokens = result.unwrap();
    assert!(tokens.last().map(|t| matches!(t.token, Token::Eof)).unwrap_or(false));
}

// ============================================================================
// INDENTATION TESTS
// ============================================================================

#[test]
fn test_indent_basic() {
    let source = "foo\n    bar";
    let result = KataLexer::lex_with_indent(source);
    assert!(result.is_ok());
    let tokens = result.unwrap();

    // foo, Newline, INDENT, bar, DEDENT, EOF
    assert_eq!(tokens[0].token, Token::Ident("foo".to_string()));
    assert_eq!(tokens[1].token, Token::Newline);
    assert_eq!(tokens[2].token, Token::Indent);
    assert_eq!(tokens[3].token, Token::Ident("bar".to_string()));
    assert_eq!(tokens[4].token, Token::Dedent); // Dedent at EOF
    assert!(matches!(tokens[5].token, Token::Eof));
}

#[test]
fn test_indent_dedent() {
    let source = "foo\n    bar\nbaz";
    let result = KataLexer::lex_with_indent(source);
    assert!(result.is_ok());
    let tokens = result.unwrap();

    // foo, Newline, INDENT, bar, Newline, DEDENT, baz, DEDENT, EOF
    assert_eq!(tokens[0].token, Token::Ident("foo".to_string()));
    assert_eq!(tokens[1].token, Token::Newline);
    assert_eq!(tokens[2].token, Token::Indent);
    assert_eq!(tokens[3].token, Token::Ident("bar".to_string()));
    assert_eq!(tokens[4].token, Token::Newline);
    assert_eq!(tokens[5].token, Token::Dedent);
    assert_eq!(tokens[6].token, Token::Ident("baz".to_string()));
    // Final DEDENT at EOF
    assert!(tokens.iter().filter(|t| matches!(t.token, Token::Dedent)).count() >= 1);
}

#[test]
fn test_indent_nested() {
    let source = "foo\n    bar\n        baz\n    qux";
    let result = KataLexer::lex_with_indent(source);
    assert!(result.is_ok());
    let tokens = result.unwrap();

    // Count INDENTs and DEDENTs
    let indent_count = tokens.iter().filter(|t| matches!(t.token, Token::Indent)).count();
    let dedent_count = tokens.iter().filter(|t| matches!(t.token, Token::Dedent)).count();

    assert_eq!(indent_count, 2); // Two indentation levels
    // 2 DEDENTs: one for qux, one at EOF
    assert!(dedent_count >= 2);
}

#[test]
fn test_indent_same_level() {
    let source = "foo\n    bar\n    baz";
    let result = KataLexer::lex_with_indent(source);
    assert!(result.is_ok());
    let tokens = result.unwrap();

    // foo, Newline, INDENT, bar, Newline, baz, DEDENT, EOF
    assert_eq!(tokens[0].token, Token::Ident("foo".to_string()));
    assert_eq!(tokens[1].token, Token::Newline);
    assert_eq!(tokens[2].token, Token::Indent);
    assert_eq!(tokens[3].token, Token::Ident("bar".to_string()));
    assert_eq!(tokens[4].token, Token::Newline);
    // baz should be at same indent level, no new INDENT
    assert_eq!(tokens[5].token, Token::Ident("baz".to_string()));
}

#[test]
fn test_indent_with_action() {
    // Simulating a lambda body
    let source = r#"lambda (n)
    + n 1"#;
    let result = KataLexer::lex_with_indent(source);
    assert!(result.is_ok());
    let tokens = result.unwrap();

    assert_eq!(tokens[0].token, Token::Lambda);
    assert_eq!(tokens[1].token, Token::LParen);
    assert_eq!(tokens[2].token, Token::Ident("n".to_string()));
    assert_eq!(tokens[3].token, Token::RParen);
    assert_eq!(tokens[4].token, Token::Newline);
    assert_eq!(tokens[5].token, Token::Indent);
    assert_eq!(tokens[6].token, Token::Ident("+".to_string()));
}

#[test]
fn test_indent_comment_line() {
    let source = "foo\n    # comment\n    bar";
    let result = KataLexer::lex_with_indent(source);
    assert!(result.is_ok());
    let tokens = result.unwrap();

    // Comments are skipped, indentation is calculated from first non-comment token
    assert_eq!(tokens[0].token, Token::Ident("foo".to_string()));
    assert_eq!(tokens[1].token, Token::Newline);
    assert_eq!(tokens[2].token, Token::Indent);
    // Comment is consumed, bar follows
    assert_eq!(tokens[3].token, Token::Ident("bar".to_string()));
}