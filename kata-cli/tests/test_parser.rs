use kata::lexer::KataLexer;
use kata::parser::expr::expression;
use chumsky::Parser;

fn main() {
    let src = "[x:y:zs]";
    let tokens = KataLexer::lex_with_indent(src).unwrap();
    let (ast, errs) = expression().parse_recovery(tokens);
    println!("AST: {:#?}", ast);
    for e in errs {
        println!("{:?}", e);
    }
}
