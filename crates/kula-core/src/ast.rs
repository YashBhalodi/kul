//! Typed AST for Kula documents.
//!
//! The AST grows additively across Phase 2: each new statement or field
//! variant lands as the corresponding rule slice does. References are stored
//! as raw [`Ident`]s here; resolution happens in [`crate::semantic`].

use crate::date::DateLit;
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

/// A `person <id> <field>...` statement, plus any indented sub-statements.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PersonStmt {
    pub span: ByteSpan,
    pub keyword_span: ByteSpan,
    pub id: Ident,
    pub fields: Vec<PersonField>,
    /// At most one biological-birth sub-statement per spec section 5.1.
    pub birth: Option<BirthSub>,
    pub adoptions: Vec<AdoptionSub>,
}

/// `birth <marriage-ref>` sub-statement.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BirthSub {
    pub span: ByteSpan,
    pub keyword_span: ByteSpan,
    pub marriage_ref: Ident,
}

/// `adoption <marriage-ref> start:<date> [end:<date>]` sub-statement.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AdoptionSub {
    pub span: ByteSpan,
    pub keyword_span: ByteSpan,
    pub marriage_ref: Ident,
    pub fields: Vec<AdoptionField>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AdoptionField {
    pub span: ByteSpan,
    pub name_span: ByteSpan,
    pub kind: AdoptionFieldKind,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AdoptionFieldKind {
    Start(DateLit),
    End(DateLit),
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
    Family(StringValue),
    Given(StringValue),
    Born(DateLit),
    Died(DateLit),
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
    Start(DateLit),
    End(DateLit),
    EndReason(EndReasonValue),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum EndReason {
    Divorce,
    /// A value that is not in the v1 vocabulary; surfaced by the validator
    /// as KULA-R05b. Stored verbatim so the diagnostic can quote it.
    Unknown(String),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EndReasonValue {
    pub value: EndReason,
    pub span: ByteSpan,
}
