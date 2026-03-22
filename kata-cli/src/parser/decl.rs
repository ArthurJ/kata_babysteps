//! Declaration parsers for Kata Language
//!
//! Parses top-level declarations:
//! - import/export
//! - Function definitions
//! - Action definitions
//! - Data types (structs)
//! - Enum types
//! - Interface definitions
//! - Implementations
//! - Type aliases

use chumsky::prelude::*;
use crate::lexer::{Token, SpannedToken};
use crate::ast::id::{Ident, QualifiedIdent, Directive};
use crate::ast::types::Type;
use crate::ast::stmt::Stmt;
use crate::ast::decl::{
    Module, TopLevel, FunctionDef, ActionDef, DataDef, DataKind, FieldDef,
    EnumDef, VariantDef, VariantPayload, InterfaceDef, InterfaceMember,
    ImplDef, AliasDef, Import, Export,
};
use crate::ast::expr::{LambdaClause, GuardClause, GuardCondition, WithBinding};
use crate::ast::Spanned;
use super::common::{ident, pure_ident, token, newline, indent, dedent, between, separated1, ParserError, ParserSpan};
use super::expr::expression;
use super::r#type::{type_expr, function_sig};
use super::stmt::recursive_statement;
use crate::ast::expr::Expr;

/// Parse a complete module
pub fn module() -> impl Parser<SpannedToken, Module, Error = ParserError> + Clone {
    log::debug!("module(): Starting parser construction");
    // Create expression parser once and share it
    let expr = expression();
    log::debug!("module(): expression() created");

    // Parse a sequence of top-level declarations (including imports/exports anywhere)
    let result = newline().repeated().or_not()
        .ignore_then(
            top_level_decl(expr)
                .padded_by(newline().repeated().or_not())
                .repeated()
        )
        .then_ignore(newline().repeated().or_not())
        .then_ignore(end())
        .map(|declarations| {
            let mut imports = Vec::new();
            let mut exports = Vec::new();
            let mut final_decls = Vec::new();

            for spanned_decl in declarations {
                let span = spanned_decl.span;
                match spanned_decl.node {
                    TopLevel::Import(i) => imports.push(i),
                    TopLevel::Export(e) => exports.extend(e.items),
                    node => final_decls.push(Spanned::new(node, span)),
                }
            }

            Module {
                name: String::new(), // Will be set by caller
                declarations: final_decls,
                exports,
                imports,
            }
        });
    log::debug!("module(): Parser construction complete");
    result
}

/// Parse a top-level declaration (with optional preceding directives)
fn top_level_decl<E>(expr: E) -> impl Parser<SpannedToken, Spanned<TopLevel>, Error = ParserError> + Clone
where
    E: Parser<SpannedToken, Spanned<Expr>, Error = ParserError> + Clone + 'static,
{
    log::debug!("top_level_decl(): Starting");
    // Parse any preceding directives, then the declaration
    let result = directive().repeated()
        .then(top_level(expr))
        .map_with_span(|(directives, mut spanned_decl), span: ParserSpan| {
            // Apply directives to the declaration
            match &mut spanned_decl.node {
                TopLevel::Function(f) => f.directives.extend(directives),
                TopLevel::Action(a) => a.directives.extend(directives),
                _ => {}
            }
            // If directives were present, the span should ideally include them.
            // map_with_span here gives us the span of (directives + top_level).
            Spanned::new(spanned_decl.node, span.into())
        });
    log::debug!("top_level_decl(): Complete");
    result
}

/// Parse a top-level declaration
pub fn top_level<E>(expr: E) -> impl Parser<SpannedToken, Spanned<TopLevel>, Error = ParserError> + Clone
where
    E: Parser<SpannedToken, Spanned<Expr>, Error = ParserError> + Clone + 'static,
{
    log::debug!("top_level(): Starting");
    let result = choice((
        // Import declaration
        import_decl().map(TopLevel::Import),

        // Export declaration
        export_decl().map(TopLevel::Export),

        // Action definition
        action_def(expr.clone()).map(|a| TopLevel::Action(a.node)),

        // Function definition or signature
        function_def(expr.clone()).map(TopLevel::Function),

        // Data definition
        data_def().map(TopLevel::Data),

        // Enum definition
        enum_def().map(TopLevel::Enum),

        // Interface definition
        interface_def(expr.clone()).map(TopLevel::Interface),

        // Implementation
        impl_def(expr.clone()).map(TopLevel::Implements),

        // Type alias
        alias_def().map(TopLevel::Alias),

        // Raw statement (usually top-level action calls)
        recursive_statement(expr).map(|s| TopLevel::Statement(s.node)),
    )).map_with_span(|node, span| Spanned::new(node, span.into()));
    log::debug!("top_level(): Complete");
    result
}

// ============================================================================
// IMPORT AND EXPORT
// ============================================================================

/// Parse an import declaration
fn import_decl() -> impl Parser<SpannedToken, Import, Error = ParserError> + Clone {
    token(Token::Import)
        .ignore_then(choice((
            // import module.(Item1, Item2)
            ident()
                .then_ignore(token(Token::Dot))
                .then_ignore(token(Token::LParen))
                .then(ident().repeated().at_least(1))
                .then_ignore(token(Token::RParen))
                .map(|(module, items)| Import::Items {
                    module,
                    items,
                }),

            // import module.Item
            ident()
                .then_ignore(token(Token::Dot))
                .then(ident())
                .map(|(module, item)| Import::Item { module, item }),

            // import module
            ident().map(|module| Import::Namespace { module }),
        )))
        .then_ignore(newline().or_not())
}

/// Parse an export declaration
fn export_decl() -> impl Parser<SpannedToken, Export, Error = ParserError> + Clone {
    token(Token::Export)
        .ignore_then(ident().map(Ident::new).repeated().at_least(1))
        .then_ignore(newline().repeated().or_not())
        .map(|items| Export { items })
}

// ============================================================================
// FUNCTION DEFINITION
// ============================================================================

/// Parse a function definition: name :: Sig => body
fn function_def<E>(expr: E) -> impl Parser<SpannedToken, FunctionDef, Error = ParserError> + Clone
where
    E: Parser<SpannedToken, Spanned<Expr>, Error = ParserError> + Clone + 'static,
{
    function_name()
        .then_ignore(token(Token::DoubleColon))
        .then(function_sig())
        .then(
            choice((
                lambda_clause(expr.clone()),
                otherwise_clause(expr.clone()),
            ))
            .padded_by(newline().repeated().or_not())
            .repeated()
        )
        .map(|((name, sig), clauses)| {
            let arity = sig.arity();
            FunctionDef {
                name,
                sig,
                arity,
                directives: vec![],
                clauses,
            }
        })
}

/// Parse an otherwise clause: otherwise: body
fn otherwise_clause<E>(expr: E) -> impl Parser<SpannedToken, LambdaClause, Error = ParserError> + Clone
where
    E: Parser<SpannedToken, Spanned<Expr>, Error = ParserError> + Clone + 'static,
{
    token(Token::Otherwise)
        .ignore_then(token(Token::Colon))
        .ignore_then(expr)
        .map(|body| {
            LambdaClause {
                patterns: vec![], // Will be filled with wildcards if empty during type checking
                guards: vec![],
                body: Some(body),
                with: vec![],
            }
        })
}

/// Parse a function name (can be an operator)
fn function_name() -> impl Parser<SpannedToken, Ident, Error = ParserError> + Clone {
    choice((
        // Regular identifier
        pure_ident().map(Ident::new),
        // Operator symbols
        operator_name(),
    ))
}

/// Parse an operator name: +, -, *, etc.
fn operator_name() -> impl Parser<SpannedToken, Ident, Error = ParserError> + Clone {
    filter_map(|_span: ParserSpan, spanned: SpannedToken| {
        match &spanned.token {
            Token::Ident(s) if s.chars().all(|c| matches!(c, '+' | '-' | '*' | '/' | '=' | '!' | '<' | '>' | '|' | '&' | '^' | '@' | '#' | '$' | '%' | '\\')) => {
                Ok(Ident::new(s))
            }
            _ => Err(ParserError::custom(_span, "expected operator".to_string())),
        }
    })
}

use super::pattern::base_pattern;
use crate::ast::pattern::Pattern;

/// Parse a lambda clause: λ (pattern) body
fn lambda_clause<E>(expr: E) -> impl Parser<SpannedToken, LambdaClause, Error = ParserError> + Clone
where
    E: Parser<SpannedToken, Spanned<Expr>, Error = ParserError> + Clone + 'static,
{
    // λ or lambda keyword
    let lambda_keyword = token(Token::Lambda);

    // Pattern list - supports multiple patterns for multi-argument functions
    // e.g. λ (0) acc: body
    let pattern_parser = base_pattern().padded_by(newline().repeated().or_not()).repeated()
        .map(|p: Vec<Spanned<Pattern>>| {
            log::debug!("lambda_clause pattern_parser matched {} patterns: {:?}", p.len(), p);
            p
        });

    // Guards and With bindings block
    let guards_and_with = newline().repeated().at_least(1)
        .ignore_then(indent())
        .ignore_then(
            guard_clause(expr.clone())
                .then_ignore(newline().repeated().or_not())
                .repeated().at_least(1)
        )
        .then(with_bindings(expr.clone()).or_not())
        .then_ignore(newline().repeated().or_not())
        .then_ignore(dedent());

    // Body expression (Inline or Indented without guards)
    let body_parser = choice((
        // Indented body
        newline().repeated().at_least(1).ignore_then(indent())
            .ignore_then(expr.clone().then(newline().repeated().or_not().ignore_then(with_bindings(expr.clone())).or_not()))
            .then_ignore(newline().repeated().or_not()).then_ignore(dedent()),
        // Inline body
        expr.clone().then(newline().repeated().or_not().ignore_then(with_bindings(expr.clone())).or_not()),
    ));

    lambda_keyword
        .ignore_then(pattern_parser)
        .then_ignore(token(Token::Colon))
        .then(choice((
            // Either guards (with optional 'with')
            guards_and_with.map(|(guards, with_opt)| (Some(guards), with_opt, None)),
            // Or normal body (with optional 'with')
            body_parser.map(|(body, with_opt)| (None, with_opt, Some(body))),
        )))
        .try_map(|(patterns, (guards_opt, with_opt, body_opt)): (Vec<Spanned<Pattern>>, (Option<Vec<GuardClause>>, Option<Vec<WithBinding>>, Option<Spanned<Expr>>)), span| {
            log::debug!("lambda_clause matched {} patterns", patterns.len());
            let guards = guards_opt.unwrap_or_default();
            let with_bindings = with_opt.unwrap_or_default();
            
            if body_opt.is_none() && guards.is_empty() {
                return Err(ParserError::custom(span, "Lambda clause must have either a body or guard clauses".to_string()));
            }
            
            Ok(LambdaClause {
                patterns,
                guards,
                body: body_opt,
                with: with_bindings,
            })
        })
}

/// Parse a guard clause: label: body
fn guard_clause<E>(expr: E) -> impl Parser<SpannedToken, GuardClause, Error = ParserError> + Clone
where
    E: Parser<SpannedToken, Spanned<Expr>, Error = ParserError> + Clone + 'static,
{
    choice((
        ident().map(Ident::new).map(|i| (i.clone(), GuardCondition::Named(i))),
        token(Token::Otherwise).map(|_| (Ident::new("otherwise"), GuardCondition::Otherwise)),
    ))
    .then_ignore(token(Token::Colon))
    .then(expr)
    .map(|((label, guard), body)| GuardClause {
        label,
        guard,
        body,
    })
}

/// Parse with bindings
fn with_bindings<E>(expr: E) -> impl Parser<SpannedToken, Vec<crate::ast::expr::WithBinding>, Error = ParserError> + Clone
where
    E: Parser<SpannedToken, Spanned<Expr>, Error = ParserError> + Clone + 'static,
{
    token(Token::With)
        .ignore_then(newline().repeated().or_not())
        .ignore_then(indent())
        .ignore_then(
            with_binding(expr)
                .padded_by(newline().repeated().or_not())
                .repeated()
        )
        .then_ignore(newline().repeated().or_not())
        .then_ignore(dedent())
}

/// Parse a single with binding: name as expr, name :: sig, or type implements interface
fn with_binding<E>(expr: E) -> impl Parser<SpannedToken, crate::ast::expr::WithBinding, Error = ParserError> + Clone
where
    E: Parser<SpannedToken, Spanned<Expr>, Error = ParserError> + Clone + 'static,
{
    choice((
        // 1. Signature constraint: `+ :: A B => C`
        ident().map(Ident::new)
            .then_ignore(token(Token::DoubleColon))
            .then(function_sig())
            .map(|(name, sig)| crate::ast::expr::WithBinding::Signature { name, sig }),

        // 2. Interface constraint: `T implements ORD`
        type_expr()
            .then_ignore(token(Token::Implements))
            .then(ident().map(Ident::new))
            .map(|(typ, interface)| crate::ast::expr::WithBinding::Interface { typ: typ.node, interface }),

        // 3. Value binding: `name as expr`
        ident().map(Ident::new)
            .then_ignore(token(Token::As))
            .then(expr)
            .map(|(name, value)| crate::ast::expr::WithBinding::Value { name, value }),
    ))
}

// ============================================================================
// ACTION DEFINITION
// ============================================================================

/// Parse an action definition: action name (params) body
fn action_def<E>(expr: E) -> impl Parser<SpannedToken, Spanned<ActionDef>, Error = ParserError> + Clone
where
    E: Parser<SpannedToken, Spanned<Expr>, Error = ParserError> + Clone + 'static,
{
    log::debug!("action_def(): Starting");
    let result = token(Token::Action)
        .ignore_then(pure_ident().map(Ident::new))
        .then(
            // Optional parameters
            between(
                token(Token::LParen),
                token(Token::RParen),
                pure_ident().map(Ident::new)
                    .padded_by(token(Token::Comma).ignored().or(newline()).repeated().or_not())
                    .repeated()
            ).or_not()
        )
        .then(            // Optional return type
            token(Token::Arrow)
                .ignore_then(type_expr())
                .or_not()
            )
            .then_ignore(newline().repeated().or_not())
            // Optional body
            .then(choice((
                indent()
                    .ignore_then(
                        recursive_statement(expr.clone())
                            .map(|s| {
                                log::debug!("action_def: matched statement");
                                s
                            })
                            .padded_by(newline().repeated().or_not())
                            .repeated().at_least(1)
                    )
                    .then_ignore(newline().repeated().or_not())
                    .then_ignore(dedent()),
                recursive_statement(expr).map(|s| vec![s]),
            )).or_not())
            .then_ignore(newline().repeated().or_not())
            .map_with_span(|(((name, params), return_type), body), span: ParserSpan| {
            log::debug!("action_def: fully matched action '{}'", name.0);
            Spanned::new(ActionDef {
                name,
                params: params.unwrap_or_default(),
                return_type: return_type.map(|t| t.node),
                directives: vec![],
                body: body.unwrap_or_default(),
            }, span.into())
            })
;
    log::debug!("action_def(): Complete");
    result
}

// ============================================================================
// DATA DEFINITION
// ============================================================================

/// Parse a data definition: data Name (fields) or data Name as (Type, Predicate)
fn data_def() -> impl Parser<SpannedToken, DataDef, Error = ParserError> + Clone {
    token(Token::Data)
        .ignore_then(type_name_ident())
        .then(
            // Optional type parameters
            type_params().or_not()
        )
        .then(choice((
            // Fields in parentheses (Product)
            between(
                token(Token::LParen),
                token(Token::RParen),
                field_def().padded_by(newline().repeated().or_not()).repeated()
            ).map(DataKind::Product),

            // Fields in an indented block (Product)
            newline().repeated().at_least(1)
                .ignore_then(indent())
                .ignore_then(
                    field_def().padded_by(newline().repeated().or_not()).repeated().at_least(1)
                )
                .then_ignore(newline().repeated().or_not())
                .then_ignore(dedent())
                .map(DataKind::Product),

            // Refinement syntax: data Name as (Base, Predicate)
            token(Token::As)
                .ignore_then(type_expr())
                .map(|t| DataKind::Refinement(t.node)),
        )))
        .map(|((name, type_params), kind): ((Ident, Option<Vec<Ident>>), DataKind)| {
            log::debug!("data_def matched '{}' kind={:?}", name.0, kind);
            DataDef {
                name,
                type_params: type_params.unwrap_or_default(),
                kind,
            }
        })
}

/// Parse a field definition: name or name::Type
fn field_def() -> impl Parser<SpannedToken, FieldDef, Error = ParserError> + Clone {
    ident().map(Ident::new)
        .then(
            token(Token::DoubleColon)
                .ignore_then(type_expr())
                .or_not()
        )
        .map(|(name, type_annotation)| FieldDef { name, type_annotation: type_annotation.map(|t| t.node) })
}

/// Parse type parameters: ::T::E
fn type_params() -> impl Parser<SpannedToken, Vec<Ident>, Error = ParserError> + Clone {
    token(Token::DoubleColon)
        .ignore_then(type_var_ident())
        .then(token(Token::DoubleColon).ignore_then(type_var_ident()).repeated())
        .map(|(first, rest)| std::iter::once(first).chain(rest).collect())
}

// ============================================================================
// ENUM DEFINITION
// ============================================================================

/// Parse an enum definition: enum Name | Variant1 | Variant2(T)
fn enum_def() -> impl Parser<SpannedToken, EnumDef, Error = ParserError> + Clone {
    token(Token::Enum)
        .ignore_then(type_name_ident())
        .then(type_params().or_not())
        .then(
            choice((
                // 1. Indented block of variants
                newline().repeated().at_least(1)
                    .ignore_then(indent())
                    .ignore_then(variant_def().padded_by(newline().repeated().or_not()).repeated())
                    .then_ignore(newline().repeated().or_not())
                    .then_ignore(dedent()),
                
                // 2. Variants on the next line without indent (not recommended but handled)
                newline().repeated().at_least(1)
                    .ignore_then(variant_def().padded_by(newline().repeated().or_not()).repeated().at_least(1)),

                // 3. Variants on the same line
                variant_def().padded_by(newline().repeated().or_not()).repeated(),
            ))
            .or_not()
        )
        .map(|((name, type_params), variants)| {
            EnumDef {
                name,
                type_params: type_params.unwrap_or_default(),
                variants: variants.unwrap_or_default(),
            }
        })
}

/// Parse a variant definition: | Name or | Name(Type)
fn variant_def() -> impl Parser<SpannedToken, VariantDef, Error = ParserError> + Clone {
    token(Token::Pipe)
        .ignore_then(type_name_ident())
        .then(
            between(
                token(Token::LParen),
                token(Token::RParen),
                variant_payload()
            ).or_not()
        )
        .map(|(name, payload)| {
            VariantDef {
                name,
                payload: payload.unwrap_or(VariantPayload::Unit),
            }
        })
}

/// Parse a variant payload
fn variant_payload() -> impl Parser<SpannedToken, VariantPayload, Error = ParserError> + Clone {
    choice((
        // Type: (T)
        type_expr().map(|t| VariantPayload::Typed(t.node)),

        // Fixed value: (42)
        literal_value().map(VariantPayload::FixedValue),

        // Predicated: (Int, < _ 10)
        predicate_payload(),
    ))
}

/// Parse a predicated payload
fn predicate_payload() -> impl Parser<SpannedToken, VariantPayload, Error = ParserError> + Clone {
    variant_predicate()
        .map(|predicate| {
            VariantPayload::Predicated { predicate }
        })
}

/// Parse a variant predicate
fn variant_predicate() -> impl Parser<SpannedToken, crate::ast::decl::VariantPredicate, Error = ParserError> + Clone {
    compare_op()
        .then_ignore(token(Token::Hole).or_not())
        .then(literal_value())
        .map(|(op, value)| crate::ast::decl::VariantPredicate::Comparison { op, value })
}

/// Parse a literal value for variant payloads
fn literal_value() -> impl Parser<SpannedToken, crate::ast::decl::LiteralValue, Error = ParserError> + Clone {
    filter_map(|_span: ParserSpan, spanned: SpannedToken| {
        match &spanned.token {
            Token::Int(s) => Ok(crate::ast::decl::LiteralValue::Int(s.clone())),
            Token::Float(s) => Ok(crate::ast::decl::LiteralValue::Float(s.clone())),
            Token::String(s) => Ok(crate::ast::decl::LiteralValue::String(s.clone())),
            _ => Err(ParserError::custom(_span, "expected literal".to_string())),
        }
    })
}

/// Parse a comparison operator for variant predicates
fn compare_op() -> impl Parser<SpannedToken, crate::ast::decl::CompareOp, Error = ParserError> + Clone {
    choice((
        ident_named("<").map(|_| crate::ast::decl::CompareOp::Lt),
        ident_named("<=").map(|_| crate::ast::decl::CompareOp::Le),
        ident_named(">").map(|_| crate::ast::decl::CompareOp::Gt),
        ident_named(">=").map(|_| crate::ast::decl::CompareOp::Ge),
    ))
}

// ============================================================================
// INTERFACE DEFINITION
// ============================================================================

/// Parse an interface definition
fn interface_def<E>(expr: E) -> impl Parser<SpannedToken, InterfaceDef, Error = ParserError> + Clone
where
    E: Parser<SpannedToken, Spanned<Expr>, Error = ParserError> + Clone + 'static,
{
    token(Token::Interface)
        .ignore_then(type_name_ident())
        .then(
            // Optional extends clause
            token(Token::Implements)
                .ignore_then(type_name_ident().repeated())
                .or_not()
        )
        .then_ignore(newline().repeated().or_not())
        .then_ignore(indent())
        .then(interface_member(expr).padded_by(newline().repeated().or_not()).repeated())
        .then_ignore(newline().repeated().or_not())
        .then_ignore(dedent())
        .map(|((name, extends), members)| {
            InterfaceDef {
                name,
                extends: extends.unwrap_or_default(),
                members,
            }
        })
}

/// Parse an interface member: signature or function with default impl
fn interface_member<E>(expr: E) -> impl Parser<SpannedToken, InterfaceMember, Error = ParserError> + Clone
where
    E: Parser<SpannedToken, Spanned<Expr>, Error = ParserError> + Clone + 'static,
{
    directive().repeated()
        .then(function_def(expr))
        .map(|(directives, mut f)| {
            if f.clauses.is_empty() {
                InterfaceMember::Signature(f.name, f.sig)
            } else {
                f.directives = directives;
                InterfaceMember::FunctionDef(f)
            }
        })
}

// ============================================================================
// IMPLEMENTATION DEFINITION
// ============================================================================

/// Parse an implementation: Type implements Interface
fn impl_def<E>(expr: E) -> impl Parser<SpannedToken, ImplDef, Error = ParserError> + Clone
where
    E: Parser<SpannedToken, Spanned<Expr>, Error = ParserError> + Clone + 'static,
{
    let body = newline().repeated().at_least(1)
        .ignore_then(indent())
        .ignore_then(
            directive().repeated()
                .then(function_def(expr))
                .map(|(directives, mut f)| {
                    f.directives = directives;
                    f
                })
                .padded_by(newline().repeated().or_not())
                .repeated()
        )
        .then_ignore(newline().repeated().or_not())
        .then_ignore(dedent());

    qualified_type_name()
        .then_ignore(token(Token::Implements))

        .then(type_name_ident())
        .then(body.or_not())
        .map(|((type_name, interface), implementations_opt)| {
            ImplDef {
                type_name,
                interface,
                implementations: implementations_opt.unwrap_or_default(),
            }
        })
}

// ============================================================================
// TYPE ALIAS
// ============================================================================

/// Parse a type alias: alias Type as NewName
fn alias_def() -> impl Parser<SpannedToken, AliasDef, Error = ParserError> + Clone {
    token(Token::Alias)
        .ignore_then(type_expr())
        .then_ignore(token(Token::As))
        .then(type_name_ident())
        .then_ignore(newline().or_not())
        .map(|(target, name)| {
            AliasDef { name, target: target.node }
        })
}

// ============================================================================
// DIRECTIVES
// ============================================================================

/// Parse a compiler directive: @test("desc"), @parallel, etc.
fn directive() -> impl Parser<SpannedToken, Directive, Error = ParserError> + Clone {
    token(Token::AtSymbol)
        .ignore_then(choice((
            // @test("description")
            keyword("test")
                .ignore_then(between(token(Token::LParen), token(Token::RParen), string_content()))
                .map(|desc| Directive::Test { description: desc }),

            // @parallel
            keyword("parallel").map(|_| Directive::Parallel),

            // @comutative
            keyword("comutative").map(|_| Directive::Comutative),

            // @predicate
            keyword("predicate").map(|_| Directive::Predicate),

            // @comptime
            keyword("comptime").map(|_| Directive::Comptime),

            // @ffi("symbol")
            keyword("ffi")
                .ignore_then(between(token(Token::LParen), token(Token::RParen), string_content()))
                .map(|symbol| Directive::Ffi { symbol }),
        )))
        .padded_by(newline().repeated().or_not())
}

/// Parse string content (without quotes)
fn string_content() -> impl Parser<SpannedToken, String, Error = ParserError> + Clone {
    filter_map(|_span: ParserSpan, spanned: SpannedToken| {
        match &spanned.token {
            Token::String(s) => Ok(s.clone()),
            _ => Err(ParserError::custom(_span, "expected string".to_string())),
        }
    })
}

// ============================================================================
// HELPER FUNCTIONS
// ============================================================================

/// Parse a type name (uppercase identifier)
fn type_name_ident() -> impl Parser<SpannedToken, Ident, Error = ParserError> + Clone {
    filter_map(|_span: ParserSpan, spanned: SpannedToken| {
        match &spanned.token {
            Token::Ident(s) if s.chars().next().map(|c| c.is_ascii_uppercase()).unwrap_or(false) => Ok(Ident::new(s)),
            _ => Err(ParserError::custom(_span, "expected type name".to_string())),
        }
    })
}

/// Parse a type variable (generic parameter)
fn type_var_ident() -> impl Parser<SpannedToken, Ident, Error = ParserError> + Clone {
    filter_map(|_span: ParserSpan, spanned: SpannedToken| {
        match &spanned.token {
            // Type variables can be uppercase (T, E) or lowercase (a, b)
            Token::Ident(s) => Ok(Ident::new(s)),
            _ => Err(ParserError::custom(_span, "expected type identifier".to_string())),
        }
    })
}

/// Parse a qualified type name: Module::Item
fn qualified_type_name() -> impl Parser<SpannedToken, QualifiedIdent, Error = ParserError> + Clone {
    type_name_ident()
        .then(token(Token::DoubleColon).ignore_then(type_name_ident()).repeated())
        .map(|(first, rest): (Ident, Vec<Ident>)| {
            let all: Vec<_> = std::iter::once(first).chain(rest).collect();
            if all.len() == 1 {
                QualifiedIdent::simple(all[0].0.clone())
            } else {
                let name = all.last().unwrap().0.clone();
                let module = all[..all.len() - 1].iter().map(|i| i.0.clone()).collect::<Vec<_>>().join("::");
                QualifiedIdent::qualified(&module, name)
            }
        })
}

/// Match a keyword
fn keyword(kw: &str) -> impl Parser<SpannedToken, (), Error = ParserError> + Clone {
    let kw_str = kw.to_string();
    filter_map(move |_span: ParserSpan, spanned: SpannedToken| {
        match &spanned.token {
            Token::Ident(s) if s == &kw_str => Ok(()),
            _ => Err(ParserError::custom(_span, format!("expected '{}'", kw_str))),
        }
    })
}

/// Match an identifier by name
fn ident_named(name: &str) -> impl Parser<SpannedToken, String, Error = ParserError> + Clone {
    let name = name.to_string();
    filter_map(move |_span: ParserSpan, spanned: SpannedToken| {
        match &spanned.token {
            Token::Ident(s) if s == &name => Ok(s.clone()),
            _ => Err(ParserError::custom(_span, format!("expected '{}'", name))),
        }
    })
}

/// Match end of input
fn end() -> impl Parser<SpannedToken, (), Error = ParserError> + Clone {
    filter_map(|_span: ParserSpan, spanned: SpannedToken| {
        match &spanned.token {
            Token::Eof => Ok(()),
            _ => Err(ParserError::custom(_span, "expected end of input".to_string())),
        }
    })
}
