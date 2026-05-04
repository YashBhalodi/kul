//! Hand-written recursive-descent parser for Kula.
//!
//! Consumes the token stream from [`crate::lexer`] and produces a
//! [`Document`] together with parse diagnostics. On an unexpected token the
//! parser emits a diagnostic, advances to the next newline (panic-mode
//! recovery), and resumes parsing the next statement.

use crate::ast::{
    Document, Gender, GenderValue, Ident, PersonField, PersonFieldKind, PersonStmt, Statement,
    StringValue, VersionDecl,
};
use crate::diagnostic::Diagnostic;
use crate::lexer::{EnumKw, FieldName, Token, TokenKind};

pub fn parse(tokens: &[Token]) -> (Document, Vec<Diagnostic>) {
    Parser::new(tokens).run()
}

struct Parser<'a> {
    tokens: &'a [Token],
    pos: usize,
    diagnostics: Vec<Diagnostic>,
}

impl<'a> Parser<'a> {
    fn new(tokens: &'a [Token]) -> Self {
        Self {
            tokens,
            pos: 0,
            diagnostics: Vec::new(),
        }
    }

    fn run(mut self) -> (Document, Vec<Diagnostic>) {
        let mut document = Document {
            version: None,
            statements: Vec::new(),
        };

        // Surface lex errors as diagnostics before discarding the tokens.
        for tok in self.tokens {
            if let TokenKind::Error(msg) = &tok.kind {
                self.diagnostics
                    .push(Diagnostic::error("KULA-L01", msg.clone(), tok.span));
            }
        }

        // Skip any leading newlines / indents.
        self.skip_blank_lines();

        // Optional version declaration (must be the first non-blank line).
        if matches!(self.peek_kind(), TokenKind::KulaKw) {
            if let Some(v) = self.parse_version_decl() {
                document.version = Some(v);
            }
            self.skip_blank_lines();
        }

        loop {
            match self.peek_kind() {
                TokenKind::Eof => break,
                TokenKind::Newline | TokenKind::Indent => {
                    self.advance();
                }
                TokenKind::PersonKw => {
                    if let Some(stmt) = self.parse_person_stmt() {
                        document.statements.push(Statement::Person(stmt));
                    }
                }
                _ => {
                    let tok = self.peek();
                    self.diagnostics.push(Diagnostic::error(
                        "KULA-P01",
                        format!(
                            "expected `person` (top-level statement), found {}",
                            describe_token(&tok.kind)
                        ),
                        tok.span,
                    ));
                    self.recover_to_newline();
                }
            }
        }

        (document, self.diagnostics)
    }

    fn parse_version_decl(&mut self) -> Option<VersionDecl> {
        let kula_tok = self.advance().clone();
        let mut span = kula_tok.span;
        let version_tok = self.peek().clone();
        match &version_tok.kind {
            TokenKind::Bare(v) | TokenKind::Ident(v) => {
                self.advance();
                span = span.merge(version_tok.span);
                self.expect_newline_or_eof("after version declaration");
                Some(VersionDecl {
                    span,
                    version: v.clone(),
                    version_span: version_tok.span,
                })
            }
            _ => {
                self.diagnostics.push(Diagnostic::error(
                    "KULA-P02",
                    format!(
                        "expected version literal after `kula`, found {}",
                        describe_token(&version_tok.kind)
                    ),
                    version_tok.span,
                ));
                self.recover_to_newline();
                None
            }
        }
    }

    fn parse_person_stmt(&mut self) -> Option<PersonStmt> {
        let person_tok = self.advance().clone();
        let keyword_span = person_tok.span;
        let mut span = keyword_span;

        let id_tok = self.peek().clone();
        let id = match &id_tok.kind {
            TokenKind::Ident(name) => {
                self.advance();
                span = span.merge(id_tok.span);
                Ident {
                    name: name.clone(),
                    span: id_tok.span,
                }
            }
            _ => {
                self.diagnostics.push(Diagnostic::error(
                    "KULA-P03",
                    format!(
                        "expected an identifier (the person's id) after `person`, found {}",
                        describe_token(&id_tok.kind)
                    ),
                    id_tok.span,
                ));
                self.recover_to_newline();
                return None;
            }
        };

        let mut fields = Vec::new();
        loop {
            match self.peek_kind() {
                TokenKind::Newline | TokenKind::Eof => break,
                TokenKind::FieldKw(_) => {
                    if let Some(field) = self.parse_person_field() {
                        span = span.merge(field.span);
                        fields.push(field);
                    } else {
                        // recover_to_newline already invoked inside parse_person_field
                        break;
                    }
                }
                _ => {
                    let tok = self.peek().clone();
                    self.diagnostics.push(Diagnostic::error(
                        "KULA-P04",
                        format!(
                            "expected a field (`name:`, `gender:`, …) or end of line, found {}",
                            describe_token(&tok.kind)
                        ),
                        tok.span,
                    ));
                    self.recover_to_newline();
                    break;
                }
            }
        }
        self.consume_newline();

        Some(PersonStmt {
            span,
            keyword_span,
            id,
            fields,
        })
    }

    fn parse_person_field(&mut self) -> Option<PersonField> {
        let name_tok = self.advance().clone();
        let TokenKind::FieldKw(field_name) = name_tok.kind else {
            unreachable!("parse_person_field called with non-field token");
        };
        let name_span = name_tok.span;
        let mut span = name_span;

        // Expect colon.
        let colon_tok = self.peek().clone();
        if !matches!(colon_tok.kind, TokenKind::Colon) {
            self.diagnostics.push(Diagnostic::error(
                "KULA-P05",
                format!(
                    "expected `:` after `{}`, found {}",
                    field_name.as_str(),
                    describe_token(&colon_tok.kind)
                ),
                colon_tok.span,
            ));
            self.recover_to_newline();
            return None;
        }
        self.advance();
        span = span.merge(colon_tok.span);

        let kind = match field_name {
            FieldName::Name => {
                let s = self.parse_string_value(field_name)?;
                PersonFieldKind::Name(s)
            }
            FieldName::Gender => self.parse_gender_value()?,
            FieldName::Family
            | FieldName::Given
            | FieldName::Born
            | FieldName::Died
            | FieldName::Start
            | FieldName::End
            | FieldName::EndReason => {
                self.diagnostics.push(Diagnostic::error(
                    "KULA-P06",
                    format!(
                        "field `{}` is not yet supported on `person`; this slice handles only `name` and `gender`",
                        field_name.as_str()
                    ),
                    name_span,
                ));
                self.recover_to_newline();
                return None;
            }
        };

        let value_span = match &kind {
            PersonFieldKind::Name(s) => s.span,
            PersonFieldKind::Gender(g) => g.span,
        };
        span = span.merge(value_span);

        Some(PersonField {
            span,
            name_span,
            kind,
        })
    }

    fn parse_string_value(&mut self, field: FieldName) -> Option<StringValue> {
        let tok = self.peek().clone();
        match tok.kind {
            TokenKind::String(value) => {
                self.advance();
                Some(StringValue {
                    value,
                    span: tok.span,
                })
            }
            _ => {
                self.diagnostics.push(Diagnostic::error(
                    "KULA-P07",
                    format!(
                        "expected a string literal for `{}:`, found {}",
                        field.as_str(),
                        describe_token(&tok.kind)
                    ),
                    tok.span,
                ));
                self.recover_to_newline();
                None
            }
        }
    }

    fn parse_gender_value(&mut self) -> Option<PersonFieldKind> {
        let tok = self.peek().clone();
        let value = match &tok.kind {
            TokenKind::EnumKw(EnumKw::Male) => Gender::Male,
            TokenKind::EnumKw(EnumKw::Female) => Gender::Female,
            TokenKind::EnumKw(EnumKw::Other) => Gender::Other,
            _ => {
                self.diagnostics.push(Diagnostic::error(
                    "KULA-P08",
                    format!(
                        "expected one of `male`, `female`, `other` for `gender:`, found {}",
                        describe_token(&tok.kind)
                    ),
                    tok.span,
                ));
                self.recover_to_newline();
                return None;
            }
        };
        self.advance();
        Some(PersonFieldKind::Gender(GenderValue {
            value,
            span: tok.span,
        }))
    }

    /// Consume a `Newline` if present. No-op at EOF or if recovery has
    /// already moved past the newline (e.g. into the next statement).
    fn consume_newline(&mut self) {
        if matches!(self.peek_kind(), TokenKind::Newline) {
            self.advance();
        }
    }

    fn expect_newline_or_eof(&mut self, ctx: &str) {
        match self.peek_kind() {
            TokenKind::Newline => {
                self.advance();
            }
            TokenKind::Eof => {}
            _ => {
                let tok = self.peek().clone();
                self.diagnostics.push(Diagnostic::error(
                    "KULA-P09",
                    format!(
                        "expected end of line {ctx}, found {}",
                        describe_token(&tok.kind)
                    ),
                    tok.span,
                ));
                self.recover_to_newline();
            }
        }
    }

    fn skip_blank_lines(&mut self) {
        while matches!(self.peek_kind(), TokenKind::Newline | TokenKind::Indent) {
            self.advance();
        }
    }

    fn recover_to_newline(&mut self) {
        while !matches!(self.peek_kind(), TokenKind::Newline | TokenKind::Eof) {
            self.advance();
        }
        if matches!(self.peek_kind(), TokenKind::Newline) {
            self.advance();
        }
    }

    fn peek(&self) -> &Token {
        // After construction the token stream always ends with Eof, so this
        // index is always valid.
        &self.tokens[self.pos.min(self.tokens.len() - 1)]
    }

    fn peek_kind(&self) -> &TokenKind {
        &self.peek().kind
    }

    fn advance(&mut self) -> &Token {
        let idx = self.pos.min(self.tokens.len() - 1);
        if self.pos < self.tokens.len() - 1 {
            self.pos += 1;
        }
        &self.tokens[idx]
    }
}

fn describe_token(kind: &TokenKind) -> String {
    match kind {
        TokenKind::KulaKw => "`kula`".into(),
        TokenKind::PersonKw => "`person`".into(),
        TokenKind::MarriageKw => "`marriage`".into(),
        TokenKind::BirthKw => "`birth`".into(),
        TokenKind::AdoptionKw => "`adoption`".into(),
        TokenKind::FieldKw(f) => format!("`{}`", f.as_str()),
        TokenKind::EnumKw(e) => format!("`{}`", e.as_str()),
        TokenKind::Ident(name) => format!("identifier `{name}`"),
        TokenKind::String(_) => "a string literal".into(),
        TokenKind::Bare(text) => format!("`{text}`"),
        TokenKind::Colon => "`:`".into(),
        TokenKind::Newline => "end of line".into(),
        TokenKind::Indent => "indentation".into(),
        TokenKind::Eof => "end of input".into(),
        TokenKind::Error(_) => "invalid input".into(),
    }
}
