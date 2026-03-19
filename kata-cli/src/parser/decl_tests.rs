#[cfg(test)]
mod tests {
    use chumsky::Parser;
    use crate::lexer::{KataLexer, SpannedToken};
    use crate::parser::common::{convert_result, ParserError};
    use crate::parser::expr::expression;
    use crate::parser::decl::top_level;
    use crate::ast::decl::{TopLevel, ActionDef};

    fn parse_top(source: &str) -> Result<TopLevel, Vec<crate::parser::error::ParseError>> {
        let tokens = KataLexer::lex_with_indent(source).unwrap();
        let expr = expression();
        convert_result(top_level(expr).parse(tokens))
    }

    #[test]
    fn test_action_unit() {
        let source = "action foo\n    echo! 1";
        let result = parse_top(source);
        assert!(result.is_ok(), "Parse failed: {:?}", result.err());
        match result.unwrap() {
            TopLevel::Action(a) => assert_eq!(a.name.0, "foo"),
            _ => panic!("Expected Action"),
        }
    }
}
