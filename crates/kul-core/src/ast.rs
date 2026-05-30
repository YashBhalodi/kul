//! Typed AST for Kul documents.
//!
//! AST nodes carry bare [`ByteSpan`]s; the owning [`KulFile`] supplies file
//! context. Cross-cutting consumers (diagnostics, resolved id index) carry
//! [`crate::span::FileSpan`]s instead. References are stored as raw
//! [`Ident`]s here; resolution happens in [`crate::semantic`].

use std::sync::Arc;

use crate::date::DateLit;
use crate::lexer::FieldName;
use crate::span::{ByteSpan, FileId};

/// A parsed `.kul` source file.
///
/// `name` is the opaque label the consumer passed in at the toolchain edge
/// (path / URI / JS host label). `kul-core` does not interpret it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct KulFile {
    pub name: String,
    pub source: String,
    pub statements: Vec<Statement>,
}

impl KulFile {
    /// Build a [`KulFile`] from already-parsed statements.
    #[must_use]
    pub fn new(
        name: impl Into<String>,
        source: impl Into<String>,
        statements: Vec<Statement>,
    ) -> Self {
        Self {
            name: name.into(),
            source: source.into(),
            statements,
        }
    }
}

/// One input file at the toolchain edge — name plus raw source bytes.
/// Public input shape for [`crate::check`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InputFile {
    pub name: String,
    pub source: String,
}

impl InputFile {
    #[must_use]
    pub fn new(name: impl Into<String>, source: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            source: source.into(),
        }
    }
}

/// One parsed Kul *project*: the manifest plus zero or more `.kul` files.
///
/// The manifest lives at [`FileId::MANIFEST`] (= `FileId(0)`); `.kul` files
/// follow at `FileId(1..)` in input order. Each `KulFile` is held behind an
/// [`Arc`] so callers can share cheap handles without copying source bytes.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Document {
    /// The manifest's name (typically the path to `kul.yml`). Kept on the
    /// [`Document`] so diagnostics can anchor at `FileId::MANIFEST` even
    /// when the YAML body failed to parse.
    pub manifest_name: String,
    /// Raw `kul.yml` source bytes. Empty when the manifest was missing on
    /// disk (KUL-M01 covers that case).
    pub manifest_source: String,
    /// Parsed `.kul` files in input order. `kul_files[i]` lives at
    /// `FileId(i + 1)`.
    pub kul_files: Vec<Arc<KulFile>>,
}

impl Document {
    /// Build a [`Document`] without manifest source bytes. For in-memory
    /// fixtures; production callers that loaded `kul.yml` should use
    /// [`Document::with_manifest_source`] so manifest-anchored diagnostics
    /// have bytes to render against.
    #[must_use]
    pub fn new(manifest_name: impl Into<String>, kul_files: Vec<Arc<KulFile>>) -> Self {
        Self::with_manifest_source(manifest_name, String::new(), kul_files)
    }

    /// Build a [`Document`] with explicit `kul.yml` source bytes.
    #[must_use]
    pub fn with_manifest_source(
        manifest_name: impl Into<String>,
        manifest_source: impl Into<String>,
        kul_files: Vec<Arc<KulFile>>,
    ) -> Self {
        Self {
            manifest_name: manifest_name.into(),
            manifest_source: manifest_source.into(),
            kul_files,
        }
    }

    /// Resolve a [`FileId`] to its source bytes, or `None` if out of range.
    #[must_use]
    pub fn source_of(&self, file: FileId) -> Option<&str> {
        if file == FileId::MANIFEST {
            return Some(self.manifest_source.as_str());
        }
        self.kul_file(file).map(|k| k.source.as_str())
    }

    /// Resolve a [`FileId`] to its canonical name, or `None` if out of range.
    #[must_use]
    pub fn name_of(&self, file: FileId) -> Option<&str> {
        if file == FileId::MANIFEST {
            return Some(self.manifest_name.as_str());
        }
        self.kul_file(file).map(|k| k.name.as_str())
    }

    /// Resolve a [`FileId`] to a `.kul` [`KulFile`], or `None` if the id
    /// is the manifest or out of range.
    #[must_use]
    pub fn kul_file(&self, file: FileId) -> Option<&KulFile> {
        let idx = file.as_u32().checked_sub(1)? as usize;
        self.kul_files.get(idx).map(|a| a.as_ref())
    }

    /// Iterate every `.kul` file with its [`FileId`], in input order.
    pub fn kul_files(&self) -> impl Iterator<Item = (FileId, &KulFile)> + '_ {
        self.kul_files
            .iter()
            .enumerate()
            .map(|(i, k)| (FileId((i + 1) as u32), k.as_ref()))
    }

    /// The [`FileId`]s of every `.kul` file, in input order (excludes the
    /// manifest).
    pub fn kul_file_ids(&self) -> impl Iterator<Item = FileId> + '_ {
        (0..self.kul_files.len()).map(|i| FileId((i + 1) as u32))
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Statement {
    Person(PersonStmt),
    Marriage(MarriageStmt),
}

impl Statement {
    #[must_use]
    pub fn id(&self) -> &Ident {
        match self {
            Statement::Person(p) => &p.id,
            Statement::Marriage(m) => &m.id,
        }
    }

    #[must_use]
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
    /// At most one biological-birth sub-statement (spec 5.1).
    pub birth: Option<BirthSub>,
    pub adoptions: Vec<AdoptionSub>,
    /// Field-name keywords whose value failed to parse. Recorded so the
    /// validator skips R03 ("missing required field") on top of the parse
    /// error.
    pub malformed_fields: Vec<FieldName>,
}

impl PersonStmt {
    /// First `name:` field, or `None` (R03 fires when absent).
    #[must_use]
    pub fn name(&self) -> Option<&StringValue> {
        self.fields.iter().find_map(|f| match &f.kind {
            PersonFieldKind::Name(v) => Some(v),
            _ => None,
        })
    }

    /// The person's `name:` value if present, otherwise their id. The
    /// canonical short label for tooling.
    #[must_use]
    pub fn display_name(&self) -> &str {
        self.name()
            .map(|n| n.value.as_str())
            .unwrap_or(self.id.name.as_str())
    }

    /// First `family:` field, or `None`.
    #[must_use]
    pub fn family(&self) -> Option<&StringValue> {
        self.fields.iter().find_map(|f| match &f.kind {
            PersonFieldKind::Family(v) => Some(v),
            _ => None,
        })
    }

    /// First `given:` field, or `None`.
    #[must_use]
    pub fn given(&self) -> Option<&StringValue> {
        self.fields.iter().find_map(|f| match &f.kind {
            PersonFieldKind::Given(v) => Some(v),
            _ => None,
        })
    }

    /// First `born:` date, or `None`.
    #[must_use]
    pub fn born(&self) -> Option<&DateLit> {
        self.fields.iter().find_map(|f| match &f.kind {
            PersonFieldKind::Born(d) => Some(d),
            _ => None,
        })
    }

    /// First `died:` date, or `None` (absence means alive, spec 4.2).
    #[must_use]
    pub fn died(&self) -> Option<&DateLit> {
        self.fields.iter().find_map(|f| match &f.kind {
            PersonFieldKind::Died(d) => Some(d),
            _ => None,
        })
    }

    /// First `gender:` field, or `None` (R03 fires when absent).
    #[must_use]
    pub fn gender(&self) -> Option<&GenderValue> {
        self.fields.iter().find_map(|f| match &f.kind {
            PersonFieldKind::Gender(v) => Some(v),
            _ => None,
        })
    }

    /// `FieldName` of every parsed field, in source order. Excludes
    /// `malformed_fields`.
    pub fn declared_field_names(&self) -> impl Iterator<Item = FieldName> + '_ {
        self.fields.iter().map(|f| f.kind.field_name())
    }

    /// True if `name` was either parsed cleanly or attempted-but-malformed.
    /// Used by R03 so a parse-error field doesn't also trigger "missing".
    #[must_use]
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
    /// First `start:` date, or `None` (R03 fires when absent).
    #[must_use]
    pub fn start(&self) -> Option<&DateLit> {
        self.fields.iter().find_map(|f| match &f.kind {
            AdoptionFieldKind::Start(d) => Some(d),
            _ => None,
        })
    }

    /// First `end:` date, or `None` (open-ended adoption, spec 5.2).
    #[must_use]
    pub fn end(&self) -> Option<&DateLit> {
        self.fields.iter().find_map(|f| match &f.kind {
            AdoptionFieldKind::End(d) => Some(d),
            _ => None,
        })
    }

    /// `FieldName` of every parsed field, in source order.
    pub fn declared_field_names(&self) -> impl Iterator<Item = FieldName> + '_ {
        self.fields.iter().map(|f| f.kind.field_name())
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
    #[must_use]
    pub fn field_name(&self) -> FieldName {
        match self {
            AdoptionFieldKind::Start(_) => FieldName::Start,
            AdoptionFieldKind::End(_) => FieldName::End,
        }
    }

    #[must_use]
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
    #[must_use]
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

    #[must_use]
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
    /// First `start:` date, or `None` (R03 fires when absent).
    #[must_use]
    pub fn start(&self) -> Option<&DateLit> {
        self.fields.iter().find_map(|f| match &f.kind {
            MarriageFieldKind::Start(d) => Some(d),
            _ => None,
        })
    }

    /// First `end:` date, or `None` (an ongoing marriage).
    #[must_use]
    pub fn end(&self) -> Option<&DateLit> {
        self.fields.iter().find_map(|f| match &f.kind {
            MarriageFieldKind::End(d) => Some(d),
            _ => None,
        })
    }

    /// First `end_reason:` field, or `None`. R05 requires `end` and
    /// `end_reason` to be both present or both absent.
    #[must_use]
    pub fn end_reason(&self) -> Option<&EndReasonValue> {
        self.fields.iter().find_map(|f| match &f.kind {
            MarriageFieldKind::EndReason(v) => Some(v),
            _ => None,
        })
    }

    /// `FieldName` of every parsed field, in source order.
    pub fn declared_field_names(&self) -> impl Iterator<Item = FieldName> + '_ {
        self.fields.iter().map(|f| f.kind.field_name())
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
    #[must_use]
    pub fn field_name(&self) -> FieldName {
        match self {
            MarriageFieldKind::Start(_) => FieldName::Start,
            MarriageFieldKind::End(_) => FieldName::End,
            MarriageFieldKind::EndReason(_) => FieldName::EndReason,
        }
    }

    #[must_use]
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
    /// A value not in the v1 vocabulary; surfaced as KUL-R05b. Stored
    /// verbatim so the diagnostic can quote it.
    Unknown(String),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EndReasonValue {
    pub value: EndReason,
    pub span: ByteSpan,
}
