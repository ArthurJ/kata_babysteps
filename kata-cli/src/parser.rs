use crate::ast::*;
use crate::error::KataError;
use crate::lexer::{KataLexer, Token};
use logos::Span;
use std::iter::Peekable;

pub struct Parser<'a> {
    lexer: Peekable<KataLexer<'a>>,
    current_span: Span,
}

impl<'a> Parser<'a> {
    pub fn new(source: &'a str) -> Self {
        let mut lexer = KataLexer::new(source).peekable();
        Self {
            lexer,
            current_span: 0..0,
        }
    }

    fn next_token(&mut self) -> Result<Option<Token>, KataError> {
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
        match self.lexer.peek() {
            Some(Ok((t, _))) => Ok(Some(t.clone())),
            Some(Err(_)) => {
                let err = self.next_token().unwrap_err();
                Err(err)
            }
            None => Ok(None),
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

        // Variáveis de estado para agrupar múltiplos lambdas numa única Definition vinculada à assinatura anterior.
        let mut current_func_name: Option<Ident> = None;
        let mut current_lambdas: Vec<LambdaBranch> = Vec::new();
        // Guarda os atributos (ex: @ffi) pendentes para a próxima declaração TopLevel
        let mut pending_attrs: Vec<TopLevelAttr> = Vec::new();

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
            match token {
                Token::Newline | Token::Indent | Token::Dedent | Token::Whitespace => {
                    self.next_token()?;
                }
                Token::InterfaceKw => {
                    commit_lambdas(&mut declarations, &mut current_func_name, &mut current_lambdas);
                    self.next_token()?;
                    
                    let name_tok = self.next_token()?.ok_or(KataError::UnexpectedEOF)?;
                    let name = match name_tok {
                        Token::InterfaceIdent(s) => Ident::Interface(s),
                        _ => return Err(KataError::UnexpectedToken { msg: "Esperado nome da Interface em ALL_CAPS".into(), span: (0,0) }),
                    };
                    
                    declarations.push(TopLevelDecl::InterfaceDef {
                        name,
                        supertraits: vec![],
                        signatures: vec![], // Corpo ignorado nesta fase 8
                    });
                    pending_attrs.clear();
                }
                Token::Annotation(name) => {
                    self.next_token()?;
                    let mut args = Vec::new();
                    
                    // Verifica se a anotação tem argumentos (ex: @ffi("nome"))
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
                                self.next_token()?; // Pula virgulas ou espaços
                            }
                        }
                    }
                    
                    pending_attrs.push(TopLevelAttr { name, args });
                }
                Token::ImportKw => {
                    commit_lambdas(&mut declarations, &mut current_func_name, &mut current_lambdas);
                    self.next_token()?;
                    declarations.push(self.parse_import()?);
                    pending_attrs.clear();
                }
                Token::ExportKw => {
                    commit_lambdas(&mut declarations, &mut current_func_name, &mut current_lambdas);
                    self.next_token()?;
                    declarations.push(self.parse_export()?);
                    pending_attrs.clear();
                }
                Token::DataKw => {
                    commit_lambdas(&mut declarations, &mut current_func_name, &mut current_lambdas);
                    self.next_token()?;
                    declarations.push(self.parse_data_def()?);
                    pending_attrs.clear();
                }
                Token::EnumKw => {
                    commit_lambdas(&mut declarations, &mut current_func_name, &mut current_lambdas);
                    self.next_token()?;
                    declarations.push(self.parse_enum_def()?);
                    pending_attrs.clear();
                }
                Token::ActionKw => {
                    commit_lambdas(&mut declarations, &mut current_func_name, &mut current_lambdas);
                    self.next_token()?;
                    
                    let mut def = self.parse_action_def()?;
                    if let TopLevelDecl::ActionDef { ref mut attrs, .. } = def {
                        *attrs = std::mem::take(&mut pending_attrs);
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

                    // Verifica se é uma assinatura (::)
                    if let Some(Token::DoubleColon) = self.peek_token_safe()? {
                        commit_lambdas(&mut declarations, &mut current_func_name, &mut current_lambdas);
                        self.next_token()?; // consome `::`
                        let sig = self.parse_type_signature()?;
                        declarations.push(TopLevelDecl::SignatureDecl {
                            attrs: std::mem::take(&mut pending_attrs),
                            name: ident.clone(),
                            sig,
                        });
                        current_func_name = Some(ident); // Prepara para receber os lambdas abaixo
                    } else {
                        // Trata expressões soltas (Para suporte REPL iterativo)
                        let mut seq = vec![DataExpr::Identifier(ident)];
                        let mut rest = self.parse_data_expr()?;
                        if let DataExpr::Seq(mut v) = rest {
                            seq.append(&mut v);
                        } else {
                            if rest != DataExpr::Tuple(vec![]) {
                                seq.push(rest);
                            }
                        }
                        
                        let synthetic_name = Ident::Func("repl_eval".to_string());
                        declarations.push(TopLevelDecl::Definition {
                            name: synthetic_name.clone(),
                            expr: if seq.len() == 1 { seq.pop().unwrap() } else { DataExpr::Seq(seq) },
                        });
                        pending_attrs.clear();
                    }
                }
                Token::SendChan | Token::RecvChan | Token::TryRecvChan => {
                    // Operadores Especiais CSP que também recebem assinatura na StdLib
                    let ident = match token {
                        Token::SendChan => Ident::Symbol(">!".into()),
                        Token::RecvChan => Ident::Symbol("<!".into()),
                        Token::TryRecvChan => Ident::Symbol("<!?".into()),
                        _ => unreachable!(),
                    };
                    self.next_token()?;
                    
                    if let Some(Token::DoubleColon) = self.peek_token_safe()? {
                        commit_lambdas(&mut declarations, &mut current_func_name, &mut current_lambdas);
                        self.next_token()?; // consome `::`
                        let sig = self.parse_type_signature()?;
                        declarations.push(TopLevelDecl::SignatureDecl {
                            attrs: std::mem::take(&mut pending_attrs),
                            name: ident.clone(),
                            sig,
                        });
                        current_func_name = Some(ident);
                    } else {
                        let mut seq = vec![DataExpr::Identifier(ident)];
                        let mut rest = self.parse_data_expr()?;
                        if let DataExpr::Seq(mut v) = rest { seq.append(&mut v); } else if rest != DataExpr::Tuple(vec![]) { seq.push(rest); }
                        declarations.push(TopLevelDecl::Definition { name: Ident::Func("repl_eval".into()), expr: DataExpr::Seq(seq) });
                    }
                }
                Token::TypeIdent(name) => {
                    let ident = Ident::Type(name.clone());
                    self.next_token()?;
                    
                    if let Some(Token::ImplementsKw) = self.peek_token_safe()? {
                        commit_lambdas(&mut declarations, &mut current_func_name, &mut current_lambdas);
                        self.next_token()?; // consome `implements`
                        declarations.push(self.parse_implements(ident)?);
                        pending_attrs.clear();
                    } else {
                        let _ = self.parse_data_expr()?; // descarta expressao errada top-level
                        pending_attrs.clear();
                    }
                }
                Token::LambdaKw => {
                    self.next_token()?;
                    let branch = self.parse_lambda_branch()?;
                    current_lambdas.push(branch);
                    pending_attrs.clear(); // Lambdas pertencem à signature anterior
                }
                Token::ActionIdent(name) => {
                    // Execução de Action top-level (ex: `main!`)
                    commit_lambdas(&mut declarations, &mut current_func_name, &mut current_lambdas);
                    self.next_token()?;
                    
                    let mut seq = vec![DataExpr::Identifier(Ident::Action(name.clone()))];
                    let rest = self.parse_data_expr()?;
                    if let DataExpr::Seq(mut v) = rest {
                        seq.append(&mut v);
                    } else if rest != DataExpr::Tuple(vec![]) {
                        seq.push(rest);
                    }
                    
                    // Envelopa a chamada top-level sinteticamente
                    let synthetic_name = Ident::Func("repl_eval".to_string());
                    declarations.push(TopLevelDecl::Definition {
                        name: synthetic_name.clone(),
                        expr: if seq.len() == 1 { seq.pop().unwrap() } else { DataExpr::Seq(seq) },
                    });
                    pending_attrs.clear();
                }
                Token::Comment => {
                    self.next_token()?;
                }
                _ => {
                    // Expressão solta (ex: REPL `+ 10 5` sem identificador no início)
                    let expr = self.parse_data_expr()?;
                    let synthetic_name = Ident::Func("repl_eval".to_string());
                    declarations.push(TopLevelDecl::Definition {
                        name: synthetic_name.clone(),
                        expr,
                    });
                    pending_attrs.clear();
                    
                    // SEGURANÇA CONTRA INFINITE LOOP: Se cairmos no fallback e ele for a última coisa, garanta consumo
                    if self.peek_token_safe()?.is_none() {
                        break;
                    }
                }
            }
        }
        
        commit_lambdas(&mut declarations, &mut current_func_name, &mut current_lambdas);
        Ok(ModuleAST { declarations })
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
                    self.next_token()?; // Consome delimitador de namespace
                }
                Token::LParen => {
                    // Importação agrupada ex: import types::(Int Float)
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
                Token::TryRecvChan => {
                    self.next_token()?;
                    exports.push(Ident::Symbol("<!?".into()));
                }
                Token::Newline | Token::Dedent | Token::Indent | Token::Comment | Token::Whitespace => {
                    break;
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
            _ => return Err(KataError::UnexpectedEOF),
        };
        // O corpo dos dados pode ser inline `data Vec (x y)` ou identado com `as` refinado.
        // Simplificaremos consumindo tokens até o final da linha e ignorando os fields.
        let _ = self.parse_data_expr()?;
        Ok(TopLevelDecl::DataDef { name, fields: vec![] })
    }

    fn parse_enum_def(&mut self) -> Result<TopLevelDecl, KataError> {
        let name_tok = self.next_token()?.ok_or(KataError::UnexpectedEOF)?;
        let name = match name_tok {
            Token::TypeIdent(s) => Ident::Type(s),
            _ => return Err(KataError::UnexpectedEOF),
        };
        // Simplificando o consumo do corpo
        let _ = self.parse_data_expr()?;
        Ok(TopLevelDecl::EnumDef { name, variants: vec![] })
    }

    fn parse_implements(&mut self, target_type: Ident) -> Result<TopLevelDecl, KataError> {
        let iface_tok = self.next_token()?.ok_or(KataError::UnexpectedEOF)?;
        let interface = match iface_tok {
            Token::InterfaceIdent(s) => Ident::Interface(s),
            _ => return Err(KataError::UnexpectedEOF),
        };

        // Ignora corpo recursivo por enquanto consumindo até achar algo que quebre a indentação
        while let Some(tok) = self.peek_token_safe()? {
            match tok {
                Token::Newline | Token::Indent | Token::Whitespace => {
                    self.next_token()?;
                }
                Token::Dedent => {
                    self.next_token()?;
                    break;
                }
                Token::TypeIdent(_) | Token::FuncIdent(_) | Token::ActionIdent(_) | Token::ExportKw => {
                    // Achou o início de outra declaração top-level
                    break;
                }
                _ => {
                    self.next_token()?;
                }
            }
        }

        Ok(TopLevelDecl::Implements { target_type, interface, methods: vec![] })
    }

    fn parse_type_signature(&mut self) -> Result<TypeSignature, KataError> {
        let mut args = Vec::new();
        
        while let Some(tok) = self.peek_token_safe()? {
            match tok {
                Token::TypeIdent(mut t) => {
                    self.next_token()?;
                    
                    // Verifica se é um tipo genérico/parametrizado (ex: List::Int)
                    while let Some(Token::DoubleColon) = self.peek_token_safe()? {
                        self.next_token()?; // consome ::
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
                    self.next_token()?; // ignora parênteses na assinatura
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
        
        // Consome espaços antes do tipo de retorno
        while let Some(Token::Whitespace | Token::Indent | Token::Newline) = self.peek_token_safe()? {
            self.next_token()?;
        }

        // Tipo de retorno (Também deve suportar generics `List::Text`)
        let ret_tok = self.next_token()?.ok_or(KataError::UnexpectedEOF)?;
        let mut ret = match ret_tok {
            Token::TypeIdent(t) => Ident::Type(t),
            Token::InterfaceIdent(t) => Ident::Interface(t),
            _ => Ident::Type("Unknown".into()),
        };

        if let Ident::Type(mut t) = ret {
             while let Some(Token::DoubleColon) = self.peek_token_safe()? {
                 self.next_token()?; 
                 if let Some(Token::TypeIdent(inner)) = self.peek_token_safe()? {
                     self.next_token()?;
                     t.push_str("::");
                     t.push_str(&inner);
                 } else {
                     break;
                 }
             }
             ret = Ident::Type(t);
        }

        Ok(TypeSignature { args, ret })
    }

    // ==========================================
    // PARSING DE DOMÍNIO PURO E IMPURO
    // ==========================================

    fn parse_lambda_branch(&mut self) -> Result<LambdaBranch, KataError> {
        self.expect(Token::LParen, "Esperado '(' após lambda")?;
        
        let mut params = Vec::new();
        while let Some(tok) = self.peek_token_safe()? {
            if tok == Token::RParen { 
                self.next_token()?; 
                break; 
            }
            params.push(self.parse_pattern()?);
        }

        // Checa se o corpo abre com indentação (bloco multi-linha)
        let mut is_block = false;
        if let Some(Token::Newline) = self.peek_token_safe()? {
            self.next_token()?;
            if let Some(Token::Indent) = self.peek_token_safe()? {
                self.next_token()?;
                is_block = true;
            }
        }

        let mut body_expr: Option<DataExpr> = None;
        let mut guards = Vec::new();
        let mut otherwise_expr: Option<DataExpr> = None;
        let mut with_bindings = Vec::new();

        if is_block {
            loop {
                match self.peek_token_safe()? {
                    Some(Token::Dedent) | None => {
                        self.next_token()?;
                        break;
                    }
                    Some(Token::Newline) | Some(Token::Whitespace) | Some(Token::Comment) => {
                        self.next_token()?;
                    }
                    Some(Token::OtherwiseKw) => {
                        self.next_token()?;
                        otherwise_expr = Some(self.parse_data_expr()?);
                    }
                    Some(Token::WithKw) => {
                        self.next_token()?;
                        // Se houver Newline+Indent após with
                        if let Some(Token::Newline) = self.peek_token_safe()? { self.next_token()?; }
                        let has_with_block = if let Some(Token::Indent) = self.peek_token_safe()? { self.next_token()?; true } else { false };
                        
                        loop {
                            match self.peek_token_safe()? {
                                Some(Token::Dedent) if has_with_block => { self.next_token()?; break; }
                                Some(Token::Dedent) | None => { break; } // Termina o escopo principal
                                Some(Token::Newline) | Some(Token::Whitespace) => { self.next_token()?; }
                                _ => {
                                    // Parse: nome as expr
                                    let pat = self.parse_pattern()?;
                                    if let Some(Token::AsKw) = self.peek_token_safe()? {
                                        self.next_token()?;
                                    }
                                    let val = self.parse_data_expr()?;
                                    with_bindings.push(Binding { pattern: pat, expr: val });
                                    if !has_with_block { break; }
                                }
                            }
                        }
                    }
                    Some(_) => {
                        // Pode ser uma expressão plana ou um Guard (condition: result)
                        let expr = self.parse_data_expr()?;
                        
                        // O parse_data_expr vai parar no `:` se achar um (pois é separador estrito)
                        if let Some(Token::Colon) = self.peek_token_safe()? {
                            self.next_token()?; // consome `:`
                            let result = self.parse_data_expr()?;
                            guards.push(GuardBranch { condition: expr, result });
                        } else {
                            // Era só uma expressão
                            if body_expr.is_none() {
                                body_expr = Some(expr);
                            }
                        }
                    }
                }
            }
        } else {
            // Corpo em uma única linha
            body_expr = Some(self.parse_data_expr()?);
        }

        // Montagem Final do Corpo
        let mut final_expr = if !guards.is_empty() {
            DataExpr::GuardBlock {
                branches: guards,
                otherwise: Box::new(otherwise_expr.unwrap_or(DataExpr::Tuple(vec![]))),
            }
        } else {
            body_expr.unwrap_or(DataExpr::Tuple(vec![]))
        };

        if !with_bindings.is_empty() {
            final_expr = DataExpr::ScopedBlock {
                bindings: vec![], // let inline não parseado aqui ainda
                body: Box::new(final_expr),
                with_clauses: with_bindings,
            };
        }

        Ok(LambdaBranch { params, body: final_expr })
    }

    fn parse_pattern(&mut self) -> Result<Pattern, KataError> {
        let tok = self.next_token()?.ok_or(KataError::UnexpectedEOF)?;
        match tok {
            Token::Hole => Ok(Pattern::Wildcard),
            Token::IntLiteral(n) => Ok(Pattern::Literal(Literal::Int(n))),
            Token::FloatLiteral(n) => Ok(Pattern::Literal(Literal::Float(n))),
            Token::StringLiteral(s) => Ok(Pattern::Literal(Literal::String(s))),
            Token::FuncIdent(name) => Ok(Pattern::Identifier(Ident::Func(name))),
            Token::LParen => {
                // tuple de padrões ou cons
                let mut inner = Vec::new();
                while let Some(t) = self.peek_token_safe()? {
                    if t == Token::RParen {
                        self.next_token()?;
                        break;
                    }
                    if t == Token::Colon {
                        self.next_token()?; // Destructuring list (x:xs)
                        let tail = self.parse_pattern()?;
                        self.expect(Token::RParen, "Falta fechar parêntese do destructuring da lista")?;
                        
                        // Simplifica retornando ListCons direto
                        let head = inner.pop().unwrap_or(Pattern::Wildcard);
                        return Ok(Pattern::ListCons { head: Box::new(head), tail: Box::new(tail) });
                    }
                    inner.push(self.parse_pattern()?);
                }
                Ok(Pattern::Tuple(inner))
            }
            Token::LBracket => {
                // Array/Lista vazia `[]` nos patterns
                self.expect(Token::RBracket, "Expected ']'")?;
                Ok(Pattern::Identifier(Ident::Type("EmptyList".to_string())))
            }
            Token::Whitespace | Token::Comma => self.parse_pattern(), // skip
            _ => Ok(Pattern::Wildcard), // Fallback de robustez
        }
    }

    fn parse_action_def(&mut self) -> Result<TopLevelDecl, KataError> {
        let name_token = self.next_token()?.ok_or(KataError::UnexpectedEOF)?;
        let name = match name_token {
            Token::FuncIdent(s) | Token::ActionIdent(s) => Ident::Func(s),
            _ => return Err(KataError::UnexpectedEOF),
        };

        let mut params = Vec::new();
        if let Some(Token::LParen) = self.peek_token_safe()? {
            self.next_token()?; 
            while let Some(tok) = self.peek_token_safe()? {
                if tok == Token::RParen { break; }
                params.push(self.parse_pattern()?);
            }
            self.expect(Token::RParen, "Esperado ')'")?;
        }

        // Action block
        let mut body = Vec::new();
        while let Some(tok) = self.peek_token_safe()? {
            if tok == Token::Newline || tok == Token::Indent {
                self.next_token()?;
                body = self.parse_action_block()?;
                break;
            } else {
                break; // Action top-level sem bloco (ex: mock vazio)
            }
        }

        Ok(TopLevelDecl::ActionDef { attrs: vec![], name, params, body })
    }

    fn parse_action_block(&mut self) -> Result<Vec<ActionStmt>, KataError> {
        let mut stmts = Vec::new();

        while let Some(tok) = self.peek_token_safe()? {
            match tok {
                Token::Dedent | Token::ActionKw | Token::FuncIdent(_) => {
                    // Final do bloco
                    if tok == Token::Dedent { self.next_token()?; }
                    break;
                }
                Token::Newline | Token::Indent | Token::Comment => {
                    self.next_token()?;
                }
                Token::LetKw => {
                    self.next_token()?;
                    let pat = self.parse_pattern()?;
                    let expr = self.parse_data_expr()?;
                    stmts.push(ActionStmt::LetBind { pattern: pat, expr });
                }
                Token::VarKw => {
                    self.next_token()?;
                    let ident_tok = self.next_token()?.ok_or(KataError::UnexpectedEOF)?;
                    let name = match ident_tok {
                        Token::FuncIdent(n) => Ident::Func(n),
                        _ => Ident::Func("unknown".to_string()),
                    };
                    let expr = self.parse_data_expr()?;
                    stmts.push(ActionStmt::VarBind { name, expr });
                }
                Token::ActionIdent(name) => {
                    self.next_token()?;
                    let args_expr = self.parse_data_expr()?;
                    let args = match args_expr {
                        DataExpr::Seq(seq) => seq,
                        _ => vec![args_expr],
                    };
                    stmts.push(ActionStmt::ActionCall { target: Ident::Action(name), args });
                }
                _ => {
                    let expr = self.parse_data_expr()?;
                    stmts.push(ActionStmt::Expr(expr));
                }
            }
        }

        Ok(stmts)
    }

    fn parse_data_expr(&mut self) -> Result<DataExpr, KataError> {
        let mut seq = Vec::new();

        while let Some(tok) = self.peek_token_safe()? {
            match tok {
                Token::Newline | Token::Dedent | Token::RParen |
                Token::RBracket | Token::RBrace | Token::Colon | Token::WithKw | Token::OtherwiseKw => {
                    break;
                }
                Token::LParen => {
                    self.next_token()?;
                    let inner = self.parse_data_expr()?;
                    self.expect(Token::RParen, "Faltando fechamento ')'")?;
                    seq.push(inner);
                }
                Token::LBracket => {
                    self.next_token()?;
                    let inner = self.parse_data_expr()?;
                    self.expect(Token::RBracket, "Faltando fechamento ']'")?;
                    // Array/Lista literal tratado como Função de criação
                    seq.push(DataExpr::Call { 
                        target: Box::new(DataExpr::Identifier(Ident::Type("List".into()))),
                        args: vec![inner],
                    });
                }
                Token::LBrace => {
                    self.next_token()?;
                    let inner = self.parse_data_expr()?;
                    self.expect(Token::RBrace, "Faltando fechamento '}'")?;
                    seq.push(DataExpr::Call { 
                        target: Box::new(DataExpr::Identifier(Ident::Type("Array".into()))),
                        args: vec![inner],
                    });
                }
                Token::IntLiteral(n) => {
                    self.next_token()?;
                    seq.push(DataExpr::Literal(Literal::Int(n)));
                }
                Token::FloatLiteral(n) => {
                    self.next_token()?;
                    seq.push(DataExpr::Literal(Literal::Float(n)));
                }
                Token::StringLiteral(s) => {
                    self.next_token()?;
                    seq.push(DataExpr::Literal(Literal::String(s)));
                }
                Token::FuncIdent(name) => {
                    self.next_token()?;
                    seq.push(DataExpr::Identifier(Ident::Func(name)));
                }
                Token::ActionIdent(name) => {
                    self.next_token()?;
                    seq.push(DataExpr::Identifier(Ident::Action(name)));
                }
                Token::TypeIdent(name) => {
                    self.next_token()?;
                    seq.push(DataExpr::Identifier(Ident::Type(name)));
                }
                Token::InterfaceIdent(name) => {
                    self.next_token()?;
                    seq.push(DataExpr::Identifier(Ident::Interface(name)));
                }
                Token::SymbolIdent(name) => {
                    self.next_token()?;
                    seq.push(DataExpr::Identifier(Ident::Symbol(name)));
                }
                Token::Pipe => {
                    // Sugar para pipelining `|>`
                    self.next_token()?;
                    let right = self.parse_data_expr()?;
                    // Se o lado esquerdo for um Seq bruto, envelopa ele antes do Pipe
                    let left = if seq.len() == 1 { seq.pop().unwrap() } else { DataExpr::Seq(seq) };
                    return Ok(DataExpr::Pipe {
                        left: Box::new(left),
                        right: Box::new(right),
                    });
                }
                Token::DoubleDot => {
                    self.next_token()?;
                    seq.push(DataExpr::Identifier(Ident::Symbol("..".into())));
                }
                Token::Hole => {
                    self.next_token()?;
                    seq.push(DataExpr::Identifier(Ident::Symbol("_".into())));
                }
                Token::Whitespace | Token::Indent | Token::Comma => { 
                    self.next_token()?; 
                }
                _ => {
                    // Token inválido no meio de dados puros (ex: uma keyword).
                    // Para evitar infinite loop no TopLevel, precisamos parar a sequência, mas não podemos congelar.
                    break;
                }
            }
        }

        if seq.is_empty() {
            // Se entramos nesta função e ela falhou imediatamente no primeiro token (gerando empty seq),
            // temos que consumir pelo menos um token para forçar a máquina de estados a andar,
            // senão o `parse_module` vai ficar chamando `parse_data_expr()` para sempre no mesmo token.
            if let Some(_) = self.peek_token_safe()? {
                 self.next_token()?;
            }
            Ok(DataExpr::Tuple(vec![])) // Equivalente ao Unit `()`
        } else if seq.len() == 1 {
            Ok(seq.pop().unwrap())
        } else {
            Ok(DataExpr::Seq(seq))
        }
    }
}