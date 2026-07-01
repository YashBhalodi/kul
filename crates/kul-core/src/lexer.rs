//! Lexer: source bytes → typed [`Token`] stream.
//!
//! Total: every byte sequence yields tokens. Lex errors become
//! [`TokenKind::Error`] tokens that the parser surfaces as diagnostics
//! anchored on the token span.

use crate::span::ByteSpan;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Token {
    pub kind: TokenKind,
    pub span: ByteSpan,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TokenKind {
    PersonKw,
    MarriageKw,
    BirthKw,
    AdoptionKw,
    /// `name:`, `gender:`, etc. Followed by `:` and a value.
    FieldKw(FieldName),
    /// `male`, `female`, `other`, `divorce`.
    EnumKw(EnumKw),
    /// Identifier (IDs and references).
    Ident(String),
    /// Double-quoted string, stored unescaped.
    String(String),
    /// Bareword (dates, identifier-like values, etc.): a run of bytes that
    /// aren't whitespace, `:`, `#`, or `"`.
    Bare(String),
    Colon,
    Newline,
    /// Leading horizontal whitespace at the start of a line.
    Indent,
    Eof,
    /// Lex error; the parser surfaces this as a span-anchored diagnostic.
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
        self.pos += 1;
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
                Some(b'\n') | Some(b'\r') => {
                    // Embedded newlines are legal inside string literals
                    // (spec §3.3), so we normally consume them as body. But a
                    // forgotten closing quote would otherwise swallow every
                    // subsequent line — including well-formed statements — as
                    // string body and emit one Error token to EOF. As a
                    // recovery heuristic, if the next line begins at column 0
                    // with a top-level keyword, treat the string as
                    // unterminated ending before that line so a stray missing
                    // quote can't cascade past the next real statement. See
                    // ADR-0023.
                    let next_line = self.line_terminator_end(self.pos);
                    if starts_top_level_keyword(&self.bytes[next_line..]) {
                        self.push(
                            TokenKind::Error(
                                "unterminated string literal: missing closing `\"`".into(),
                            ),
                            start,
                            self.pos,
                        );
                        return;
                    }
                    let ch_start = self.pos;
                    let ch_end = next_char_boundary(self.bytes, ch_start);
                    value.push_str(&self.source[ch_start..ch_end]);
                    self.pos = ch_end;
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

    /// Byte offset of the line that follows the terminator at `pos`, where
    /// `self.bytes[pos]` is `\r` or `\n`. Consumes a `\r\n` pair as one
    /// terminator.
    fn line_terminator_end(&self, pos: usize) -> usize {
        if self.bytes[pos] == b'\r' && self.bytes.get(pos + 1) == Some(&b'\n') {
            pos + 2
        } else {
            pos + 1
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
            // Defensive: ensure progress on a non-special byte.
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
        // Continuation byte mid-character; advance by 1 to make progress.
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

/// True iff `bytes` begins, at column 0 with no leading whitespace, with a
/// top-level statement keyword (`person` / `marriage`). Used by
/// [`Lexer::lex_string`]'s unterminated-string recovery (ADR-0023). The
/// leading-word extraction mirrors [`Lexer::lex_word_or_bare`] and defers to
/// [`classify_word`] so the reserved top-level set stays single-sourced: only
/// the two `Statement` keywords the parser dispatches on trigger recovery.
fn starts_top_level_keyword(bytes: &[u8]) -> bool {
    let end = bytes
        .iter()
        .position(|&b| matches!(b, b' ' | b'\t' | b'\r' | b'\n' | b'#' | b':' | b'"'))
        .unwrap_or(bytes.len());
    let Ok(word) = std::str::from_utf8(&bytes[..end]) else {
        return false;
    };
    matches!(
        classify_word(word),
        TokenKind::PersonKw | TokenKind::MarriageKw
    )
}

/// Match the Kul identifier production `[A-Za-z_][A-Za-z0-9_-]*`. The
/// canonical check; consumers validating candidate ids should call this
/// rather than re-implement the rule.
pub fn is_identifier(text: &str) -> bool {
    let mut chars = text.chars();
    let Some(first) = chars.next() else {
        return false;
    };
    if !(first.is_ascii_alphabetic() || first == '_') {
        return false;
    }
    chars.all(|c| c.is_ascii_alphanumeric() || c == '_' || c == '-')
}

/// True iff `text` would tokenize as a reserved keyword (statement,
/// field, or enum) rather than an identifier. Derived from
/// [`classify_word`].
pub fn is_reserved_word(text: &str) -> bool {
    !matches!(
        classify_word(text),
        TokenKind::Ident(_) | TokenKind::Bare(_)
    )
}

#[cfg(test)]
mod tests {
    use super::{is_identifier, is_reserved_word};

    #[test]
    fn is_identifier_accepts_letter_underscore_digit_hyphen() {
        assert!(is_identifier("alice"));
        assert!(is_identifier("_underscore_first"));
        assert!(is_identifier("a"));
        assert!(is_identifier("alice_2"));
        assert!(is_identifier("alice-bob"));
        assert!(is_identifier("A"));
    }

    #[test]
    fn is_identifier_rejects_empty_or_invalid() {
        assert!(!is_identifier(""));
        assert!(!is_identifier("1leading_digit"));
        assert!(!is_identifier("-leading-hyphen"));
        assert!(!is_identifier("has space"));
        assert!(!is_identifier("weird!"));
        assert!(!is_identifier("dot.in.middle"));
    }

    #[test]
    fn is_reserved_word_covers_statement_field_and_enum_keywords() {
        for kw in [
            "person",
            "marriage",
            "birth",
            "adoption",
            "name",
            "family",
            "given",
            "born",
            "died",
            "gender",
            "start",
            "end",
            "end_reason",
            "male",
            "female",
            "other",
            "divorce",
        ] {
            assert!(is_reserved_word(kw), "expected `{kw}` to be reserved");
        }
    }

    #[test]
    fn is_reserved_word_rejects_normal_identifiers() {
        for name in ["alice", "bob_42", "kul", "Person", "BIRTH"] {
            assert!(
                !is_reserved_word(name),
                "expected `{name}` to NOT be reserved"
            );
        }
    }
}
