//! Typed AST for Kul documents.
//!
//! The AST grows additively (see the additivity principle in `CONTEXT.md`):
//! new statement or field variants land alongside the corresponding rule
//! slice; existing variants are never reordered, renamed, or removed.
//! References are stored as raw [`Ident`]s here; resolution happens in
//! [`crate::semantic`].

use crate::date::DateLit;
use crate::lexer::FieldName;
use crate::span::ByteSpan;

/// A `.kul` document: a sequence of top-level statements. The Kul language
/// version is project-level metadata (see [`crate::manifest::Manifest`]),
/// not part of the document itself.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Document {
    pub statements: Vec<Statement>,
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
    /// Field-name keywords whose value the parser couldn't parse (e.g.
    /// unquoted `name:Alice`). The malformed value is reported as a parse
    /// diagnostic; recording the name here lets the validator skip the
    /// "missing required field" check that would otherwise pile a second,
    /// misleading error on top of the first.
    pub malformed_fields: Vec<FieldName>,
}

impl PersonStmt {
    /// First `name:` field as written, or `None` if absent (rule 3 fires).
    pub fn name(&self) -> Option<&StringValue> {
        self.fields.iter().find_map(|f| match &f.kind {
            PersonFieldKind::Name(v) => Some(v),
            _ => None,
        })
    }

    /// The person's `name:` value if present, otherwise their id. The
    /// canonical short label for tooling (LSP hover, completion details,
    /// document-symbol headers).
    pub fn display_name(&self) -> &str {
        self.name()
            .map(|n| n.value.as_str())
            .unwrap_or(self.id.name.as_str())
    }

    /// First `family:` field, or `None`.
    pub fn family(&self) -> Option<&StringValue> {
        self.fields.iter().find_map(|f| match &f.kind {
            PersonFieldKind::Family(v) => Some(v),
            _ => None,
        })
    }

    /// First `given:` field, or `None`.
    pub fn given(&self) -> Option<&StringValue> {
        self.fields.iter().find_map(|f| match &f.kind {
            PersonFieldKind::Given(v) => Some(v),
            _ => None,
        })
    }

    /// First `born:` date, or `None`.
    pub fn born(&self) -> Option<&DateLit> {
        self.fields.iter().find_map(|f| match &f.kind {
            PersonFieldKind::Born(d) => Some(d),
            _ => None,
        })
    }

    /// First `died:` date, or `None` (absence means alive per spec section 4.2).
    pub fn died(&self) -> Option<&DateLit> {
        self.fields.iter().find_map(|f| match &f.kind {
            PersonFieldKind::Died(d) => Some(d),
            _ => None,
        })
    }

    /// First `gender:` field, or `None` (rule 3 fires when absent).
    pub fn gender(&self) -> Option<&GenderValue> {
        self.fields.iter().find_map(|f| match &f.kind {
            PersonFieldKind::Gender(v) => Some(v),
            _ => None,
        })
    }

    /// True if `name` was either parsed cleanly or attempted-but-malformed.
    /// Used by R03 to avoid emitting "missing required field" when the
    /// writer clearly typed the field but got the value wrong (e.g.
    /// forgot quotes around a `name:` value).
    pub fn has_field(&self, name: FieldName) -> bool {
        if self.malformed_fields.contains(&name) {
            return true;
        }
        self.fields.iter().any(|f| {
            matches!(
                (&f.kind, name),
                (PersonFieldKind::Name(_), FieldName::Name)
                    | (PersonFieldKind::Family(_), FieldName::Family)
                    | (PersonFieldKind::Given(_), FieldName::Given)
                    | (PersonFieldKind::Born(_), FieldName::Born)
                    | (PersonFieldKind::Died(_), FieldName::Died)
                    | (PersonFieldKind::Gender(_), FieldName::Gender)
            )
        })
    }
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

impl AdoptionSub {
    /// First `start:` date, or `None` (rule 3 fires when absent).
    pub fn start(&self) -> Option<&DateLit> {
        self.fields.iter().find_map(|f| match &f.kind {
            AdoptionFieldKind::Start(d) => Some(d),
            _ => None,
        })
    }

    /// First `end:` date, or `None` (an open-ended adoption per spec 5.2).
    pub fn end(&self) -> Option<&DateLit> {
        self.fields.iter().find_map(|f| match &f.kind {
            AdoptionFieldKind::End(d) => Some(d),
            _ => None,
        })
    }
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

impl AdoptionFieldKind {
    pub fn field_name(&self) -> FieldName {
        match self {
            AdoptionFieldKind::Start(_) => FieldName::Start,
            AdoptionFieldKind::End(_) => FieldName::End,
        }
    }

    pub fn value_span(&self) -> ByteSpan {
        match self {
            AdoptionFieldKind::Start(d) | AdoptionFieldKind::End(d) => d.span,
        }
    }
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

impl PersonFieldKind {
    pub fn field_name(&self) -> FieldName {
        match self {
            PersonFieldKind::Name(_) => FieldName::Name,
            PersonFieldKind::Family(_) => FieldName::Family,
            PersonFieldKind::Given(_) => FieldName::Given,
            PersonFieldKind::Born(_) => FieldName::Born,
            PersonFieldKind::Died(_) => FieldName::Died,
            PersonFieldKind::Gender(_) => FieldName::Gender,
        }
    }

    pub fn value_span(&self) -> ByteSpan {
        match self {
            PersonFieldKind::Name(s) | PersonFieldKind::Family(s) | PersonFieldKind::Given(s) => {
                s.span
            }
            PersonFieldKind::Born(d) | PersonFieldKind::Died(d) => d.span,
            PersonFieldKind::Gender(g) => g.span,
        }
    }
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

impl MarriageStmt {
    /// First `start:` date, or `None` (rule 3 fires when absent).
    pub fn start(&self) -> Option<&DateLit> {
        self.fields.iter().find_map(|f| match &f.kind {
            MarriageFieldKind::Start(d) => Some(d),
            _ => None,
        })
    }

    /// First `end:` date, or `None` (an ongoing marriage).
    pub fn end(&self) -> Option<&DateLit> {
        self.fields.iter().find_map(|f| match &f.kind {
            MarriageFieldKind::End(d) => Some(d),
            _ => None,
        })
    }

    /// First `end_reason:` field, or `None`. The validator's rule 5 requires
    /// `end` and `end_reason` to be both present or both absent.
    pub fn end_reason(&self) -> Option<&EndReasonValue> {
        self.fields.iter().find_map(|f| match &f.kind {
            MarriageFieldKind::EndReason(v) => Some(v),
            _ => None,
        })
    }
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

impl MarriageFieldKind {
    pub fn field_name(&self) -> FieldName {
        match self {
            MarriageFieldKind::Start(_) => FieldName::Start,
            MarriageFieldKind::End(_) => FieldName::End,
            MarriageFieldKind::EndReason(_) => FieldName::EndReason,
        }
    }

    pub fn value_span(&self) -> ByteSpan {
        match self {
            MarriageFieldKind::Start(d) | MarriageFieldKind::End(d) => d.span,
            MarriageFieldKind::EndReason(r) => r.span,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum EndReason {
    Divorce,
    /// A value that is not in the v1 vocabulary; surfaced by the validator
    /// as KUL-R05b. Stored verbatim so the diagnostic can quote it.
    Unknown(String),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EndReasonValue {
    pub value: EndReason,
    pub span: ByteSpan,
}
