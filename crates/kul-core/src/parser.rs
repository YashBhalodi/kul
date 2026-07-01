//! Hand-written recursive-descent parser. Panic-mode recovery: on an
//! unexpected token the parser emits a diagnostic, advances to the next
//! newline, and resumes. AST nodes keep bare [`ByteSpan`]s; diagnostic
//! spans get wrapped into [`FileSpan`]s anchored at the parser's
//! [`FileId`].

use crate::ast::{
    AdoptionField, AdoptionFieldKind, AdoptionSub, BirthSub, EndReason, EndReasonValue, Gender,
    GenderValue, Ident, MarriageField, MarriageFieldKind, MarriageStmt, PersonField,
    PersonFieldKind, PersonStmt, Statement, StringValue,
};
use crate::date::{DateLit, DateParseError, parse_date};
use crate::diagnostic::{Diagnostic, fspan};
use crate::lexer::{EnumKw, FieldName, Token, TokenKind};
use crate::span::{ByteSpan, FileId};

/// Parse a token stream into top-level statements plus diagnostics.
pub fn parse(tokens: &[Token], file: FileId) -> (Vec<Statement>, Vec<Diagnostic>) {
    Parser::new(tokens, file).run()
}

/// Outcome of parsing one field on a person, marriage, or adoption.
enum FieldOutcome<T> {
    Field(T),
    /// Field-name matched, value parse failed; recovery stopped at the next
    /// field boundary, so scanning continues. The name is recorded so R03's
    /// "missing field" check is silenced for the affected field.
    Malformed(FieldName),
    /// Statement-level unrecoverable (e.g. missing colon); stop scanning.
    Fatal,
}

struct Parser<'a> {
    tokens: &'a [Token],
    pos: usize,
    file: FileId,
    diagnostics: Vec<Diagnostic>,
}

impl<'a> Parser<'a> {
    fn new(tokens: &'a [Token], file: FileId) -> Self {
        Self {
            tokens,
            pos: 0,
            file,
            diagnostics: Vec::new(),
        }
    }

    fn fspan(&self, span: ByteSpan) -> crate::span::FileSpan {
        fspan(self.file, span)
    }

    fn run(mut self) -> (Vec<Statement>, Vec<Diagnostic>) {
        let mut statements: Vec<Statement> = Vec::new();

        for tok in self.tokens {
            if let TokenKind::Error(msg) = &tok.kind {
                self.diagnostics.push(Diagnostic::error(
                    "KUL-L01",
                    msg.clone(),
                    self.fspan(tok.span),
                ));
            }
        }

        self.skip_blank_lines();

        loop {
            match self.peek_kind() {
                TokenKind::Eof => break,
                TokenKind::Newline | TokenKind::Indent => {
                    self.advance();
                }
                TokenKind::PersonKw => {
                    if let Some(stmt) = self.parse_person_stmt() {
                        statements.push(Statement::Person(stmt));
                    }
                }
                TokenKind::MarriageKw => {
                    if let Some(stmt) = self.parse_marriage_stmt() {
                        statements.push(Statement::Marriage(stmt));
                    }
                }
                _ => {
                    let span = self.peek().span;
                    let description = describe_token(&self.peek().kind);
                    self.diagnostics.push(Diagnostic::error(
                        "KUL-P01",
                        format!(
                            "expected `person` or `marriage` (top-level statement), found {description}"
                        ),
                        self.fspan(span),
                    ));
                    self.recover_to_newline();
                }
            }
        }

        (statements, self.diagnostics)
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
                    "KUL-P03",
                    format!(
                        "expected an identifier (the person's id) after `person`, found {}",
                        describe_token(&id_tok.kind)
                    ),
                    self.fspan(id_tok.span),
                ));
                self.recover_to_newline();
                return None;
            }
        };

        let mut fields = Vec::new();
        let mut malformed_fields: Vec<FieldName> = Vec::new();
        loop {
            match self.peek_kind() {
                TokenKind::Newline | TokenKind::Eof => break,
                TokenKind::FieldKw(_) => match self.parse_person_field() {
                    FieldOutcome::Field(field) => {
                        span = span.merge(field.span);
                        fields.push(field);
                    }
                    FieldOutcome::Malformed(name) => {
                        malformed_fields.push(name);
                    }
                    FieldOutcome::Fatal => break,
                },
                _ => {
                    let span = self.peek().span;
                    let description = describe_token(&self.peek().kind);
                    self.diagnostics.push(Diagnostic::error(
                        "KUL-P04",
                        format!(
                            "expected a field (`name:`, `gender:`, …) or end of line, found {description}"
                        ),
                        self.fspan(span),
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
            malformed_fields,
        })
    }

    /// Consume indented `birth`/`adoption` sub-statements after a person.
    /// Returns the parsed subs and their combined span.
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
            match self.peek_kind() {
                TokenKind::Newline => {
                    self.advance();
                    continue;
                }
                TokenKind::Indent => match self.peek_kind_at(1) {
                    TokenKind::Newline => {
                        self.advance();
                        self.advance();
                        continue;
                    }
                    TokenKind::BirthKw => {
                        self.advance();
                        if let Some(b) = self.parse_birth_sub() {
                            total_span = Some(match total_span {
                                Some(s) => s.merge(b.span),
                                None => b.span,
                            });
                            if birth.is_some() {
                                self.diagnostics.push(Diagnostic::error(
                                    "KUL-P13",
                                    "a person may have at most one `birth` sub-statement",
                                    self.fspan(b.span),
                                ));
                            } else {
                                birth = Some(b);
                            }
                        }
                    }
                    TokenKind::AdoptionKw => {
                        self.advance();
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
                TokenKind::FieldKw(_) => match self.parse_adoption_field() {
                    FieldOutcome::Field(f) => {
                        span = span.merge(f.span);
                        fields.push(f);
                    }
                    FieldOutcome::Malformed(_) => {}
                    FieldOutcome::Fatal => break,
                },
                _ => {
                    let span = self.peek().span;
                    let description = describe_token(&self.peek().kind);
                    self.diagnostics.push(Diagnostic::error(
                        "KUL-P04",
                        format!("expected `start:` or `end:` on adoption, found {description}"),
                        self.fspan(span),
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

    fn parse_adoption_field(&mut self) -> FieldOutcome<AdoptionField> {
        let name_tok = self.advance().clone();
        let TokenKind::FieldKw(field_name) = name_tok.kind else {
            unreachable!("parse_adoption_field called with non-field token");
        };
        let name_span = name_tok.span;
        let mut span = name_span;

        let colon_tok = self.peek().clone();
        if !matches!(colon_tok.kind, TokenKind::Colon) {
            self.diagnostics.push(Diagnostic::error(
                "KUL-P05",
                format!(
                    "expected `:` after `{}`, found {}",
                    field_name.as_str(),
                    describe_token(&colon_tok.kind)
                ),
                self.fspan(colon_tok.span),
            ));
            self.recover_to_newline();
            return FieldOutcome::Fatal;
        }
        self.advance();
        span = span.merge(colon_tok.span);

        let kind = match field_name {
            FieldName::Start => match self.parse_date_value(field_name) {
                Some(d) => AdoptionFieldKind::Start(d),
                None => return FieldOutcome::Malformed(field_name),
            },
            FieldName::End => match self.parse_date_value(field_name) {
                Some(d) => AdoptionFieldKind::End(d),
                None => return FieldOutcome::Malformed(field_name),
            },
            _ => {
                self.diagnostics.push(Diagnostic::error(
                    "KUL-P14",
                    format!(
                        "field `{}` isn't valid on `adoption` — valid fields are `start:` and `end:`",
                        field_name.as_str()
                    ),
                    self.fspan(name_span),
                ));
                self.recover_to_newline();
                return FieldOutcome::Fatal;
            }
        };

        let value_span = match &kind {
            AdoptionFieldKind::Start(d) | AdoptionFieldKind::End(d) => d.span,
        };
        span = span.merge(value_span);

        FieldOutcome::Field(AdoptionField {
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
                TokenKind::FieldKw(_) => match self.parse_marriage_field() {
                    FieldOutcome::Field(field) => {
                        span = span.merge(field.span);
                        fields.push(field);
                    }
                    FieldOutcome::Malformed(_) => {}
                    FieldOutcome::Fatal => break,
                },
                _ => {
                    let span = self.peek().span;
                    let description = describe_token(&self.peek().kind);
                    self.diagnostics.push(Diagnostic::error(
                        "KUL-P04",
                        format!(
                            "expected a field (`start:`, `end:`, `end_reason:`) or end of line, found {description}"
                        ),
                        self.fspan(span),
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
                    "KUL-P03",
                    format!(
                        "expected an identifier ({role}) {ctx}, found {}",
                        describe_token(&tok.kind)
                    ),
                    self.fspan(tok.span),
                ));
                self.recover_to_newline();
                None
            }
        }
    }

    fn parse_marriage_field(&mut self) -> FieldOutcome<MarriageField> {
        let name_tok = self.advance().clone();
        let TokenKind::FieldKw(field_name) = name_tok.kind else {
            unreachable!("parse_marriage_field called with non-field token");
        };
        let name_span = name_tok.span;
        let mut span = name_span;

        let colon_tok = self.peek().clone();
        if !matches!(colon_tok.kind, TokenKind::Colon) {
            self.diagnostics.push(Diagnostic::error(
                "KUL-P05",
                format!(
                    "expected `:` after `{}`, found {}",
                    field_name.as_str(),
                    describe_token(&colon_tok.kind)
                ),
                self.fspan(colon_tok.span),
            ));
            self.recover_to_newline();
            return FieldOutcome::Fatal;
        }
        self.advance();
        span = span.merge(colon_tok.span);

        let kind = match field_name {
            FieldName::Start => match self.parse_date_value(field_name) {
                Some(d) => MarriageFieldKind::Start(d),
                None => return FieldOutcome::Malformed(field_name),
            },
            FieldName::End => match self.parse_date_value(field_name) {
                Some(d) => MarriageFieldKind::End(d),
                None => return FieldOutcome::Malformed(field_name),
            },
            FieldName::EndReason => match self.parse_end_reason_value() {
                Some(v) => MarriageFieldKind::EndReason(v),
                None => return FieldOutcome::Fatal,
            },
            FieldName::Name
            | FieldName::Family
            | FieldName::Given
            | FieldName::Born
            | FieldName::Died
            | FieldName::Gender => {
                self.diagnostics.push(Diagnostic::error(
                    "KUL-P10",
                    format!(
                        "field `{}` isn't valid on `marriage` — valid fields are `start:`, `end:`, and `end_reason:`",
                        field_name.as_str()
                    ),
                    self.fspan(name_span),
                ));
                self.recover_to_newline();
                return FieldOutcome::Fatal;
            }
        };

        let value_span = match &kind {
            MarriageFieldKind::Start(d) | MarriageFieldKind::End(d) => d.span,
            MarriageFieldKind::EndReason(v) => v.span,
        };
        span = span.merge(value_span);

        FieldOutcome::Field(MarriageField {
            span,
            name_span,
            kind,
        })
    }

    fn parse_date_value(&mut self, field: FieldName) -> Option<DateLit> {
        let (raw, span) = self.expect_value(
            "KUL-P11",
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
                    DateParseError::Malformed(_) => "KUL-P15",
                    DateParseError::OutOfRange(_) => "KUL-P16",
                };
                self.diagnostics.push(Diagnostic::error(
                    code,
                    format!("invalid date for `{}:`: {}", field.as_str(), err.message()),
                    self.fspan(span),
                ));
                // Stop at the next field keyword so the remaining well-formed
                // fields on the line still parse — mirrors `parse_string_value`.
                self.recover_to_field_boundary();
                None
            }
        }
    }

    fn parse_end_reason_value(&mut self) -> Option<EndReasonValue> {
        self.expect_value(
            "KUL-P12",
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

    fn parse_person_field(&mut self) -> FieldOutcome<PersonField> {
        let name_tok = self.advance().clone();
        let TokenKind::FieldKw(field_name) = name_tok.kind else {
            unreachable!("parse_person_field called with non-field token");
        };
        let name_span = name_tok.span;
        let mut span = name_span;

        let colon_tok = self.peek().clone();
        if !matches!(colon_tok.kind, TokenKind::Colon) {
            self.diagnostics.push(Diagnostic::error(
                "KUL-P05",
                format!(
                    "expected `:` after `{}`, found {}",
                    field_name.as_str(),
                    describe_token(&colon_tok.kind)
                ),
                self.fspan(colon_tok.span),
            ));
            self.recover_to_newline();
            return FieldOutcome::Fatal;
        }
        self.advance();
        span = span.merge(colon_tok.span);

        let kind = match field_name {
            FieldName::Name => match self.parse_string_value(field_name) {
                Some(s) => PersonFieldKind::Name(s),
                None => return FieldOutcome::Malformed(field_name),
            },
            FieldName::Family => match self.parse_string_value(field_name) {
                Some(s) => PersonFieldKind::Family(s),
                None => return FieldOutcome::Malformed(field_name),
            },
            FieldName::Given => match self.parse_string_value(field_name) {
                Some(s) => PersonFieldKind::Given(s),
                None => return FieldOutcome::Malformed(field_name),
            },
            FieldName::Born => match self.parse_date_value(field_name) {
                Some(d) => PersonFieldKind::Born(d),
                None => return FieldOutcome::Malformed(field_name),
            },
            FieldName::Died => match self.parse_date_value(field_name) {
                Some(d) => PersonFieldKind::Died(d),
                None => return FieldOutcome::Malformed(field_name),
            },
            FieldName::Gender => match self.parse_gender_value() {
                Some(g) => PersonFieldKind::Gender(g),
                None => return FieldOutcome::Malformed(field_name),
            },
            FieldName::Start | FieldName::End | FieldName::EndReason => {
                self.diagnostics.push(Diagnostic::error(
                    "KUL-P10",
                    format!(
                        "field `{}` isn't valid on `person` — valid fields are `name:`, `family:`, `given:`, `gender:`, `born:`, and `died:`",
                        field_name.as_str()
                    ),
                    self.fspan(name_span),
                ));
                self.recover_to_newline();
                return FieldOutcome::Fatal;
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

        FieldOutcome::Field(PersonField {
            span,
            name_span,
            kind,
        })
    }

    fn parse_string_value(&mut self, field: FieldName) -> Option<StringValue> {
        let tok = self.peek().clone();
        if let TokenKind::String(value) = &tok.kind {
            self.advance();
            return Some(StringValue {
                value: value.clone(),
                span: tok.span,
            });
        }
        self.diagnostics.push(Diagnostic::error(
            "KUL-P07",
            format!(
                "expected a quoted string for `{name}:` (e.g. `{name}:\"…\"`), found {found}",
                name = field.as_str(),
                found = describe_token(&tok.kind)
            ),
            self.fspan(tok.span),
        ));
        // Stop at the next field keyword so e.g. a following `gender:female`
        // on the same line still parses.
        self.recover_to_field_boundary();
        None
    }

    fn parse_gender_value(&mut self) -> Option<GenderValue> {
        let tok = self.peek().clone();
        let value = match &tok.kind {
            TokenKind::EnumKw(EnumKw::Male) => Gender::Male,
            TokenKind::EnumKw(EnumKw::Female) => Gender::Female,
            TokenKind::EnumKw(EnumKw::Other) => Gender::Other,
            _ => {
                self.diagnostics.push(Diagnostic::error(
                    "KUL-P08",
                    format!(
                        "expected one of `male`, `female`, `other` for `gender:`, found {}",
                        describe_token(&tok.kind)
                    ),
                    self.fspan(tok.span),
                ));
                // Stop at the next field keyword so remaining well-formed
                // fields on the line still parse — mirrors `parse_string_value`
                // so a malformed `gender:` is recorded as `Malformed`, not
                // `Fatal` (which would let R03 also report gender as missing).
                self.recover_to_field_boundary();
                return None;
            }
        };
        self.advance();
        Some(GenderValue {
            value,
            span: tok.span,
        })
    }

    /// Peek the next token via `accept`; on match, advance. On miss, push
    /// `"<expected>, found <token>"` with `code` and recover to newline.
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
            self.fspan(tok.span),
        ));
        self.recover_to_newline();
        None
    }

    /// Consume a `Newline` if present; no-op at EOF.
    fn consume_newline(&mut self) {
        if matches!(self.peek_kind(), TokenKind::Newline) {
            self.advance();
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

    /// Skip up to (not past) the next field keyword, newline, or EOF.
    /// E.g. `name:Alice Sharma gender:female` still parses `gender:female`.
    fn recover_to_field_boundary(&mut self) {
        while !matches!(
            self.peek_kind(),
            TokenKind::FieldKw(_) | TokenKind::Newline | TokenKind::Eof
        ) {
            self.advance();
        }
    }

    fn peek(&self) -> &Token {
        // Token stream always ends with Eof, so the index is always valid.
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
