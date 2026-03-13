use crate::ast::*;
use crate::error::KataError;
use crate::lexer::{KataLexer, Token};
use logos::Span;
use std::iter::Peekable;

pub struct Parser<'a> {
    lexer: Peekable<KataLexer<'a>>,
    current_span: Span,
    /// Token que foi consumido "extra" e precisa ser reprocessado
    /// Usado quando saímos de um with por encontrar uma assinatura de função
    pushed_back_token: Option<Token>,
}

impl<'a> Parser<'a> {
    pub fn new(source: &'a str) -> Self {
        let lexer = KataLexer::new(source).peekable();
        Self {
            lexer,
            current_span: 0..0,
            pushed_back_token: None,
        }
    }

    fn next_token(&mut self) -> Result<Option<Token>, KataError> {
        // Se há um token empurrado de volta, retorna ele primeiro
        if let Some(token) = self.pushed_back_token.take() {
            log::debug!("next_token retornando pushed_back_token: {:?}", token);
            return Ok(Some(token));
        }
        match self.lexer.next() {
            Some(Ok((token, span))) => {
                self.current_span = span;
                Ok(Some(token))
            }
            Some(Err(e)) => Err(e),
            None => Ok(None),
        }
    }

    fn peek_token_safe(&mut self) -> Result<Option<Token>, KataError> {
        // Se há um token empurrado de volta, retorna ele
        if let Some(ref token) = self.pushed_back_token {
            return Ok(Some(token.clone()));
        }
        match self.lexer.peek() {
            Some(Ok((t, _))) => Ok(Some(t.clone())),
            Some(Err(_)) => {
                let err = self.next_token().unwrap_err();
                Err(err)
            }
            None => Ok(None),
        }
    }

    /// Empurra um token de volta ao buffer para ser reprocessado
    fn push_back_token(&mut self, token: Token) {
        log::debug!("push_back_token chamado com {:?}", token);
        self.pushed_back_token = Some(token);
    }

    /// Verifica se o próximo token seria ::
    /// Usado para detectar assinaturas de função vs bindings de with
    fn is_next_token_doublecolon(&mut self) -> Result<bool, KataError> {
        // Esta função assume que já consumimos o FuncIdent
        // Então verificamos o próximo token
        match self.peek_token_safe()? {
            Some(Token::DoubleColon) => Ok(true),
            _ => Ok(false),
        }
    }

    fn expect(&mut self, expected: Token, err_msg: &str) -> Result<(), KataError> {
        let span = self.current_span.clone();
        match self.next_token()? {
            Some(token) if token == expected => Ok(()),
            Some(wrong) => Err(KataError::UnexpectedToken { msg: format!("{} (Recebeu: {:?})", err_msg, wrong), span: (span.start, span.end) }),
            None => Err(KataError::UnexpectedEOF),
        }
    }

    // ==========================================
    // PARSING DE TOP-LEVEL (MÓDULOS)
    // ==========================================

    pub fn parse_module(&mut self) -> Result<ModuleAST, KataError> {
        let mut declarations = Vec::new();
        let mut current_func_name: Option<Ident> = None;
        let mut current_lambdas: Vec<LambdaBranch> = Vec::new();
        let mut pending_attrs: Vec<TopLevelAttr> = Vec::new();

        self.parse_top_level_into(&mut declarations, &mut current_func_name, &mut current_lambdas, &mut pending_attrs, false)?;
        
        // Finaliza qualquer lambda pendente
        if let Some(n) = current_func_name.take() {
            if !current_lambdas.is_empty() {
                declarations.push(TopLevelDecl::Definition {
                    name: n,
                    expr: DataExpr::LambdaGroup { branches: std::mem::take(&mut current_lambdas) },
                });
            }
        }

        Ok(ModuleAST { declarations })
    }

    fn parse_top_level_into(
        &mut self,
        declarations: &mut Vec<TopLevelDecl>,
        current_func_name: &mut Option<Ident>,
        current_lambdas: &mut Vec<LambdaBranch>,
        pending_attrs: &mut Vec<TopLevelAttr>,
        until_dedent: bool,
    ) -> Result<(), KataError> {
        let mut commit_lambdas = |decls: &mut Vec<TopLevelDecl>, name: &mut Option<Ident>, lambdas: &mut Vec<LambdaBranch>| {
            if let Some(n) = name.take() {
                if !lambdas.is_empty() {
                    decls.push(TopLevelDecl::Definition {
                        name: n.clone(),
                        expr: DataExpr::LambdaGroup { branches: std::mem::take(lambdas) },
                    });
                }
            }
        };

        while let Some(token) = self.peek_token_safe()? {
            if until_dedent && token == Token::Dedent {
                break;
            }

            match token {
                Token::Newline | Token::Indent | Token::Dedent | Token::Whitespace => {
                    self.next_token()?;
                }
                Token::InterfaceKw => {
                    commit_lambdas(declarations, current_func_name, current_lambdas);
                    self.next_token()?;
                    
                    let name_tok = self.next_token()?.ok_or(KataError::UnexpectedEOF)?;
                    let name = match name_tok {
                        Token::InterfaceIdent(s) => Ident::Interface(s),
                        _ => return Err(KataError::UnexpectedToken { msg: "Esperado nome da Interface em ALL_CAPS".into(), span: (self.current_span.start, self.current_span.end) }),
                    };
                    
                    let mut methods = Vec::new();
                    let mut inner_func_name = None;
                    let mut inner_lambdas = Vec::new();
                    let mut inner_attrs = Vec::new();

                    if let Some(Token::Newline) = self.peek_token_safe()? {
                        self.next_token()?;
                        if let Some(Token::Indent) = self.peek_token_safe()? {
                            self.next_token()?;
                            while let Some(tok) = self.peek_token_safe()? {
                                if tok == Token::Dedent {
                                    self.next_token()?;
                                    break;
                                }
                                if matches!(tok, Token::Newline | Token::Whitespace | Token::Comment) {
                                    self.next_token()?;
                                    continue;
                                }
                                self.parse_top_level_into(&mut methods, &mut inner_func_name, &mut inner_lambdas, &mut inner_attrs, true)?;
                            }
                        }
                    }

                    declarations.push(TopLevelDecl::InterfaceDef {
                        name,
                        supertraits: vec![],
                        methods, 
                    });
                    pending_attrs.clear();
                }
                Token::Annotation(name) => {
                    self.next_token()?;
                    let mut args = Vec::new();
                    if let Some(Token::LParen) = self.peek_token_safe()? {
                        self.next_token()?;
                        while let Some(tok) = self.peek_token_safe()? {
                            if tok == Token::RParen {
                                self.next_token()?;
                                break;
                            }
                            if let Token::StringLiteral(s) = tok {
                                self.next_token()?;
                                args.push(s);
                            } else {
                                self.next_token()?;
                            }
                        }
                    }
                    pending_attrs.push(TopLevelAttr { name, args });
                }
                Token::ImportKw => {
                    commit_lambdas(declarations, current_func_name, current_lambdas);
                    self.next_token()?;
                    declarations.push(self.parse_import()?);
                    pending_attrs.clear();
                }
                Token::ExportKw => {
                    commit_lambdas(declarations, current_func_name, current_lambdas);
                    self.next_token()?;
                    declarations.push(self.parse_export()?);
                    pending_attrs.clear();
                }
                Token::DataKw => {
                    commit_lambdas(declarations, current_func_name, current_lambdas);
                    self.next_token()?;
                    declarations.push(self.parse_data_def()?);
                    pending_attrs.clear();
                }
                Token::EnumKw => {
                    commit_lambdas(declarations, current_func_name, current_lambdas);
                    self.next_token()?;
                    declarations.push(self.parse_enum_def()?);
                    pending_attrs.clear();
                }
                Token::ActionKw => {
                    commit_lambdas(declarations, current_func_name, current_lambdas);
                    self.next_token()?;
                    let mut def = self.parse_action_def()?;
                    if let TopLevelDecl::ActionDef { ref mut attrs, .. } = def {
                        *attrs = std::mem::take(pending_attrs);
                    }
                    declarations.push(def);
                }
                Token::FuncIdent(ref name) | Token::SymbolIdent(ref name) | Token::ActionIdent(ref name) => {
                    let is_symbol = matches!(&token, Token::SymbolIdent(_));
                    let is_action = matches!(&token, Token::ActionIdent(_));
                    let name_clone = name.clone();
                    self.next_token()?;
                    
                    let ident = if is_symbol {
                        Ident::Symbol(name_clone)
                    } else if is_action {
                        Ident::Action(name_clone)
                    } else {
                        Ident::Func(name_clone)
                    };

                    if let Some(Token::DoubleColon) = self.peek_token_safe()? {
                        commit_lambdas(declarations, current_func_name, current_lambdas);
                        self.next_token()?; 
                        let sig = self.parse_type_signature()?;
                        declarations.push(TopLevelDecl::SignatureDecl {
                            attrs: std::mem::take(pending_attrs),
                            name: ident.clone(),
                            sig,
                        });
                        *current_func_name = Some(ident);
                    } else if let Some(Token::Colon) = self.peek_token_safe()? {
                        self.next_token()?;
                    } else {
                        let mut seq = vec![DataExpr::Identifier(ident)];
                        let rest = self.parse_data_expr()?;
                        if let DataExpr::Seq(mut v) = rest {
                            seq.append(&mut v);
                        } else if rest != DataExpr::Tuple(vec![]) {
                            seq.push(rest);
                        }
                        
                        let synthetic_name = Ident::Func("repl_eval".to_string());
                        declarations.push(TopLevelDecl::Definition {
                            name: synthetic_name,
                            expr: if seq.len() == 1 { seq.pop().unwrap() } else { DataExpr::Seq(seq) },
                        });
                        pending_attrs.clear();
                    }
                }
                Token::SendChan | Token::RecvChan | Token::TryRecvChan => {
                    let ident = match token {
                        Token::SendChan => Ident::Symbol(">!".into()),
                        Token::RecvChan => Ident::Symbol("<!".into()),
                        Token::TryRecvChan => Ident::Symbol("<!?".into()),
                        _ => unreachable!(),
                    };
                    self.next_token()?;
                    
                    if let Some(Token::DoubleColon) = self.peek_token_safe()? {
                        commit_lambdas(declarations, current_func_name, current_lambdas);
                        self.next_token()?;
                        let sig = self.parse_type_signature()?;
                        declarations.push(TopLevelDecl::SignatureDecl {
                            attrs: std::mem::take(pending_attrs),
                            name: ident.clone(),
                            sig,
                        });
                        *current_func_name = Some(ident);
                    } else {
                        let mut seq = vec![DataExpr::Identifier(ident)];
                        let rest = self.parse_data_expr()?;
                        if let DataExpr::Seq(mut v) = rest { seq.append(&mut v); } else if rest != DataExpr::Tuple(vec![]) { seq.push(rest); }
                        declarations.push(TopLevelDecl::Definition { name: Ident::Func("repl_eval".into()), expr: DataExpr::Seq(seq) });
                    }
                }
                Token::TypeIdent(name) => {
                    let ident = Ident::Type(name.clone());
                    self.next_token()?;
                    
                    if let Some(Token::ImplementsKw) = self.peek_token_safe()? {
                        commit_lambdas(declarations, current_func_name, current_lambdas);
                        self.next_token()?; 
                        declarations.push(self.parse_implements(ident)?);
                        pending_attrs.clear();
                    } else {
                        // Apenas um tipo solto (possivelmente um construtor ou erro)
                        let _ = self.parse_data_expr()?;
                        pending_attrs.clear();
                    }
                }
                Token::LambdaKw => {
                    self.next_token()?;
                    let branch = self.parse_lambda_branch()?;
                    
                    if current_func_name.is_some() {
                        current_lambdas.push(branch);
                    } else {
                        // Lambda solta fora de uma assinatura
                        let synthetic_name = Ident::Func("repl_eval_lambda".to_string());
                        declarations.push(TopLevelDecl::Definition {
                            name: synthetic_name,
                            expr: DataExpr::LambdaGroup { branches: vec![branch] },
                        });
                    }
                    pending_attrs.clear();
                }
                // Nota: WithKw, OtherwiseKw e AsKw só são válidos dentro de expressões (guard/lambda)
                // Não devem ser tratados no top-level. Se aparecerem aqui, deixamos o parse_data_expr
                // lidar com eles (ou reportar erro apropriado).
                Token::OtherwiseKw | Token::WithKw | Token::AsKw => {
                    // Pula whitespace e tenta parsear como expressão
                    let expr = self.parse_data_expr()?;
                    let synthetic_name = Ident::Func("repl_eval".to_string());
                    declarations.push(TopLevelDecl::Definition {
                        name: synthetic_name,
                        expr,
                    });
                    pending_attrs.clear();
                }
                Token::Comment => {
                    self.next_token()?;
                }
                other => {
                    // Fallback para expressões ou tokens inesperados
                    let start_span = self.current_span.start;
                    let expr = self.parse_data_expr()?;

                    // VERIFICAÇÃO CRÍTICA: Se há um token empurrado de volta (ex: assinatura de função
                    // detectada dentro de um with), precisamos processá-lo antes de criar a definição sintética
                    if let Some(ref pushed) = self.pushed_back_token {
                        log::debug!("Token empurrado detectado após parse_data_expr: {:?}", pushed);
                        // Se for um FuncIdent, é uma assinatura de função que precisa ser processada
                        if let Token::FuncIdent(name) = pushed.clone() {
                            // Limpa o pushed_back_token pois vamos processar manualmente
                            self.pushed_back_token = None;
                            // Commit lambdas pendentes antes de processar a assinatura
                            commit_lambdas(declarations, current_func_name, current_lambdas);
                            // Processa a assinatura de função
                            self.next_token()?; // Consome o FuncIdent
                            if let Some(Token::DoubleColon) = self.peek_token_safe()? {
                                self.next_token()?; // Consome ::
                                let sig = self.parse_type_signature()?;
                                declarations.push(TopLevelDecl::SignatureDecl {
                                    attrs: std::mem::take(pending_attrs),
                                    name: Ident::Func(name.clone()),
                                    sig,
                                });
                                *current_func_name = Some(Ident::Func(name));
                            }
                            // Agora sim, cria a definição sintética para a expressão anterior (se houver)
                            if expr != DataExpr::Tuple(vec![]) {
                                declarations.push(TopLevelDecl::Definition {
                                    name: Ident::Func("repl_eval".to_string()),
                                    expr,
                                });
                            }
                            pending_attrs.clear();
                            continue;
                        }
                    }

                    // Se parse_data_expr não consumiu nada e é um token válido mas não processado, pulamos
                    if expr == DataExpr::Tuple(vec![]) {
                        if matches!(other, Token::Newline | Token::Whitespace | Token::Comment) {
                            self.next_token()?;
                            continue;
                        }
                        // Se há um token empurrado, deixamos o loop processá-lo na próxima iteração
                        if self.pushed_back_token.is_some() {
                            continue;
                        }
                        return Err(KataError::UnexpectedToken {
                            msg: format!("Token inesperado no Top-Level: {:?}", other),
                            span: (start_span, self.current_span.end)
                        });
                    }

                    let synthetic_name = Ident::Func("repl_eval".to_string());
                    declarations.push(TopLevelDecl::Definition {
                        name: synthetic_name,
                        expr,
                    });
                    pending_attrs.clear();
                }
            }
        }
        Ok(())
    }

    fn parse_implements(&mut self, target_type: Ident) -> Result<TopLevelDecl, KataError> {
        let iface_tok = self.next_token()?.ok_or(KataError::UnexpectedEOF)?;
        let interface = match iface_tok {
            Token::InterfaceIdent(s) => Ident::Interface(s),
            _ => return Err(KataError::UnexpectedToken { msg: "Esperado Identificador de Interface".into(), span: (self.current_span.start, self.current_span.end) }),
        };

        let mut methods = Vec::new();
        let mut current_func_name = None;
        let mut current_lambdas = Vec::new();
        let mut pending_attrs = Vec::new();

        if let Some(Token::Newline) = self.peek_token_safe()? {
            self.next_token()?;
            if let Some(Token::Indent) = self.peek_token_safe()? {
                self.next_token()?;
                
                while let Some(tok) = self.peek_token_safe()? {
                    if tok == Token::Dedent {
                        self.next_token()?;
                        break;
                    }
                    
                    if matches!(tok, Token::Newline | Token::Whitespace | Token::Comment) {
                        self.next_token()?;
                        continue;
                    }

                    self.parse_top_level_into(&mut methods, &mut current_func_name, &mut current_lambdas, &mut pending_attrs, true)?;
                }
                
                // Flush lambdas do bloco implements
                if let Some(n) = current_func_name {
                    if !current_lambdas.is_empty() {
                        methods.push(TopLevelDecl::Definition {
                            name: n,
                            expr: DataExpr::LambdaGroup { branches: current_lambdas },
                        });
                    }
                }
            }
        }

        Ok(TopLevelDecl::Implements { target_type, interface, methods })
    }

    fn parse_type_signature(&mut self) -> Result<TypeSignature, KataError> {
        let mut args = Vec::new();
        while let Some(tok) = self.peek_token_safe()? {
            match tok {
                Token::TypeIdent(mut t) => {
                    self.next_token()?;
                    while let Some(Token::DoubleColon) = self.peek_token_safe()? {
                        self.next_token()?;
                        if let Some(Token::TypeIdent(inner)) = self.peek_token_safe()? {
                            self.next_token()?;
                            t.push_str("::");
                            t.push_str(&inner);
                        } else if let Some(Token::InterfaceIdent(inner)) = self.peek_token_safe()? {
                            self.next_token()?;
                            t.push_str("::");
                            t.push_str(&inner);
                        } else {
                            break;
                        }
                    }
                    args.push(Ident::Type(t));
                }
                Token::InterfaceIdent(t) => {
                    self.next_token()?;
                    args.push(Ident::Interface(t));
                }
                Token::LParen | Token::RParen => {
                    self.next_token()?;
                }
                Token::LBracket => {
                    // Açúcar sintático: [T] -> List::T
                    self.next_token()?; // consome '['
                    // Parse o tipo interno recursivamente
                    let inner_type = self.parse_list_type_argument()?;
                    args.push(Ident::Type(format!("List::{}" , inner_type)));
                }
                Token::FatArrow => {
                    self.next_token()?;
                    break;
                }
                Token::Newline | Token::Dedent | Token::Indent | Token::Whitespace => {
                    self.next_token()?;
                }
                _ => { self.next_token()?; }
            }
        }
        
        while let Some(tok) = self.peek_token_safe()? {
            if matches!(tok, Token::Whitespace | Token::Indent | Token::Newline) {
                self.next_token()?;
            } else {
                break;
            }
        }

        let ret_tok_res = self.next_token()?.ok_or(KataError::UnexpectedEOF)?;
        let ret = match ret_tok_res {
            Token::TypeIdent(mut t) => {
                while let Some(Token::DoubleColon) = self.peek_token_safe()? {
                    self.next_token()?;
                    if let Some(Token::TypeIdent(inner)) = self.peek_token_safe()? {
                        self.next_token()?;
                        t.push_str("::");
                        t.push_str(&inner);
                    } else { break; }
                }
                Ident::Type(t)
            },
            Token::InterfaceIdent(t) => Ident::Interface(t),
            Token::LParen => {
                // Tupla de tipos no retorno (ex: (Tx Rx) ou ())
                let mut inner_types = Vec::new();
                while let Some(tok) = self.peek_token_safe()? {
                    if tok == Token::RParen {
                        self.next_token()?;
                        break;
                    }
                    match tok {
                        Token::TypeIdent(mut t) => {
                            self.next_token()?;
                            while let Some(Token::DoubleColon) = self.peek_token_safe()? {
                                self.next_token()?;
                                if let Some(Token::TypeIdent(inner)) = self.peek_token_safe()? {
                                    self.next_token()?;
                                    t.push_str("::");
                                    t.push_str(&inner);
                                } else { break; }
                            }
                            inner_types.push(t);
                        }
                        Token::Whitespace | Token::Newline | Token::Comma => { self.next_token()?; }
                        _ => { return Err(KataError::UnexpectedToken { msg: "Esperado tipo dentro da tupla de retorno".into(), span: (self.current_span.start, self.current_span.end) }); }
                    }
                }
                // Convertemos os tipos para uma string representativa da Tupla ou mantemos Ident::Type
                Ident::Type(format!("({})", inner_types.join(" ")))
            }
            Token::LBracket => {
                // Açúcar sintático: [T] no retorno -> List::T
                // O '[' já foi consumido quando fizemos pattern matching em ret_tok_res
                let inner_type = self.parse_list_type_argument()?;
                Ident::Type(format!("List::{}", inner_type))
            }
            other => return Err(KataError::UnexpectedToken {
                msg: format!("Esperado tipo de retorno (Encontrou: {:?})", other),
                span: (self.current_span.start, self.current_span.end)
            }),
        };

        Ok(TypeSignature { args, ret })
    }

    /// Parse o tipo dentro de [ ] para o açúcar sintático [T] -> List::T
    fn parse_list_type_argument(&mut self) -> Result<String, KataError> {
        let mut result = String::new();
        log::debug!("parse_list_type_argument iniciado");

        while let Some(tok) = self.peek_token_safe()? {
            log::debug!("token no loop: {:?}", tok);
            match tok {
                Token::RBracket => {
                    self.next_token()?; // consome ']'
                    break;
                }
                Token::TypeIdent(mut t) => {
                    self.next_token()?;
                    // Verifica se há ::Tipo após o identificador
                    while let Some(Token::DoubleColon) = self.peek_token_safe()? {
                        self.next_token()?;
                        match self.peek_token_safe()? {
                            Some(Token::TypeIdent(inner)) | Some(Token::InterfaceIdent(inner)) => {
                                self.next_token()?;
                                t.push_str("::");
                                t.push_str(&inner);
                            }
                            _ => break,
                        }
                    }
                    result.push_str(&t);
                }
                Token::InterfaceIdent(t) => {
                    self.next_token()?;
                    result.push_str(&t);
                }
                Token::LBracket => {
                    // Aninhamento: [[Int]]
                    self.next_token()?;
                    let inner = self.parse_list_type_argument()?;
                    result.push_str(&format!("List::({})" , inner));
                }
                Token::Whitespace | Token::Newline | Token::Indent | Token::Dedent => {
                    self.next_token()?;
                }
                _ => {
                    return Err(KataError::UnexpectedToken {
                        msg: format!("Esperado tipo dentro de [ ] (Encontrou: {:?})", tok),
                        span: (self.current_span.start, self.current_span.end),
                    });
                }
            }
        }

        if result.is_empty() {
            return Err(KataError::UnexpectedToken {
                msg: "Tipo vazio dentro de [ ]".into(),
                span: (self.current_span.start, self.current_span.end),
            });
        }

        Ok(result)
    }

    fn parse_import(&mut self) -> Result<TopLevelDecl, KataError> {
        let mut path = Vec::new();
        while let Some(tok) = self.peek_token_safe()? {
            match tok {
                Token::FuncIdent(name) | Token::TypeIdent(name) | Token::InterfaceIdent(name) | Token::SymbolIdent(name) | Token::ActionIdent(name) => {
                    self.next_token()?;
                    path.push(Ident::Func(name));
                }
                Token::SendChan => {
                    self.next_token()?;
                    path.push(Ident::Symbol(">!".into()));
                }
                Token::RecvChan => {
                    self.next_token()?;
                    path.push(Ident::Symbol("<!".into()));
                }
                Token::TryRecvChan => {
                    self.next_token()?;
                    path.push(Ident::Symbol("<!?".into()));
                }
                Token::Dot | Token::DoubleColon => {
                    self.next_token()?; 
                }
                Token::LParen => {
                    self.next_token()?;
                    while let Some(t) = self.peek_token_safe()? {
                        match t {
                            Token::RParen => { self.next_token()?; break; }
                            Token::FuncIdent(n) | Token::TypeIdent(n) | Token::InterfaceIdent(n) | Token::SymbolIdent(n) | Token::ActionIdent(n) => {
                                self.next_token()?;
                                path.push(Ident::Func(n));
                            }
                            Token::SendChan => { self.next_token()?; path.push(Ident::Symbol(">!".into())); }
                            Token::RecvChan => { self.next_token()?; path.push(Ident::Symbol("<!".into())); }
                            Token::TryRecvChan => { self.next_token()?; path.push(Ident::Symbol("<!?".into())); }
                            _ => { self.next_token()?; }
                        }
                    }
                }
                Token::AsKw => {
                    self.next_token()?;
                    break;
                }
                Token::Newline | Token::Dedent | Token::Indent | Token::Comment | Token::Whitespace => {
                    break;
                }
                _ => break,
            }
        }
        
        let mut alias = None;
        if let Some(Token::TypeIdent(name)) = self.peek_token_safe()? {
            self.next_token()?;
            alias = Some(Ident::Type(name));
        }

        Ok(TopLevelDecl::Import { path, alias })
    }

    fn parse_export(&mut self) -> Result<TopLevelDecl, KataError> {
        let mut exports = Vec::new();
        while let Some(tok) = self.peek_token_safe()? {
            match tok {
                Token::FuncIdent(name) => {
                    self.next_token()?;
                    exports.push(Ident::Func(name));
                }
                Token::TypeIdent(name) => {
                    self.next_token()?;
                    exports.push(Ident::Type(name));
                }
                Token::InterfaceIdent(name) => {
                    self.next_token()?;
                    exports.push(Ident::Interface(name));
                }
                Token::SymbolIdent(name) => {
                    self.next_token()?;
                    exports.push(Ident::Symbol(name));
                }
                Token::ActionIdent(name) => {
                    self.next_token()?;
                    exports.push(Ident::Action(name));
                }
                Token::SendChan => {
                    self.next_token()?;
                    exports.push(Ident::Symbol(">!".into()));
                }
                Token::RecvChan => {
                    self.next_token()?;
                    exports.push(Ident::Symbol("<!".into()));
                }
                Token::Newline | Token::Dedent | Token::Indent | Token::Whitespace | Token::Comment => {
                    self.next_token()?;
                }
                _ => break,
            }
        }
        Ok(TopLevelDecl::Export(exports))
    }

    fn parse_data_def(&mut self) -> Result<TopLevelDecl, KataError> {
        let name_tok = self.next_token()?.ok_or(KataError::UnexpectedEOF)?;
        let name = match name_tok {
            Token::TypeIdent(s) => Ident::Type(s),
            _ => return Err(KataError::UnexpectedToken { msg: "Esperado nome do Tipo".into(), span: (self.current_span.start, self.current_span.end) }),
        };

        let mut fields = Vec::new();
        if let Some(Token::LParen) = self.peek_token_safe()? {
            self.next_token()?;
            while let Some(tok) = self.peek_token_safe()? {
                if tok == Token::RParen {
                    self.next_token()?;
                    break;
                }
                match tok {
                    Token::FuncIdent(s) | Token::SymbolIdent(s) => {
                        self.next_token()?;
                        fields.push(Ident::Func(s));
                    }
                    _ => { self.next_token()?; }
                }
            }
        }

        Ok(TopLevelDecl::DataDef { name, fields })
    }

    fn parse_enum_def(&mut self) -> Result<TopLevelDecl, KataError> {
        let name_tok = self.next_token()?.ok_or(KataError::UnexpectedEOF)?;
        let name = match name_tok {
            Token::TypeIdent(s) => Ident::Type(s),
            _ => return Err(KataError::UnexpectedToken { msg: "Esperado nome do Enum".into(), span: (self.current_span.start, self.current_span.end) }),
        };

        let mut variants = Vec::new();
        if let Some(Token::Newline) = self.peek_token_safe()? {
            self.next_token()?;
            if let Some(Token::Indent) = self.peek_token_safe()? {
                self.next_token()?;
                while let Some(tok) = self.peek_token_safe()? {
                    if tok == Token::Dedent {
                        self.next_token()?;
                        break;
                    }
                    if let Token::TypeIdent(v) = tok {
                        self.next_token()?;
                        variants.push(Ident::Type(v));
                    } else {
                        self.next_token()?;
                    }
                }
            }
        }

        Ok(TopLevelDecl::EnumDef { name, variants })
    }

    fn parse_action_def(&mut self) -> Result<TopLevelDecl, KataError> {
        let name_tok = self.next_token()?.ok_or(KataError::UnexpectedEOF)?;
        let name = match name_tok {
            Token::ActionIdent(s) => Ident::Action(s),
            Token::FuncIdent(s) => Ident::Action(format!("{}!", s)),
            _ => return Err(KataError::UnexpectedToken { msg: "Esperado nome da Action".into(), span: (self.current_span.start, self.current_span.end) }),
        };

        let mut params = Vec::new();
        if let Some(Token::LParen) = self.peek_token_safe()? {
            self.next_token()?;
            while let Some(tok) = self.peek_token_safe()? {
                if tok == Token::RParen {
                    self.next_token()?;
                    break;
                }
                if let Token::FuncIdent(p) = tok {
                    self.next_token()?;
                    params.push(Pattern::Identifier(Ident::Func(p)));
                } else {
                    self.next_token()?;
                }
            }
        }

        let mut body = Vec::new();
        if let Some(Token::Newline) = self.peek_token_safe()? {
            self.next_token()?;
            if let Some(Token::Indent) = self.peek_token_safe()? {
                self.next_token()?;
                while let Some(tok) = self.peek_token_safe()? {
                    match tok {
                        Token::Dedent => {
                            self.next_token()?;
                            break;
                        }
                        Token::Newline | Token::Whitespace | Token::Comment => {
                            self.next_token()?;
                        }
                        _ => {
                            body.push(self.parse_action_stmt()?);
                        }
                    }
                }
            }
        }

        Ok(TopLevelDecl::ActionDef { attrs: vec![], name, params, body })
    }

    fn parse_action_stmt(&mut self) -> Result<ActionStmt, KataError> {
        while let Some(tok) = self.peek_token_safe()? {
            match tok {
                Token::Newline | Token::Whitespace => { self.next_token()?; }
                _ => break,
            }
        }

        let tok = self.peek_token_safe()?.ok_or(KataError::UnexpectedEOF)?;
        match tok {
            Token::Dedent => {
                // Se chegamos num Dedent aqui, é porque o loop superior deveria ter parado.
                // Retornamos um erro ou um sinal de parada. 
                // Por simplicidade na arquitetura atual, vamos tratar como fim de bloco no chamador.
                Err(KataError::UnexpectedToken { msg: "Fim de bloco inesperado".into(), span: (self.current_span.start, self.current_span.end) })
            }
            Token::LetKw => {
                self.next_token()?;
                // Parse o pattern (identificador ou tupla)
                let pattern = self.parse_pattern()?;
                // Verifica se há anotação de tipo ::Tipo
                let type_annotation = if let Some(Token::DoubleColon) = self.peek_token_safe()? {
                    self.next_token()?; // consome ::
                    // Parse o tipo (pode ser simples ou composto)
                    let mut type_str = String::new();
                    while let Some(tok) = self.peek_token_safe()? {
                        match tok {
                            Token::TypeIdent(t) | Token::InterfaceIdent(t) => {
                                self.next_token()?;
                                type_str.push_str(&t);
                                // Verifica se há mais ::Tipo
                                while let Some(Token::DoubleColon) = self.peek_token_safe()? {
                                    self.next_token()?;
                                    if let Some(Token::TypeIdent(inner)) | Some(Token::InterfaceIdent(inner)) = self.peek_token_safe()? {
                                        self.next_token()?;
                                        type_str.push_str("::");
                                        type_str.push_str(&inner);
                                    } else { break; }
                                }
                                break;
                            }
                            Token::LBracket => {
                                // Açúcar [T] -> List::T
                                self.next_token()?;
                                let inner_type = self.parse_list_type_argument()?;
                                type_str.push_str(&format!("List::{}", inner_type));
                                break;
                            }
                            Token::Whitespace | Token::Newline => {
                                self.next_token()?;
                                if !type_str.is_empty() { break; }
                            }
                            _ => {
                                if type_str.is_empty() {
                                    return Err(KataError::UnexpectedToken {
                                        msg: format!("Esperado tipo após :: (Encontrou: {:?})", tok),
                                        span: (self.current_span.start, self.current_span.end),
                                    });
                                }
                                break;
                            }
                        }
                    }
                    if type_str.is_empty() {
                        return Err(KataError::UnexpectedToken {
                            msg: "Esperado tipo após ::".into(),
                            span: (self.current_span.start, self.current_span.end),
                        });
                    }
                    Some(type_str)
                } else {
                    None
                };
                let expr = self.parse_data_expr()?;
                Ok(ActionStmt::LetBind { pattern, expr, type_annotation })
            }
            Token::VarKw => {
                self.next_token()?;
                let name = match self.next_token()?.ok_or(KataError::UnexpectedEOF)? {
                    Token::FuncIdent(s) => Ident::Func(s),
                    _ => return Err(KataError::UnexpectedToken { msg: "Esperado nome da variável".into(), span: (self.current_span.start, self.current_span.end) }),
                };
                let expr = self.parse_data_expr()?;
                Ok(ActionStmt::VarBind { name, expr })
            }
            Token::LoopKw => {
                self.next_token()?;
                let mut body = Vec::new();
                if let Some(Token::Newline) = self.peek_token_safe()? {
                    self.next_token()?;
                    if let Some(Token::Indent) = self.peek_token_safe()? {
                        self.next_token()?;
                        while let Some(tok) = self.peek_token_safe()? {
                            if tok == Token::Dedent {
                                self.next_token()?;
                                break;
                            }
                            body.push(self.parse_action_stmt()?);
                        }
                    }
                }
                Ok(ActionStmt::Loop(body))
            }
            Token::ActionIdent(n) => {
                self.next_token()?;
                let args = match self.parse_data_expr()? {
                    DataExpr::Seq(v) => v,
                    DataExpr::Tuple(v) => v,
                    other => if other == DataExpr::Tuple(vec![]) { vec![] } else { vec![other] }
                };
                Ok(ActionStmt::ActionCall { target: Ident::Action(n), args })
            }
            _ => {
                let start_span = self.current_span.start;
                let next_tok = self.peek_token_safe()?;
                let expr = self.parse_data_expr()?;
                
                // SEGURANÇA: Previne loop infinito se nada for consumido
                if expr == DataExpr::Tuple(vec![]) {
                    return Err(KataError::UnexpectedToken { 
                        msg: format!("Token inesperado dentro de Action: {:?}", next_tok), 
                        span: (start_span, self.current_span.end) 
                    });
                }
                Ok(ActionStmt::Expr(expr))
            }
        }
    }

    fn parse_pattern(&mut self) -> Result<Pattern, KataError> {
        // Pula whitespace/newlines
        while let Some(tok) = self.peek_token_safe()? {
            match tok {
                Token::Whitespace | Token::Newline | Token::Indent | Token::Dedent => {
                    self.next_token()?;
                }
                _ => break,
            }
        }

        let tok = self.next_token()?.ok_or(KataError::UnexpectedEOF)?;
        match tok {
            Token::FuncIdent(s) => {
                // Verifica se há : após (list cons)
                if let Some(Token::Colon) = self.peek_token_safe()? {
                    self.next_token()?; // consome :
                    let tail = self.parse_pattern()?;
                    Ok(Pattern::ListCons {
                        head: Box::new(Pattern::Identifier(Ident::Func(s))),
                        tail: Box::new(tail),
                    })
                } else {
                    Ok(Pattern::Identifier(Ident::Func(s)))
                }
            }
            Token::TypeIdent(s) => Ok(Pattern::Identifier(Ident::Type(s))),
            Token::Hole => Ok(Pattern::Wildcard),
            Token::IntLiteral(n) => Ok(Pattern::Literal(Literal::Int(n))),
            Token::StringLiteral(s) => Ok(Pattern::Literal(Literal::String(s))),
            Token::LBracket => {
                if let Some(Token::RBracket) = self.peek_token_safe()? {
                    self.next_token()?;
                    Ok(Pattern::Literal(Literal::String("[]".to_string()))) // Lista vazia
                } else {
                    // Pattern de lista cons [x:xs] ou [x y z]
                    self.parse_list_pattern()
                }
            }
            Token::LParen => {
                // Pattern de tupla aninhada (a b c) ou (a (b c))
                let mut inner_patterns = Vec::new();

                while let Some(inner_tok) = self.peek_token_safe()? {
                    match inner_tok {
                        Token::RParen => {
                            self.next_token()?;
                            break;
                        }
                        Token::Whitespace | Token::Newline | Token::Indent | Token::Dedent => {
                            self.next_token()?;
                        }
                        Token::Comma => {
                            self.next_token()?; // Ignora vírgula em patterns
                        }
                        _ => {
                            // Recursivamente parse pattern
                            let pat = self.parse_pattern()?;
                            inner_patterns.push(pat);
                        }
                    }
                }

                if inner_patterns.len() == 1 {
                    Ok(inner_patterns.pop().unwrap())
                } else {
                    Ok(Pattern::Tuple(inner_patterns))
                }
            }
            _ => Err(KataError::UnexpectedToken {
                msg: format!("Pattern inválido: {:?}", tok),
                span: (self.current_span.start, self.current_span.end)
            }),
        }
    }

    /// Parse um pattern de lista [x:xs] ou [x y z]
    fn parse_list_pattern(&mut self) -> Result<Pattern, KataError> {
        let first = self.parse_pattern()?;

        // Verifica se é list cons com :
        if let Some(Token::Colon) = self.peek_token_safe()? {
            self.next_token()?; // consome :
            let tail = self.parse_list_pattern_tail()?;
            self.expect(Token::RBracket, "Esperado ']' em pattern de lista")?;
            Ok(Pattern::ListCons {
                head: Box::new(first),
                tail: Box::new(tail),
            })
        } else {
            // Lista com múltiplos elementos ou fim da lista
            // Para simplificar, tratamos como tupla dentro de lista
            let mut items = vec![first];
            while let Some(tok) = self.peek_token_safe()? {
                match tok {
                    Token::RBracket => {
                        self.next_token()?;
                        break;
                    }
                    Token::Whitespace | Token::Newline | Token::Comma => {
                        self.next_token()?;
                    }
                    Token::Colon => {
                        // List cons no meio da lista
                        self.next_token()?;
                        let tail = self.parse_list_pattern_tail()?;
                        self.expect(Token::RBracket, "Esperado ']' em pattern de lista")?;
                        return Ok(Pattern::ListCons {
                            head: Box::new(Pattern::Tuple(items)),
                            tail: Box::new(tail),
                        });
                    }
                    _ => {
                        let pat = self.parse_pattern()?;
                        items.push(pat);
                    }
                }
            }
            Ok(Pattern::Tuple(items))
        }
    }

    /// Parse o resto de um pattern de lista após o :
    fn parse_list_pattern_tail(&mut self) -> Result<Pattern, KataError> {
        while let Some(tok) = self.peek_token_safe()? {
            match tok {
                Token::Whitespace | Token::Newline => {
                    self.next_token()?;
                }
                _ => break,
            }
        }

        match self.peek_token_safe()? {
            Some(Token::RBracket) => {
                // [x:] - tail é lista vazia
                Ok(Pattern::Literal(Literal::String("[]".to_string())))
            }
            _ => self.parse_pattern(),
        }
    }

    fn parse_data_expr(&mut self) -> Result<DataExpr, KataError> {
        let mut items = Vec::new();

        log::debug!("parse_data_expr iniciado, pushed_back_token: {:?}", self.pushed_back_token);

        // VERIFICAÇÃO CRÍTICA: Se há um token empurrado de volta (ex: assinatura de função
        // detectada dentro de um with), precisamos parar imediatamente para não consumi-lo.
        // Isso permite que o token seja processado pelo contexto superior (top-level).
        if self.pushed_back_token.is_some() {
            log::debug!("Token empurrado detectado no início de parse_data_expr, parando imediatamente");
            return Ok(DataExpr::Tuple(vec![]));
        }

        while let Some(tok) = self.peek_token_safe()? {
            match tok {
                Token::Newline | Token::Dedent | Token::RParen | Token::RBracket | Token::RBrace | Token::FatArrow | Token::Arrow | Token::Semicolon => {
                    break;
                }
                Token::Comma => {
                    // Vírgula é separador opcional - consumimos e continuamos
                    self.next_token()?;
                    continue;
                }
                Token::Indent => {
                    self.next_token()?; // Ignora indentação
                }
                Token::Whitespace => {
                    self.next_token()?;
                }
                Token::Pipe => {
                    self.next_token()?;
                    // Operador Pipe: left |> right
                    // Constrói um Pipe node com o item anterior (left) e o resto da expressão (right)
                    if let Some(left) = items.pop() {
                        let right = self.parse_data_expr()?;
                        items.push(DataExpr::Pipe {
                            left: Box::new(left),
                            right: Box::new(right),
                        });
                    } else {
                        return Err(KataError::UnexpectedToken {
                            msg: "Operador |> precisa de um operando à esquerda".into(),
                            span: (self.current_span.start, self.current_span.end),
                        });
                    }
                }
                Token::LParen => {
                    self.next_token()?;
                    let inner = self.parse_data_expr()?;
                    self.expect(Token::RParen, "Esperado ')'")?;
                    items.push(inner);
                }
                Token::LBracket => {
                    self.next_token()?;
                    if let Some(Token::RBracket) = self.peek_token_safe()? {
                        self.next_token()?;
                        items.push(DataExpr::Identifier(Ident::Type("List".into())));
                    } else if self.is_range_syntax()? {
                        // Parse como Range: [start..end] ou [start..=end] ou [..end] ou [start..]
                        let range = self.parse_range()?;
                        self.expect(Token::RBracket, "Esperado ']'")?;
                        items.push(range);
                    } else {
                        let inner = self.parse_data_expr()?;
                        self.expect(Token::RBracket, "Esperado ']'")?;
                        let mut seq = vec![DataExpr::Identifier(Ident::Type("List".into()))];
                        match inner {
                            DataExpr::Seq(mut v) => seq.append(&mut v),
                            DataExpr::Tuple(v) => seq.extend(v),
                            _ => seq.push(inner),
                        }
                        items.push(DataExpr::Seq(seq));
                    }
                }
                Token::LBrace => {
                    self.next_token()?;
                    // Verifica se há ; dentro (tensor) ou não (array)
                    let mut tensor_rows = Vec::new();
                    let mut current_row = Vec::new();
                    let mut is_tensor = false;
                    let mut brace_depth = 1;

                    while let Some(tok) = self.peek_token_safe()? {
                        match tok {
                            Token::RBrace => {
                                self.next_token()?;
                                brace_depth -= 1;
                                if brace_depth == 0 {
                                    break;
                                }
                            }
                            Token::LBrace => {
                                self.next_token()?;
                                brace_depth += 1;
                            }
                            Token::Semicolon => {
                                self.next_token()?;
                                is_tensor = true;
                                if !current_row.is_empty() {
                                    tensor_rows.push(std::mem::take(&mut current_row));
                                }
                            }
                            Token::Newline | Token::Whitespace | Token::Indent | Token::Dedent => {
                                self.next_token()?;
                            }
                            Token::Comma => {
                                self.next_token()?; // Ignora vírgula em tensores
                            }
                            _ => {
                                // Elementos simples: literais e identificadores
                                match tok {
                                    Token::IntLiteral(n) => {
                                        self.next_token()?;
                                        current_row.push(DataExpr::Literal(Literal::Int(n)));
                                    }
                                    Token::FloatLiteral(n) => {
                                        self.next_token()?;
                                        current_row.push(DataExpr::Literal(Literal::Float(n)));
                                    }
                                    Token::FuncIdent(s) | Token::SymbolIdent(s) => {
                                        self.next_token()?;
                                        current_row.push(DataExpr::Identifier(Ident::Func(s)));
                                    }
                                    Token::TypeIdent(s) => {
                                        self.next_token()?;
                                        current_row.push(DataExpr::Identifier(Ident::Type(s)));
                                    }
                                    _ => {
                                        // Outros tokens, consome e ignora
                                        self.next_token()?;
                                    }
                                }
                            }
                        }
                    }

                    if is_tensor {
                        if !current_row.is_empty() {
                            tensor_rows.push(current_row);
                        }
                        // Calcula dimensões
                        let num_rows = tensor_rows.len();
                        let num_cols = if num_rows > 0 { tensor_rows[0].len() } else { 0 };
                        let dimensions = vec![num_rows, num_cols];
                        let elements = tensor_rows.into_iter().flatten().collect();
                        items.push(DataExpr::Tensor { elements, dimensions });
                    } else {
                        // Array normal - comportamento anterior
                        let inner = if current_row.len() == 1 {
                            current_row.pop().unwrap_or(DataExpr::Tuple(vec![]))
                        } else {
                            DataExpr::Seq(current_row)
                        };
                        items.push(inner);
                    }
                }
                Token::LambdaKw => {
                    self.next_token()?;
                    let branch = self.parse_lambda_branch()?;
                    items.push(DataExpr::LambdaGroup { branches: vec![branch] });
                }
                Token::IntLiteral(n) => {
                    self.next_token()?;
                    items.push(DataExpr::Literal(Literal::Int(n)));
                }
                Token::FloatLiteral(n) => {
                    self.next_token()?;
                    items.push(DataExpr::Literal(Literal::Float(n)));
                }
                Token::StringLiteral(s) => {
                    self.next_token()?;
                    items.push(DataExpr::Literal(Literal::String(s)));
                }
                Token::Hole => {
                    self.next_token()?;
                    // Hole (_) em uma expressão de dados vira um identificador especial
                    items.push(DataExpr::Identifier(Ident::Func("_".to_string())));
                }
                Token::FuncIdent(s) => {
                    self.next_token()?;
                    // Verifica se há dot notation (module.func ou obj.field)
                    let mut ident_expr = DataExpr::Identifier(Ident::Func(s));
                    if let Some(Token::Dot) = self.peek_token_safe()? {
                        ident_expr = self.try_parse_dot_access(ident_expr)?;
                    }
                    // Se depois do possível dot access vier :, então é um guard
                    if let Some(Token::Colon) = self.peek_token_safe()? {
                        self.next_token()?; // Consome o ':' do primeiro branch
                        // Guard precisa do nome da função como condição
                        // Extrai o nome se for FieldAccess simples, ou usa o ident
                        let guard_name = match &ident_expr {
                            DataExpr::Identifier(Ident::Func(name)) => name.clone(),
                            DataExpr::FieldAccess { field, .. } => field.clone(),
                            _ => "guard".to_string(),
                        };
                        let first_result = self.parse_data_expr()?;
                        let mut branches = vec![GuardBranch {
                            condition: DataExpr::Identifier(Ident::Func(guard_name)),
                            result: first_result
                        }];
                        let mut otherwise = Box::new(DataExpr::Tuple(vec![]));
                        let mut with_clauses = Vec::new();

                        // Loop para capturar ramos ADICIONAIS e WITH
                        while let Some(tok) = self.peek_token_safe()? {
                            match tok {
                                Token::Newline | Token::Whitespace => { self.next_token()?; }
                                Token::FuncIdent(ref next_s) => {
                                    // Verificação manual de Peek(2) para ver se é 'ident:'
                                    // Primeiro verifica se o próximo após FuncIdent é ':'
                                    self.next_token()?; // consome FuncIdent
                                    if let Some(Token::Colon) = self.peek_token_safe()? {
                                        self.next_token()?; // consome ':'
                                        let result = self.parse_data_expr()?;
                                        branches.push(GuardBranch {
                                            condition: DataExpr::Identifier(Ident::Func(next_s.clone())),
                                            result,
                                        });
                                    } else {
                                        // Não é um ident:, empurra o token de volta e sai
                                        self.push_back_token(Token::FuncIdent(next_s.clone()));
                                        break;
                                    }
                                }
                                Token::OtherwiseKw => {
                                    self.next_token()?;
                                    // OtherwiseKw já inclui o ':', não precisamos consumir novamente
                                    otherwise = Box::new(self.parse_data_expr()?);
                                }
                                Token::WithKw => {
                                    self.next_token()?;
                                    'with_loop: while let Some(t) = self.peek_token_safe()? {
                                        log::debug!("Token no with loop: {:?}", t);
                                        if matches!(t, Token::Newline | Token::Dedent | Token::Whitespace | Token::Indent) { self.next_token()?; continue; }
                                        match t {
                                            Token::FuncIdent(name) => {
                                                log::debug!("With binding candidato: {}", name);
                                                // CONSOME o FuncIdent primeiro
                                                self.next_token()?;
                                                // Agora verifica se o próximo é ::
                                                let is_signature = self.is_next_token_doublecolon()?;
                                                log::debug!("Próximo é ::? {}", is_signature);

                                                if is_signature {
                                                    log::debug!("{} é assinatura de função, saindo do with", name);
                                                    // Re-empurra o FuncIdent para ser reprocessado pelo parser principal
                                                    self.push_back_token(Token::FuncIdent(name));
                                                    break 'with_loop;
                                                }
                                                // Verifica se há anotação de tipo ::Tipo no binding
                                                let type_annotation = if let Some(Token::DoubleColon) = self.peek_token_safe()? {
                                                    self.next_token()?; // consome ::
                                                    // Parse o tipo (pode ser simples ou composto)
                                                    let mut type_str = String::new();
                                                    while let Some(tok) = self.peek_token_safe()? {
                                                        match tok {
                                                            Token::TypeIdent(t) | Token::InterfaceIdent(t) => {
                                                                self.next_token()?;
                                                                type_str.push_str(&t);
                                                                // Verifica se há mais ::Tipo
                                                                while let Some(Token::DoubleColon) = self.peek_token_safe()? {
                                                                    self.next_token()?;
                                                                    if let Some(Token::TypeIdent(inner)) | Some(Token::InterfaceIdent(inner)) = self.peek_token_safe()? {
                                                                        self.next_token()?;
                                                                        type_str.push_str("::");
                                                                        type_str.push_str(&inner);
                                                                    } else { break; }
                                                                }
                                                                break; // Terminou o tipo
                                                            }
                                                            Token::AsKw => {
                                                                // Encontrou 'as' sem tipo - erro
                                                                return Err(KataError::UnexpectedToken {
                                                                    msg: "Esperado tipo após :: antes de 'as'".into(),
                                                                    span: (self.current_span.start, self.current_span.end),
                                                                });
                                                            }
                                                            Token::Whitespace | Token::Newline => {
                                                                self.next_token()?;
                                                                if !type_str.is_empty() { break; }
                                                            }
                                                            _ => {
                                                                // Se já temos um tipo, paramos. Senão, é erro.
                                                                if type_str.is_empty() {
                                                                    return Err(KataError::UnexpectedToken {
                                                                        msg: format!("Esperado tipo após :: (Encontrou: {:?})", tok),
                                                                        span: (self.current_span.start, self.current_span.end),
                                                                    });
                                                                }
                                                                break;
                                                            }
                                                        }
                                                    }
                                                    if type_str.is_empty() {
                                                        return Err(KataError::UnexpectedToken {
                                                            msg: "Esperado tipo após ::".into(),
                                                            span: (self.current_span.start, self.current_span.end),
                                                        });
                                                    }
                                                    Some(type_str)
                                                } else {
                                                    None
                                                };
                                                self.expect(Token::AsKw, "Esperado 'as' na cláusula with")?;
                                                // Consumir '=' opcional (para compatibilidade com 'fizz as = 0 (mod x 3)')
                                                if let Some(Token::SymbolIdent(ref s)) = self.peek_token_safe()? {
                                                    if s == "=" {
                                                        self.next_token()?;
                                                    }
                                                }
                                                let expr = self.parse_data_expr()?;
                                                with_clauses.push(Binding {
                                                    pattern: Pattern::Identifier(Ident::Func(name)),
                                                    expr,
                                                    type_annotation,
                                                });
                                            }
                                            _ => {
                                                break;
                                            }
                                        }
                                    }
                                    break;
                                }
                                _ => break,
                            }
                        }

                        log::debug!("Saindo do with, pushed_back_token: {:?}", self.pushed_back_token);

                        // VERIFICAÇÃO CRÍTICA: Se há um token empurrado (assinatura de função detectada),
                        // precisamos sair do parse_data_expr imediatamente para não consumir o token.
                        // Retornamos o GuardBlock construído até agora.
                        if self.pushed_back_token.is_some() {
                            log::debug!("Token empurrado detectado após with, saindo de parse_data_expr");
                            // Constrói o GuardBlock com o que temos até agora
                            let guard_block = DataExpr::GuardBlock { branches, otherwise, with_clauses };
                            items.push(guard_block);
                            // Retorna imediatamente - o token empurrado será processado pelo top-level
                            return if items.len() == 1 {
                                Ok(items.pop().unwrap())
                            } else {
                                Ok(DataExpr::Seq(items))
                            };
                        }

                        items.push(DataExpr::GuardBlock { branches, otherwise, with_clauses });
                    } else {
                        items.push(ident_expr);
                    }
                }
                Token::SymbolIdent(s) => {
                    self.next_token()?;
                    items.push(DataExpr::Identifier(Ident::Symbol(s)));
                }
                Token::ActionIdent(s) => {
                    self.next_token()?;
                    items.push(DataExpr::Identifier(Ident::Action(s)));
                }
                Token::TypeIdent(s) => {
                    self.next_token()?;
                    let mut t_name = s;
                    while let Some(Token::DoubleColon) = self.peek_token_safe()? {
                        self.next_token()?;
                        if let Some(Token::TypeIdent(inner)) = self.peek_token_safe()? {
                            self.next_token()?;
                            t_name.push_str("::");
                            t_name.push_str(&inner);
                        } else { break; }
                    }
                    items.push(DataExpr::Identifier(Ident::Type(t_name)));
                }
                Token::SendChan => { self.next_token()?; items.push(DataExpr::Identifier(Ident::Symbol(">!".into()))); }
                Token::RecvChan => { self.next_token()?; items.push(DataExpr::Identifier(Ident::Symbol("<!".into()))); }
                Token::TryRecvChan => { self.next_token()?; items.push(DataExpr::Identifier(Ident::Symbol("<!?".into()))); }
                _ => {
                    break;
                }
            }
        }

        if items.is_empty() {
            Ok(DataExpr::Tuple(vec![]))
        } else if items.len() == 1 {
            Ok(items.pop().unwrap())
        } else {
            Ok(DataExpr::Seq(items))
        }
    }

    fn parse_lambda_branch(&mut self) -> Result<LambdaBranch, KataError> {
        // Lambda pode ter pattern entre () ou []
        let mut params = Vec::new();

        match self.peek_token_safe()? {
            Some(Token::LParen) => {
                self.next_token()?; // consome (
                params = self.parse_lambda_params()?;
                self.expect(Token::RParen, "Esperado ')'")?;
            }
            Some(Token::LBracket) => {
                self.next_token()?; // consome [
                // Pattern de lista: [] ou [x:xs]
                if let Some(Token::RBracket) = self.peek_token_safe()? {
                    self.next_token()?;
                    params.push(Pattern::Literal(Literal::String("[]".to_string())));
                } else {
                    let pat = self.parse_pattern()?;
                    self.expect(Token::RBracket, "Esperado ']' em pattern de lista")?;
                    params.push(pat);
                }
            }
            other => {
                return Err(KataError::UnexpectedToken {
                    msg: format!("Esperado '(' ou '[' para parâmetros do lambda (Recebeu: {:?})", other),
                    span: (self.current_span.start, self.current_span.end),
                });
            }
        }

        // Consome newlines iniciais
        while let Some(Token::Newline) = self.peek_token_safe()? {
            self.next_token()?;
        }

        // Para lambdas, o corpo é uma única expressão (que pode ser um GuardBlock)
        // Se o próximo token é Dedent, corpo vazio
        let body = if matches!(self.peek_token_safe()?, Some(Token::Dedent) | None) {
            DataExpr::Tuple(vec![])
        } else {
            self.parse_data_expr()?
        };

        Ok(LambdaBranch { params, body })
    }

    fn parse_lambda_params(&mut self) -> Result<Vec<Pattern>, KataError> {
        let mut params = Vec::new();
        while let Some(tok) = self.peek_token_safe()? {
            if tok == Token::RParen {
                // Não consome aqui - deixa para o caller
                break;
            }
            match tok {
                Token::Whitespace | Token::Newline | Token::Indent | Token::Dedent | Token::Comma => {
                    self.next_token()?;
                }
                _ => {
                    // Usa parse_pattern recursivo para qualquer pattern
                    let pat = self.parse_pattern()?;
                    params.push(pat);
                }
            }
        }

        Ok(params)
    }

    // ==========================================
    // PARSING DE RANGE
    // ==========================================

    /// Verifica se o conteúdo dentro de [ ] é um range (contém .. ou ..=)
    fn is_range_syntax(&mut self) -> Result<bool, KataError> {
        // Faz lookahead: se encontrar DoubleDot logo após um valor ou no início, é range
        // Precisamos checar sem consumir tokens
        let saved_pushed = self.pushed_back_token.clone();

        // Simplesmente verifica se o próximo token é um valor opcional seguido de DoubleDot
        // ou se é DoubleDot direto (range aberto no início)
        match self.peek_token_safe()? {
            Some(Token::DoubleDot) => {
                // [..end] ou [..=end] - range aberto no início
                Ok(true)
            }
            Some(Token::IntLiteral(_)) | Some(Token::FuncIdent(_)) => {
                // Potencialmente [start..end] - precisamos lookahead mais profundo
                // Para simplificar, vamos tentar parse como range se tiver DoubleDot depois
                // Salvamos estado e tentamos parse
                Ok(self.has_doubledot_after_expr()?)
            }
            _ => Ok(false),
        }
    }

    /// Verifica se há um DoubleDot após a próxima expressão simples
    fn has_doubledot_after_expr(&mut self) -> Result<bool, KataError> {
        // Faz lookahead: salva token atual, consome expressão simples, verifica se vem DoubleDot
        // Depois restaura o estado

        // Para simplificar, vamos assumir que se o próximo token é um identificador ou número
        // e depois vem DoubleDot, então é um range
        let mut temp_lexer = self.lexer.clone();

        // Pula o primeiro token (já sabemos que é Int ou FuncIdent)
        let _ = temp_lexer.next();

        // Verifica tokens seguintes até encontrar algo que não seja whitespace ou identificador
        while let Some(Ok((tok, _))) = temp_lexer.next() {
            match tok {
                Token::Whitespace | Token::Newline | Token::Indent | Token::Dedent => continue,
                Token::DoubleDot => return Ok(true),
                _ => return Ok(false),
            }
        }

        Ok(false)
    }

    /// Parse uma expressão de range: [start..end], [start..=end], [..end], [start..]
    /// Assume que o '[' já foi consumido e estamos no primeiro token do conteúdo
    fn parse_range(&mut self) -> Result<DataExpr, KataError> {
        log::debug!("parse_range iniciado");

        // Verifica se começa com .. (range aberto no início)
        if let Some(Token::DoubleDot) = self.peek_token_safe()? {
            self.next_token()?; // consome ..

            // Verifica se é ..= (inclusivo)
            let inclusive = if let Some(Token::DoubleDotEqual) = self.peek_token_safe()? {
                self.next_token()?; // consome ..=
                true
            } else {
                false
            };

            // Parse o end (pode ser vazio para [..] - infinito)
            let end = match self.peek_token_safe()? {
                Some(Token::RBracket) => None,
                _ => {
                    let expr = self.parse_range_bound()?;
                    Some(Box::new(expr))
                }
            };

            log::debug!("parse_range retornando Range aberto no início, inclusive={}", inclusive);
            return Ok(DataExpr::Range {
                start: None,
                end,
                inclusive,
            });
        }

        // Parse o start
        let start = Some(Box::new(self.parse_range_bound()?));

        // Agora esperamos DoubleDot
        match self.peek_token_safe()? {
            Some(Token::DoubleDot) => {
                self.next_token()?; // consome ..

                // Verifica se é ..= (inclusivo)
                let inclusive = if let Some(Token::FatArrow) = self.peek_token_safe()? {
                    // ..= (usamos FatArrow como proxy para = após ..)
                    self.next_token()?; // consome =
                    true
                } else {
                    false
                };

                // Parse o end (opcional)
                let end = match self.peek_token_safe()? {
                    Some(Token::RBracket) => None,
                    _ => {
                        let expr = self.parse_range_bound()?;
                        Some(Box::new(expr))
                    }
                };

                log::debug!("parse_range retornando Range completo");
                Ok(DataExpr::Range {
                    start,
                    end,
                    inclusive,
                })
            }
            other => {
                log::debug!("parse_range esperava DoubleDot, encontrou {:?}", other);
                Err(KataError::UnexpectedToken {
                    msg: format!("Esperado '..' em range (Recebeu: {:?})", other),
                    span: (self.current_span.start, self.current_span.end),
                })
            }
        }
    }

    /// Parse um bound de range (número ou identificador)
    fn parse_range_bound(&mut self) -> Result<DataExpr, KataError> {
        // Pula whitespace/newlines
        while let Some(tok) = self.peek_token_safe()? {
            match tok {
                Token::Whitespace | Token::Newline | Token::Indent | Token::Dedent => {
                    self.next_token()?;
                }
                _ => break,
            }
        }

        match self.peek_token_safe()? {
            Some(Token::IntLiteral(n)) => {
                self.next_token()?;
                Ok(DataExpr::Literal(Literal::Int(n)))
            }
            Some(Token::FuncIdent(name)) => {
                self.next_token()?;
                Ok(DataExpr::Identifier(Ident::Func(name)))
            }
            other => Err(KataError::UnexpectedToken {
                msg: format!("Esperado número ou identificador em range (Recebeu: {:?})", other),
                span: (self.current_span.start, self.current_span.end),
            }),
        }
    }

    // ==========================================
    // PARSING DE DOT NOTATION
    // ==========================================

    /// Verifica se há dot notation após um identificador
    fn try_parse_dot_access(&mut self, base_expr: DataExpr) -> Result<DataExpr, KataError> {
        let mut current = base_expr;

        while let Some(Token::Dot) = self.peek_token_safe()? {
            self.next_token()?; // consome .

            // Esperamos um identificador após o ponto
            match self.peek_token_safe()? {
                Some(Token::FuncIdent(field)) => {
                    self.next_token()?;
                    current = DataExpr::FieldAccess {
                        target: Box::new(current),
                        field,
                    };
                }
                Some(Token::TypeIdent(field)) => {
                    self.next_token()?;
                    current = DataExpr::FieldAccess {
                        target: Box::new(current),
                        field,
                    };
                }
                other => {
                    return Err(KataError::UnexpectedToken {
                        msg: format!("Esperado identificador após '.' (Recebeu: {:?})", other),
                        span: (self.current_span.start, self.current_span.end),
                    });
                }
            }
        }

        Ok(current)
    }
}
