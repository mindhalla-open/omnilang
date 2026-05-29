//! Parser for OmniLang source files.
//!
//! Transforms a token stream into an Abstract Syntax Tree (AST).
//! Uses recursive descent with Pratt parsing for expressions.

use crate::ast::*;
use crate::error::ParseError;
use crate::span::Span;
use crate::token::{Token, TokenKind};

/// Parser state: walks through the token stream.
pub struct Parser {
    tokens: Vec<Token>,
    pos: usize,
    errors: Vec<ParseError>,
}

impl Parser {
    pub fn new(tokens: Vec<Token>) -> Self {
        // Filter out line and block comments, but keep DocComment for AST doc comment support
        let tokens: Vec<Token> = tokens
            .into_iter()
            .filter(|t| !matches!(t.kind, TokenKind::LineComment | TokenKind::BlockComment))
            .collect();

        Self {
            tokens,
            pos: 0,
            errors: Vec::new(),
        }
    }

    fn consume_newlines(&mut self) {
        while self.check(TokenKind::Newline) {
            self.advance();
        }
    }

    fn expect_block_start(&mut self) -> TokenKind {
        self.consume_newlines();
        if self.check(TokenKind::BraceOpen) {
            self.advance(); // consume '{'
            self.consume_newlines();
            TokenKind::BraceClose
        } else {
            self.expect(TokenKind::Indent);
            TokenKind::Dedent
        }
    }

    fn parse_block<F, T>(&mut self, mut parse_stmt: F) -> Vec<T>
    where
        F: FnMut(&mut Self) -> Option<T>,
    {
        let close = self.expect_block_start();
        let mut results = Vec::new();
        while !self.check(close) && !self.is_at_end() {
            self.consume_newlines();
            if self.check(close) || self.is_at_end() {
                break;
            }
            if let Some(item) = parse_stmt(self) {
                results.push(item);
            } else {
                self.advance();
            }
            self.consume_newlines();
        }
        self.expect(close);
        results
    }

    /// Parse a complete source file.
    pub fn parse(mut self) -> (SourceFile, Vec<ParseError>) {
        let module = self.parse_module_decl();
        let mut imports = Vec::new();
        let mut declarations = Vec::new();
        let mut exports = Vec::new();

        while !self.is_at_end() {
            self.consume_newlines();
            if self.is_at_end() {
                break;
            }
            let doc_comment = self.consume_doc_comments();
            match self.peek_kind() {
                TokenKind::KwUse => {
                    if let Some(import) = self.parse_import() {
                        imports.push(import);
                    }
                }
                TokenKind::KwExport => {
                    self.advance(); // consume 'export'
                    // Export the next identifier(s)
                    if self.check(TokenKind::Ident) || self.peek_kind().is_keyword() {
                        let tok = self.advance();
                        exports.push(tok.text);
                    }
                }
                TokenKind::KwPrivate => {
                    self.advance(); // consume 'private'
                    // The next declaration is private
                    match self.peek_kind() {
                        TokenKind::KwType => {
                            if let Some(mut decl) = self.parse_type_decl() {
                                decl.visibility = Visibility::Private;
                                decl.doc_comment = doc_comment;
                                declarations.push(Declaration::Type(decl));
                            }
                        }
                        TokenKind::KwService => {
                            if let Some(mut decl) = self.parse_service_decl() {
                                decl.visibility = Visibility::Private;
                                decl.doc_comment = doc_comment;
                                declarations.push(Declaration::Service(decl));
                            }
                        }
                        TokenKind::KwMixin => {
                            if let Some(mut decl) = self.parse_mixin_decl() {
                                decl.visibility = Visibility::Private;
                                declarations.push(Declaration::Mixin(decl));
                            }
                        }
                        _ => {
                            let tok = self.advance();
                            self.errors.push(ParseError::Expected {
                                expected: "declaration after 'private'".to_string(),
                                found: format!("'{}'", tok.text),
                                span: tok.span,
                            });
                        }
                    }
                }
                TokenKind::KwMixin => {
                    if let Some(decl) = self.parse_mixin_decl() {
                        declarations.push(Declaration::Mixin(decl));
                    }
                }
                TokenKind::KwType => {
                    if let Some(mut decl) = self.parse_type_decl() {
                        decl.doc_comment = doc_comment;
                        declarations.push(Declaration::Type(decl));
                    }
                }
                TokenKind::KwEntity => {
                    if let Some(mut decl) = self.parse_entity_decl() {
                        decl.doc_comment = doc_comment;
                        declarations.push(Declaration::Entity(decl));
                    }
                }
                TokenKind::KwAction => {
                    if let Some(mut decl) = self.parse_action_decl() {
                        decl.doc_comment = doc_comment;
                        declarations.push(Declaration::Action(decl));
                    }
                }
                TokenKind::KwRule => {
                    if let Some(mut decl) = self.parse_rule_decl() {
                        decl.doc_comment = doc_comment;
                        declarations.push(Declaration::Rule(decl));
                    }
                }
                TokenKind::KwService => {
                    if let Some(mut decl) = self.parse_service_decl() {
                        decl.doc_comment = doc_comment;
                        declarations.push(Declaration::Service(decl));
                    }
                }
                TokenKind::KwComponent => {
                    if let Some(decl) = self.parse_component_decl() {
                        declarations.push(Declaration::Component(decl));
                    }
                }
                TokenKind::KwPipeline => {
                    if let Some(decl) = self.parse_pipeline_decl() {
                        declarations.push(Declaration::Pipeline(decl));
                    }
                }
                TokenKind::KwWorkflow => {
                    if let Some(decl) = self.parse_workflow_decl() {
                        declarations.push(Declaration::Workflow(decl));
                    }
                }
                TokenKind::KwAgent => {
                    if let Some(decl) = self.parse_agent_decl() {
                        declarations.push(Declaration::Agent(decl));
                    }
                }
                TokenKind::KwSchema => {
                    if let Some(decl) = self.parse_schema_decl() {
                        declarations.push(Declaration::Schema(decl));
                    }
                }
                TokenKind::KwPolicy => {
                    if let Some(decl) = self.parse_policy_decl() {
                        declarations.push(Declaration::Policy(decl));
                    }
                }
                TokenKind::KwConstraint => {
                    if let Some(decl) = self.parse_constraint_decl() {
                        declarations.push(Declaration::Constraint(decl));
                    } else {
                        self.advance();
                    }
                }
                TokenKind::KwTargetDependencies => {
                    if let Some(decl) = self.parse_target_dependencies_decl() {
                        declarations.push(Declaration::TargetDependencies(decl));
                    }
                }
                TokenKind::Newline => {
                    self.advance();
                }
                TokenKind::Eof => break,
                _ => {
                    let tok = self.advance();
                    self.errors.push(ParseError::Expected {
                        expected: "top-level declaration (type, service, schema, target_dependencies, mixin, use)".to_string(),
                        found: format!("'{}'", tok.text),
                        span: tok.span,
                    });
                    // Skip to next potential top-level token
                    self.synchronize();
                }
            }
        }

        let file = SourceFile {
            module,
            imports,
            exports,
            declarations,
        };
        (file, self.errors)
    }

    // ── Module declaration ────────────────────────────

    fn parse_module_decl(&mut self) -> ModuleDecl {
        let start = self.current_span();

        if !self.expect(TokenKind::KwModule) {
            return ModuleDecl {
                path: vec!["unnamed".to_string()],
                span: start,
            };
        }

        let path = self.parse_dotted_path();
        let end = self.previous_span();

        ModuleDecl {
            path,
            span: start.merge(end),
        }
    }

    // ── Import declarations ──────────────────────────

    fn parse_import(&mut self) -> Option<ImportDecl> {
        let start = self.current_span();
        self.advance(); // consume 'use'

        let mut path = Vec::new();
        let items;

        // Parse path: std.http.{Request, Response} or std.auth.*
        loop {
            if self.check(TokenKind::Ident) || self.peek_kind().is_keyword() {
                let tok = self.advance();
                path.push(tok.text);
            } else {
                break;
            }

            if self.check(TokenKind::Dot) {
                self.advance(); // consume '.'

                // Check for wildcard: `use std.auth.*`
                if self.check(TokenKind::Star) {
                    self.advance();
                    items = ImportItems::Wildcard;
                    let end = self.previous_span();
                    return Some(ImportDecl {
                        path,
                        items,
                        kind: ImportKind::Standard,
                        span: start.merge(end),
                    });
                }

                // Check for selective: `use std.http.{Request, Response}`
                if self.check(TokenKind::BraceOpen) {
                    items = self.parse_import_items();
                    let end = self.previous_span();
                    return Some(ImportDecl {
                        path,
                        items,
                        kind: ImportKind::Standard,
                        span: start.merge(end),
                    });
                }
            } else {
                break;
            }
        }

        // Simple import: `use std.http`
        items = ImportItems::Named(vec![]);
        let end = self.previous_span();
        Some(ImportDecl {
            path,
            items,
            kind: ImportKind::Standard,
            span: start.merge(end),
        })
    }

    fn parse_import_items(&mut self) -> ImportItems {
        self.advance(); // consume '{'
        let mut items = Vec::new();

        while !self.check(TokenKind::BraceClose) && !self.is_at_end() {
            let start = self.current_span();
            if self.check(TokenKind::Ident) || self.peek_kind().is_keyword() {
                let tok = self.advance();
                let name = tok.text;
                let alias = if self.check(TokenKind::KwAs) {
                    self.advance();
                    Some(self.advance().text)
                } else {
                    None
                };
                let end = self.previous_span();
                items.push(ImportItem {
                    name,
                    alias,
                    span: start.merge(end),
                });
            } else {
                break;
            }
            if self.check(TokenKind::Comma) {
                self.advance();
            }
        }

        self.expect(TokenKind::BraceClose);
        ImportItems::Named(items)
    }

    // ── Type declarations ────────────────────────────

    fn parse_type_decl(&mut self) -> Option<TypeDecl> {
        let start = self.current_span();
        self.advance(); // consume 'type'

        let name_tok = self.advance();
        let name = name_tok.text.clone();

        // Parse optional type parameters, e.g. <T: Bound1 + Bound2, U>
        let mut type_params = Vec::new();
        if self.check(TokenKind::Lt) {
            self.advance(); // consume '<'
            loop {
                let pstart = self.current_span();
                let pname_tok = self.advance();
                let pname = pname_tok.text.clone();
                let mut bounds = Vec::new();
                if self.check(TokenKind::Colon) {
                    self.advance(); // consume ':'
                    loop {
                        bounds.push(self.parse_type_ref());
                        if self.check(TokenKind::Plus) {
                            self.advance(); // consume '+'
                        } else {
                            break;
                        }
                    }
                }
                let pend = self.previous_span();
                type_params.push(TypeParam {
                    name: pname,
                    bounds,
                    span: pstart.merge(pend),
                });
                if self.check(TokenKind::Comma) {
                    self.advance();
                } else {
                    break;
                }
            }
            self.expect(TokenKind::Gt);
        }

        // Check for `=` (alias, enum, struct, refined)
        if self.check(TokenKind::Eq) {
            self.advance(); // consume '='

            match self.peek_kind() {
                TokenKind::KwEnum => {
                    self.advance();
                    let kind = TypeKind::Enum(self.parse_enum_body());
                    let end = self.previous_span();
                    return Some(TypeDecl {
                        name,
                        type_params,
                        kind,
                        visibility: Visibility::Public,
                        doc_comment: None,
                        span: start.merge(end),
                    });
                }
                TokenKind::KwStruct => {
                    self.advance();
                    let kind = TypeKind::Struct(self.parse_struct_body());
                    let end = self.previous_span();
                    return Some(TypeDecl {
                        name,
                        type_params,
                        kind,
                        visibility: Visibility::Public,
                        doc_comment: None,
                        span: start.merge(end),
                    });
                }
                TokenKind::BraceOpen => {
                    // Refined type without base type: `type OrderId = { format: regex(...) }`
                    let constraints = self.parse_type_constraints();
                    let end = self.previous_span();
                    return Some(TypeDecl {
                        name,
                        type_params,
                        kind: TypeKind::Refined(RefinedType {
                            base: None,
                            constraints,
                            span: start.merge(end),
                        }),
                        visibility: Visibility::Public,
                        doc_comment: None,
                        span: start.merge(end),
                    });
                }
                _ => {
                    // Could be alias: `type UserId = UUID`
                    // or refined: `type OrderId = String { ... }`
                    let type_ref = self.parse_type_ref();

                    if self.check(TokenKind::BraceOpen) {
                        // Refined type with base type: `type OrderId = String { format: regex(...) }`
                        let constraints = self.parse_type_constraints();
                        let end = self.previous_span();
                        return Some(TypeDecl {
                            name,
                            type_params,
                            kind: TypeKind::Refined(RefinedType {
                                base: Some(type_ref),
                                constraints,
                                span: start.merge(end),
                            }),
                            visibility: Visibility::Public,
                            doc_comment: None,
                            span: start.merge(end),
                        });
                    }

                    // Simple alias: `type UserId = UUID`
                    let end = self.previous_span();
                    return Some(TypeDecl {
                        name,
                        type_params,
                        kind: TypeKind::Alias(type_ref),
                        visibility: Visibility::Public,
                        doc_comment: None,
                        span: start.merge(end),
                    });
                }
            }
        }

        self.errors.push(ParseError::Expected {
            expected: "'=' after type name".to_string(),
            found: format!("'{}'", self.peek_text()),
            span: self.current_span(),
        });
        None
    }

    fn parse_enum_body(&mut self) -> EnumType {
        let start = self.current_span();
        let close = self.expect_block_start();

        let mut variants = Vec::new();
        while !self.check(close) && !self.is_at_end() {
            self.consume_newlines();
            if self.check(close) || self.is_at_end() {
                break;
            }
            let vstart = self.current_span();
            let vtok = self.advance();
            let vname = vtok.text;

            let fields = if self.check(TokenKind::ParenOpen) {
                self.parse_field_list(TokenKind::ParenOpen, TokenKind::ParenClose)
            } else {
                Vec::new()
            };

            let vend = self.previous_span();
            variants.push(EnumVariant {
                name: vname,
                fields,
                span: vstart.merge(vend),
            });
            self.consume_newlines();
        }

        self.expect(close);
        let end = self.previous_span();
        EnumType {
            variants,
            span: start.merge(end),
        }
    }

    fn parse_struct_body(&mut self) -> StructType {
        let start = self.current_span();
        let close = self.expect_block_start();

        let mut fields = Vec::new();
        while !self.check(close) && !self.is_at_end() {
            self.consume_newlines();
            if self.check(close) || self.is_at_end() {
                break;
            }
            if self.check(TokenKind::Comma) {
                self.advance();
                continue;
            }
            let doc_comment = self.consume_doc_comments();
            let field_start = self.current_span();
            let name_tok = self.advance();
            let name = name_tok.text;

            // Optional colon between name and type
            if self.check(TokenKind::Colon) {
                self.advance();
            }
            let ty = self.parse_type_ref();

            let default = if self.check(TokenKind::Eq) {
                self.advance();
                Some(self.parse_expression())
            } else {
                None
            };

            let end = self.previous_span();
            fields.push(Field {
                name,
                ty,
                default,
                doc_comment,
                span: field_start.merge(end),
            });
            self.consume_newlines();
        }

        self.expect(close);
        let end = self.previous_span();
        StructType {
            fields,
            span: start.merge(end),
        }
    }

    fn parse_type_constraints(&mut self) -> Vec<TypeConstraint> {
        self.advance(); // consume '{'
        let mut constraints = Vec::new();

        while !self.check(TokenKind::BraceClose) && !self.is_at_end() {
            let start = self.current_span();
            let name_tok = self.advance();
            let name = name_tok.text;

            self.expect(TokenKind::Colon);
            let value = self.parse_expression();
            let end = self.previous_span();

            constraints.push(TypeConstraint {
                name,
                value,
                span: start.merge(end),
            });
        }

        self.expect(TokenKind::BraceClose);
        constraints
    }

    fn parse_base_type_ref(&mut self) -> TypeRef {
        let start = self.current_span();
        let tok = self.advance();
        let name = tok.text;

        let mut type_args = Vec::new();
        if self.check(TokenKind::Lt) {
            self.advance(); // consume '<'
            loop {
                type_args.push(self.parse_type_ref());
                if self.check(TokenKind::Comma) {
                    self.advance();
                } else {
                    break;
                }
            }
            self.expect(TokenKind::Gt);
        }

        let end = self.previous_span();
        TypeRef {
            name,
            type_args,
            union_members: Vec::new(),
            intersection_members: Vec::new(),
            span: start.merge(end),
        }
    }

    fn parse_type_ref(&mut self) -> TypeRef {
        self.parse_type_ref_prec(0)
    }

    fn parse_type_ref_prec(&mut self, min_prec: u8) -> TypeRef {
        let mut ty = self.parse_base_type_ref();

        // Handle postfix '?' (highest precedence)
        if self.check(TokenKind::Question) {
            let q_span = self.current_span();
            self.advance(); // consume '?'
            let span = ty.span.merge(q_span);
            ty = TypeRef {
                name: "Option".to_string(),
                type_args: vec![ty],
                union_members: Vec::new(),
                intersection_members: Vec::new(),
                span,
            };
        }

        loop {
            let op_prec = match self.peek_kind() {
                TokenKind::Pipe => 1,
                TokenKind::Amp => 2,
                _ => 0,
            };

            if op_prec == 0 || op_prec < min_prec {
                break;
            }

            let op = self.peek_kind();
            self.advance(); // consume '|' or '&'

            let right = self.parse_type_ref_prec(op_prec + 1);
            let span = ty.span.merge(right.span);

            if op == TokenKind::Pipe {
                if ty.name == "Union" {
                    ty.union_members.push(right);
                    ty.span = span;
                } else {
                    ty = TypeRef {
                        name: "Union".to_string(),
                        type_args: Vec::new(),
                        union_members: vec![ty, right],
                        intersection_members: Vec::new(),
                        span,
                    };
                }
            } else {
                if ty.name == "Intersection" {
                    ty.intersection_members.push(right);
                    ty.span = span;
                } else {
                    ty = TypeRef {
                        name: "Intersection".to_string(),
                        type_args: Vec::new(),
                        union_members: Vec::new(),
                        intersection_members: vec![ty, right],
                        span,
                    };
                }
            }
        }

        ty
    }

    fn parse_field_list(&mut self, open: TokenKind, close: TokenKind) -> Vec<Field> {
        self.expect(open);
        let mut fields = Vec::new();

        while !self.check(close) && !self.is_at_end() {
            if self.check(TokenKind::Comma) {
                self.advance();
                continue;
            }
            let doc_comment = self.consume_doc_comments();
            let start = self.current_span();
            let name_tok = self.advance();
            let name = name_tok.text;

            self.expect(TokenKind::Colon);
            let ty = self.parse_type_ref();

            let default = if self.check(TokenKind::Eq) {
                self.advance();
                Some(self.parse_expression())
            } else {
                None
            };

            let end = self.previous_span();
            fields.push(Field {
                name,
                ty,
                default,
                doc_comment,
                span: start.merge(end),
            });
        }

        self.expect(close);
        fields
    }

    // ── Service declarations ─────────────────────────

    fn parse_service_decl(&mut self) -> Option<ServiceDecl> {
        let start = self.current_span();
        self.advance(); // consume 'service'

        let name = if self.check(TokenKind::Ident) {
            self.advance().text.clone()
        } else {
            self.errors.push(ParseError::Expected {
                expected: "service name (identifier)".to_string(),
                found: format!("'{}'", self.peek().text),
                span: self.current_span(),
            });
            String::new()
        };

        let close = self.expect_block_start();

        let mut goal = None;
        let mut target = None;
        let mut constraints = Vec::new();
        let mut depends_on = Vec::new();
        let mut dependencies = Vec::new();
        let mut policies = Vec::new();
        let mut operations = Vec::new();
        let mut budget = None;
        let mut metrics = Vec::new();
        let mut invariants = Vec::new();

        while !self.check(close) && !self.is_at_end() {
            self.consume_newlines();
            if self.check(close) || self.is_at_end() {
                break;
            }
            let doc_comment = self.consume_doc_comments();
            match self.peek_kind() {
                TokenKind::KwGoal => {
                    self.advance();
                    if self.check(TokenKind::Colon) {
                        self.advance();
                    }
                    if self.check(TokenKind::StringLiteral) {
                        let tok = self.advance();
                        // Strip quotes from string
                        goal = Some(tok.text[1..tok.text.len() - 1].to_string());
                    }
                }
                TokenKind::KwTarget => {
                    self.advance();
                    if self.check(TokenKind::Colon) {
                        self.advance();
                    }
                    // Per-service target language, e.g. `target rust`.
                    if self.check(TokenKind::Ident) {
                        target = Some(self.advance().text.clone());
                    }
                }
                TokenKind::KwConstraint | TokenKind::KwConstraints | TokenKind::Ident
                    if self.peek_text() == "constraints" =>
                {
                    self.advance();
                    if self.check(TokenKind::Colon) {
                        self.advance();
                    }
                    constraints = self.parse_constraint_list();
                }
                TokenKind::KwDependsOn => {
                    self.advance();
                    if self.check(TokenKind::Colon) {
                        self.advance();
                    }
                    depends_on = self.parse_identifier_list();
                }
                TokenKind::KwDependencies => {
                    self.advance();
                    if self.check(TokenKind::Colon) {
                        self.advance();
                    }
                    self.consume_newlines();
                    let has_dep_indent = self.check(TokenKind::Indent);
                    if has_dep_indent {
                        self.advance();
                    }
                    while self.check(TokenKind::Minus) {
                        if let Some(dep) = self.parse_dependency_ref() {
                            dependencies.push(dep);
                        }
                        self.consume_newlines();
                    }
                    if has_dep_indent {
                        self.expect(TokenKind::Dedent);
                    }
                }
                TokenKind::KwPolicies => {
                    self.advance();
                    if self.check(TokenKind::Colon) {
                        self.advance();
                    }
                    self.consume_newlines();
                    let has_pol_indent = self.check(TokenKind::Indent);
                    if has_pol_indent {
                        self.advance();
                    }
                    while self.check(TokenKind::Minus) {
                        if let Some(policy) = self.parse_service_policy() {
                            policies.push(policy);
                        }
                        self.consume_newlines();
                    }
                    if has_pol_indent {
                        self.expect(TokenKind::Dedent);
                    }
                }
                TokenKind::KwOperation => {
                    if let Some(mut op) = self.parse_operation_decl() {
                        op.doc_comment = doc_comment;
                        operations.push(op);
                    }
                }
                TokenKind::KwBudget => {
                    self.advance();
                    if self.check(TokenKind::Colon) {
                        self.advance();
                    }
                    budget = Some(self.parse_budget_block());
                }
                TokenKind::KwMetrics => {
                    self.advance();
                    if self.check(TokenKind::Colon) {
                        self.advance();
                    }
                    metrics = self.parse_metrics_list();
                }
                TokenKind::KwInvariants => {
                    self.advance();
                    if self.check(TokenKind::Colon) {
                        self.advance();
                    }
                    invariants = self.parse_expression_list();
                }
                _ => {
                    // Skip unknown tokens inside service
                    self.advance();
                }
            }
            self.consume_newlines();
        }

        self.expect(close);
        let end = self.previous_span();

        Some(ServiceDecl {
            name,
            goal,
            target,
            constraints,
            depends_on,
            dependencies,
            policies,
            operations,
            budget,
            metrics,
            invariants,
            applies: Vec::new(),
            visibility: Visibility::Public,
            doc_comment: None,
            span: start.merge(end),
        })
    }

    fn parse_operation_decl(&mut self) -> Option<OperationDecl> {
        let start = self.current_span();
        self.advance(); // consume 'operation'

        let name_tok = self.advance();
        let name = name_tok.text.clone();

        let mut inputs = Vec::new();
        let mut outputs = Vec::new();
        let mut preconditions = Vec::new();
        let mut postconditions = Vec::new();
        let mut errors = Vec::new();
        let mut constraints = Vec::new();
        let mut tests = Vec::new();

        let is_shorthand = self.check(TokenKind::ParenOpen);

        if is_shorthand {
            inputs = self.parse_field_list(TokenKind::ParenOpen, TokenKind::ParenClose);
            if self.check(TokenKind::Arrow) {
                self.advance(); // consume '->'
                let ret_start = self.current_span();
                let ty = self.parse_type_ref();
                let ret_end = self.previous_span();
                outputs = vec![Field {
                    name: "result".to_string(),
                    ty,
                    default: None,
                    doc_comment: None,
                    span: ret_start.merge(ret_end),
                }];
            }
        }

        if !is_shorthand || self.check(TokenKind::BraceOpen) || self.check(TokenKind::Indent) {
            let close = self.expect_block_start();

            while !self.check(close) && !self.is_at_end() {
                self.consume_newlines();
                if self.check(close) || self.is_at_end() {
                    break;
                }
                match self.peek_kind() {
                    TokenKind::KwInputs => {
                        self.advance();
                        if self.check(TokenKind::Colon) {
                            self.advance();
                        }
                        inputs = self.parse_inline_fields();
                    }
                    TokenKind::KwOutputs => {
                        self.advance();
                        if self.check(TokenKind::Colon) {
                            self.advance();
                        }
                        outputs = self.parse_inline_fields();
                    }
                    TokenKind::KwPreconditions => {
                        self.advance();
                        if self.check(TokenKind::Colon) {
                            self.advance();
                        }
                        preconditions = self.parse_expression_list();
                    }
                    TokenKind::KwPostconditions => {
                        self.advance();
                        if self.check(TokenKind::Colon) {
                            self.advance();
                        }
                        postconditions = self.parse_expression_list();
                    }
                    TokenKind::KwErrors => {
                        self.advance();
                        if self.check(TokenKind::Colon) {
                            self.advance();
                        }
                        errors = self.parse_error_list();
                    }
                    TokenKind::KwTests => {
                        self.advance();
                        if self.check(TokenKind::Colon) {
                            self.advance();
                        }
                        tests = self.parse_test_list();
                    }
                    TokenKind::KwConstraints | TokenKind::Ident
                        if self.peek_text() == "constraints" =>
                    {
                        self.advance();
                        if self.check(TokenKind::Colon) {
                            self.advance();
                        }
                        constraints = self.parse_constraint_list();
                    }
                    _ => {
                        self.advance();
                    }
                }
                self.consume_newlines();
            }

            self.expect(close);
        }

        let end = self.previous_span();

        Some(OperationDecl {
            name,
            inputs,
            outputs,
            preconditions,
            postconditions,
            errors,
            constraints,
            tests,
            doc_comment: None,
            span: start.merge(end),
        })
    }

    // ── Constraint parsing ───────────────────────────

    fn parse_constraint_list(&mut self) -> Vec<Constraint> {
        let mut constraints = Vec::new();

        self.consume_newlines();

        let has_indent = self.check(TokenKind::Indent);
        if has_indent {
            self.advance();
        }

        while self.check(TokenKind::Minus) {
            self.advance(); // consume '-'
            let start = self.current_span();

            let name_tok = self.advance();
            let name = name_tok.text;

            let args = if self.check(TokenKind::ParenOpen) {
                self.parse_constraint_args()
            } else {
                Vec::new()
            };

            let end = self.previous_span();
            constraints.push(Constraint {
                name,
                args,
                span: start.merge(end),
            });
            self.consume_newlines();
        }

        if has_indent && self.check(TokenKind::Dedent) {
            self.advance();
        }

        constraints
    }

    fn parse_metrics_list(&mut self) -> Vec<MetricDecl> {
        let mut metrics = Vec::new();

        self.consume_newlines();

        let has_indent = self.check(TokenKind::Indent);
        if has_indent {
            self.advance();
        }

        while self.check(TokenKind::Minus) {
            self.advance(); // consume '-'
            let start = self.current_span();

            let kind_tok = self.advance();
            let kind = match kind_tok.kind {
                TokenKind::KwCounter => MetricKind::Counter,
                TokenKind::KwGauge => MetricKind::Gauge,
                TokenKind::KwHistogram => MetricKind::Histogram,
                _ => MetricKind::Counter,
            };

            let name_tok = self.advance();
            let name = name_tok.text;

            let mut description = None;
            let mut labels = Vec::new();
            let mut buckets = None;

            if self.check(TokenKind::BraceOpen)
                || self.check(TokenKind::Newline)
                || self.check(TokenKind::Indent)
            {
                // Use expect_block_start to support both { } and indentation
                let metric_close = if self.check(TokenKind::BraceOpen) {
                    self.advance();
                    TokenKind::BraceClose
                } else {
                    self.consume_newlines();
                    if self.check(TokenKind::Indent) {
                        self.advance();
                        TokenKind::Dedent
                    } else {
                        // No body — skip
                        TokenKind::Dedent // won't match anything, loop won't enter
                    }
                };

                while !self.check(metric_close) && !self.is_at_end() {
                    self.consume_newlines();
                    if self.check(metric_close) || self.is_at_end() {
                        break;
                    }
                    let key_tok = self.advance();
                    if self.check(TokenKind::Colon) {
                        self.advance();
                    }
                    match key_tok.text.as_str() {
                        "description" => {
                            if self.check(TokenKind::StringLiteral) {
                                let val_tok = self.advance();
                                description =
                                    Some(val_tok.text[1..val_tok.text.len() - 1].to_string());
                            }
                        }
                        "labels" => {
                            self.expect(TokenKind::BracketOpen);
                            while !self.check(TokenKind::BracketClose) && !self.is_at_end() {
                                let label_tok = self.advance();
                                labels.push(label_tok.text);
                                if self.check(TokenKind::Comma) {
                                    self.advance();
                                }
                            }
                            self.expect(TokenKind::BracketClose);
                        }
                        "buckets" => {
                            self.expect(TokenKind::BracketOpen);
                            let mut bucket_exprs = Vec::new();
                            while !self.check(TokenKind::BracketClose) && !self.is_at_end() {
                                bucket_exprs.push(self.parse_expression());
                                if self.check(TokenKind::Comma) {
                                    self.advance();
                                }
                            }
                            self.expect(TokenKind::BracketClose);
                            buckets = Some(bucket_exprs);
                        }
                        _ => {
                            let _ = self.parse_expression();
                        }
                    }
                    self.consume_newlines();
                }
                if self.check(metric_close) {
                    self.advance();
                }
            }

            let end = self.previous_span();
            metrics.push(MetricDecl {
                name,
                kind,
                description,
                labels,
                buckets,
                span: start.merge(end),
            });
            self.consume_newlines();
        }

        if has_indent && self.check(TokenKind::Dedent) {
            self.advance();
        }

        metrics
    }

    fn parse_constraint_args(&mut self) -> Vec<ConstraintArg> {
        self.advance(); // consume '('
        let mut args = Vec::new();

        while !self.check(TokenKind::ParenClose) && !self.is_at_end() {
            let start = self.current_span();

            // Check for named arg: `p95: <200ms`
            let name = if self.peek_at(1).map(|t| t.kind) == Some(TokenKind::Colon) {
                let n = self.advance().text;
                self.advance(); // consume ':'
                Some(n)
            } else {
                None
            };

            let value = self.parse_expression();
            let end = self.previous_span();

            args.push(ConstraintArg {
                name,
                value,
                span: start.merge(end),
            });

            if self.check(TokenKind::Comma) {
                self.advance();
            }
        }

        self.expect(TokenKind::ParenClose);
        args
    }

    // ── Field parsing (inline, indented style) ───────

    fn parse_inline_fields(&mut self) -> Vec<Field> {
        let mut fields = Vec::new();

        // Consume any newlines after the section keyword
        self.consume_newlines();

        // Check if fields are in an indented block
        let has_indent = self.check(TokenKind::Indent);
        if has_indent {
            self.advance();
        }

        // Fields come in "name: Type" or "name Type" pairs, one per line
        // Stop when we hit a section keyword, Dedent, or end
        while !self.is_at_end() {
            self.consume_newlines();
            if has_indent && self.check(TokenKind::Dedent) {
                break;
            }
            if self.is_at_end() {
                break;
            }

            let doc_comment = self.consume_doc_comments();

            // Check if this looks like a field declaration:
            // Either "name: Type" (with colon) or "name Type" (without colon)
            let is_field = if let Some(tok) = self.peek_at(0) {
                if self.is_section_keyword(tok) {
                    false
                } else if tok.kind == TokenKind::Ident || tok.kind.is_keyword() {
                    if let Some(next) = self.peek_at(1) {
                        // "name: Type" or "name Type" (next is ident/keyword that's a type)
                        next.kind == TokenKind::Colon
                            || next.kind == TokenKind::Ident
                            || next.kind.is_keyword()
                    } else {
                        false
                    }
                } else {
                    false
                }
            } else {
                false
            };

            if is_field {
                let start = self.current_span();
                let name_tok = self.advance();
                let name = name_tok.text;

                // Consume optional colon
                if self.check(TokenKind::Colon) {
                    self.advance();
                }

                let ty = self.parse_type_ref();

                let default = if self.check(TokenKind::Eq) {
                    self.advance();
                    Some(self.parse_expression())
                } else {
                    None
                };

                let end = self.previous_span();
                fields.push(Field {
                    name,
                    ty,
                    default,
                    doc_comment,
                    span: start.merge(end),
                });
            } else {
                break;
            }
        }

        if has_indent {
            self.expect(TokenKind::Dedent);
        }

        fields
    }

    // ── Error declarations ───────────────────────────

    fn parse_error_list(&mut self) -> Vec<ErrorDecl> {
        let mut errors = Vec::new();

        self.consume_newlines();

        let has_indent = self.check(TokenKind::Indent);
        if has_indent {
            self.advance();
        }

        while self.check(TokenKind::Minus) {
            self.advance(); // consume '-'
            let start = self.current_span();
            let name_tok = self.advance();
            let name = name_tok.text;

            let fields = if self.check(TokenKind::ParenOpen) {
                self.parse_field_list(TokenKind::ParenOpen, TokenKind::ParenClose)
            } else {
                Vec::new()
            };

            let end = self.previous_span();
            errors.push(ErrorDecl {
                name,
                fields,
                span: start.merge(end),
            });
            self.consume_newlines();
        }

        if has_indent && self.check(TokenKind::Dedent) {
            self.advance();
        }

        errors
    }

    // ── Test blocks ──────────────────────────────────

    fn parse_test_list(&mut self) -> Vec<TestBlock> {
        let mut tests = Vec::new();

        self.consume_newlines();

        let has_indent = self.check(TokenKind::Indent);
        if has_indent {
            self.advance();
        }

        while self.check(TokenKind::Minus) {
            self.advance(); // consume '-'
            let start = self.current_span();

            match self.peek_kind() {
                TokenKind::KwScenario => {
                    self.advance();
                    self.expect(TokenKind::Colon);
                    let name = if self.check(TokenKind::StringLiteral) {
                        let tok = self.advance();
                        tok.text[1..tok.text.len() - 1].to_string()
                    } else {
                        "unnamed".to_string()
                    };

                    let mut given = Vec::new();
                    let mut when = Vec::new();
                    let mut expect = Vec::new();
                    let mut expect_error = None;

                    // Parse scenario fields
                    self.consume_newlines();
                    let has_scenario_indent = self.check(TokenKind::Indent);
                    if has_scenario_indent {
                        self.advance();
                    }
                    loop {
                        self.consume_newlines();
                        if has_scenario_indent && self.check(TokenKind::Dedent) {
                            self.advance();
                            break;
                        }
                        match self.peek_kind() {
                            TokenKind::KwGiven => {
                                self.advance();
                                if self.check(TokenKind::Colon) {
                                    self.advance();
                                }
                                given.push(self.parse_expression());
                            }
                            TokenKind::KwWhen => {
                                self.advance();
                                if self.check(TokenKind::Colon) {
                                    self.advance();
                                }
                                when.push(self.parse_expression());
                            }
                            TokenKind::KwExpect => {
                                self.advance();
                                if self.check(TokenKind::Colon) {
                                    self.advance();
                                }
                                expect.push(self.parse_expression());
                            }
                            TokenKind::KwExpectError => {
                                self.advance();
                                if self.check(TokenKind::Colon) {
                                    self.advance();
                                }
                                let tok = self.advance();
                                expect_error = Some(tok.text);
                            }
                            _ => break,
                        }
                    }

                    let end = self.previous_span();
                    tests.push(TestBlock {
                        kind: TestKind::Scenario {
                            name,
                            given,
                            when,
                            expect,
                            expect_error,
                        },
                        span: start.merge(end),
                    });
                }
                TokenKind::KwProperty => {
                    self.advance();
                    self.expect(TokenKind::Colon);
                    let name = if self.check(TokenKind::StringLiteral) {
                        let tok = self.advance();
                        tok.text[1..tok.text.len() - 1].to_string()
                    } else {
                        "unnamed".to_string()
                    };

                    let mut quantifiers = Vec::new();
                    let mut given = Vec::new();
                    let mut when = Vec::new();
                    let mut assert_exprs = Vec::new();

                    loop {
                        match self.peek_kind() {
                            TokenKind::KwForall => {
                                self.advance();
                                self.expect(TokenKind::Colon);
                                loop {
                                    let name_tok = self.advance();
                                    let name = name_tok.text.clone();

                                    if self.check(TokenKind::KwIn) {
                                        self.advance();
                                    } else if self.check(TokenKind::Lt)
                                        && self.peek_at(1).map(|t| t.kind) == Some(TokenKind::Minus)
                                    {
                                        self.advance(); // consume '<'
                                        self.advance(); // consume '-'
                                    } else {
                                        self.expect(TokenKind::KwIn);
                                    }

                                    let generator = self.parse_expression();
                                    let span = name_tok.span.merge(self.previous_span());
                                    quantifiers.push(Quantifier {
                                        name,
                                        generator,
                                        span,
                                    });

                                    if self.check(TokenKind::Comma) {
                                        self.advance();
                                    } else {
                                        break;
                                    }
                                }
                            }
                            TokenKind::KwGiven => {
                                self.advance();
                                self.expect(TokenKind::Colon);
                                given.push(self.parse_expression());
                            }
                            TokenKind::KwWhen => {
                                self.advance();
                                self.expect(TokenKind::Colon);
                                when.push(self.parse_expression());
                            }
                            TokenKind::KwAssert => {
                                self.advance();
                                self.expect(TokenKind::Colon);
                                assert_exprs.push(self.parse_expression());
                            }
                            _ => break,
                        }
                    }

                    let end = self.previous_span();
                    tests.push(TestBlock {
                        kind: TestKind::Property {
                            name,
                            quantifiers,
                            given,
                            when,
                            assert: assert_exprs,
                        },
                        span: start.merge(end),
                    });
                }
                _ => {
                    self.advance();
                }
            }
            self.consume_newlines();
        }

        if has_indent && self.check(TokenKind::Dedent) {
            self.advance();
        }

        tests
    }

    // ── Budget blocks ────────────────────────────────

    fn parse_budget_block(&mut self) -> BudgetBlock {
        let start = self.current_span();
        self.expect(TokenKind::BraceOpen);

        let mut entries = Vec::new();
        while !self.check(TokenKind::BraceClose) && !self.is_at_end() {
            let estart = self.current_span();
            let key_tok = self.advance();
            let key = key_tok.text;
            self.expect(TokenKind::Colon);
            let value = self.parse_expression();
            let eend = self.previous_span();
            entries.push(BudgetEntry {
                key,
                value,
                span: estart.merge(eend),
            });
        }

        self.expect(TokenKind::BraceClose);
        let end = self.previous_span();
        BudgetBlock {
            entries,
            span: start.merge(end),
        }
    }

    // ── Identifier lists ─────────────────────────────

    fn parse_identifier_list(&mut self) -> Vec<String> {
        let mut list = Vec::new();

        self.consume_newlines();

        let has_indent = self.check(TokenKind::Indent);
        if has_indent {
            self.advance();
        }

        while self.check(TokenKind::Minus) {
            self.advance(); // consume '-'
            if self.check(TokenKind::Ident) || self.peek_kind().is_keyword() {
                list.push(self.advance().text);
            }
            self.consume_newlines();
        }

        if has_indent && self.check(TokenKind::Dedent) {
            self.advance();
        }

        list
    }

    fn parse_dependency_ref(&mut self) -> Option<DependencyRef> {
        let start = self.current_span();
        if !self.check(TokenKind::Minus) {
            return None;
        }
        self.advance(); // consume '-'

        let name_tok = self.advance();
        let name = name_tok.text.clone();

        let mut notes = None;
        if self.check(TokenKind::ParenOpen) {
            self.advance(); // consume '('
            let mut notes_text = String::new();
            while !self.check(TokenKind::ParenClose) && !self.is_at_end() {
                let tok = self.advance();
                if !notes_text.is_empty() {
                    notes_text.push(' ');
                }
                notes_text.push_str(&tok.text);
            }
            self.expect(TokenKind::ParenClose);
            notes = Some(notes_text);
        }

        let end = self.previous_span();
        Some(DependencyRef {
            name,
            notes,
            span: start.merge(end),
        })
    }

    fn parse_service_policy(&mut self) -> Option<ServicePolicy> {
        let start = self.current_span();
        if !self.check(TokenKind::Minus) {
            return None;
        }
        self.advance(); // consume '-'

        let name_tok = self.advance();
        let name = name_tok.text.clone();

        if self.check(TokenKind::Colon) {
            self.advance();
        }

        self.consume_newlines();

        let has_policy_indent = self.check(TokenKind::Indent);
        if has_policy_indent {
            self.advance();
        }

        let mut entries = Vec::new();
        while !self.is_at_end() {
            self.consume_newlines();
            if has_policy_indent && self.check(TokenKind::Dedent) {
                break;
            }
            if (self.check(TokenKind::Ident) || self.peek_kind().is_keyword())
                && self.peek_at(1).map(|t| t.kind) == Some(TokenKind::Colon)
                && !self.is_section_keyword(self.peek())
            {
                let estart = self.current_span();
                let key = self.advance().text;
                self.expect(TokenKind::Colon);
                let value = self.parse_expression();
                let eend = self.previous_span();
                entries.push(ConfigEntry {
                    key,
                    value,
                    span: estart.merge(eend),
                });
            } else {
                break;
            }
        }

        if has_policy_indent && self.check(TokenKind::Dedent) {
            self.advance();
        }

        let end = self.previous_span();
        Some(ServicePolicy {
            name,
            entries,
            span: start.merge(end),
        })
    }

    // ── Expression lists ─────────────────────────────

    fn parse_expression_list(&mut self) -> Vec<Expression> {
        let mut exprs = Vec::new();

        self.consume_newlines();

        let has_indent = self.check(TokenKind::Indent);
        if has_indent {
            self.advance();
        }

        while self.check(TokenKind::Minus) {
            self.advance(); // consume '-'
            if self.check(TokenKind::Ident)
                && self.peek_at(1).map(|t| t.kind) == Some(TokenKind::Colon)
            {
                let start = self.current_span();
                let label_tok = self.advance();
                self.expect(TokenKind::Colon);
                let expr = self.parse_expression();
                let span = start.merge(self.previous_span());
                exprs.push(Expression::BinaryOp {
                    left: Box::new(Expression::Identifier(label_tok.text, label_tok.span)),
                    op: BinaryOperator::Eq,
                    right: Box::new(expr),
                    span,
                });
            } else {
                exprs.push(self.parse_expression());
            }
            self.consume_newlines();
        }

        if has_indent && self.check(TokenKind::Dedent) {
            self.advance();
        }

        exprs
    }

    // ── Expression parsing (simplified Pratt parser) ─

    fn parse_expression(&mut self) -> Expression {
        self.parse_or_expr()
    }

    fn parse_or_expr(&mut self) -> Expression {
        let mut left = self.parse_and_expr();

        while self.check(TokenKind::PipePipe) || self.check(TokenKind::KwOr) {
            let start = self.current_span();
            self.advance();
            let right = self.parse_and_expr();
            let span = start.merge(self.previous_span());
            left = Expression::BinaryOp {
                left: Box::new(left),
                op: BinaryOperator::Or,
                right: Box::new(right),
                span,
            };
        }

        left
    }

    fn parse_and_expr(&mut self) -> Expression {
        let mut left = self.parse_comparison();

        while self.check(TokenKind::AmpAmp) || self.check(TokenKind::KwAnd) {
            let start = self.current_span();
            self.advance();
            let right = self.parse_comparison();
            let span = start.merge(self.previous_span());
            left = Expression::BinaryOp {
                left: Box::new(left),
                op: BinaryOperator::And,
                right: Box::new(right),
                span,
            };
        }

        left
    }

    fn parse_comparison(&mut self) -> Expression {
        let mut left = self.parse_range();

        if let Some(op) = self.match_comparison_op() {
            let right = self.parse_range();
            let span = left.span().merge(self.previous_span());
            left = Expression::BinaryOp {
                left: Box::new(left),
                op,
                right: Box::new(right),
                span,
            };
        }

        left
    }

    fn match_comparison_op(&mut self) -> Option<BinaryOperator> {
        match self.peek_kind() {
            TokenKind::EqEq => {
                self.advance();
                Some(BinaryOperator::Eq)
            }
            TokenKind::BangEq => {
                self.advance();
                Some(BinaryOperator::NotEq)
            }
            TokenKind::Lt => {
                self.advance();
                Some(BinaryOperator::Lt)
            }
            TokenKind::Gt => {
                self.advance();
                Some(BinaryOperator::Gt)
            }
            TokenKind::LtEq => {
                self.advance();
                Some(BinaryOperator::LtEq)
            }
            TokenKind::GtEq => {
                self.advance();
                Some(BinaryOperator::GtEq)
            }
            TokenKind::KwIn => {
                self.advance();
                Some(BinaryOperator::In)
            }
            TokenKind::KwNot if self.peek_at(1).map(|t| t.kind) == Some(TokenKind::KwIn) => {
                self.advance();
                self.advance();
                Some(BinaryOperator::NotIn)
            }
            _ => None,
        }
    }

    fn parse_range(&mut self) -> Expression {
        let left = self.parse_additive();

        if self.check(TokenKind::DotDot) {
            let start = self.current_span();
            self.advance();
            let right = self.parse_additive();
            let span = start.merge(self.previous_span());
            return Expression::BinaryOp {
                left: Box::new(left),
                op: BinaryOperator::Range,
                right: Box::new(right),
                span,
            };
        }

        if self.check(TokenKind::DotDotLt) {
            let start = self.current_span();
            self.advance();
            let right = self.parse_additive();
            let span = start.merge(self.previous_span());
            return Expression::BinaryOp {
                left: Box::new(left),
                op: BinaryOperator::RangeExc,
                right: Box::new(right),
                span,
            };
        }

        left
    }

    fn parse_additive(&mut self) -> Expression {
        let mut left = self.parse_unary();

        while self.check(TokenKind::Plus)
            || (self.check(TokenKind::Minus) && !self.is_bullet_point())
        {
            let _op_tok = self.advance();
            let right = self.parse_unary();
            let span = left.span().merge(self.previous_span());
            left = Expression::BinaryOp {
                left: Box::new(left),
                op: BinaryOperator::Eq, // simplified: treat +/- as Eq for now
                right: Box::new(right),
                span,
            };
        }

        left
    }

    fn parse_unary(&mut self) -> Expression {
        if self.check(TokenKind::Bang) || self.check(TokenKind::KwNot) {
            let start = self.current_span();
            self.advance();
            let operand = self.parse_unary();
            let span = start.merge(self.previous_span());
            return Expression::UnaryOp {
                op: UnaryOperator::Not,
                operand: Box::new(operand),
                span,
            };
        }

        if self.check(TokenKind::Minus) {
            let start = self.current_span();
            self.advance();
            let operand = self.parse_unary();
            let span = start.merge(self.previous_span());
            return Expression::UnaryOp {
                op: UnaryOperator::Neg,
                operand: Box::new(operand),
                span,
            };
        }

        self.parse_postfix()
    }

    fn parse_postfix(&mut self) -> Expression {
        let mut expr = self.parse_primary();

        loop {
            if self.check(TokenKind::Dot) {
                self.advance();
                let field_tok = self.advance();
                let span = expr.span().merge(self.previous_span());
                expr = Expression::FieldAccess {
                    object: Box::new(expr),
                    field: field_tok.text,
                    span,
                };
            } else if self.check(TokenKind::ParenOpen) {
                // Function call: already parsed name as Identifier
                if let Expression::Identifier(name, _) = &expr {
                    let name = name.clone();
                    let start = expr.span();
                    self.advance(); // consume '('
                    let mut args = Vec::new();
                    while !self.check(TokenKind::ParenClose) && !self.is_at_end() {
                        args.push(self.parse_expression());
                        if self.check(TokenKind::Comma) {
                            self.advance();
                        }
                    }
                    self.expect(TokenKind::ParenClose);
                    let span = start.merge(self.previous_span());
                    expr = Expression::Call {
                        function: name,
                        type_args: None,
                        args,
                        span,
                    };
                } else {
                    break;
                }
            } else {
                break;
            }
        }

        expr
    }

    fn parse_primary(&mut self) -> Expression {
        match self.peek_kind() {
            TokenKind::IntLiteral => {
                let tok = self.advance();
                let val: i64 = tok.text.replace('_', "").parse().unwrap_or(0);
                Expression::Literal(Literal::Int(val))
            }
            TokenKind::FloatLiteral => {
                let tok = self.advance();
                let val: f64 = tok.text.replace('_', "").parse().unwrap_or(0.0);
                Expression::Literal(Literal::Float(val))
            }
            TokenKind::StringLiteral => {
                let tok = self.advance();
                let val = tok.text[1..tok.text.len() - 1].to_string();
                Expression::Literal(Literal::String(val))
            }
            TokenKind::DurationLiteral => {
                let tok = self.advance();
                Expression::Literal(Literal::Duration(tok.text))
            }
            TokenKind::MoneyLiteral => {
                let tok = self.advance();
                Expression::Literal(Literal::Money(tok.text))
            }
            TokenKind::PercentageLiteral => {
                let tok = self.advance();
                // Treat percentage as float for now
                let val: f64 = tok.text.trim_end_matches('%').parse().unwrap_or(0.0);
                Expression::Literal(Literal::Float(val))
            }
            TokenKind::KwTrue => {
                self.advance();
                Expression::Literal(Literal::Bool(true))
            }
            TokenKind::KwFalse => {
                self.advance();
                Expression::Literal(Literal::Bool(false))
            }
            TokenKind::KwNull | TokenKind::KwNone => {
                self.advance();
                Expression::Literal(Literal::Null)
            }
            TokenKind::BracketOpen => {
                let start = self.current_span();
                self.advance(); // consume '['
                let mut items = Vec::new();
                while !self.check(TokenKind::BracketClose) && !self.is_at_end() {
                    items.push(self.parse_expression());
                    if self.check(TokenKind::Comma) {
                        self.advance();
                    }
                }
                self.expect(TokenKind::BracketClose);
                let span = start.merge(self.previous_span());
                Expression::List(items, span)
            }
            TokenKind::ParenOpen => {
                self.advance();
                let expr = self.parse_expression();
                self.expect(TokenKind::ParenClose);
                expr
            }
            TokenKind::Ident
                if self.peek_text() == "arbitrary"
                    && self.peek_at(1).map(|t| t.kind) == Some(TokenKind::Lt) =>
            {
                let tok = self.advance(); // consume "arbitrary"
                self.expect(TokenKind::Lt);
                let type_arg = self.parse_type_ref();
                self.expect(TokenKind::Gt);
                let span = tok.span.merge(self.previous_span());
                Expression::Call {
                    function: "arbitrary".to_string(),
                    type_args: Some(vec![type_arg]),
                    args: Vec::new(),
                    span,
                }
            }
            TokenKind::Ident | TokenKind::KwOld | TokenKind::KwSelfKw => {
                let tok = self.advance();
                Expression::Identifier(tok.text, tok.span)
            }
            _ => {
                // If we hit a keyword that could be an identifier in expression context
                if self.peek_kind().is_keyword() {
                    let tok = self.advance();
                    Expression::Identifier(tok.text, tok.span)
                } else {
                    let tok = self.advance();
                    self.errors.push(ParseError::Expected {
                        expected: "expression".to_string(),
                        found: format!("'{}'", tok.text),
                        span: tok.span,
                    });
                    Expression::Literal(Literal::Null)
                }
            }
        }
    }

    // ── Dotted path parsing ──────────────────────────

    fn parse_dotted_path(&mut self) -> Vec<String> {
        let mut path = Vec::new();

        if self.check(TokenKind::Ident) || self.peek_kind().is_keyword() {
            path.push(self.advance().text);
        }

        while self.check(TokenKind::Dot) {
            self.advance(); // consume '.'
            if self.check(TokenKind::Ident) || self.peek_kind().is_keyword() {
                path.push(self.advance().text);
            }
        }

        path
    }

    // ── Token operations ─────────────────────────────

    fn is_at_end(&self) -> bool {
        self.peek_kind() == TokenKind::Eof
    }

    fn peek(&self) -> &Token {
        &self.tokens[self.pos.min(self.tokens.len() - 1)]
    }

    fn peek_kind(&self) -> TokenKind {
        self.peek().kind
    }

    fn peek_text(&self) -> &str {
        &self.peek().text
    }

    fn peek_at(&self, offset: usize) -> Option<&Token> {
        self.tokens.get(self.pos + offset)
    }

    fn consume_doc_comments(&mut self) -> Option<String> {
        let mut docs = Vec::new();
        loop {
            // Skip newlines between doc comment lines
            self.consume_newlines();
            if !self.check(TokenKind::DocComment) {
                break;
            }
            let tok = self.advance();
            let mut line = tok.text.trim_start_matches('/');
            if line.starts_with(' ') {
                line = &line[1..];
            }
            docs.push(line.trim_end().to_string());
        }
        if docs.is_empty() {
            None
        } else {
            Some(docs.join("\n"))
        }
    }

    fn advance(&mut self) -> Token {
        let token = self.tokens[self.pos.min(self.tokens.len() - 1)].clone();
        if self.pos < self.tokens.len() - 1 {
            self.pos += 1;
        }
        token
    }

    fn check(&self, kind: TokenKind) -> bool {
        self.peek_kind() == kind
    }

    fn expect(&mut self, kind: TokenKind) -> bool {
        if self.check(kind) {
            self.advance();
            true
        } else {
            self.errors.push(ParseError::Expected {
                expected: format!("{kind}"),
                found: format!("'{}'", self.peek_text()),
                span: self.current_span(),
            });
            false
        }
    }

    fn current_span(&self) -> Span {
        self.peek().span
    }

    fn previous_span(&self) -> Span {
        if self.pos > 0 {
            self.tokens[self.pos - 1].span
        } else {
            Span::new(0, 0)
        }
    }

    /// Check if a token is a section keyword (used as headers in RPC/service/component/etc. blocks).
    fn is_section_keyword(&self, token: &Token) -> bool {
        if token.text == "constraints" {
            return true;
        }
        matches!(
            token.kind,
            TokenKind::KwInputs
                | TokenKind::KwOutputs
                | TokenKind::KwErrors
                | TokenKind::KwTests
                | TokenKind::KwPreconditions
                | TokenKind::KwPostconditions
                | TokenKind::KwInvariants
                | TokenKind::KwGoal
                | TokenKind::KwBudget
                | TokenKind::KwDependsOn
                | TokenKind::KwDependencies
                | TokenKind::KwPolicies
                | TokenKind::KwOperation
                | TokenKind::KwProps
                | TokenKind::KwState
                | TokenKind::KwEvents
                | TokenKind::KwSlots
                | TokenKind::KwSource
                | TokenKind::KwStages
                | TokenKind::KwSink
                | TokenKind::KwStates
                | TokenKind::KwTransitions
                | TokenKind::KwTriggers
                | TokenKind::KwRules
                | TokenKind::KwSchedule
                | TokenKind::KwCapabilities
                | TokenKind::KwBoundaries
                | TokenKind::KwTools
                | TokenKind::KwModel
                | TokenKind::KwStyleGuide
                | TokenKind::KwVisualSpec
                | TokenKind::KwDescription
                | TokenKind::KwScope
                | TokenKind::KwTarget
                | TokenKind::KwEntity
                | TokenKind::KwRelations
                | TokenKind::KwIndexes
        )
    }

    fn is_bullet_point(&self) -> bool {
        if !self.check(TokenKind::Minus) {
            return false;
        }
        if let Some(next) = self.peek_at(1) {
            self.peek().span.end < next.span.start
        } else {
            true
        }
    }

    fn parse_component_decl(&mut self) -> Option<ComponentDecl> {
        let start = self.current_span();
        self.advance(); // consume 'component'

        let name_tok = self.advance();
        let name = name_tok.text.clone();

        let close = self.expect_block_start();

        let mut goal = None;
        let mut props = Vec::new();
        let mut state = Vec::new();
        let mut events = Vec::new();
        let mut slots = Vec::new();
        let mut constraints = Vec::new();
        let mut style_guide = None;
        let mut visual_spec = Vec::new();
        let mut tests = Vec::new();

        while !self.check(close) && !self.is_at_end() {
            self.consume_newlines();
            if self.check(close) || self.is_at_end() {
                break;
            }
            match self.peek_kind() {
                TokenKind::KwGoal => {
                    self.advance();
                    if self.check(TokenKind::Colon) {
                        self.advance();
                    }
                    if self.check(TokenKind::StringLiteral) {
                        let tok = self.advance();
                        goal = Some(tok.text[1..tok.text.len() - 1].to_string());
                    }
                }
                TokenKind::KwProps => {
                    self.advance();
                    if self.check(TokenKind::Colon) {
                        self.advance();
                    }
                    props = self.parse_inline_fields();
                }
                TokenKind::KwState => {
                    self.advance();
                    if self.check(TokenKind::Colon) {
                        self.advance();
                    }
                    state = self.parse_inline_fields();
                }
                TokenKind::KwEvents => {
                    self.advance();
                    if self.check(TokenKind::Colon) {
                        self.advance();
                    }
                    self.consume_newlines();
                    let has_events_indent = self.check(TokenKind::Indent);
                    if has_events_indent {
                        self.advance(); // consume Indent
                    }
                    while self.check(TokenKind::Minus) {
                        self.advance(); // consume '-'
                        let start_event = self.current_span();
                        let event_name = self.advance().text;
                        let params = if self.check(TokenKind::ParenOpen) {
                            self.parse_field_list(TokenKind::ParenOpen, TokenKind::ParenClose)
                        } else {
                            Vec::new()
                        };
                        let end_event = self.previous_span();
                        events.push(EventDecl {
                            name: event_name,
                            params,
                            span: start_event.merge(end_event),
                        });
                        self.consume_newlines();
                    }
                    if has_events_indent {
                        self.expect(TokenKind::Dedent);
                    }
                }
                TokenKind::KwSlots => {
                    self.advance();
                    if self.check(TokenKind::Colon) {
                        self.advance();
                    }
                    slots = self.parse_inline_fields();
                }
                TokenKind::KwStyleGuide => {
                    self.advance();
                    if self.check(TokenKind::Colon) {
                        self.advance();
                    }
                    if self.check(TokenKind::StringLiteral) {
                        let tok = self.advance();
                        style_guide = Some(tok.text[1..tok.text.len() - 1].to_string());
                    } else if self.check(TokenKind::Ident) {
                        style_guide = Some(self.advance().text);
                    }
                }
                TokenKind::KwVisualSpec => {
                    self.advance();
                    if self.check(TokenKind::Colon) {
                        self.advance();
                    }
                    self.consume_newlines();
                    let has_spec_indent = self.check(TokenKind::Indent);
                    if has_spec_indent {
                        self.advance(); // consume Indent
                    }
                    while self.is_bullet_point() {
                        self.advance(); // consume '-'
                        let mut spec_str = String::new();
                        let mut last_end = 0;
                        while !self.is_bullet_point()
                            && !self.check(close)
                            && !self.check(TokenKind::Dedent)
                            && !self.is_section_keyword(self.peek())
                            && !self.is_at_end()
                        {
                            let tok = self.advance();
                            if last_end > 0 && tok.span.start > last_end {
                                spec_str.push(' ');
                            }
                            spec_str.push_str(&tok.text);
                            last_end = tok.span.end;
                        }
                        visual_spec.push(spec_str.trim().to_string());
                        self.consume_newlines();
                    }
                    if has_spec_indent {
                        self.expect(TokenKind::Dedent);
                    }
                }
                TokenKind::KwTests => {
                    self.advance();
                    if self.check(TokenKind::Colon) {
                        self.advance();
                    }
                    tests = self.parse_test_list();
                }
                TokenKind::KwConstraints | TokenKind::Ident
                    if self.peek_text() == "constraints" =>
                {
                    self.advance();
                    if self.check(TokenKind::Colon) {
                        self.advance();
                    }
                    constraints = self.parse_constraint_list();
                }
                _ => {
                    self.advance();
                }
            }
            self.consume_newlines();
        }

        self.expect(close);
        let end = self.previous_span();

        Some(ComponentDecl {
            name,
            goal,
            props,
            state,
            events,
            slots,
            constraints,
            style_guide,
            visual_spec,
            tests,
            span: start.merge(end),
        })
    }

    fn parse_pipeline_decl(&mut self) -> Option<PipelineDecl> {
        let start = self.current_span();
        self.advance(); // consume 'pipeline'

        let name_tok = self.advance();
        let name = name_tok.text.clone();

        let close = self.expect_block_start();

        let mut goal = None;
        let mut source = Vec::new();
        let mut stages = Vec::new();
        let mut sink = Vec::new();
        let mut constraints = Vec::new();
        let mut schedule = None;
        let mut tests = Vec::new();

        while !self.check(close) && !self.is_at_end() {
            self.consume_newlines();
            if self.check(close) || self.is_at_end() {
                break;
            }
            match self.peek_kind() {
                TokenKind::KwGoal => {
                    self.advance();
                    if self.check(TokenKind::Colon) {
                        self.advance();
                    }
                    if self.check(TokenKind::StringLiteral) {
                        let tok = self.advance();
                        goal = Some(tok.text[1..tok.text.len() - 1].to_string());
                    }
                }
                TokenKind::KwSource => {
                    self.advance();
                    if self.check(TokenKind::Colon) {
                        self.advance();
                    }
                    source = self.parse_config_entries();
                }
                TokenKind::KwSink => {
                    self.advance();
                    if self.check(TokenKind::Colon) {
                        self.advance();
                    }
                    sink = self.parse_config_entries();
                }
                TokenKind::KwStages => {
                    self.advance();
                    if self.check(TokenKind::Colon) {
                        self.advance();
                    }
                    self.consume_newlines();
                    let has_stages_indent = self.check(TokenKind::Indent);
                    if has_stages_indent {
                        self.advance(); // consume Indent
                    }
                    while self.check(TokenKind::Minus) {
                        self.advance(); // consume '-'
                        let start_stage = self.current_span();
                        let mut stage_name = "unnamed".to_string();
                        let mut entries = Vec::new();

                        while (self.check(TokenKind::Ident) || self.peek_kind().is_keyword())
                            && !self.is_section_keyword(self.peek())
                        {
                            let key = self.advance().text;
                            if self.check(TokenKind::Colon) {
                                self.advance(); // consume ':'
                            }
                            let val = self.parse_expression();
                            let entry_end = self.previous_span();
                            if key == "name" {
                                if let Expression::Literal(Literal::String(s)) = &val {
                                    stage_name = s.clone();
                                } else {
                                    stage_name = format!("{:?}", val);
                                }
                            }
                            entries.push(ConfigEntry {
                                key,
                                value: val,
                                span: start_stage.merge(entry_end),
                            });
                            self.consume_newlines();
                        }
                        let end_stage = self.previous_span();
                        stages.push(PipelineStage {
                            name: stage_name,
                            entries,
                            span: start_stage.merge(end_stage),
                        });
                        self.consume_newlines();
                    }
                    if has_stages_indent {
                        self.expect(TokenKind::Dedent);
                    }
                }
                TokenKind::KwSchedule => {
                    self.advance();
                    if self.check(TokenKind::Colon) {
                        self.advance();
                    }
                    if self.check(TokenKind::StringLiteral) {
                        let tok = self.advance();
                        schedule = Some(tok.text[1..tok.text.len() - 1].to_string());
                    }
                }
                TokenKind::KwTests => {
                    self.advance();
                    if self.check(TokenKind::Colon) {
                        self.advance();
                    }
                    tests = self.parse_test_list();
                }
                TokenKind::KwConstraints | TokenKind::Ident
                    if self.peek_text() == "constraints" =>
                {
                    self.advance();
                    if self.check(TokenKind::Colon) {
                        self.advance();
                    }
                    constraints = self.parse_constraint_list();
                }
                _ => {
                    self.advance();
                }
            }
            self.consume_newlines();
        }

        self.expect(close);
        let end = self.previous_span();

        Some(PipelineDecl {
            name,
            goal,
            source,
            stages,
            sink,
            constraints,
            schedule,
            tests,
            span: start.merge(end),
        })
    }

    fn parse_config_entries(&mut self) -> Vec<ConfigEntry> {
        let mut entries = Vec::new();
        self.consume_newlines();
        let has_indent = self.check(TokenKind::Indent);
        if has_indent {
            self.advance();
        }
        while (self.check(TokenKind::Ident) || self.peek_kind().is_keyword())
            && !self.is_section_keyword(self.peek())
            && !self.is_at_end()
        {
            let start = self.current_span();
            let key = self.advance().text;
            if self.check(TokenKind::Colon) {
                self.advance(); // consume ':'
            }
            let value = self.parse_expression();
            let end = self.previous_span();
            entries.push(ConfigEntry {
                key,
                value,
                span: start.merge(end),
            });
            self.consume_newlines();
        }
        if has_indent {
            self.expect(TokenKind::Dedent);
        }
        entries
    }

    fn parse_workflow_decl(&mut self) -> Option<WorkflowDecl> {
        let start = self.current_span();
        self.advance(); // consume 'workflow' (or 'orchestrator')

        let name_tok = self.advance();
        let name = name_tok.text.clone();

        let close = self.expect_block_start();

        let mut goal = None;
        let mut states = Vec::new();
        let mut transitions = Vec::new();
        let mut constraints = Vec::new();
        let mut tests = Vec::new();
        let mut dependencies = Vec::new();
        let mut policies = Vec::new();
        let mut invariants = Vec::new();

        while !self.check(close) && !self.is_at_end() {
            self.consume_newlines();
            if self.check(close) || self.is_at_end() {
                break;
            }
            match self.peek_kind() {
                TokenKind::KwGoal => {
                    self.advance();
                    if self.check(TokenKind::Colon) {
                        self.advance();
                    }
                    if self.check(TokenKind::StringLiteral) {
                        let tok = self.advance();
                        goal = Some(tok.text[1..tok.text.len() - 1].to_string());
                    }
                }
                TokenKind::KwStates => {
                    self.advance();
                    if self.check(TokenKind::Colon) {
                        self.advance();
                    }
                    self.consume_newlines();
                    let has_brackets = if self.check(TokenKind::BracketOpen) {
                        self.advance();
                        true
                    } else {
                        false
                    };
                    let has_states_indent = !has_brackets && self.check(TokenKind::Indent);
                    if has_states_indent {
                        self.advance();
                    }
                    while ((self.check(TokenKind::Ident)
                        || self.peek_kind().is_keyword()
                        || self.check(TokenKind::Minus))
                        && !self.is_section_keyword(self.peek()))
                        && !self.is_at_end()
                    {
                        if self.check(TokenKind::Minus) {
                            self.advance();
                        }
                        states.push(self.advance().text);
                        if has_brackets && self.check(TokenKind::Comma) {
                            self.advance();
                        }
                        self.consume_newlines();
                    }
                    if has_states_indent {
                        self.expect(TokenKind::Dedent);
                    }
                    if has_brackets {
                        self.expect(TokenKind::BracketClose);
                    }
                }
                TokenKind::KwTransitions => {
                    self.advance();
                    if self.check(TokenKind::Colon) {
                        self.advance();
                    }
                    self.consume_newlines();
                    let has_trans_indent = self.check(TokenKind::Indent);
                    if has_trans_indent {
                        self.advance();
                    }
                    while (self.check(TokenKind::Ident)
                        || self.check(TokenKind::Star)
                        || self.check(TokenKind::Minus))
                        && !self.is_section_keyword(self.peek())
                        && !self.is_at_end()
                    {
                        if self.check(TokenKind::Minus) {
                            self.advance(); // consume '-'
                        }
                        let start_trans = self.current_span();

                        let mut states_in_chain = Vec::new();
                        states_in_chain.push(self.advance().text); // first state
                        while self.check(TokenKind::Arrow) {
                            self.advance(); // consume '->'
                            states_in_chain.push(self.advance().text); // next state
                        }

                        let mut trigger = None;
                        let mut timeout = None;
                        let mut guard = None;
                        let mut actions = Vec::new();

                        if self.check(TokenKind::Colon) {
                            self.advance(); // consume ':'
                            self.consume_newlines();
                            let has_trans_opts_indent = self.check(TokenKind::Indent);
                            if has_trans_opts_indent {
                                self.advance();
                            }
                            while (self.check(TokenKind::Ident) || self.peek_kind().is_keyword())
                                && self.peek_at(1).map(|t| t.kind) == Some(TokenKind::Colon)
                                && !self.is_section_keyword(self.peek())
                            {
                                let key = self.advance().text;
                                self.advance(); // consume ':'
                                if key == "trigger" {
                                    trigger = Some(self.advance().text);
                                } else if key == "timeout" {
                                    let duration = self.parse_expression();
                                    self.expect(TokenKind::Arrow);
                                    let target_state = self.advance().text;
                                    timeout = Some(WorkflowTimeout {
                                        duration,
                                        target_state,
                                        span: start_trans.merge(self.previous_span()),
                                    });
                                } else if key == "guard" {
                                    guard = Some(self.parse_expression());
                                } else if key == "action" {
                                    while self.check(TokenKind::Ident) {
                                        actions.push(self.advance().text);
                                        if self.check(TokenKind::Comma) {
                                            self.advance();
                                        } else {
                                            break;
                                        }
                                    }
                                } else {
                                    self.parse_expression();
                                }
                                self.consume_newlines();
                            }
                            if has_trans_opts_indent {
                                self.expect(TokenKind::Dedent);
                            }
                        }

                        if self.check(TokenKind::BracketOpen) {
                            self.advance(); // consume '['
                            while !self.check(TokenKind::BracketClose) && !self.is_at_end() {
                                let key = if self.check(TokenKind::Ident) {
                                    self.advance().text
                                } else {
                                    break;
                                };
                                self.expect(TokenKind::Colon);
                                if key == "on" {
                                    let mut trigger_text = String::new();
                                    while !self.check(TokenKind::BracketClose)
                                        && !self.check(TokenKind::Comma)
                                        && !self.is_at_end()
                                    {
                                        let tok = self.advance();
                                        if !trigger_text.is_empty() {
                                            trigger_text.push(' ');
                                        }
                                        trigger_text.push_str(&tok.text);
                                    }
                                    trigger = Some(trigger_text);
                                } else if key == "guard" {
                                    guard = Some(self.parse_expression());
                                }
                                if self.check(TokenKind::Comma) {
                                    self.advance();
                                }
                            }
                            self.expect(TokenKind::BracketClose);
                        }

                        // Generate sequential transitions from the chain
                        for i in 0..states_in_chain.len().saturating_sub(1) {
                            transitions.push(WorkflowTransition {
                                from: states_in_chain[i].clone(),
                                to: states_in_chain[i + 1].clone(),
                                trigger: trigger.clone(),
                                timeout: timeout.clone(),
                                guard: guard.clone(),
                                actions: actions.clone(),
                                span: start_trans.merge(self.previous_span()),
                            });
                        }
                        self.consume_newlines();
                    }
                    if has_trans_indent {
                        self.expect(TokenKind::Dedent);
                    }
                }
                TokenKind::KwDependencies => {
                    self.advance();
                    if self.check(TokenKind::Colon) {
                        self.advance();
                    }
                    self.consume_newlines();
                    let has_dep_indent = self.check(TokenKind::Indent);
                    if has_dep_indent {
                        self.advance();
                    }
                    while self.check(TokenKind::Minus) {
                        if let Some(dep) = self.parse_dependency_ref() {
                            dependencies.push(dep);
                        }
                        self.consume_newlines();
                    }
                    if has_dep_indent {
                        self.expect(TokenKind::Dedent);
                    }
                }
                TokenKind::KwPolicies => {
                    self.advance();
                    if self.check(TokenKind::Colon) {
                        self.advance();
                    }
                    self.consume_newlines();
                    let has_pol_indent = self.check(TokenKind::Indent);
                    if has_pol_indent {
                        self.advance();
                    }
                    while self.check(TokenKind::Minus) {
                        if let Some(policy) = self.parse_service_policy() {
                            policies.push(policy);
                        }
                        self.consume_newlines();
                    }
                    if has_pol_indent {
                        self.expect(TokenKind::Dedent);
                    }
                }
                TokenKind::KwInvariants => {
                    self.advance();
                    if self.check(TokenKind::Colon) {
                        self.advance();
                    }
                    invariants = self.parse_expression_list();
                }
                TokenKind::KwTests => {
                    self.advance();
                    if self.check(TokenKind::Colon) {
                        self.advance();
                    }
                    tests = self.parse_test_list();
                }
                TokenKind::KwConstraints | TokenKind::Ident
                    if self.peek_text() == "constraints" =>
                {
                    self.advance();
                    if self.check(TokenKind::Colon) {
                        self.advance();
                    }
                    constraints = self.parse_constraint_list();
                }
                _ => {
                    self.advance();
                }
            }
            self.consume_newlines();
        }

        self.expect(close);
        let end = self.previous_span();

        Some(WorkflowDecl {
            name,
            goal,
            states,
            transitions,
            constraints,
            tests,
            dependencies,
            policies,
            invariants,
            span: start.merge(end),
        })
    }

    fn parse_agent_decl(&mut self) -> Option<AgentDecl> {
        let start = self.current_span();
        self.advance(); // consume 'agent'

        let name_tok = self.advance();
        let name = name_tok.text.clone();

        let close = self.expect_block_start();

        let mut goal = None;
        let mut capabilities = Vec::new();
        let mut boundaries = Vec::new();
        let mut tools = Vec::new();
        let mut model = Vec::new();
        let mut budget = None;
        let mut tests = Vec::new();

        while !self.check(close) && !self.is_at_end() {
            self.consume_newlines();
            if self.check(close) || self.is_at_end() {
                break;
            }
            match self.peek_kind() {
                TokenKind::KwGoal => {
                    self.advance();
                    if self.check(TokenKind::Colon) {
                        self.advance();
                    }
                    if self.check(TokenKind::StringLiteral) {
                        let tok = self.advance();
                        goal = Some(tok.text[1..tok.text.len() - 1].to_string());
                    }
                }
                TokenKind::KwCapabilities => {
                    self.advance();
                    if self.check(TokenKind::Colon) {
                        self.advance();
                    }
                    self.consume_newlines();
                    let has_cap_indent = self.check(TokenKind::Indent);
                    if has_cap_indent {
                        self.advance(); // consume Indent
                    }
                    while self.check(TokenKind::Minus) {
                        self.advance(); // consume '-'
                        capabilities.push(self.advance().text);
                        self.consume_newlines();
                    }
                    if has_cap_indent {
                        self.expect(TokenKind::Dedent);
                    }
                }
                TokenKind::KwBoundaries => {
                    self.advance();
                    if self.check(TokenKind::Colon) {
                        self.advance();
                    }
                    self.consume_newlines();
                    let has_bound_indent = self.check(TokenKind::Indent);
                    if has_bound_indent {
                        self.advance(); // consume Indent
                    }
                    while self.check(TokenKind::Minus) {
                        self.advance(); // consume '-'
                        let start_bound = self.current_span();
                        let kind = if self.check(TokenKind::KwMust) {
                            self.advance();
                            BoundaryKind::Must
                        } else if self.check(TokenKind::KwCannot) {
                            self.advance();
                            BoundaryKind::Cannot
                        } else {
                            BoundaryKind::Must
                        };
                        if self.check(TokenKind::Colon) {
                            self.advance();
                        }
                        let expr = self.parse_expression();
                        let end_bound = self.previous_span();
                        boundaries.push(AgentBoundary {
                            kind,
                            expr,
                            span: start_bound.merge(end_bound),
                        });
                        self.consume_newlines();
                    }
                    if has_bound_indent {
                        self.expect(TokenKind::Dedent);
                    }
                }
                TokenKind::KwTools => {
                    self.advance();
                    if self.check(TokenKind::Colon) {
                        self.advance();
                    }
                    self.consume_newlines();
                    let has_tools_indent = self.check(TokenKind::Indent);
                    if has_tools_indent {
                        self.advance(); // consume Indent
                    }
                    while self.check(TokenKind::Minus) {
                        self.advance(); // consume '-'
                        let start_tool = self.current_span();
                        let tool_name = self.advance().text;
                        self.expect(TokenKind::ParenOpen);

                        let mut inputs = Vec::new();
                        let mut outputs = Vec::new();

                        while !self.check(TokenKind::ParenClose) && !self.is_at_end() {
                            let field_start = self.current_span();
                            let param_name = self.advance().text;
                            if self.check(TokenKind::Colon) {
                                self.advance();
                            }
                            let ty = self.parse_type_ref();
                            let field_end = self.previous_span();
                            let field = Field {
                                name: param_name.clone(),
                                ty,
                                default: None,
                                doc_comment: None,
                                span: field_start.merge(field_end),
                            };

                            if param_name == "input" {
                                inputs.push(field);
                            } else if param_name == "output" {
                                outputs.push(field);
                            } else {
                                inputs.push(field);
                            }

                            if self.check(TokenKind::Comma) {
                                self.advance();
                            }
                        }
                        self.expect(TokenKind::ParenClose);
                        let end_tool = self.previous_span();
                        tools.push(AgentTool {
                            name: tool_name,
                            inputs,
                            outputs,
                            span: start_tool.merge(end_tool),
                        });
                        self.consume_newlines();
                    }
                    if has_tools_indent {
                        self.expect(TokenKind::Dedent);
                    }
                }
                TokenKind::KwModel => {
                    self.advance();
                    if self.check(TokenKind::Colon) {
                        self.advance();
                    }
                    model = self.parse_config_entries();
                }
                TokenKind::KwBudget => {
                    self.advance();
                    if self.check(TokenKind::Colon) {
                        self.advance();
                    }
                    budget = Some(self.parse_budget_block());
                }
                TokenKind::KwTests => {
                    self.advance();
                    if self.check(TokenKind::Colon) {
                        self.advance();
                    }
                    tests = self.parse_test_list();
                }
                _ => {
                    self.advance();
                }
            }
            self.consume_newlines();
        }

        self.expect(close);
        let end = self.previous_span();

        Some(AgentDecl {
            name,
            goal,
            capabilities,
            boundaries,
            tools,
            model,
            budget,
            tests,
            span: start.merge(end),
        })
    }

    fn parse_schema_decl(&mut self) -> Option<SchemaDecl> {
        let start = self.current_span();
        self.advance(); // consume 'schema'

        let name_tok = self.advance();
        let name = name_tok.text.clone();

        let close = self.expect_block_start();

        let mut goal = None;
        let mut target = None;
        let mut entities = Vec::new();
        let mut relations = Vec::new();
        let mut indexes = Vec::new();
        let mut constraints = Vec::new();

        while !self.check(close) && !self.is_at_end() {
            self.consume_newlines();
            if self.check(close) || self.is_at_end() {
                break;
            }
            let doc_comment = self.consume_doc_comments();
            match self.peek_kind() {
                TokenKind::KwGoal => {
                    self.advance();
                    if self.check(TokenKind::Colon) {
                        self.advance();
                    }
                    if self.check(TokenKind::StringLiteral) {
                        let tok = self.advance();
                        goal = Some(tok.text[1..tok.text.len() - 1].to_string());
                    }
                }
                TokenKind::KwTarget => {
                    self.advance();
                    if self.check(TokenKind::Colon) {
                        self.advance();
                    }
                    target = Some(self.advance().text);
                }
                TokenKind::KwEntity => {
                    self.advance(); // consume 'entity'
                    let start_entity = self.current_span();
                    let entity_name = self.advance().text;
                    let close_entity = self.expect_block_start();

                    let mut fields = Vec::new();
                    while !self.check(close_entity) && !self.is_at_end() {
                        self.consume_newlines();
                        if self.check(close_entity) || self.is_at_end() {
                            break;
                        }
                        let field_doc_comment = self.consume_doc_comments();
                        let field_start = self.current_span();
                        let field_name = self.advance().text;
                        if self.check(TokenKind::Colon) {
                            self.advance();
                        }
                        let ty = self.parse_type_ref();

                        let default = if self.check(TokenKind::Eq) {
                            self.advance();
                            Some(self.parse_expression())
                        } else {
                            None
                        };

                        let mut decorators = Vec::new();
                        while self.check(TokenKind::At) {
                            self.advance(); // consume '@'
                            let dec_start = self.current_span();
                            let dec_name = self.advance().text;
                            let mut args = Vec::new();
                            if self.check(TokenKind::ParenOpen) {
                                self.advance(); // consume '('
                                while !self.check(TokenKind::ParenClose) && !self.is_at_end() {
                                    args.push(self.parse_expression());
                                    if self.check(TokenKind::Comma) {
                                        self.advance();
                                    }
                                }
                                self.expect(TokenKind::ParenClose);
                            }
                            let dec_end = self.previous_span();
                            decorators.push(Decorator {
                                name: dec_name,
                                args,
                                span: dec_start.merge(dec_end),
                            });
                        }

                        let field_end = self.previous_span();
                        fields.push(EntityField {
                            name: field_name,
                            ty,
                            default,
                            decorators,
                            doc_comment: field_doc_comment,
                            span: field_start.merge(field_end),
                        });
                        self.consume_newlines();
                    }
                    self.expect(close_entity);
                    let end_entity = self.previous_span();
                    entities.push(EntityDecl {
                        name: entity_name,
                        fields,
                        doc_comment,
                        span: start_entity.merge(end_entity),
                    });
                }
                TokenKind::KwRelations => {
                    self.advance();
                    if self.check(TokenKind::Colon) {
                        self.advance();
                    }
                    self.consume_newlines();
                    let has_rel_indent = self.check(TokenKind::Indent);
                    if has_rel_indent {
                        self.advance();
                    }
                    while self.check(TokenKind::Minus) {
                        self.advance(); // consume '-'
                        let start_rel = self.current_span();
                        let lhs = self.advance().text;
                        let rel_type = self.advance().text;
                        let rhs = self.advance().text;
                        let mut args = Vec::new();
                        if self.check(TokenKind::ParenOpen) {
                            self.advance();
                            args = self.parse_config_entries();
                            self.expect(TokenKind::ParenClose);
                        }
                        let end_rel = self.previous_span();
                        relations.push(RelationDecl {
                            lhs,
                            rel_type,
                            rhs,
                            args,
                            span: start_rel.merge(end_rel),
                        });
                        self.consume_newlines();
                    }
                    if has_rel_indent {
                        self.expect(TokenKind::Dedent);
                    }
                }
                TokenKind::KwIndexes => {
                    self.advance();
                    if self.check(TokenKind::Colon) {
                        self.advance();
                    }
                    self.consume_newlines();
                    let has_idx_indent = self.check(TokenKind::Indent);
                    if has_idx_indent {
                        self.advance();
                    }
                    while self.check(TokenKind::Minus) {
                        self.advance(); // consume '-'
                        let start_idx = self.current_span();
                        let entity = self.advance().text;
                        self.expect(TokenKind::ParenOpen);
                        let mut fields = Vec::new();
                        while !self.check(TokenKind::ParenClose) && !self.is_at_end() {
                            fields.push(self.advance().text);
                            if self.check(TokenKind::Comma) {
                                self.advance();
                            }
                        }
                        self.expect(TokenKind::ParenClose);
                        let mut r#where = None;
                        if self.check(TokenKind::KwIf)
                            || (self.check(TokenKind::Ident) && self.peek_text() == "where")
                        {
                            self.advance();
                            r#where = Some(self.parse_expression());
                        }
                        let end_idx = self.previous_span();
                        indexes.push(IndexDecl {
                            entity,
                            fields,
                            r#where,
                            span: start_idx.merge(end_idx),
                        });
                        self.consume_newlines();
                    }
                    if has_idx_indent {
                        self.expect(TokenKind::Dedent);
                    }
                }
                TokenKind::KwConstraints | TokenKind::Ident
                    if self.peek_text() == "constraints" =>
                {
                    self.advance();
                    if self.check(TokenKind::Colon) {
                        self.advance();
                    }
                    constraints = self.parse_constraint_list();
                }
                _ => {
                    self.advance();
                }
            }
            self.consume_newlines();
        }

        self.expect(close);
        let end = self.previous_span();

        Some(SchemaDecl {
            name,
            goal,
            target,
            entities,
            relations,
            indexes,
            constraints,
            span: start.merge(end),
        })
    }

    fn parse_policy_decl(&mut self) -> Option<PolicyDecl> {
        let start = self.current_span();
        self.advance(); // consume 'policy'

        let name_tok = self.advance();
        let name = name_tok.text.clone();

        let close = self.expect_block_start();

        let mut description = None;
        let mut scope = None;
        let mut rules = Vec::new();

        while !self.check(close) && !self.is_at_end() {
            self.consume_newlines();
            if self.check(close) || self.is_at_end() {
                break;
            }
            match self.peek_kind() {
                TokenKind::KwDescription => {
                    self.advance();
                    if self.check(TokenKind::Colon) {
                        self.advance();
                    }
                    if self.check(TokenKind::StringLiteral) {
                        let tok = self.advance();
                        description = Some(tok.text[1..tok.text.len() - 1].to_string());
                    }
                }
                TokenKind::KwScope => {
                    self.advance();
                    if self.check(TokenKind::Colon) {
                        self.advance();
                    }
                    scope = Some(self.advance().text);
                }
                TokenKind::KwRules => {
                    self.advance();
                    if self.check(TokenKind::Colon) {
                        self.advance();
                    }
                    self.consume_newlines();
                    let has_rules_indent = self.check(TokenKind::Indent);
                    if has_rules_indent {
                        self.advance();
                    }
                    while self.is_bullet_point() {
                        self.advance(); // consume '-'
                        let start_rule = self.current_span();

                        let mut condition = String::new();
                        let mut last_end = 0;
                        while !self.check(TokenKind::Colon) && !self.is_at_end() {
                            let tok = self.advance();
                            if last_end > 0 && tok.span.start > last_end {
                                condition.push(' ');
                            }
                            condition.push_str(&tok.text);
                            last_end = tok.span.end;
                        }
                        self.expect(TokenKind::Colon);
                        condition = condition.trim().to_string();

                        self.consume_newlines();
                        let has_clauses_indent = self.check(TokenKind::Indent);
                        if has_clauses_indent {
                            self.advance();
                        }

                        let mut clauses = Vec::new();
                        while self.is_bullet_point() {
                            self.advance(); // consume '-'
                            let start_clause = self.current_span();
                            if (self.check(TokenKind::Ident) || self.peek_kind().is_keyword())
                                && self.peek_at(1).map(|t| t.kind) == Some(TokenKind::Colon)
                            {
                                let verb = self.advance().text;
                                self.advance(); // consume ':'
                                let val = self.parse_expression();
                                let end_clause = self.previous_span();
                                clauses.push(PolicyClause::Action {
                                    verb,
                                    value: val,
                                    span: start_clause.merge(end_clause),
                                });
                            } else {
                                let mut clause_str = String::new();
                                let mut last_end = 0;
                                while !self.is_bullet_point()
                                    && !self.check(TokenKind::Dedent)
                                    && !self.check(close)
                                    && !self.is_section_keyword(self.peek())
                                    && !self.is_at_end()
                                {
                                    let tok = self.advance();
                                    if last_end > 0 && tok.span.start > last_end {
                                        clause_str.push(' ');
                                    }
                                    clause_str.push_str(&tok.text);
                                    last_end = tok.span.end;
                                }
                                let end_clause = self.previous_span();
                                clauses.push(PolicyClause::Simple(
                                    clause_str.trim().to_string(),
                                    start_clause.merge(end_clause),
                                ));
                            }
                            self.consume_newlines();
                        }
                        if has_clauses_indent {
                            self.expect(TokenKind::Dedent);
                        }
                        let end_rule = self.previous_span();
                        rules.push(PolicyRule {
                            condition,
                            clauses,
                            span: start_rule.merge(end_rule),
                        });
                        self.consume_newlines();
                    }
                    if has_rules_indent {
                        self.expect(TokenKind::Dedent);
                    }
                }
                _ => {
                    self.advance();
                }
            }
            self.consume_newlines();
        }

        self.expect(close);
        let end = self.previous_span();

        Some(PolicyDecl {
            name,
            description,
            scope,
            rules,
            span: start.merge(end),
        })
    }

    fn parse_constraint_decl(&mut self) -> Option<ConstraintDecl> {
        let start = self.current_span();
        self.advance(); // consume 'constraint'

        let name_tok = self.advance();
        let name = name_tok.text.clone();

        let close = self.expect_block_start();

        let mut requires = Vec::new();
        let mut verification = Vec::new();

        while !self.check(close) && !self.is_at_end() {
            self.consume_newlines();
            if self.check(close) || self.is_at_end() {
                break;
            }
            match self.peek_kind() {
                TokenKind::Ident if self.peek_text() == "requires" => {
                    self.advance(); // consume "requires"
                    if self.check(TokenKind::Colon) {
                        self.advance();
                    }
                    requires = self.parse_constraint_list();
                }
                TokenKind::Ident if self.peek_text() == "verification" => {
                    self.advance(); // consume "verification"
                    if self.check(TokenKind::Colon) {
                        self.advance();
                    }
                    self.consume_newlines();
                    let has_verif_indent = self.check(TokenKind::Indent);
                    if has_verif_indent {
                        self.advance();
                    }
                    while self.is_bullet_point() {
                        self.advance(); // consume '-'
                        let vstart = self.current_span();

                        let mut tool = String::new();
                        let mut evidence = String::new();

                        while (self.check(TokenKind::Ident) || self.peek_kind().is_keyword())
                            && !self.is_section_keyword(self.peek())
                        {
                            let key = self.advance().text;
                            if self.check(TokenKind::Colon) {
                                self.advance(); // consume ':'
                            }
                            let val_expr = self.parse_expression();
                            let val_str = match val_expr {
                                Expression::Identifier(s, _) => s,
                                Expression::Literal(Literal::String(s)) => s,
                                _ => "unknown".to_string(),
                            };
                            if key == "tool" {
                                tool = val_str;
                            } else if key == "evidence" {
                                evidence = val_str;
                            }
                            self.consume_newlines();
                        }

                        let vend = self.previous_span();
                        verification.push(VerificationEntry {
                            tool,
                            evidence,
                            span: vstart.merge(vend),
                        });
                        self.consume_newlines();
                    }
                    if has_verif_indent {
                        self.expect(TokenKind::Dedent);
                    }
                }
                _ => {
                    self.advance();
                }
            }
            self.consume_newlines();
        }

        self.expect(close);
        let end = self.previous_span();

        Some(ConstraintDecl {
            name,
            requires,
            verification,
            span: start.merge(end),
        })
    }

    fn parse_mixin_decl(&mut self) -> Option<MixinDecl> {
        let start = self.current_span();
        self.advance(); // consume 'mixin'

        let name_tok = self.advance();
        let name = name_tok.text.clone();

        let close = self.expect_block_start();

        let mut constraints = Vec::new();
        let mut postconditions = Vec::new();
        let mut tests = Vec::new();

        while !self.check(close) && !self.is_at_end() {
            self.consume_newlines();
            if self.check(close) || self.is_at_end() {
                break;
            }
            match self.peek_kind() {
                TokenKind::KwConstraints | TokenKind::Ident
                    if self.peek_text() == "constraints" =>
                {
                    self.advance(); // consume "constraints"
                    if self.check(TokenKind::Colon) {
                        self.advance();
                    }
                    constraints = self.parse_constraint_list();
                }
                TokenKind::KwPostconditions => {
                    self.advance();
                    if self.check(TokenKind::Colon) {
                        self.advance();
                    }
                    postconditions = self.parse_expression_list();
                }
                TokenKind::KwTests => {
                    self.advance();
                    if self.check(TokenKind::Colon) {
                        self.advance();
                    }
                    tests = self.parse_test_list();
                }
                _ => {
                    self.advance();
                }
            }
            self.consume_newlines();
        }

        self.expect(close);
        let end = self.previous_span();

        Some(MixinDecl {
            name,
            constraints,
            postconditions,
            tests,
            visibility: Visibility::Public,
            span: start.merge(end),
        })
    }

    fn parse_target_dependencies_decl(&mut self) -> Option<TargetDependenciesDecl> {
        let start = self.current_span();
        self.advance(); // consume 'target_dependencies'

        let close = self.expect_block_start();

        let mut entries = Vec::new();

        while !self.check(close) && !self.is_at_end() {
            self.consume_newlines();
            if self.check(close) || self.is_at_end() {
                break;
            }
            let entry_start = self.current_span();
            let target_tok = self.advance(); // typically Ident e.g. "typescript"
            let target = target_tok.text.clone();

            if self.check(TokenKind::Colon) {
                self.advance();
            }

            let entry_close = self.expect_block_start();

            let mut packages = Vec::new();

            while !self.check(entry_close) && !self.is_at_end() {
                self.consume_newlines();
                if self.check(entry_close) || self.is_at_end() {
                    break;
                }
                let pkg_start = self.current_span();
                // package name can be StringLiteral (e.g., "@types/node") or Ident (e.g., "dotenv")
                let name = match self.peek_kind() {
                    TokenKind::StringLiteral => {
                        let name_tok = self.advance();
                        // Strip quotes
                        let s = name_tok.text;
                        if s.starts_with('"') && s.ends_with('"') && s.len() >= 2 {
                            s[1..s.len() - 1].to_string()
                        } else {
                            s
                        }
                    }
                    TokenKind::Ident => {
                        let name_tok = self.advance();
                        name_tok.text
                    }
                    _ => {
                        let tok = self.advance();
                        self.errors.push(ParseError::Expected {
                            expected: "package name (identifier or string literal)".to_string(),
                            found: format!("'{}'", tok.text),
                            span: tok.span,
                        });
                        self.consume_newlines();
                        continue;
                    }
                };

                if self.check(TokenKind::Colon) {
                    self.advance();
                }

                let version = match self.peek_kind() {
                    TokenKind::StringLiteral => {
                        let ver_tok = self.advance();
                        let s = ver_tok.text;
                        if s.starts_with('"') && s.ends_with('"') && s.len() >= 2 {
                            s[1..s.len() - 1].to_string()
                        } else {
                            s
                        }
                    }
                    _ => {
                        let tok = self.advance();
                        self.errors.push(ParseError::Expected {
                            expected: "package version (string literal)".to_string(),
                            found: format!("'{}'", tok.text),
                            span: tok.span,
                        });
                        self.consume_newlines();
                        continue;
                    }
                };

                if self.check(TokenKind::Comma) {
                    self.advance();
                }
                self.consume_newlines();

                let pkg_end = self.previous_span();
                packages.push(DependencyPackage {
                    name,
                    version,
                    span: pkg_start.merge(pkg_end),
                });
            }

            self.expect(entry_close);
            let entry_end = self.previous_span();
            entries.push(TargetDependencyEntry {
                target,
                packages,
                span: entry_start.merge(entry_end),
            });
            self.consume_newlines();
        }

        self.expect(close);
        let end = self.previous_span();

        Some(TargetDependenciesDecl {
            entries,
            span: start.merge(end),
        })
    }

    fn synchronize(&mut self) {
        while !self.is_at_end() {
            match self.peek_kind() {
                TokenKind::KwModule
                | TokenKind::KwUse
                | TokenKind::KwType
                | TokenKind::KwService
                | TokenKind::KwEntity
                | TokenKind::KwAction
                | TokenKind::KwRule
                | TokenKind::KwSchema
                | TokenKind::KwConstraint
                | TokenKind::KwMixin
                | TokenKind::KwExport
                | TokenKind::KwPrivate
                | TokenKind::KwTargetDependencies => return,
                _ => {
                    self.advance();
                }
            }
        }
    }

    fn parse_entity_decl(&mut self) -> Option<EntityDecl> {
        let start = self.current_span();
        self.advance(); // consume 'entity'

        let name_tok = self.advance();
        let name = name_tok.text.clone();

        self.consume_newlines();

        let fields = self.parse_block(|parser| {
            let doc_comment = parser.consume_doc_comments();
            let fstart = parser.current_span();

            if !parser.check(TokenKind::Ident) && !parser.peek_kind().is_keyword() {
                return None;
            }
            let fname = parser.advance().text;

            if parser.check(TokenKind::Colon) {
                parser.advance();
            }

            let ty = parser.parse_type_ref();

            let default = if parser.check(TokenKind::Eq) {
                parser.advance();
                Some(parser.parse_expression())
            } else {
                None
            };

            let mut decorators = Vec::new();
            while parser.check(TokenKind::At) {
                parser.advance(); // consume '@'
                let dec_start = parser.current_span();
                let dec_name = parser.advance().text;
                let mut args = Vec::new();
                if parser.check(TokenKind::ParenOpen) {
                    parser.advance();
                    loop {
                        args.push(parser.parse_expression());
                        if parser.check(TokenKind::Comma) {
                            parser.advance();
                        } else {
                            break;
                        }
                    }
                    parser.expect(TokenKind::ParenClose);
                }
                let dec_end = parser.previous_span();
                decorators.push(Decorator {
                    name: dec_name,
                    args,
                    span: dec_start.merge(dec_end),
                });
            }

            // Consume optional trailing newline or comma
            if parser.check(TokenKind::Comma) {
                parser.advance();
            }
            parser.consume_newlines();

            let fend = parser.previous_span();
            Some(EntityField {
                name: fname,
                ty,
                default,
                decorators,
                doc_comment,
                span: fstart.merge(fend),
            })
        });

        let end = self.previous_span();
        Some(EntityDecl {
            name,
            fields,
            doc_comment: None,
            span: start.merge(end),
        })
    }

    fn parse_action_decl(&mut self) -> Option<ActionDecl> {
        let start = self.current_span();
        self.advance(); // consume 'action'

        let name = self.advance().text;
        self.consume_newlines();

        let mut inputs = Vec::new();
        let mut outputs = Vec::new();
        let mut preconditions = Vec::new();
        let mut postconditions = Vec::new();

        self.expect(TokenKind::Indent);
        while !self.check(TokenKind::Dedent) && !self.is_at_end() {
            self.consume_newlines();
            if self.check(TokenKind::Dedent) || self.is_at_end() {
                break;
            }

            match self.peek_kind() {
                TokenKind::KwInputs => {
                    self.advance();
                    if self.check(TokenKind::Colon) {
                        self.advance();
                    }
                    self.consume_newlines();
                    inputs = self.parse_block(|parser| {
                        let fstart = parser.current_span();
                        let fname = parser.advance().text;
                        if parser.check(TokenKind::Colon) {
                            parser.advance();
                        }
                        let ty = parser.parse_type_ref();
                        let default = if parser.check(TokenKind::Eq) {
                            parser.advance();
                            Some(parser.parse_expression())
                        } else {
                            None
                        };
                        if parser.check(TokenKind::Comma) {
                            parser.advance();
                        }
                        parser.consume_newlines();
                        let fend = parser.previous_span();
                        Some(Field {
                            name: fname,
                            ty,
                            default,
                            doc_comment: None,
                            span: fstart.merge(fend),
                        })
                    });
                }
                TokenKind::KwOutputs => {
                    self.advance();
                    if self.check(TokenKind::Colon) {
                        self.advance();
                    }
                    self.consume_newlines();
                    outputs = self.parse_block(|parser| {
                        let fstart = parser.current_span();
                        let fname = parser.advance().text;
                        if parser.check(TokenKind::Colon) {
                            parser.advance();
                        }
                        let ty = parser.parse_type_ref();
                        let default = if parser.check(TokenKind::Eq) {
                            parser.advance();
                            Some(parser.parse_expression())
                        } else {
                            None
                        };
                        if parser.check(TokenKind::Comma) {
                            parser.advance();
                        }
                        parser.consume_newlines();
                        let fend = parser.previous_span();
                        Some(Field {
                            name: fname,
                            ty,
                            default,
                            doc_comment: None,
                            span: fstart.merge(fend),
                        })
                    });
                }
                TokenKind::KwPreconditions | TokenKind::Ident
                    if self.peek_text() == "require" || self.peek_text() == "preconditions" =>
                {
                    self.advance();
                    if self.check(TokenKind::Colon) {
                        self.advance();
                    }
                    self.consume_newlines();

                    preconditions = self.parse_block(|parser| {
                        if parser.check(TokenKind::Minus) {
                            parser.advance();
                        }
                        let expr = parser.parse_expression();
                        parser.consume_newlines();
                        Some(expr)
                    });
                }
                TokenKind::KwPostconditions | TokenKind::Ident
                    if self.peek_text() == "ensure" || self.peek_text() == "postconditions" =>
                {
                    self.advance();
                    if self.check(TokenKind::Colon) {
                        self.advance();
                    }
                    self.consume_newlines();

                    postconditions = self.parse_block(|parser| {
                        if parser.check(TokenKind::Minus) {
                            parser.advance();
                        }
                        let expr = parser.parse_expression();
                        parser.consume_newlines();
                        Some(expr)
                    });
                }
                _ => {
                    self.advance();
                }
            }
            self.consume_newlines();
        }
        self.expect(TokenKind::Dedent);

        let end = self.previous_span();
        Some(ActionDecl {
            name,
            inputs,
            outputs,
            preconditions,
            postconditions,
            doc_comment: None,
            span: start.merge(end),
        })
    }

    fn parse_rule_decl(&mut self) -> Option<RuleDecl> {
        let start = self.current_span();
        self.advance(); // consume 'rule'

        let name = self.advance().text;

        if self.check(TokenKind::Ident) && self.peek_text() == "on" {
            self.advance();
        }

        let target = self.advance().text;
        self.consume_newlines();

        self.expect(TokenKind::Indent);
        self.consume_newlines();
        let condition = self.parse_expression();
        self.consume_newlines();
        self.expect(TokenKind::Dedent);

        let end = self.previous_span();
        Some(RuleDecl {
            name,
            target,
            condition,
            doc_comment: None,
            span: start.merge(end),
        })
    }
}

// Add span method to Expression
impl Expression {
    pub fn span(&self) -> Span {
        match self {
            Expression::Literal(_) => Span::new(0, 0), // TODO: track literal spans
            Expression::Identifier(_, span) => *span,
            Expression::BinaryOp { span, .. } => *span,
            Expression::UnaryOp { span, .. } => *span,
            Expression::Call { span, .. } => *span,
            Expression::FieldAccess { span, .. } => *span,
            Expression::List(_, span) => *span,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::lexer::Lexer;

    fn parse(input: &str) -> (SourceFile, Vec<ParseError>) {
        let (tokens, lex_errors) = Lexer::new(input).tokenize();
        assert!(lex_errors.is_empty(), "lexer errors: {lex_errors:?}");
        Parser::new(tokens).parse()
    }

    fn parse_ok(input: &str) -> SourceFile {
        let (file, errors) = parse(input);
        assert!(errors.is_empty(), "parse errors: {errors:?}");
        file
    }

    #[test]
    fn parse_module_decl() {
        let file = parse_ok("module acme.payments.checkout");
        assert_eq!(file.module.path, vec!["acme", "payments", "checkout"]);
    }

    #[test]
    fn parse_import_wildcard() {
        let file = parse_ok("module test\nuse std.auth.*");
        assert_eq!(file.imports.len(), 1);
        assert_eq!(file.imports[0].path, vec!["std", "auth"]);
        assert!(matches!(file.imports[0].items, ImportItems::Wildcard));
    }

    #[test]
    fn parse_import_selective() {
        let file = parse_ok("module test\nuse std.http.{Request, Response}");
        assert_eq!(file.imports.len(), 1);
        if let ImportItems::Named(items) = &file.imports[0].items {
            assert_eq!(items.len(), 2);
            assert_eq!(items[0].name, "Request");
            assert_eq!(items[1].name, "Response");
        } else {
            panic!("expected named imports");
        }
    }

    #[test]
    fn parse_type_alias() {
        let file = parse_ok("module test\ntype UserId = UUID");
        assert_eq!(file.declarations.len(), 1);
        if let Declaration::Type(t) = &file.declarations[0] {
            assert_eq!(t.name, "UserId");
            assert!(matches!(t.kind, TypeKind::Alias(_)));
        }
    }

    #[test]
    fn parse_enum_type() {
        let file =
            parse_ok("module test\ntype Status = enum {\n  Draft\n  Pending\n  Confirmed\n}");
        if let Declaration::Type(t) = &file.declarations[0] {
            assert_eq!(t.name, "Status");
            if let TypeKind::Enum(e) = &t.kind {
                assert_eq!(e.variants.len(), 3);
                assert_eq!(e.variants[0].name, "Draft");
                assert_eq!(e.variants[1].name, "Pending");
                assert_eq!(e.variants[2].name, "Confirmed");
            } else {
                panic!("expected enum");
            }
        }
    }

    #[test]
    fn parse_struct_type() {
        let file =
            parse_ok("module test\ntype Customer = struct {\n  id: CustomerId\n  email: Email\n}");
        if let Declaration::Type(t) = &file.declarations[0] {
            assert_eq!(t.name, "Customer");
            if let TypeKind::Struct(s) = &t.kind {
                assert_eq!(s.fields.len(), 2);
                assert_eq!(s.fields[0].name, "id");
                assert_eq!(s.fields[1].name, "email");
            } else {
                panic!("expected struct");
            }
        }
    }

    #[test]
    fn parse_service_with_goal() {
        let file = parse_ok(
            r#"module test
service Checkout {
  goal: "Process orders"
}"#,
        );
        if let Declaration::Service(s) = &file.declarations[0] {
            assert_eq!(s.name, "Checkout");
            assert_eq!(s.goal, Some("Process orders".to_string()));
        }
    }

    #[test]
    fn parse_service_with_operation() {
        let file = parse_ok(
            r#"module test
service Checkout {
  goal: "Process orders"
  operation PlaceOrder {
    inputs:
      customer_id: CustomerId
      items: List
    outputs:
      order_id: OrderId
  }
}"#,
        );
        if let Declaration::Service(s) = &file.declarations[0] {
            assert_eq!(s.operations.len(), 1);
            assert_eq!(s.operations[0].name, "PlaceOrder");
            assert_eq!(s.operations[0].inputs.len(), 2);
            assert_eq!(s.operations[0].outputs.len(), 1);
        }
    }

    #[test]
    fn parse_constraint_list() {
        let file = parse_ok(
            r#"module test
service API {
  goal: "API"
  constraints:
    - idempotent
    - latency(p95: 200ms)
}"#,
        );
        if let Declaration::Service(s) = &file.declarations[0] {
            assert_eq!(s.constraints.len(), 2);
            assert_eq!(s.constraints[0].name, "idempotent");
            assert_eq!(s.constraints[1].name, "latency");
            assert_eq!(s.constraints[1].args.len(), 1);
        }
    }

    #[test]
    fn parse_multiple_declarations() {
        let file = parse_ok(
            r#"module test
type OrderId = UUID
type Status = enum { Active, Inactive }
service Orders {
  goal: "Manage orders"
}"#,
        );
        assert_eq!(file.declarations.len(), 3);
    }

    #[test]
    fn parse_operation_with_tests() {
        let file = parse_ok(
            r#"module test
service Inventory {
  goal: "Manage inventory"
  operation CheckStock {
    inputs:
      product_id: ProductId
    outputs:
      available: Int
    tests:
      - scenario: "Product in stock"
        given: product_exists
        expect: available == 42
  }
}"#,
        );
        if let Declaration::Service(s) = &file.declarations[0] {
            assert_eq!(s.operations[0].tests.len(), 1);
            if let TestKind::Scenario { name, .. } = &s.operations[0].tests[0].kind {
                assert_eq!(name, "Product in stock");
            }
        }
    }

    #[test]
    fn parse_operation_shorthand_syntax_test() {
        let file = parse_ok(
            r#"module test
service GreetService {
  operation GreetShorthand(name: String) -> String
  operation GreetShorthandWithBody(name: String) -> String {
    preconditions:
      - name != ""
  }
  operation GreetNoReturn(name: String)
}"#,
        );
        if let Declaration::Service(s) = &file.declarations[0] {
            assert_eq!(s.operations.len(), 3);

            // First Operation: GreetShorthand(name: String) -> String
            let op1 = &s.operations[0];
            assert_eq!(op1.name, "GreetShorthand");
            assert_eq!(op1.inputs.len(), 1);
            assert_eq!(op1.inputs[0].name, "name");
            assert_eq!(op1.outputs.len(), 1);
            assert_eq!(op1.outputs[0].name, "result");

            // Second Operation: GreetShorthandWithBody(name: String) -> String { preconditions: - name != "" }
            let op2 = &s.operations[1];
            assert_eq!(op2.name, "GreetShorthandWithBody");
            assert_eq!(op2.inputs.len(), 1);
            assert_eq!(op2.inputs[0].name, "name");
            assert_eq!(op2.outputs.len(), 1);
            assert_eq!(op2.outputs[0].name, "result");
            assert_eq!(op2.preconditions.len(), 1);

            // Third Operation: GreetNoReturn(name: String)
            let op3 = &s.operations[2];
            assert_eq!(op3.name, "GreetNoReturn");
            assert_eq!(op3.inputs.len(), 1);
            assert_eq!(op3.inputs[0].name, "name");
            assert!(op3.outputs.is_empty());
        } else {
            panic!("Expected Service declaration");
        }
    }

    #[test]
    fn parse_target_dependencies_decl_test() {
        let file = parse_ok(
            r#"module test
target_dependencies {
  typescript {
    "prisma/client": "^5.14.0"
    dotenv: "^16.4.5"
  }
}"#,
        );
        assert_eq!(file.declarations.len(), 1);
        if let Declaration::TargetDependencies(d) = &file.declarations[0] {
            assert_eq!(d.entries.len(), 1);
            let entry = &d.entries[0];
            assert_eq!(entry.target, "typescript");
            assert_eq!(entry.packages.len(), 2);
            assert_eq!(entry.packages[0].name, "prisma/client");
            assert_eq!(entry.packages[0].version, "^5.14.0");
            assert_eq!(entry.packages[1].name, "dotenv");
            assert_eq!(entry.packages[1].version, "^16.4.5");
        } else {
            panic!("Expected TargetDependencies declaration");
        }
    }

    #[test]
    fn error_recovery_continues_parsing() {
        let (file, errors) = parse(
            r#"module test
* invalid stuff
type OrderId = UUID"#,
        );
        // Should have errors but still parse the type declaration
        assert!(!errors.is_empty());
        assert_eq!(file.declarations.len(), 1);
    }

    #[test]
    fn parse_metrics_list_test() {
        let file = parse_ok(
            r#"module test
service Checkout {
  goal: "Process orders"
  metrics:
    - counter payment_attempts_total {
        description: "Total payment attempts"
        labels: [payment_method, status]
      }
    - histogram checkout_value_usd {
        description: "Distribution of transaction amounts"
        buckets: [10, 50, 100, 500, 1000]
      }
}"#,
        );
        if let Declaration::Service(s) = &file.declarations[0] {
            assert_eq!(s.metrics.len(), 2);

            assert_eq!(s.metrics[0].name, "payment_attempts_total");
            assert_eq!(s.metrics[0].kind, MetricKind::Counter);
            assert_eq!(
                s.metrics[0].description,
                Some("Total payment attempts".to_string())
            );
            assert_eq!(
                s.metrics[0].labels,
                vec!["payment_method".to_string(), "status".to_string()]
            );
            assert!(s.metrics[0].buckets.is_none());

            assert_eq!(s.metrics[1].name, "checkout_value_usd");
            assert_eq!(s.metrics[1].kind, MetricKind::Histogram);
            assert_eq!(
                s.metrics[1].description,
                Some("Distribution of transaction amounts".to_string())
            );
            assert!(s.metrics[1].labels.is_empty());
            assert!(s.metrics[1].buckets.is_some());
            assert_eq!(s.metrics[1].buckets.as_ref().unwrap().len(), 5);
        }
    }

    #[test]
    fn parse_schema_decl_test() {
        let file = parse_ok(
            r#"module test
schema ECommerceDB {
  goal: "E-commerce data model"
  target: postgresql
  entity Product {
    id: ProductId @primary
    name: String @indexed
  }
  relations:
    - Customer has_many Orders
  indexes:
    - Order(customer, placed_at)
}"#,
        );
        assert_eq!(file.declarations.len(), 1);
        if let Declaration::Schema(s) = &file.declarations[0] {
            assert_eq!(s.name, "ECommerceDB");
            assert_eq!(s.goal, Some("E-commerce data model".to_string()));
            assert_eq!(s.target, Some("postgresql".to_string()));
            assert_eq!(s.entities.len(), 1);
            assert_eq!(s.entities[0].name, "Product");
            assert_eq!(s.entities[0].fields.len(), 2);
            assert_eq!(s.entities[0].fields[0].decorators.len(), 1);
            assert_eq!(s.entities[0].fields[0].decorators[0].name, "primary");
            assert_eq!(s.relations.len(), 1);
            assert_eq!(s.relations[0].lhs, "Customer");
            assert_eq!(s.relations[0].rel_type, "has_many");
            assert_eq!(s.relations[0].rhs, "Orders");
            assert_eq!(s.indexes.len(), 1);
            assert_eq!(s.indexes[0].entity, "Order");
            assert_eq!(s.indexes[0].fields.len(), 2);
        } else {
            panic!("expected schema declaration");
        }
    }

    #[test]
    fn parse_union_intersection_option_test() {
        let file = parse_ok(
            r#"module test
type Val = Int | Float & String
type Opt = String?
"#,
        );
        assert_eq!(file.declarations.len(), 2);
        if let Declaration::Type(t) = &file.declarations[0] {
            assert_eq!(t.name, "Val");
            if let TypeKind::Alias(ref ty) = t.kind {
                assert_eq!(ty.name, "Union");
                assert_eq!(ty.union_members.len(), 2);
                assert_eq!(ty.union_members[0].name, "Int");
                assert_eq!(ty.union_members[1].name, "Intersection");
                assert_eq!(ty.union_members[1].intersection_members.len(), 2);
                assert_eq!(ty.union_members[1].intersection_members[0].name, "Float");
                assert_eq!(ty.union_members[1].intersection_members[1].name, "String");
            } else {
                panic!("expected alias");
            }
        }
        if let Declaration::Type(t) = &file.declarations[1] {
            assert_eq!(t.name, "Opt");
            if let TypeKind::Alias(ref ty) = t.kind {
                assert_eq!(ty.name, "Option");
                assert_eq!(ty.type_args.len(), 1);
                assert_eq!(ty.type_args[0].name, "String");
            } else {
                panic!("expected alias");
            }
        }
    }

    #[test]
    fn parse_generics_bounds_test() {
        let file = parse_ok(
            r#"module test
type Res<T: S + C, E> = UUID
"#,
        );
        assert_eq!(file.declarations.len(), 1);
        if let Declaration::Type(t) = &file.declarations[0] {
            assert_eq!(t.name, "Res");
            assert_eq!(t.type_params.len(), 2);
            assert_eq!(t.type_params[0].name, "T");
            assert_eq!(t.type_params[0].bounds.len(), 2);
            assert_eq!(t.type_params[0].bounds[0].name, "S");
            assert_eq!(t.type_params[0].bounds[1].name, "C");
            assert_eq!(t.type_params[1].name, "E");
            assert!(t.type_params[1].bounds.is_empty());
        }
    }

    #[test]
    fn parse_refined_types_test() {
        let file = parse_ok(
            r#"module test
type Ref1 = String { min_length: 5 }
type Ref2 = { format: "regex" }
"#,
        );
        assert_eq!(file.declarations.len(), 2);
        if let Declaration::Type(t1) = &file.declarations[0] {
            assert_eq!(t1.name, "Ref1");
            if let TypeKind::Refined(ref r) = t1.kind {
                assert!(r.base.is_some());
                assert_eq!(r.base.as_ref().unwrap().name, "String");
                assert_eq!(r.constraints.len(), 1);
                assert_eq!(r.constraints[0].name, "min_length");
            } else {
                panic!("expected refined type");
            }
        }
        if let Declaration::Type(t2) = &file.declarations[1] {
            assert_eq!(t2.name, "Ref2");
            if let TypeKind::Refined(ref r) = t2.kind {
                assert!(r.base.is_none());
                assert_eq!(r.constraints.len(), 1);
                assert_eq!(r.constraints[0].name, "format");
            } else {
                panic!("expected refined type");
            }
        }
    }

    #[test]
    fn parse_constraint_decl_test() {
        let file = parse_ok(
            r#"module test
constraint PCI_compliant {
  requires:
    - authenticated
    - audit_logging
  verification:
    - tool: SonarQube evidence: "sonar-report.json"
}"#,
        );
        assert_eq!(file.declarations.len(), 1);
        if let Declaration::Constraint(c) = &file.declarations[0] {
            assert_eq!(c.name, "PCI_compliant");
            assert_eq!(c.requires.len(), 2);
            assert_eq!(c.requires[0].name, "authenticated");
            assert_eq!(c.requires[1].name, "audit_logging");
            assert_eq!(c.verification.len(), 1);
            assert_eq!(c.verification[0].tool, "SonarQube");
            assert_eq!(c.verification[0].evidence, "sonar-report.json");
        } else {
            panic!("expected constraint declaration");
        }
    }

    #[test]
    fn parse_invariants_and_quantifiers_test() {
        let file = parse_ok(
            r#"module test
service Bank {
  goal: "Manage customer balance"
  invariants:
    - balance >= 0
    - total_deposits >= total_withdrawals
  operation Withdraw {
    inputs:
      amount: Int
    outputs:
      new_balance: Int
    tests:
      - property: "Withdraw reduction"
        forall: p in arbitrary<ProductId>, q <- Int(1, 100)
        given: balance >= q
        assert: balance == old(balance) - q
  }
}"#,
        );
        assert_eq!(file.declarations.len(), 1);
        if let Declaration::Service(s) = &file.declarations[0] {
            assert_eq!(s.name, "Bank");
            println!("INVARIANTS: {:#?}", s.invariants);
            assert_eq!(s.invariants.len(), 2);
            assert_eq!(s.operations.len(), 1);
            let op = &s.operations[0];
            assert_eq!(op.tests.len(), 1);
            if let TestKind::Property {
                name, quantifiers, ..
            } = &op.tests[0].kind
            {
                assert_eq!(name, "Withdraw reduction");
                assert_eq!(quantifiers.len(), 2);
                assert_eq!(quantifiers[0].name, "p");
                assert_eq!(quantifiers[1].name, "q");
            } else {
                panic!("expected property test");
            }
        } else {
            panic!("expected service declaration");
        }
    }

    #[test]
    fn parse_doc_comments_test() {
        let file = parse_ok(
            r#"module test

/// This is a doc comment
/// for GreetService
service GreetService {
  /// This is a doc comment
  /// for Greet method
  operation Greet(name: String) -> String
}

/// Doc comment for MyType
type MyType = String

schema MySchema {
  /// Doc comment for MyEntity
  entity MyEntity {
    /// Doc comment for id field
    id: UUID @primary
  }
}"#,
        );
        assert_eq!(file.declarations.len(), 3);

        // 1. Service GreetService and Operation Greet
        if let Declaration::Service(s) = &file.declarations[0] {
            assert_eq!(s.name, "GreetService");
            assert_eq!(
                s.doc_comment,
                Some("This is a doc comment\nfor GreetService".to_string())
            );

            let op = &s.operations[0];
            assert_eq!(op.name, "Greet");
            assert_eq!(
                op.doc_comment,
                Some("This is a doc comment\nfor Greet method".to_string())
            );
        } else {
            panic!("Expected Service declaration");
        }

        // 2. Type MyType
        if let Declaration::Type(t) = &file.declarations[1] {
            assert_eq!(t.name, "MyType");
            assert_eq!(t.doc_comment, Some("Doc comment for MyType".to_string()));
        } else {
            panic!("Expected Type declaration");
        }

        // 3. Schema MySchema, Entity MyEntity, Field id
        if let Declaration::Schema(s) = &file.declarations[2] {
            assert_eq!(s.name, "MySchema");
            assert_eq!(s.entities.len(), 1);

            let entity = &s.entities[0];
            assert_eq!(entity.name, "MyEntity");
            assert_eq!(
                entity.doc_comment,
                Some("Doc comment for MyEntity".to_string())
            );

            let field = &entity.fields[0];
            assert_eq!(field.name, "id");
            assert_eq!(
                field.doc_comment,
                Some("Doc comment for id field".to_string())
            );
        } else {
            panic!("Expected Schema declaration");
        }
    }

    #[test]
    fn parse_service_dependencies_and_policies_test() {
        let file = parse_ok(
            r#"module test
service Checkout {
  dependencies:
    - InternalStorage (DB)
    - LoyaltyService (gRPC API)
  policies:
    - enrich_user_loyalty:
        action: "enrich"
        rule: "vip"
  invariants:
    - money_safety: "safe"
}"#,
        );
        assert_eq!(file.declarations.len(), 1);
        if let Declaration::Service(s) = &file.declarations[0] {
            assert_eq!(s.name, "Checkout");
            assert_eq!(s.dependencies.len(), 2);
            assert_eq!(s.dependencies[0].name, "InternalStorage");
            assert_eq!(s.dependencies[0].notes, Some("DB".to_string()));
            assert_eq!(s.dependencies[1].name, "LoyaltyService");
            assert_eq!(s.dependencies[1].notes, Some("gRPC API".to_string()));

            assert_eq!(s.policies.len(), 1);
            assert_eq!(s.policies[0].name, "enrich_user_loyalty");
            assert_eq!(s.policies[0].entries.len(), 2);
            assert_eq!(s.policies[0].entries[0].key, "action");
            assert_eq!(s.policies[0].entries[1].key, "rule");

            assert_eq!(s.invariants.len(), 1);
            if let Expression::BinaryOp {
                left, op, right, ..
            } = &s.invariants[0]
            {
                assert_eq!(op, &BinaryOperator::Eq);
                if let Expression::Identifier(name, _) = &**left {
                    assert_eq!(name, "money_safety");
                } else {
                    panic!("expected identifier");
                }
                if let Expression::Literal(Literal::String(val)) = &**right {
                    assert_eq!(val, "safe");
                } else {
                    panic!("expected string literal");
                }
            } else {
                panic!("expected binary op for labeled invariant");
            }
        } else {
            panic!("expected service declaration");
        }
    }

    #[test]
    fn parse_orchestrator_workflow_test() {
        let file = parse_ok(
            r#"module test
orchestrator BookingFlow {
  states: [ Created, Paid, Cancelled ]
  transitions:
    - Created -> Paid -> Cancelled [on: Timeout or UserAbort]
}"#,
        );
        assert_eq!(file.declarations.len(), 1);
        if let Declaration::Workflow(w) = &file.declarations[0] {
            assert_eq!(w.name, "BookingFlow");
            assert_eq!(w.states, vec!["Created", "Paid", "Cancelled"]);
            assert_eq!(w.transitions.len(), 2);

            assert_eq!(w.transitions[0].from, "Created");
            assert_eq!(w.transitions[0].to, "Paid");
            assert_eq!(
                w.transitions[0].trigger,
                Some("Timeout or UserAbort".to_string())
            );

            assert_eq!(w.transitions[1].from, "Paid");
            assert_eq!(w.transitions[1].to, "Cancelled");
            assert_eq!(
                w.transitions[1].trigger,
                Some("Timeout or UserAbort".to_string())
            );
        } else {
            panic!("expected workflow declaration");
        }
    }
}
