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
