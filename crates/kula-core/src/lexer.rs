//! Lexer: turns source bytes into a stream of typed [`Token`]s.
//!
//! The lexer is total — every byte sequence yields a token stream. Lex errors
//! become [`TokenKind::Error`] tokens with a message; the parser surfaces
//! them as diagnostics so the source-code error position is preserved.
//!
//! INDENT/DEDENT handling lands with sub-statement support (#9). For now the
//! lexer simply emits a [`TokenKind::Indent`] when a line begins with leading
//! whitespace; the parser ignores it.

use crate::span::ByteSpan;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Token {
    pub kind: TokenKind,
    pub span: ByteSpan,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TokenKind {
    /// `kula` — version declaration keyword.
    KulaKw,
    /// `person`
    PersonKw,
    /// `marriage`
    MarriageKw,
    /// `birth`
    BirthKw,
    /// `adoption`
    AdoptionKw,
    /// A field-name keyword (`name`, `gender`, etc.). Followed by `:` and a value.
    FieldKw(FieldName),
    /// An enumeration value keyword (`male`, `female`, `other`, `divorce`).
    EnumKw(EnumKw),
    /// An identifier — used for IDs and references.
    Ident(String),
    /// A double-quoted string literal. Stored as the unescaped contents.
    String(String),
    /// A bare value: a sequence of non-whitespace, non-`:`, non-`#`, non-`"`
    /// characters. Used for dates, identifier-like values, etc.
    Bare(String),
    /// `:`
    Colon,
    /// End of a logical line.
    Newline,
    /// Leading horizontal whitespace at the start of a line.
    Indent,
    /// End of input.
    Eof,
    /// A lex error — the [`String`] explains. The parser will surface this
    /// as a diagnostic anchored to the token's span.
    Error(String),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum FieldName {
    Name,
    Family,
    Given,
    Born,
    Died,
    Gender,
    Start,
    End,
    EndReason,
}

impl FieldName {
    pub fn as_str(self) -> &'static str {
        match self {
            FieldName::Name => "name",
            FieldName::Family => "family",
            FieldName::Given => "given",
            FieldName::Born => "born",
            FieldName::Died => "died",
            FieldName::Gender => "gender",
            FieldName::Start => "start",
            FieldName::End => "end",
            FieldName::EndReason => "end_reason",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EnumKw {
    Male,
    Female,
    Other,
    Divorce,
}

impl EnumKw {
    pub fn as_str(self) -> &'static str {
        match self {
            EnumKw::Male => "male",
            EnumKw::Female => "female",
            EnumKw::Other => "other",
            EnumKw::Divorce => "divorce",
        }
    }
}

pub fn tokenize(source: &str) -> Vec<Token> {
    Lexer::new(source).run()
}

struct Lexer<'a> {
    source: &'a str,
    bytes: &'a [u8],
    pos: usize,
    tokens: Vec<Token>,
    at_line_start: bool,
}

impl<'a> Lexer<'a> {
    fn new(source: &'a str) -> Self {
        Self {
            source,
            bytes: source.as_bytes(),
            pos: 0,
            tokens: Vec::new(),
            at_line_start: true,
        }
    }

    fn run(mut self) -> Vec<Token> {
        while self.pos < self.bytes.len() {
            if self.at_line_start {
                self.at_line_start = false;
                self.lex_line_start();
                continue;
            }
            let b = self.bytes[self.pos];
            match b {
                b' ' | b'\t' => {
                    self.pos += 1;
                }
                b'\r' => {
                    let start = self.pos;
                    self.pos += 1;
                    if self.peek_byte() == Some(b'\n') {
                        self.pos += 1;
                    }
                    self.push(TokenKind::Newline, start, self.pos);
                    self.at_line_start = true;
                }
                b'\n' => {
                    let start = self.pos;
                    self.pos += 1;
                    self.push(TokenKind::Newline, start, self.pos);
                    self.at_line_start = true;
                }
                b'#' => {
                    self.skip_comment();
                }
                b':' => {
                    let start = self.pos;
                    self.pos += 1;
                    self.push(TokenKind::Colon, start, self.pos);
                }
                b'"' => {
                    self.lex_string();
                }
                _ => {
                    self.lex_word_or_bare();
                }
            }
        }
        let eof_span = ByteSpan::new(self.bytes.len(), self.bytes.len());
        self.tokens.push(Token {
            kind: TokenKind::Eof,
            span: eof_span,
        });
        self.tokens
    }

    fn lex_line_start(&mut self) {
        let start = self.pos;
        while let Some(b) = self.peek_byte() {
            if b == b' ' || b == b'\t' {
                self.pos += 1;
            } else {
                break;
            }
        }
        if self.pos > start {
            self.push(TokenKind::Indent, start, self.pos);
        }
    }

    fn skip_comment(&mut self) {
        while let Some(b) = self.peek_byte() {
            if b == b'\n' || b == b'\r' {
                break;
            }
            self.pos += 1;
        }
    }

    fn lex_string(&mut self) {
        let start = self.pos;
        self.pos += 1; // opening quote
        let mut value = String::new();
        loop {
            match self.peek_byte() {
                None => {
                    self.push(
                        TokenKind::Error(
                            "unterminated string literal: missing closing `\"`".into(),
                        ),
                        start,
                        self.pos,
                    );
                    return;
                }
                Some(b'"') => {
                    self.pos += 1;
                    self.push(TokenKind::String(value), start, self.pos);
                    return;
                }
                Some(b'\\') => {
                    self.pos += 1;
                    match self.peek_byte() {
                        Some(b'\\') => {
                            value.push('\\');
                            self.pos += 1;
                        }
                        Some(b'"') => {
                            value.push('"');
                            self.pos += 1;
                        }
                        Some(other) => {
                            self.pos += 1;
                            self.push(
                                TokenKind::Error(format!(
                                    "invalid string escape `\\{}`; only `\\\\` and `\\\"` are recognized",
                                    other as char
                                )),
                                start,
                                self.pos,
                            );
                            return;
                        }
                        None => {
                            self.push(
                                TokenKind::Error(
                                    "string literal ends with a stray backslash".into(),
                                ),
                                start,
                                self.pos,
                            );
                            return;
                        }
                    }
                }
                Some(_) => {
                    let ch_start = self.pos;
                    let ch_end = next_char_boundary(self.bytes, ch_start);
                    value.push_str(&self.source[ch_start..ch_end]);
                    self.pos = ch_end;
                }
            }
        }
    }

    fn lex_word_or_bare(&mut self) {
        let start = self.pos;
        while let Some(b) = self.peek_byte() {
            match b {
                b' ' | b'\t' | b'\r' | b'\n' | b'#' | b':' | b'"' => break,
                _ => {
                    self.pos = next_char_boundary(self.bytes, self.pos);
                }
            }
        }
        if self.pos == start {
            // Defensive: should not happen because we only enter this branch
            // when a non-special byte is present.
            self.pos += 1;
        }
        let text = &self.source[start..self.pos];
        let kind = classify_word(text);
        self.push(kind, start, self.pos);
    }

    fn peek_byte(&self) -> Option<u8> {
        self.bytes.get(self.pos).copied()
    }

    fn push(&mut self, kind: TokenKind, start: usize, end: usize) {
        self.tokens.push(Token {
            kind,
            span: ByteSpan::new(start, end),
        });
    }
}

fn next_char_boundary(bytes: &[u8], pos: usize) -> usize {
    let b = bytes[pos];
    let len = if b < 0x80 {
        1
    } else if b < 0xC0 {
        // Continuation byte mid-character — advance by 1 to make progress.
        1
    } else if b < 0xE0 {
        2
    } else if b < 0xF0 {
        3
    } else {
        4
    };
    (pos + len).min(bytes.len())
}

fn classify_word(text: &str) -> TokenKind {
    match text {
        "kula" => TokenKind::KulaKw,
        "person" => TokenKind::PersonKw,
        "marriage" => TokenKind::MarriageKw,
        "birth" => TokenKind::BirthKw,
        "adoption" => TokenKind::AdoptionKw,
        "name" => TokenKind::FieldKw(FieldName::Name),
        "family" => TokenKind::FieldKw(FieldName::Family),
        "given" => TokenKind::FieldKw(FieldName::Given),
        "born" => TokenKind::FieldKw(FieldName::Born),
        "died" => TokenKind::FieldKw(FieldName::Died),
        "gender" => TokenKind::FieldKw(FieldName::Gender),
        "start" => TokenKind::FieldKw(FieldName::Start),
        "end" => TokenKind::FieldKw(FieldName::End),
        "end_reason" => TokenKind::FieldKw(FieldName::EndReason),
        "male" => TokenKind::EnumKw(EnumKw::Male),
        "female" => TokenKind::EnumKw(EnumKw::Female),
        "other" => TokenKind::EnumKw(EnumKw::Other),
        "divorce" => TokenKind::EnumKw(EnumKw::Divorce),
        _ => {
            if is_identifier(text) {
                TokenKind::Ident(text.to_owned())
            } else {
                TokenKind::Bare(text.to_owned())
            }
        }
    }
}

fn is_identifier(text: &str) -> bool {
    let mut chars = text.chars();
    let Some(first) = chars.next() else {
        return false;
    };
    if !(first.is_ascii_alphabetic() || first == '_') {
        return false;
    }
    chars.all(|c| c.is_ascii_alphanumeric() || c == '_' || c == '-')
}
