//! Lexer for OmniLang source files.
//!
//! Transforms raw source text into a stream of tokens.
//! The lexer is a single-pass scanner that produces tokens with
//! byte-offset spans for error reporting.

use crate::error::ParseError;
use crate::span::Span;
use crate::token::{Token, TokenKind};

/// Lexer state: walks through source text character by character.
pub struct Lexer<'src> {
    source: &'src str,
    bytes: &'src [u8],
    pos: usize,
    errors: Vec<ParseError>,
    indent_stack: Vec<usize>,
    paren_depth: usize,
    at_line_start: bool,
}

impl<'src> Lexer<'src> {
    pub fn new(source: &'src str) -> Self {
        Self {
            source,
            bytes: source.as_bytes(),
            pos: 0,
            errors: Vec::new(),
            indent_stack: vec![0],
            paren_depth: 0,
            at_line_start: true,
        }
    }

    /// Tokenize the entire source, returning tokens and any errors.
    pub fn tokenize(mut self) -> (Vec<Token>, Vec<ParseError>) {
        let mut tokens = Vec::new();

        loop {
            // 1. If we are at the start of a line, handle indentation.
            if self.at_line_start {
                let start_pos = self.pos;
                let mut spaces = 0;
                while !self.is_at_end() && (self.peek() == ' ' || self.peek() == '\t') {
                    if self.peek() == '\t' {
                        spaces += 4;
                    } else {
                        spaces += 1;
                    }
                    self.advance();
                }

                // If this is an empty line or line-comment-only line, do not emit Indent/Dedent/Newline.
                // Note: block comments (/* */) are NOT treated as blank lines because
                // they may end on the same line with real code following.
                // Doc comments (///) ARE treated as real lines because they carry semantic
                // meaning and must trigger proper Indent/Dedent tracking.
                let next_ch = self.peek();
                let is_line_comment_only = next_ch == '/'
                    && self.peek_next() == '/'
                    && self.peek_at_offset(2) != Some('/');
                if self.is_at_end() || next_ch == '\n' || next_ch == '\r' || is_line_comment_only {
                    self.skip_whitespace_non_newline();
                } else {
                    // Only emit Indent/Dedent tokens when NOT inside
                    // brackets/braces/parens. Inside delimited blocks the
                    // old brace-based syntax handles nesting, so layout
                    // tokens would corrupt the parse.
                    if self.paren_depth == 0 {
                        let current_indent = *self.indent_stack.last().unwrap();
                        if spaces > current_indent {
                            self.indent_stack.push(spaces);
                            tokens.push(Token {
                                kind: TokenKind::Indent,
                                span: Span::new(start_pos, self.pos),
                                text: "indent".to_string(),
                            });
                        } else if spaces < current_indent {
                            while spaces < *self.indent_stack.last().unwrap() {
                                self.indent_stack.pop();
                                tokens.push(Token {
                                    kind: TokenKind::Dedent,
                                    span: Span::new(start_pos, self.pos),
                                    text: "dedent".to_string(),
                                });
                            }
                            if spaces != *self.indent_stack.last().unwrap() {
                                self.errors.push(ParseError::Expected {
                                    expected: format!(
                                        "indentation matching a previous level ({})",
                                        spaces
                                    ),
                                    found: format!("mismatched level ({})", spaces),
                                    span: Span::new(start_pos, self.pos),
                                });
                            }
                        }
                    }
                    self.at_line_start = false;
                }
            }

            self.skip_whitespace_non_newline();

            if self.is_at_end() {
                // Close any open indentation levels
                while self.indent_stack.len() > 1 {
                    self.indent_stack.pop();
                    tokens.push(Token {
                        kind: TokenKind::Dedent,
                        span: Span::new(self.pos, self.pos),
                        text: "dedent".to_string(),
                    });
                }
                tokens.push(Token {
                    kind: TokenKind::Eof,
                    span: Span::new(self.pos, self.pos),
                    text: String::new(),
                });
                break;
            }

            let ch = self.peek();
            if ch == '\n' {
                let start_pos = self.pos;
                self.advance();
                if self.paren_depth == 0 {
                    tokens.push(Token {
                        kind: TokenKind::Newline,
                        span: Span::new(start_pos, self.pos),
                        text: "\n".to_string(),
                    });
                }
                self.at_line_start = true;
                continue;
            } else if ch == '\r' && self.peek_next() == '\n' {
                let start_pos = self.pos;
                self.advance(); // \r
                self.advance(); // \n
                if self.paren_depth == 0 {
                    tokens.push(Token {
                        kind: TokenKind::Newline,
                        span: Span::new(start_pos, self.pos),
                        text: "\r\n".to_string(),
                    });
                }
                self.at_line_start = true;
                continue;
            }

            let token = self.next_token();

            match token.kind {
                TokenKind::ParenOpen | TokenKind::BracketOpen | TokenKind::BraceOpen => {
                    self.paren_depth += 1;
                }
                TokenKind::ParenClose | TokenKind::BracketClose | TokenKind::BraceClose
                    if self.paren_depth > 0 =>
                {
                    self.paren_depth -= 1;
                }
                _ => {}
            }

            tokens.push(token);
        }

        (tokens, self.errors)
    }

    fn skip_whitespace_non_newline(&mut self) {
        while !self.is_at_end() {
            let ch = self.peek();
            if ch == ' ' || ch == '\t' || ch == '\r' {
                self.advance();
            } else {
                break;
            }
        }
    }

    fn next_token(&mut self) -> Token {
        let start = self.pos;
        let ch = self.advance();

        match ch {
            // ── Delimiters ────────────────────────────
            '{' => self.make_token(TokenKind::BraceOpen, start),
            '}' => self.make_token(TokenKind::BraceClose, start),
            '(' => self.make_token(TokenKind::ParenOpen, start),
            ')' => self.make_token(TokenKind::ParenClose, start),
            '[' => self.make_token(TokenKind::BracketOpen, start),
            ']' => self.make_token(TokenKind::BracketClose, start),
            ':' => self.make_token(TokenKind::Colon, start),
            ',' => self.make_token(TokenKind::Comma, start),
            '?' => self.make_token(TokenKind::Question, start),
            '@' => self.make_token(TokenKind::At, start),
            '+' => self.make_token(TokenKind::Plus, start),

            // ── Operators that need lookahead ─────────
            '=' => {
                if self.match_char('=') {
                    self.make_token(TokenKind::EqEq, start)
                } else {
                    self.make_token(TokenKind::Eq, start)
                }
            }
            '!' => {
                if self.match_char('=') {
                    self.make_token(TokenKind::BangEq, start)
                } else {
                    self.make_token(TokenKind::Bang, start)
                }
            }
            '<' => {
                if self.match_char('=') {
                    self.make_token(TokenKind::LtEq, start)
                } else {
                    self.make_token(TokenKind::Lt, start)
                }
            }
            '>' => {
                if self.match_char('=') {
                    self.make_token(TokenKind::GtEq, start)
                } else {
                    self.make_token(TokenKind::Gt, start)
                }
            }
            '&' => {
                if self.match_char('&') {
                    self.make_token(TokenKind::AmpAmp, start)
                } else {
                    self.make_token(TokenKind::Amp, start)
                }
            }
            '|' => {
                if self.match_char('|') {
                    self.make_token(TokenKind::PipePipe, start)
                } else {
                    self.make_token(TokenKind::Pipe, start)
                }
            }
            '-' => {
                if self.match_char('>') {
                    self.make_token(TokenKind::Arrow, start)
                } else {
                    self.make_token(TokenKind::Minus, start)
                }
            }
            '.' => {
                if self.match_char('.') {
                    if self.match_char('<') {
                        self.make_token(TokenKind::DotDotLt, start)
                    } else {
                        self.make_token(TokenKind::DotDot, start)
                    }
                } else {
                    self.make_token(TokenKind::Dot, start)
                }
            }
            '*' => self.make_token(TokenKind::Star, start),

            // ── Comments & Slash ──────────────────────
            '/' => {
                if self.match_char('/') {
                    // Check for doc comment `///`
                    if self.match_char('/') {
                        self.scan_line_comment(start, true)
                    } else {
                        self.scan_line_comment(start, false)
                    }
                } else if self.match_char('*') {
                    self.scan_block_comment(start)
                } else {
                    self.make_token(TokenKind::Slash, start)
                }
            }

            // ── String literals ───────────────────────
            '"' => self.scan_string(start),

            // ── Money literals ────────────────────────
            '$' => self.scan_money(start),

            // ── Number literals ───────────────────────
            c if c.is_ascii_digit() => self.scan_number(start),

            // ── Identifiers & Keywords ────────────────
            c if is_ident_start(c) => self.scan_identifier(start),

            // ── Unknown character ─────────────────────
            c => {
                self.errors.push(ParseError::UnexpectedChar {
                    ch: c,
                    span: Span::new(start, self.pos),
                });
                self.make_token(TokenKind::Error, start)
            }
        }
    }

    // ── Scanning helpers ──────────────────────────────────────

    fn scan_identifier(&mut self, start: usize) -> Token {
        while !self.is_at_end() && is_ident_continue(self.peek()) {
            self.advance();
        }

        let text = &self.source[start..self.pos];

        // Check if it's a keyword
        let kind = TokenKind::from_keyword(text).unwrap_or(TokenKind::Ident);

        Token {
            kind,
            span: Span::new(start, self.pos),
            text: text.to_string(),
        }
    }

    fn scan_number(&mut self, start: usize) -> Token {
        // Consume digits (with optional underscores for readability: 1_000)
        self.consume_digits();

        // Check for float
        if self.peek() == '.' && self.peek_next() != '.' && self.peek_next().is_ascii_digit() {
            self.advance(); // consume '.'
            self.consume_digits();

            let text = &self.source[start..self.pos];
            // Check for percentage
            if self.peek() == '%' {
                self.advance();
                return Token {
                    kind: TokenKind::PercentageLiteral,
                    span: Span::new(start, self.pos),
                    text: self.source[start..self.pos].to_string(),
                };
            }

            return Token {
                kind: TokenKind::FloatLiteral,
                span: Span::new(start, self.pos),
                text: text.to_string(),
            };
        }

        let text = &self.source[start..self.pos];

        // Check for percentage: `85%`
        if self.peek() == '%' {
            self.advance();
            return Token {
                kind: TokenKind::PercentageLiteral,
                span: Span::new(start, self.pos),
                text: self.source[start..self.pos].to_string(),
            };
        }

        // Check for duration suffix: `5min`, `200ms`, `30s`, `2h`, `14days`
        if self.peek().is_ascii_alphabetic() {
            let suffix_start = self.pos;
            while !self.is_at_end() && self.peek().is_ascii_alphabetic() {
                self.advance();
            }
            let suffix = &self.source[suffix_start..self.pos];
            if matches!(
                suffix,
                "ms" | "s" | "sec" | "min" | "h" | "hr" | "d" | "day" | "days"
            ) {
                return Token {
                    kind: TokenKind::DurationLiteral,
                    span: Span::new(start, self.pos),
                    text: self.source[start..self.pos].to_string(),
                };
            }
            // Not a known suffix — rewind and return as int + separate ident
            self.pos = suffix_start;
        }

        Token {
            kind: TokenKind::IntLiteral,
            span: Span::new(start, self.pos),
            text: text.to_string(),
        }
    }

    fn scan_money(&mut self, start: usize) -> Token {
        // '$' already consumed
        self.consume_digits();
        if self.peek() == '.' {
            self.advance();
            self.consume_digits();
        }
        Token {
            kind: TokenKind::MoneyLiteral,
            span: Span::new(start, self.pos),
            text: self.source[start..self.pos].to_string(),
        }
    }

    fn scan_string(&mut self, start: usize) -> Token {
        // Opening '"' already consumed
        while !self.is_at_end() && self.peek() != '"' {
            if self.peek() == '\\' {
                self.advance(); // skip backslash
                if !self.is_at_end() {
                    self.advance(); // skip escaped char
                }
            } else if self.peek() == '\n' {
                // Unterminated string at newline
                break;
            } else {
                self.advance();
            }
        }

        if self.is_at_end() || self.peek() == '\n' {
            self.errors.push(ParseError::UnterminatedString {
                span: Span::new(start, self.pos),
            });
            return Token {
                kind: TokenKind::Error,
                span: Span::new(start, self.pos),
                text: self.source[start..self.pos].to_string(),
            };
        }

        self.advance(); // consume closing '"'

        Token {
            kind: TokenKind::StringLiteral,
            span: Span::new(start, self.pos),
            text: self.source[start..self.pos].to_string(),
        }
    }

    fn scan_line_comment(&mut self, start: usize, is_doc: bool) -> Token {
        while !self.is_at_end() && self.peek() != '\n' {
            self.advance();
        }
        Token {
            kind: if is_doc {
                TokenKind::DocComment
            } else {
                TokenKind::LineComment
            },
            span: Span::new(start, self.pos),
            text: self.source[start..self.pos].to_string(),
        }
    }

    fn scan_block_comment(&mut self, start: usize) -> Token {
        let mut depth = 1;
        while !self.is_at_end() && depth > 0 {
            if self.peek() == '/' && self.peek_next() == '*' {
                self.advance();
                self.advance();
                depth += 1;
            } else if self.peek() == '*' && self.peek_next() == '/' {
                self.advance();
                self.advance();
                depth -= 1;
            } else {
                self.advance();
            }
        }

        if depth > 0 {
            self.errors.push(ParseError::UnterminatedBlockComment {
                span: Span::new(start, self.pos),
            });
            return Token {
                kind: TokenKind::Error,
                span: Span::new(start, self.pos),
                text: self.source[start..self.pos].to_string(),
            };
        }

        Token {
            kind: TokenKind::BlockComment,
            span: Span::new(start, self.pos),
            text: self.source[start..self.pos].to_string(),
        }
    }

    // ── Low-level character operations ────────────────────────

    fn consume_digits(&mut self) {
        while !self.is_at_end() && (self.peek().is_ascii_digit() || self.peek() == '_') {
            self.advance();
        }
    }

    fn is_at_end(&self) -> bool {
        self.pos >= self.source.len()
    }

    fn peek(&self) -> char {
        if self.is_at_end() {
            '\0'
        } else {
            self.bytes[self.pos] as char
        }
    }

    fn peek_next(&self) -> char {
        if self.pos + 1 >= self.source.len() {
            '\0'
        } else {
            self.bytes[self.pos + 1] as char
        }
    }

    fn peek_at_offset(&self, offset: usize) -> Option<char> {
        let idx = self.pos + offset;
        if idx >= self.source.len() {
            None
        } else {
            Some(self.bytes[idx] as char)
        }
    }

    fn advance(&mut self) -> char {
        let ch = self.bytes[self.pos] as char;
        self.pos += 1;
        ch
    }

    fn match_char(&mut self, expected: char) -> bool {
        if self.is_at_end() || self.peek() != expected {
            false
        } else {
            self.pos += 1;
            true
        }
    }

    fn make_token(&self, kind: TokenKind, start: usize) -> Token {
        Token {
            kind,
            span: Span::new(start, self.pos),
            text: self.source[start..self.pos].to_string(),
        }
    }
}

fn is_ident_start(c: char) -> bool {
    c.is_ascii_alphabetic() || c == '_'
}

fn is_ident_continue(c: char) -> bool {
    c.is_ascii_alphanumeric() || c == '_'
}

#[cfg(test)]
mod tests {
    use super::*;

    fn lex(input: &str) -> Vec<Token> {
        let (tokens, errors) = Lexer::new(input).tokenize();
        assert!(errors.is_empty(), "unexpected errors: {errors:?}");
        tokens
    }

    fn lex_kinds(input: &str) -> Vec<TokenKind> {
        lex(input).into_iter().map(|t| t.kind).collect()
    }

    // ── Basic tokens ──────────────────────────────────

    #[test]
    fn empty_input() {
        let kinds = lex_kinds("");
        assert_eq!(kinds, vec![TokenKind::Eof]);
    }

    #[test]
    fn whitespace_only() {
        let kinds = lex_kinds("   \n\t  \n  ");
        assert_eq!(
            kinds,
            vec![TokenKind::Newline, TokenKind::Newline, TokenKind::Eof]
        );
    }

    // ── Layout (indentation) ──────────────────────────

    #[test]
    fn nested_indentation_emits_indent_dedent() {
        // One nested block opens one Indent and closes with one Dedent.
        let kinds = lex_kinds("service s\n  goal\n");
        assert_eq!(
            kinds,
            vec![
                TokenKind::KwService,
                TokenKind::Ident,
                TokenKind::Newline,
                TokenKind::Indent,
                TokenKind::KwGoal,
                TokenKind::Newline,
                TokenKind::Dedent,
                TokenKind::Eof,
            ]
        );
    }

    #[test]
    fn deeper_indentation_stacks_and_unwinds() {
        // Two levels of nesting must produce two Indents and two matching Dedents.
        let indents = lex_kinds("a\n  b\n    c\nd\n")
            .into_iter()
            .filter(|k| matches!(k, TokenKind::Indent | TokenKind::Dedent))
            .collect::<Vec<_>>();
        assert_eq!(
            indents,
            vec![
                TokenKind::Indent,
                TokenKind::Indent,
                TokenKind::Dedent,
                TokenKind::Dedent,
            ]
        );
    }

    #[test]
    fn indentation_suppressed_inside_parentheses() {
        // Newlines/indentation inside a bracketed group must not emit layout tokens.
        let kinds = lex_kinds("states: [\n  Created,\n  Paid,\n]\n");
        assert!(!kinds.contains(&TokenKind::Indent));
        assert!(!kinds.contains(&TokenKind::Dedent));
    }

    // ── Domain literals ───────────────────────────────

    #[test]
    fn domain_literals_are_distinct_kinds() {
        assert_eq!(lex_kinds("5min")[0], TokenKind::DurationLiteral);
        assert_eq!(lex_kinds("200ms")[0], TokenKind::DurationLiteral);
        assert_eq!(lex_kinds("$0.10")[0], TokenKind::MoneyLiteral);
        assert_eq!(lex_kinds("85%")[0], TokenKind::PercentageLiteral);
        assert_eq!(lex_kinds("42")[0], TokenKind::IntLiteral);
        assert_eq!(lex_kinds("3.14")[0], TokenKind::FloatLiteral);
    }

    // ── Keywords ──────────────────────────────────────

    #[test]
    fn keywords() {
        let kinds = lex_kinds("module use type struct enum service operation");
        assert_eq!(
            kinds,
            vec![
                TokenKind::KwModule,
                TokenKind::KwUse,
                TokenKind::KwType,
                TokenKind::KwStruct,
                TokenKind::KwEnum,
                TokenKind::KwService,
                TokenKind::KwOperation,
                TokenKind::Eof,
            ]
        );
    }

    #[test]
    fn all_block_keywords() {
        let kinds = lex_kinds(
            "component pipeline workflow agent schema policy contract constraint budget evidence",
        );
        assert_eq!(
            kinds,
            vec![
                TokenKind::Ident,
                TokenKind::Ident,
                TokenKind::KwWorkflow,
                TokenKind::KwAgent,
                TokenKind::KwSchema,
                TokenKind::KwPolicy,
                TokenKind::Ident,
                TokenKind::KwConstraint,
                TokenKind::KwBudget,
                TokenKind::KwEvidence,
                TokenKind::Eof,
            ]
        );
    }

    #[test]
    fn new_keywords() {
        let kinds = lex_kinds("entity relations indexes cannot must");
        assert_eq!(
            kinds,
            vec![
                TokenKind::KwEntity,
                TokenKind::KwRelations,
                TokenKind::KwIndexes,
                TokenKind::KwCannot,
                TokenKind::KwMust,
                TokenKind::Eof,
            ]
        );
    }

    #[test]
    fn test_keywords() {
        let kinds = lex_kinds("tests scenario given when expect assert forall");
        assert_eq!(
            kinds,
            vec![
                TokenKind::KwTests,
                TokenKind::KwScenario,
                TokenKind::KwGiven,
                TokenKind::KwWhen,
                TokenKind::KwExpect,
                TokenKind::KwAssert,
                TokenKind::KwForall,
                TokenKind::Eof,
            ]
        );
    }

    #[test]
    fn logic_keywords() {
        let kinds = lex_kinds("if else in not and or true false null none some ok err old self");
        assert_eq!(
            kinds,
            vec![
                TokenKind::KwIf,
                TokenKind::KwElse,
                TokenKind::KwIn,
                TokenKind::KwNot,
                TokenKind::KwAnd,
                TokenKind::KwOr,
                TokenKind::KwTrue,
                TokenKind::KwFalse,
                TokenKind::KwNull,
                TokenKind::KwNone,
                TokenKind::KwSome,
                TokenKind::KwOk,
                TokenKind::KwErr,
                TokenKind::KwOld,
                TokenKind::KwSelfKw,
                TokenKind::Eof,
            ]
        );
    }

    // ── Identifiers ───────────────────────────────────

    #[test]
    fn identifiers() {
        let tokens = lex("OrderStatus customer_id MAX_RETRIES _private");
        let texts: Vec<&str> = tokens.iter().map(|t| t.text.as_str()).collect();
        assert_eq!(
            texts,
            vec!["OrderStatus", "customer_id", "MAX_RETRIES", "_private", ""]
        );
        assert_eq!(tokens[0].kind, TokenKind::Ident);
        assert_eq!(tokens[1].kind, TokenKind::Ident);
    }

    // ── Operators ─────────────────────────────────────

    #[test]
    fn comparison_operators() {
        let kinds = lex_kinds("== != < > <= >=");
        assert_eq!(
            kinds,
            vec![
                TokenKind::EqEq,
                TokenKind::BangEq,
                TokenKind::Lt,
                TokenKind::Gt,
                TokenKind::LtEq,
                TokenKind::GtEq,
                TokenKind::Eof,
            ]
        );
    }

    #[test]
    fn logical_operators() {
        let kinds = lex_kinds("&& || !");
        assert_eq!(
            kinds,
            vec![
                TokenKind::AmpAmp,
                TokenKind::PipePipe,
                TokenKind::Bang,
                TokenKind::Eof,
            ]
        );
    }

    #[test]
    fn range_and_arrow() {
        let kinds = lex_kinds(".. ..< -> | & ?");
        assert_eq!(
            kinds,
            vec![
                TokenKind::DotDot,
                TokenKind::DotDotLt,
                TokenKind::Arrow,
                TokenKind::Pipe,
                TokenKind::Amp,
                TokenKind::Question,
                TokenKind::Eof,
            ]
        );
    }

    // ── Delimiters ────────────────────────────────────

    #[test]
    fn delimiters() {
        let kinds = lex_kinds("{ } ( ) [ ] : , . = @ *");
        assert_eq!(
            kinds,
            vec![
                TokenKind::BraceOpen,
                TokenKind::BraceClose,
                TokenKind::ParenOpen,
                TokenKind::ParenClose,
                TokenKind::BracketOpen,
                TokenKind::BracketClose,
                TokenKind::Colon,
                TokenKind::Comma,
                TokenKind::Dot,
                TokenKind::Eq,
                TokenKind::At,
                TokenKind::Star,
                TokenKind::Eof,
            ]
        );
    }

    // ── Literals ──────────────────────────────────────

    #[test]
    fn integer_literals() {
        let tokens = lex("42 0 1_000_000");
        assert_eq!(tokens[0].kind, TokenKind::IntLiteral);
        assert_eq!(tokens[0].text, "42");
        assert_eq!(tokens[1].text, "0");
        assert_eq!(tokens[2].text, "1_000_000");
    }

    #[test]
    fn float_literals() {
        let tokens = lex("3.14 0.99 999_999.99");
        assert_eq!(tokens[0].kind, TokenKind::FloatLiteral);
        assert_eq!(tokens[0].text, "3.14");
        assert_eq!(tokens[2].text, "999_999.99");
    }

    #[test]
    fn string_literals() {
        let tokens = lex(r#""hello" "world with spaces" "escaped \"quotes\"""#);
        assert_eq!(tokens[0].kind, TokenKind::StringLiteral);
        assert_eq!(tokens[0].text, r#""hello""#);
        assert_eq!(tokens[1].text, r#""world with spaces""#);
        assert_eq!(tokens[2].text, r#""escaped \"quotes\"""#);
    }

    #[test]
    fn duration_literals() {
        let tokens = lex("5min 200ms 30s 2h 14days");
        assert_eq!(tokens[0].kind, TokenKind::DurationLiteral);
        assert_eq!(tokens[0].text, "5min");
        assert_eq!(tokens[1].text, "200ms");
        assert_eq!(tokens[2].text, "30s");
        assert_eq!(tokens[3].text, "2h");
        assert_eq!(tokens[4].text, "14days");
    }

    #[test]
    fn money_literals() {
        let tokens = lex("$0.10 $1.00 $999");
        assert_eq!(tokens[0].kind, TokenKind::MoneyLiteral);
        assert_eq!(tokens[0].text, "$0.10");
        assert_eq!(tokens[1].text, "$1.00");
        assert_eq!(tokens[2].text, "$999");
    }

    #[test]
    fn percentage_literals() {
        let tokens = lex("85% 99.9% 0%");
        assert_eq!(tokens[0].kind, TokenKind::PercentageLiteral);
        assert_eq!(tokens[0].text, "85%");
        assert_eq!(tokens[1].text, "99.9%");
        assert_eq!(tokens[2].text, "0%");
    }

    // ── Comments ──────────────────────────────────────

    #[test]
    fn line_comment() {
        let tokens = lex("// this is a comment\nmodule");
        assert_eq!(tokens[0].kind, TokenKind::LineComment);
        assert_eq!(tokens[0].text, "// this is a comment");
        assert_eq!(tokens[1].kind, TokenKind::Newline);
        assert_eq!(tokens[2].kind, TokenKind::KwModule);
    }

    #[test]
    fn doc_comment() {
        let tokens = lex("/// Documentation\nservice");
        assert_eq!(tokens[0].kind, TokenKind::DocComment);
        assert_eq!(tokens[0].text, "/// Documentation");
        assert_eq!(tokens[1].kind, TokenKind::Newline);
        assert_eq!(tokens[2].kind, TokenKind::KwService);
    }

    #[test]
    fn block_comment() {
        let tokens = lex("/* block */ module");
        assert_eq!(tokens[0].kind, TokenKind::BlockComment);
        assert_eq!(tokens[1].kind, TokenKind::KwModule);
    }

    #[test]
    fn nested_block_comment() {
        let tokens = lex("/* outer /* inner */ outer */ module");
        assert_eq!(tokens[0].kind, TokenKind::BlockComment);
        assert_eq!(tokens[1].kind, TokenKind::KwModule);
    }

    // ── Error recovery ────────────────────────────────

    #[test]
    fn unterminated_string() {
        let (tokens, errors) = Lexer::new("\"unclosed string\nmodule").tokenize();
        assert!(!errors.is_empty());
        assert!(matches!(errors[0], ParseError::UnterminatedString { .. }));
        // Should still produce tokens after the error
        assert!(tokens.iter().any(|t| t.kind == TokenKind::KwModule));
    }

    #[test]
    fn unterminated_block_comment() {
        let (_, errors) = Lexer::new("/* never closed").tokenize();
        assert!(!errors.is_empty());
        assert!(matches!(
            errors[0],
            ParseError::UnterminatedBlockComment { .. }
        ));
    }

    #[test]
    fn unexpected_character() {
        let (tokens, errors) = Lexer::new("module ~ service").tokenize();
        assert!(!errors.is_empty());
        // Should recover and keep scanning
        assert_eq!(tokens[0].kind, TokenKind::KwModule);
        assert_eq!(tokens[1].kind, TokenKind::Error);
        assert_eq!(tokens[2].kind, TokenKind::KwService);
    }

    // ── Integration: real spec snippets ───────────────

    #[test]
    fn module_declaration() {
        let kinds = lex_kinds("module acme.payments.checkout");
        assert_eq!(
            kinds,
            vec![
                TokenKind::KwModule,
                TokenKind::Ident,
                TokenKind::Dot,
                TokenKind::Ident,
                TokenKind::Dot,
                TokenKind::Ident,
                TokenKind::Eof,
            ]
        );
    }

    #[test]
    fn import_statement() {
        let kinds = lex_kinds("use std.http.{Request, Response}");
        assert_eq!(
            kinds,
            vec![
                TokenKind::KwUse,
                TokenKind::Ident,
                TokenKind::Dot,
                TokenKind::Ident,
                TokenKind::Dot,
                TokenKind::BraceOpen,
                TokenKind::Ident,
                TokenKind::Comma,
                TokenKind::Ident,
                TokenKind::BraceClose,
                TokenKind::Eof,
            ]
        );
    }

    #[test]
    fn service_header() {
        let tokens = lex(r#"service Checkout {
  goal: "Process orders"
  constraints:
    - latency(p95: <200ms)
    - idempotent
}"#);
        // Verify key tokens are present
        assert_eq!(tokens[0].kind, TokenKind::KwService);
        assert_eq!(tokens[1].text, "Checkout");
        assert!(tokens.iter().any(|t| t.kind == TokenKind::KwGoal));
        assert!(
            tokens
                .iter()
                .any(|t| t.kind == TokenKind::DurationLiteral && t.text == "200ms")
        );
    }

    #[test]
    fn operation_block() {
        let kinds = lex_kinds("operation PlaceOrder { inputs: outputs: }");
        assert_eq!(
            kinds,
            vec![
                TokenKind::KwOperation,
                TokenKind::Ident,
                TokenKind::BraceOpen,
                TokenKind::KwInputs,
                TokenKind::Colon,
                TokenKind::KwOutputs,
                TokenKind::Colon,
                TokenKind::BraceClose,
                TokenKind::Eof,
            ]
        );
    }

    #[test]
    fn type_declaration() {
        let kinds = lex_kinds("type OrderId = String");
        assert_eq!(
            kinds,
            vec![
                TokenKind::KwType,
                TokenKind::Ident,
                TokenKind::Eq,
                TokenKind::Ident,
                TokenKind::Eof,
            ]
        );
    }

    #[test]
    fn refined_type_with_regex() {
        let tokens = lex(r#"type OrderId = String { format: regex("^ORD-[A-Z0-9]{12}$") }"#);
        assert_eq!(tokens[0].kind, TokenKind::KwType);
        assert_eq!(tokens[1].text, "OrderId");
        assert!(
            tokens
                .iter()
                .any(|t| t.kind == TokenKind::StringLiteral && t.text.contains("ORD"))
        );
    }

    #[test]
    fn postcondition_with_old() {
        let kinds = lex_kinds("balance == old(balance) - amount");
        assert_eq!(
            kinds,
            vec![
                TokenKind::Ident,
                TokenKind::EqEq,
                TokenKind::KwOld,
                TokenKind::ParenOpen,
                TokenKind::Ident,
                TokenKind::ParenClose,
                TokenKind::Minus,
                TokenKind::Ident,
                TokenKind::Eof,
            ]
        );
    }

    #[test]
    fn generic_type() {
        let kinds = lex_kinds("List<Order>");
        assert_eq!(
            kinds,
            vec![
                TokenKind::Ident,
                TokenKind::Lt,
                TokenKind::Ident,
                TokenKind::Gt,
                TokenKind::Eof,
            ]
        );
    }

    #[test]
    fn optional_shorthand() {
        let kinds = lex_kinds("String?");
        assert_eq!(
            kinds,
            vec![TokenKind::Ident, TokenKind::Question, TokenKind::Eof,]
        );
    }

    // ── Span accuracy ─────────────────────────────────

    #[test]
    fn spans_are_correct() {
        let tokens = lex("module test");
        assert_eq!(tokens[0].span, Span::new(0, 6));
        assert_eq!(tokens[1].span, Span::new(7, 11));
    }

    #[test]
    fn span_for_string() {
        let tokens = lex(r#""hello""#);
        assert_eq!(tokens[0].span, Span::new(0, 7));
    }

    #[test]
    fn test_lexer_indentation() {
        let input = "entity account\n  id text\n  balance money\n\n  status text\n\naction charge\n  inputs\n    amount money";
        let (tokens, errors) = Lexer::new(input).tokenize();
        assert!(errors.is_empty(), "unexpected errors: {errors:?}");

        let kinds: Vec<TokenKind> = tokens.into_iter().map(|t| t.kind).collect();
        assert!(kinds.contains(&TokenKind::Indent));
        assert!(kinds.contains(&TokenKind::Dedent));
        assert!(kinds.contains(&TokenKind::Newline));
    }
}
