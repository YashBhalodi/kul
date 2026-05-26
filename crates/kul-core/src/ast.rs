//! Typed AST for Kul documents.
//!
//! The AST grows additively (see the additivity principle in `CONTEXT.md`):
//! new statement or field variants land alongside the corresponding rule
//! slice; existing variants are never reordered, renamed, or removed.
//! References are stored as raw [`Ident`]s here; resolution happens in
//! [`crate::semantic`].
//!
//! # File identity
//!
//! A [`KulFile`] is one parsed `.kul` source. A [`Document`] is the
//! multi-file container the toolchain operates on: zero or more `KulFile`s
//! plus the project manifest, each addressable by a [`crate::span::FileId`].
//! AST nodes carry bare [`ByteSpan`]s because their owning [`KulFile`]
//! provides file context implicitly; cross-cutting consumers (diagnostics,
//! the resolved id index) carry [`crate::span::FileSpan`]s instead.

use std::sync::Arc;

use crate::date::DateLit;
use crate::lexer::FieldName;
use crate::span::{ByteSpan, FileId};

/// A parsed `.kul` source file.
///
/// One file's worth of top-level statements plus its raw source text and
/// canonical name. The `name` is the same opaque label the consumer passed
/// in at the toolchain edge: a path-string for the CLI, a URI-string for
/// the LSP, whatever the JS host chose for WASM. `kul-core` does not
/// interpret it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct KulFile {
    pub name: String,
    pub source: String,
    pub statements: Vec<Statement>,
}

impl KulFile {
    /// Build a [`KulFile`] from already-parsed statements. Convenience
    /// for callers (the pipeline in [`crate::check`], in-memory test
    /// fixtures) that have lex/parsed a source and want a `KulFile`
    /// without spelling out the three fields by name each time.
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

/// One input file at the toolchain edge — a name (path / URI / opaque
/// label) plus the raw source bytes. Public input shape for
/// [`crate::check`]; internally each `InputFile` becomes a [`KulFile`]
/// after lex/parse and is stored at a fresh [`FileId`] in the resulting
/// [`Document`].
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
/// The manifest always lives at [`FileId::MANIFEST`] (= `FileId(0)`); the
/// `.kul` files follow at `FileId(1..)` in input order. AST nodes inside
/// any [`KulFile`] keep bare [`ByteSpan`]s; project-wide consumers
/// (diagnostics, the resolved id index, kinship queries) reach for
/// [`crate::span::FileSpan`] instead.
///
/// At v1 the toolchain only ever constructs N=1 `kul_files`; the multi-
/// file shape exists so subsequent issues (cross-`.kul`-file resolution,
/// document merging) can build on file-aware spans without further
/// breaking changes. Each `KulFile` is held behind an [`Arc`] so callers
/// can keep cheap shared handles (the LSP document cache, downstream
/// tooling) without copying source bytes.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Document {
    /// The manifest's name (typically the path to `kul.yml`). The manifest
    /// is the project's identity even when its body failed to parse —
    /// keeping the name on the [`Document`] lets diagnostics anchor at
    /// `FileId::MANIFEST` independently of whether the YAML was usable.
    pub manifest_name: String,
    /// The raw `kul.yml` source bytes the manifest validator pass walked.
    /// Empty when the manifest was missing on disk; the [`KUL-M01`] code
    /// (`crate::diagnostic::manifest_codes`) covers that case.
    pub manifest_source: String,
    /// Parsed `.kul` files in input order. `kul_files[i]` lives at
    /// `FileId(i + 1)` — the `+1` accounts for the manifest occupying
    /// `FileId::MANIFEST`.
    pub kul_files: Vec<Arc<KulFile>>,
}

impl Document {
    /// Build a [`Document`] without manifest source bytes. The common
    /// shape for in-memory fixtures and test scaffolding that don't
    /// exercise manifest-anchored diagnostic rendering. Production
    /// callers that loaded `kul.yml` from disk should use
    /// [`Document::with_manifest_source`] so manifest-side spans render
    /// against the real bytes.
    #[must_use]
    pub fn new(manifest_name: impl Into<String>, kul_files: Vec<Arc<KulFile>>) -> Self {
        Self::with_manifest_source(manifest_name, String::new(), kul_files)
    }

    /// Build a [`Document`] with explicit `kul.yml` source bytes. Used
    /// by the pipeline entry points so any diagnostic anchored at
    /// [`FileId::MANIFEST`] has bytes to render against.
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

    /// Resolve a [`FileId`] to the source bytes it indexes into. Returns
    /// `None` if the id is out of range.
    #[must_use]
    pub fn source_of(&self, file: FileId) -> Option<&str> {
        if file == FileId::MANIFEST {
            return Some(self.manifest_source.as_str());
        }
        self.kul_file(file).map(|k| k.source.as_str())
    }

    /// Resolve a [`FileId`] to the canonical name it indexes into. Same
    /// out-of-range semantics as [`Document::source_of`].
    #[must_use]
    pub fn name_of(&self, file: FileId) -> Option<&str> {
        if file == FileId::MANIFEST {
            return Some(self.manifest_name.as_str());
        }
        self.kul_file(file).map(|k| k.name.as_str())
    }

    /// Resolve a [`FileId`] to a `.kul` [`KulFile`], or `None` if the id
    /// is the manifest, out of range, or otherwise not a `.kul` file.
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

    /// The [`FileId`]s of every `.kul` file, in input order. Excludes the
    /// manifest. Useful for cross-file iteration in the resolver and the
    /// validator.
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
    #[must_use]
    pub fn name(&self) -> Option<&StringValue> {
        self.fields.iter().find_map(|f| match &f.kind {
            PersonFieldKind::Name(v) => Some(v),
            _ => None,
        })
    }

    /// The person's `name:` value if present, otherwise their id. The
    /// canonical short label for tooling (LSP hover, completion details,
    /// document-symbol headers).
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

    /// First `died:` date, or `None` (absence means alive per spec section 4.2).
    #[must_use]
    pub fn died(&self) -> Option<&DateLit> {
        self.fields.iter().find_map(|f| match &f.kind {
            PersonFieldKind::Died(d) => Some(d),
            _ => None,
        })
    }

    /// First `gender:` field, or `None` (rule 3 fires when absent).
    #[must_use]
    pub fn gender(&self) -> Option<&GenderValue> {
        self.fields.iter().find_map(|f| match &f.kind {
            PersonFieldKind::Gender(v) => Some(v),
            _ => None,
        })
    }

    /// Iterator over the `FieldName` of every parsed field on this person,
    /// in source order. Excludes `malformed_fields` (which the parser
    /// records separately for R03's "writer attempted this field" check).
    pub fn declared_field_names(&self) -> impl Iterator<Item = FieldName> + '_ {
        self.fields.iter().map(|f| f.kind.field_name())
    }

    /// True if `name` was either parsed cleanly or attempted-but-malformed.
    /// Used by R03 to avoid emitting "missing required field" when the
    /// writer clearly typed the field but got the value wrong (e.g.
    /// forgot quotes around a `name:` value).
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
    /// First `start:` date, or `None` (rule 3 fires when absent).
    #[must_use]
    pub fn start(&self) -> Option<&DateLit> {
        self.fields.iter().find_map(|f| match &f.kind {
            AdoptionFieldKind::Start(d) => Some(d),
            _ => None,
        })
    }

    /// First `end:` date, or `None` (an open-ended adoption per spec 5.2).
    #[must_use]
    pub fn end(&self) -> Option<&DateLit> {
        self.fields.iter().find_map(|f| match &f.kind {
            AdoptionFieldKind::End(d) => Some(d),
            _ => None,
        })
    }

    /// Iterator over the `FieldName` of every parsed field on this
    /// adoption, in source order.
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
    /// First `start:` date, or `None` (rule 3 fires when absent).
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

    /// First `end_reason:` field, or `None`. The validator's rule 5 requires
    /// `end` and `end_reason` to be both present or both absent.
    #[must_use]
    pub fn end_reason(&self) -> Option<&EndReasonValue> {
        self.fields.iter().find_map(|f| match &f.kind {
            MarriageFieldKind::EndReason(v) => Some(v),
            _ => None,
        })
    }

    /// Iterator over the `FieldName` of every parsed field on this
    /// marriage, in source order.
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
    /// A value that is not in the v1 vocabulary; surfaced by the validator
    /// as KUL-R05b. Stored verbatim so the diagnostic can quote it.
    Unknown(String),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EndReasonValue {
    pub value: EndReason,
    pub span: ByteSpan,
}
