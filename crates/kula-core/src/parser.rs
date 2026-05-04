//! Hand-written recursive-descent parser for Kula.
//!
//! Consumes the token stream from [`crate::lexer`] and produces a
//! [`Document`] together with parse diagnostics. On an unexpected token the
//! parser emits a diagnostic, advances to the next newline (panic-mode
//! recovery), and resumes parsing the next statement.

use crate::ast::{
    AdoptionField, AdoptionFieldKind, AdoptionSub, BirthSub, Document, EndReason, EndReasonValue,
    Gender, GenderValue, Ident, MarriageField, MarriageFieldKind, MarriageStmt, PersonField,
    PersonFieldKind, PersonStmt, Statement, StringValue, VersionDecl,
};
use crate::date::{DateLit, DateParseError, parse_date};
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
                TokenKind::MarriageKw => {
                    if let Some(stmt) = self.parse_marriage_stmt() {
                        document.statements.push(Statement::Marriage(stmt));
                    }
                }
                _ => {
                    let tok = self.peek();
                    self.diagnostics.push(Diagnostic::error(
                        "KULA-P01",
                        format!(
                            "expected `person` or `marriage` (top-level statement), found {}",
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
        let keyword_span = kula_tok.span;
        let mut span = kula_tok.span;
        let version_tok = self.peek().clone();
        match &version_tok.kind {
            TokenKind::Bare(v) | TokenKind::Ident(v) => {
                self.advance();
                span = span.merge(version_tok.span);
                self.expect_newline_or_eof("after version declaration");
                Some(VersionDecl {
                    span,
                    keyword_span,
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

        let (birth, adoptions, sub_span) = self.parse_person_sub_statements();
        if let Some(s) = sub_span {
            span = span.merge(s);
        }

        Some(PersonStmt {
            span,
            keyword_span,
            id,
            fields,
            birth,
            adoptions,
        })
    }

    /// Consume zero or more indented `birth` / `adoption` sub-statements that
    /// follow a `person` statement. Returns the parsed sub-statements and the
    /// span covering all of them (for use in the parent statement's span).
    fn parse_person_sub_statements(
        &mut self,
    ) -> (
        Option<BirthSub>,
        Vec<AdoptionSub>,
        Option<crate::span::ByteSpan>,
    ) {
        let mut birth: Option<BirthSub> = None;
        let mut adoptions: Vec<AdoptionSub> = Vec::new();
        let mut total_span: Option<crate::span::ByteSpan> = None;

        loop {
            // Skip blank lines (with or without leading indent).
            match self.peek_kind() {
                TokenKind::Newline => {
                    self.advance();
                    continue;
                }
                TokenKind::Indent => match self.peek_kind_at(1) {
                    TokenKind::Newline => {
                        // blank indented line
                        self.advance();
                        self.advance();
                        continue;
                    }
                    TokenKind::BirthKw => {
                        self.advance(); // Indent
                        if let Some(b) = self.parse_birth_sub() {
                            total_span = Some(match total_span {
                                Some(s) => s.merge(b.span),
                                None => b.span,
                            });
                            if birth.is_some() {
                                self.diagnostics.push(Diagnostic::error(
                                    "KULA-P13",
                                    "a person may have at most one `birth` sub-statement",
                                    b.span,
                                ));
                            } else {
                                birth = Some(b);
                            }
                        }
                    }
                    TokenKind::AdoptionKw => {
                        self.advance(); // Indent
                        if let Some(a) = self.parse_adoption_sub() {
                            total_span = Some(match total_span {
                                Some(s) => s.merge(a.span),
                                None => a.span,
                            });
                            adoptions.push(a);
                        }
                    }
                    _ => break,
                },
                _ => break,
            }
        }

        (birth, adoptions, total_span)
    }

    fn parse_birth_sub(&mut self) -> Option<BirthSub> {
        let kw_tok = self.advance().clone();
        let keyword_span = kw_tok.span;
        let mut span = keyword_span;
        let marriage_ref = self.expect_ident("the marriage id", "after `birth`")?;
        span = span.merge(marriage_ref.span);
        self.consume_newline();
        Some(BirthSub {
            span,
            keyword_span,
            marriage_ref,
        })
    }

    fn parse_adoption_sub(&mut self) -> Option<AdoptionSub> {
        let kw_tok = self.advance().clone();
        let keyword_span = kw_tok.span;
        let mut span = keyword_span;
        let marriage_ref = self.expect_ident("the marriage id", "after `adoption`")?;
        span = span.merge(marriage_ref.span);

        let mut fields = Vec::new();
        loop {
            match self.peek_kind() {
                TokenKind::Newline | TokenKind::Eof => break,
                TokenKind::FieldKw(_) => {
                    if let Some(f) = self.parse_adoption_field() {
                        span = span.merge(f.span);
                        fields.push(f);
                    } else {
                        break;
                    }
                }
                _ => {
                    let tok = self.peek().clone();
                    self.diagnostics.push(Diagnostic::error(
                        "KULA-P04",
                        format!(
                            "expected `start:` or `end:` on adoption, found {}",
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
        Some(AdoptionSub {
            span,
            keyword_span,
            marriage_ref,
            fields,
        })
    }

    fn parse_adoption_field(&mut self) -> Option<AdoptionField> {
        let name_tok = self.advance().clone();
        let TokenKind::FieldKw(field_name) = name_tok.kind else {
            unreachable!("parse_adoption_field called with non-field token");
        };
        let name_span = name_tok.span;
        let mut span = name_span;

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
            FieldName::Start => AdoptionFieldKind::Start(self.parse_date_value(field_name)?),
            FieldName::End => AdoptionFieldKind::End(self.parse_date_value(field_name)?),
            _ => {
                self.diagnostics.push(Diagnostic::error(
                    "KULA-P14",
                    format!(
                        "field `{}` is not valid on `adoption`; only `start` and `end` are allowed",
                        field_name.as_str()
                    ),
                    name_span,
                ));
                self.recover_to_newline();
                return None;
            }
        };

        let value_span = match &kind {
            AdoptionFieldKind::Start(d) | AdoptionFieldKind::End(d) => d.span,
        };
        span = span.merge(value_span);

        Some(AdoptionField {
            span,
            name_span,
            kind,
        })
    }

    fn parse_marriage_stmt(&mut self) -> Option<MarriageStmt> {
        let kw_tok = self.advance().clone();
        let keyword_span = kw_tok.span;
        let mut span = keyword_span;

        let id = self.expect_ident("the marriage's id", "after `marriage`")?;
        span = span.merge(id.span);
        let spouse_a = self.expect_ident("the first spouse", "as the first spouse")?;
        span = span.merge(spouse_a.span);
        let spouse_b = self.expect_ident("the second spouse", "as the second spouse")?;
        span = span.merge(spouse_b.span);

        let mut fields = Vec::new();
        loop {
            match self.peek_kind() {
                TokenKind::Newline | TokenKind::Eof => break,
                TokenKind::FieldKw(_) => {
                    if let Some(field) = self.parse_marriage_field() {
                        span = span.merge(field.span);
                        fields.push(field);
                    } else {
                        break;
                    }
                }
                _ => {
                    let tok = self.peek().clone();
                    self.diagnostics.push(Diagnostic::error(
                        "KULA-P04",
                        format!(
                            "expected a field (`start:`, `end:`, `end_reason:`) or end of line, found {}",
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

        Some(MarriageStmt {
            span,
            keyword_span,
            id,
            spouse_a,
            spouse_b,
            fields,
        })
    }

    fn expect_ident(&mut self, role: &str, ctx: &str) -> Option<Ident> {
        let tok = self.peek().clone();
        match &tok.kind {
            TokenKind::Ident(name) => {
                self.advance();
                Some(Ident {
                    name: name.clone(),
                    span: tok.span,
                })
            }
            _ => {
                self.diagnostics.push(Diagnostic::error(
                    "KULA-P03",
                    format!(
                        "expected an identifier ({role}) {ctx}, found {}",
                        describe_token(&tok.kind)
                    ),
                    tok.span,
                ));
                self.recover_to_newline();
                None
            }
        }
    }

    fn parse_marriage_field(&mut self) -> Option<MarriageField> {
        let name_tok = self.advance().clone();
        let TokenKind::FieldKw(field_name) = name_tok.kind else {
            unreachable!("parse_marriage_field called with non-field token");
        };
        let name_span = name_tok.span;
        let mut span = name_span;

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
            FieldName::Start => MarriageFieldKind::Start(self.parse_date_value(field_name)?),
            FieldName::End => MarriageFieldKind::End(self.parse_date_value(field_name)?),
            FieldName::EndReason => MarriageFieldKind::EndReason(self.parse_end_reason_value()?),
            FieldName::Name
            | FieldName::Family
            | FieldName::Given
            | FieldName::Born
            | FieldName::Died
            | FieldName::Gender => {
                self.diagnostics.push(Diagnostic::error(
                    "KULA-P10",
                    format!("field `{}` is not valid on `marriage`", field_name.as_str()),
                    name_span,
                ));
                self.recover_to_newline();
                return None;
            }
        };

        let value_span = match &kind {
            MarriageFieldKind::Start(d) | MarriageFieldKind::End(d) => d.span,
            MarriageFieldKind::EndReason(v) => v.span,
        };
        span = span.merge(value_span);

        Some(MarriageField {
            span,
            name_span,
            kind,
        })
    }

    fn parse_date_value(&mut self, field: FieldName) -> Option<DateLit> {
        let (raw, span) = self.expect_value(
            "KULA-P11",
            || format!("expected a date for `{}:`", field.as_str()),
            |tok| match &tok.kind {
                TokenKind::Bare(text) | TokenKind::Ident(text) => Some((text.clone(), tok.span)),
                _ => None,
            },
        )?;
        match parse_date(&raw, span) {
            Ok(date) => Some(date),
            Err(err) => {
                let code = match err {
                    DateParseError::Malformed(_) => "KULA-P15",
                    DateParseError::OutOfRange(_) => "KULA-P16",
                };
                self.diagnostics.push(Diagnostic::error(
                    code,
                    format!("invalid date for `{}:`: {}", field.as_str(), err.message()),
                    span,
                ));
                None
            }
        }
    }

    fn parse_end_reason_value(&mut self) -> Option<EndReasonValue> {
        self.expect_value(
            "KULA-P12",
            || "expected an `end_reason:` value".into(),
            |tok| {
                let value = match &tok.kind {
                    TokenKind::EnumKw(EnumKw::Divorce) => EndReason::Divorce,
                    TokenKind::Ident(text) | TokenKind::Bare(text) | TokenKind::String(text) => {
                        EndReason::Unknown(text.clone())
                    }
                    _ => return None,
                };
                Some(EndReasonValue {
                    value,
                    span: tok.span,
                })
            },
        )
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
            FieldName::Name => PersonFieldKind::Name(self.parse_string_value(field_name)?),
            FieldName::Family => PersonFieldKind::Family(self.parse_string_value(field_name)?),
            FieldName::Given => PersonFieldKind::Given(self.parse_string_value(field_name)?),
            FieldName::Born => PersonFieldKind::Born(self.parse_date_value(field_name)?),
            FieldName::Died => PersonFieldKind::Died(self.parse_date_value(field_name)?),
            FieldName::Gender => self.parse_gender_value()?,
            FieldName::Start | FieldName::End | FieldName::EndReason => {
                self.diagnostics.push(Diagnostic::error(
                    "KULA-P10",
                    format!("field `{}` is not valid on `person`", field_name.as_str()),
                    name_span,
                ));
                self.recover_to_newline();
                return None;
            }
        };

        let value_span = match &kind {
            PersonFieldKind::Name(s) | PersonFieldKind::Family(s) | PersonFieldKind::Given(s) => {
                s.span
            }
            PersonFieldKind::Born(d) | PersonFieldKind::Died(d) => d.span,
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
        self.expect_value(
            "KULA-P07",
            || format!("expected a string literal for `{}:`", field.as_str()),
            |tok| match &tok.kind {
                TokenKind::String(value) => Some(StringValue {
                    value: value.clone(),
                    span: tok.span,
                }),
                _ => None,
            },
        )
    }

    fn parse_gender_value(&mut self) -> Option<PersonFieldKind> {
        let value = self.expect_value(
            "KULA-P08",
            || "expected one of `male`, `female`, `other` for `gender:`".into(),
            |tok| {
                let g = match &tok.kind {
                    TokenKind::EnumKw(EnumKw::Male) => Gender::Male,
                    TokenKind::EnumKw(EnumKw::Female) => Gender::Female,
                    TokenKind::EnumKw(EnumKw::Other) => Gender::Other,
                    _ => return None,
                };
                Some(GenderValue {
                    value: g,
                    span: tok.span,
                })
            },
        )?;
        Some(PersonFieldKind::Gender(value))
    }

    /// Peek the next token, hand it to `accept`. On `Some(value)`, advance
    /// and return it. On `None`, push a diagnostic of the form
    /// "<expected>, found <token>" with `code` and `recover_to_newline()`.
    ///
    /// Centralizes the "value expected here, recover on mismatch" pattern
    /// shared by every field-value parser.
    fn expect_value<T>(
        &mut self,
        code: &'static str,
        expected: impl FnOnce() -> String,
        accept: impl FnOnce(&Token) -> Option<T>,
    ) -> Option<T> {
        let tok = self.peek().clone();
        if let Some(value) = accept(&tok) {
            self.advance();
            return Some(value);
        }
        self.diagnostics.push(Diagnostic::error(
            code,
            format!("{}, found {}", expected(), describe_token(&tok.kind)),
            tok.span,
        ));
        self.recover_to_newline();
        None
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

    fn peek_kind_at(&self, offset: usize) -> &TokenKind {
        let idx = (self.pos + offset).min(self.tokens.len() - 1);
        &self.tokens[idx].kind
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
