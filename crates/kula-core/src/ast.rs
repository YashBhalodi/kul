//! Typed AST for Kula documents.
//!
//! The AST grows additively across Phase 2: each new statement or field
//! variant lands as the corresponding rule slice does. References are stored
//! as raw [`Ident`]s here; resolution happens in [`crate::semantic`].

use crate::span::ByteSpan;

/// A `.kula` document: an optional version declaration plus a sequence of
/// top-level statements.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Document {
    pub version: Option<VersionDecl>,
    pub statements: Vec<Statement>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VersionDecl {
    pub span: ByteSpan,
    /// The raw version literal, e.g. `0.1`.
    pub version: String,
    pub version_span: ByteSpan,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Statement {
    Person(PersonStmt),
    Marriage(MarriageStmt),
}

impl Statement {
    pub fn id(&self) -> &Ident {
        match self {
            Statement::Person(p) => &p.id,
            Statement::Marriage(m) => &m.id,
        }
    }

    pub fn kind_name(&self) -> &'static str {
        match self {
            Statement::Person(_) => "person",
            Statement::Marriage(_) => "marriage",
        }
    }
}

/// A `person <id> <field>...` statement.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PersonStmt {
    pub span: ByteSpan,
    pub keyword_span: ByteSpan,
    pub id: Ident,
    pub fields: Vec<PersonField>,
}

/// An identifier as written in source — name plus the span of the token.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Ident {
    pub name: String,
    pub span: ByteSpan,
}

/// A field on a `person` statement.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PersonField {
    pub span: ByteSpan,
    pub name_span: ByteSpan,
    pub kind: PersonFieldKind,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PersonFieldKind {
    Name(StringValue),
    Gender(GenderValue),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StringValue {
    pub value: String,
    pub span: ByteSpan,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Gender {
    Male,
    Female,
    Other,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GenderValue {
    pub value: Gender,
    pub span: ByteSpan,
}

/// A `marriage <id> <spouse-a> <spouse-b> <field>...` statement.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MarriageStmt {
    pub span: ByteSpan,
    pub keyword_span: ByteSpan,
    pub id: Ident,
    pub spouse_a: Ident,
    pub spouse_b: Ident,
    pub fields: Vec<MarriageField>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MarriageField {
    pub span: ByteSpan,
    pub name_span: ByteSpan,
    pub kind: MarriageFieldKind,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MarriageFieldKind {
    Start(DatePlaceholder),
    End(DatePlaceholder),
    EndReason(EndReasonValue),
}

/// Placeholder date-literal AST until full date support lands in slice #10.
/// Carries the raw source text exactly as written (e.g. `1972-05-12`,
/// `~1925`, `1925-09`) plus the span.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DatePlaceholder {
    pub raw: String,
    pub span: ByteSpan,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EndReason {
    Divorce,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EndReasonValue {
    pub value: EndReason,
    pub span: ByteSpan,
}
