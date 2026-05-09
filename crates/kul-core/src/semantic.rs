//! Semantic analysis: turns a multi-file [`Document`] into a
//! [`ResolvedDocument`].
//!
//! [`ResolvedDocument`] is the deep query module the validator and the
//! cycle-detector talk to. It owns the [`Document`] (via `Arc<Document>`)
//! and a per-file id index, and exposes typed query methods (`persons`,
//! `marriages`, `spouses_of`, `parents_of`, `person`, `marriage`); callers
//! never touch the underlying maps. New questions about kinship belong
//! here, not at every rule's call site.
//!
//! # File identity (per ADR-0014)
//!
//! Per-id queries (`person(file, id)`, `marriage(file, id)`,
//! `entity(file, id)`) take a [`FileId`] alongside the id name because v1
//! resolves *per-file* — there is no global namespace across `.kul`
//! files. The same id may be declared in two different files without
//! conflict; R01 (duplicate id) fires only within a single file. Iteration
//! queries (`persons`, `marriages`, `statements`) walk every file; the
//! `_in(file)` variants restrict to one file (the LSP uses these to
//! enumerate symbols inside the active document, the cycle-detector to
//! confine analysis to one file at a time).
//!
//! The `Arc<Document>` shape is what lets the LSP cache a single resolved
//! view per project — the borrowed-lifetime alternative was self-
//! referential and forced a re-resolve on every editor request. See
//! [ADR-0007](../../docs/adr/0007-resolved-document-owns-document.md).

use std::collections::HashMap;
use std::sync::Arc;

use crate::ast::{Document, Ident, KulFile, MarriageStmt, PersonStmt, Statement};
use crate::diagnostic::{Diagnostic, fspan};
use crate::span::{ByteSpan, FileId, FileSpan};

/// What kind of top-level entity an ID names.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EntityKind {
    Person,
    Marriage,
}

impl EntityKind {
    pub fn as_str(self) -> &'static str {
        match self {
            EntityKind::Person => "person",
            EntityKind::Marriage => "marriage",
        }
    }
}

/// One entry in the per-file ID index, returned by
/// [`ResolvedDocument::entity`].
///
/// Built on demand from the underlying statement when the caller asks; the
/// `id` borrow is tied to `&self` of the resolved document. The
/// declaration's `FileSpan` is exposed so consumers (the LSP feature
/// modules, ID-rename tooling) can render the location without re-walking
/// the AST to find it.
#[derive(Debug, Clone, Copy)]
pub struct EntityRef<'a> {
    pub kind: EntityKind,
    pub id: &'a Ident,
    /// File the declaration lives in. Combine with `id.span` to build a
    /// [`FileSpan`] anchor for diagnostics or LSP responses.
    pub file: FileId,
}

impl EntityRef<'_> {
    /// The declaration's id span as a project-wide [`FileSpan`].
    pub fn span(&self) -> FileSpan {
        fspan(self.file, self.id.span)
    }
}

/// Stored entry in the resolved id index — `kind`, the id's owning file,
/// and the position of the corresponding statement in the file's
/// `statements`. Storing an index rather than a borrow is what lets
/// `ResolvedDocument` own its `Document` instead of borrowing into it;
/// query methods rebuild the borrowed [`EntityRef`] view on demand.
#[derive(Debug, Clone, Copy)]
struct ResolvedEntity {
    kind: EntityKind,
    statement_idx: usize,
}

/// A multi-file Kul project with semantic information attached.
///
/// Built by [`resolve`]; consumed by the validator and the cycle-detector.
/// All cross-reference and kinship queries go through methods on this
/// type — callers do not enumerate the underlying maps. Owns its
/// [`Document`] via `Arc<Document>`, so it is cheap to clone (refcount
/// bump) and can be cached alongside other artifacts (the LSP document
/// cache holds one per open URI).
#[derive(Debug, Clone)]
pub struct ResolvedDocument {
    document: Arc<Document>,
    /// Per-file first-seen entity indexes, keyed by file. Populated only
    /// for `.kul` file ids (the manifest never declares persons /
    /// marriages). Lookups for the manifest's [`FileId`] return `None` by
    /// virtue of the hash-map miss.
    entities: HashMap<FileId, HashMap<String, ResolvedEntity>>,
}

/// One parent link: a directed edge `child → parent` with the source span
/// where the link is documented (the child's `birth` or `adoption`
/// marriage-ref).
#[derive(Debug, Clone)]
pub struct ParentLink<'a> {
    pub parent: &'a PersonStmt,
    pub link_span: ByteSpan,
    pub kind: ParentLinkKind,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ParentLinkKind {
    Bio,
    Adoption,
}

impl ResolvedDocument {
    /// The underlying parsed multi-file [`Document`]. Useful for downstream
    /// consumers that need access to source bytes or per-file names; rules
    /// inside this crate go through the typed queries below instead.
    pub fn document(&self) -> &Document {
        &self.document
    }

    /// A clone of the shared [`Arc<Document>`] backing this resolved view —
    /// a refcount bump, no allocation. Useful for callers that want to hold
    /// onto the document alongside the resolved view (the LSP document
    /// cache, downstream tooling) without copying source bytes.
    pub fn document_arc(&self) -> Arc<Document> {
        Arc::clone(&self.document)
    }

    /// Walk every top-level statement in every `.kul` file in source
    /// order, file-by-file.
    ///
    /// Use this when a caller wants to dispatch on the typed `Statement`
    /// enum across the whole project (the export builder, the validator's
    /// duplicate-id detection). Use [`Self::statements_in`] for a
    /// single-file iterator.
    pub fn statements(&self) -> impl Iterator<Item = &Statement> + '_ {
        self.document
            .kul_files
            .iter()
            .flat_map(|f| f.statements.iter())
    }

    /// Walk every top-level statement in `file`, in source order.
    /// Empty when `file` is the manifest or out of range.
    pub fn statements_in(&self, file: FileId) -> impl Iterator<Item = &Statement> + '_ {
        self.document
            .kul_file(file)
            .into_iter()
            .flat_map(|f| f.statements.iter())
    }

    /// Walk every `person` statement across the project in source order.
    pub fn persons(&self) -> impl Iterator<Item = &PersonStmt> + '_ {
        self.statements().filter_map(|s| match s {
            Statement::Person(p) => Some(p),
            _ => None,
        })
    }

    /// Walk every `person` statement in `file`, in source order.
    pub fn persons_in(&self, file: FileId) -> impl Iterator<Item = &PersonStmt> + '_ {
        self.statements_in(file).filter_map(|s| match s {
            Statement::Person(p) => Some(p),
            _ => None,
        })
    }

    /// Walk every `marriage` statement across the project in source order.
    pub fn marriages(&self) -> impl Iterator<Item = &MarriageStmt> + '_ {
        self.statements().filter_map(|s| match s {
            Statement::Marriage(m) => Some(m),
            _ => None,
        })
    }

    /// Walk every `marriage` statement in `file`, in source order.
    pub fn marriages_in(&self, file: FileId) -> impl Iterator<Item = &MarriageStmt> + '_ {
        self.statements_in(file).filter_map(|s| match s {
            Statement::Marriage(m) => Some(m),
            _ => None,
        })
    }

    /// Look up a person by id within `file`. Returns `None` if no entity
    /// has this id in the file, or if one does but it's a marriage. Per
    /// ADR-0014 v1 has no cross-file namespace — callers needing to scan
    /// every file enumerate `kul_file_ids()` themselves.
    pub fn person(&self, file: FileId, id: &str) -> Option<&PersonStmt> {
        let entity = self.entity_record(file, id)?;
        if entity.kind != EntityKind::Person {
            return None;
        }
        let kf = self.document.kul_file(file)?;
        match &kf.statements[entity.statement_idx] {
            Statement::Person(p) => Some(p),
            Statement::Marriage(_) => unreachable!(
                "entity index claims kind=Person but statement is a marriage; resolve() invariant broken"
            ),
        }
    }

    /// Look up a marriage by id within `file`. Same `None` rules as
    /// [`Self::person`].
    pub fn marriage(&self, file: FileId, id: &str) -> Option<&MarriageStmt> {
        let entity = self.entity_record(file, id)?;
        if entity.kind != EntityKind::Marriage {
            return None;
        }
        let kf = self.document.kul_file(file)?;
        match &kf.statements[entity.statement_idx] {
            Statement::Marriage(m) => Some(m),
            Statement::Person(_) => unreachable!(
                "entity index claims kind=Marriage but statement is a person; resolve() invariant broken"
            ),
        }
    }

    /// Look up an entity (person or marriage) by id within `file`,
    /// regardless of kind. Used by reference-resolution checks.
    pub fn entity(&self, file: FileId, id: &str) -> Option<EntityRef<'_>> {
        let entity = self.entity_record(file, id)?;
        let kf = self.document.kul_file(file)?;
        let ident = match &kf.statements[entity.statement_idx] {
            Statement::Person(p) => &p.id,
            Statement::Marriage(m) => &m.id,
        };
        Some(EntityRef {
            kind: entity.kind,
            id: ident,
            file,
        })
    }

    fn entity_record(&self, file: FileId, id: &str) -> Option<&ResolvedEntity> {
        self.entities.get(&file)?.get(id)
    }

    /// The top-level statement the cursor is sitting in, or just past,
    /// inside `file`.
    ///
    /// "Sitting in" meaning the latest statement whose `span.start` is
    /// at-or-before `byte_offset`. This is **not** strict span-containment:
    /// a cursor on a brand-new blank line below a `person` declaration is
    /// past the `PersonStmt.span.end`, but this method still returns the
    /// `Person` — callers that care about top-level vs. indented context
    /// (the completion classifier) refine that with the current line's
    /// indent. Returns `None` only when the cursor is before any
    /// statement, or `file` is out of range.
    pub fn statement_at(&self, file: FileId, byte_offset: usize) -> Option<&Statement> {
        let kf = self.document.kul_file(file)?;
        let mut chosen = None;
        for stmt in &kf.statements {
            let span = match stmt {
                Statement::Person(p) => p.span,
                Statement::Marriage(m) => m.span,
            };
            if span.start <= byte_offset {
                chosen = Some(stmt);
            } else {
                break;
            }
        }
        chosen
    }

    /// The two declared spouses of a marriage, in declaration order, with
    /// unresolved spouses skipped (rule 2 reports them).
    ///
    /// Returns at most two persons; an empty iterator if both spouses are
    /// unresolved. Both spouses are resolved within the same file as the
    /// marriage (per ADR-0014's per-file namespace decision).
    pub fn spouses_of<'a>(
        &'a self,
        file: FileId,
        marriage: &'a MarriageStmt,
    ) -> impl Iterator<Item = &'a PersonStmt> + 'a {
        [&marriage.spouse_a, &marriage.spouse_b]
            .into_iter()
            .filter_map(move |ident| self.person(file, &ident.name))
    }

    /// Every reference site for `id` in `file`, in source order.
    ///
    /// `kind` selects which positions count: spouse positions for a
    /// person, `birth`/`adoption` marriage refs for a marriage. The
    /// declaration site is **not** included — callers that want it (per
    /// LSP's `includeDeclaration` flag) prepend it themselves.
    ///
    /// Unresolved references whose name happens to match `id` are still
    /// returned. This means rename and find-references both work on
    /// partly- broken documents (the user is mid-edit), and rule 2's
    /// "no person/marriage with this id" diagnostic is the right place
    /// to surface the missing declaration — not here.
    pub fn references_to(&self, file: FileId, id: &str, kind: EntityKind) -> Vec<ByteSpan> {
        let mut out = Vec::new();
        match kind {
            EntityKind::Person => {
                for m in self.marriages_in(file) {
                    if m.spouse_a.name == id {
                        out.push(m.spouse_a.span);
                    }
                    if m.spouse_b.name == id {
                        out.push(m.spouse_b.span);
                    }
                }
            }
            EntityKind::Marriage => {
                for p in self.persons_in(file) {
                    if let Some(b) = &p.birth
                        && b.marriage_ref.name == id
                    {
                        out.push(b.marriage_ref.span);
                    }
                    for a in &p.adoptions {
                        if a.marriage_ref.name == id {
                            out.push(a.marriage_ref.span);
                        }
                    }
                }
            }
        }
        out
    }

    /// Biological + adoptive parents of a person, in source order, each
    /// tagged with the link's source span and kind. Unresolved references
    /// are skipped (rule 2 reports them). `file` is the file the person
    /// lives in; parent lookups happen in the same file.
    pub fn parents_of<'a>(&'a self, file: FileId, person: &PersonStmt) -> Vec<ParentLink<'a>> {
        let mut out = Vec::new();
        if let Some(birth) = &person.birth
            && let Some(marriage) = self.marriage(file, &birth.marriage_ref.name)
        {
            for parent in self.spouses_of(file, marriage) {
                out.push(ParentLink {
                    parent,
                    link_span: birth.marriage_ref.span,
                    kind: ParentLinkKind::Bio,
                });
            }
        }
        for adoption in &person.adoptions {
            if let Some(marriage) = self.marriage(file, &adoption.marriage_ref.name) {
                for parent in self.spouses_of(file, marriage) {
                    out.push(ParentLink {
                        parent,
                        link_span: adoption.marriage_ref.span,
                        kind: ParentLinkKind::Adoption,
                    });
                }
            }
        }
        out
    }
}

/// Build the per-file id index for `document` and return any diagnostics
/// that surface during construction. Takes ownership via
/// [`Arc<Document>`] — the returned [`ResolvedDocument`] holds a
/// refcounted handle to the same allocation, so callers can keep their
/// own clone for source-level work.
///
/// Currently emits **rule 01** (duplicate ids) inline as the entity
/// tables are populated — the duplicate check is a property of insertion
/// order and belongs in the resolver. R01 fires only within the same
/// file (per ADR-0014 per-file namespaces). All other spec rules
/// (including rule 02 — unresolved references) live in
/// [`crate::validator`] and run as a separate pass over the
/// [`ResolvedDocument`].
pub fn resolve(document: Arc<Document>) -> (ResolvedDocument, Vec<Diagnostic>) {
    let mut entities: HashMap<FileId, HashMap<String, ResolvedEntity>> = HashMap::new();
    let mut diagnostics: Vec<Diagnostic> = Vec::new();

    for (file, kf) in document.kul_files() {
        let file_index = entities.entry(file).or_default();
        resolve_file(file, kf, file_index, &mut diagnostics);
    }

    let resolved = ResolvedDocument { document, entities };
    (resolved, diagnostics)
}

fn resolve_file(
    file: FileId,
    kf: &KulFile,
    entities: &mut HashMap<String, ResolvedEntity>,
    diagnostics: &mut Vec<Diagnostic>,
) {
    for (statement_idx, stmt) in kf.statements.iter().enumerate() {
        let (kind, id) = match stmt {
            Statement::Person(p) => (EntityKind::Person, &p.id),
            Statement::Marriage(m) => (EntityKind::Marriage, &m.id),
        };
        match entities.get(id.name.as_str()) {
            Some(prior) => {
                let prior_kind = prior.kind.as_str();
                let prior_ident = match &kf.statements[prior.statement_idx] {
                    Statement::Person(p) => &p.id,
                    Statement::Marriage(m) => &m.id,
                };
                let message = if prior.kind == kind {
                    format!(
                        "id `{}` is already used by another {prior_kind} — pick a different id (every id must be unique across all persons and marriages)",
                        id.name
                    )
                } else {
                    format!(
                        "id `{}` is already used by a {prior_kind} — pick a different id (every id must be unique across all persons and marriages)",
                        id.name
                    )
                };
                diagnostics.push(
                    Diagnostic::error("KUL-R01", message, fspan(file, id.span)).with_related(
                        fspan(file, prior_ident.span),
                        format!("first declared here as a {prior_kind}"),
                    ),
                );
            }
            None => {
                entities.insert(
                    id.name.clone(),
                    ResolvedEntity {
                        kind,
                        statement_idx,
                    },
                );
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ast::{Document, KulFile};
    use crate::lexer::tokenize;
    use crate::parser::parse;

    fn resolve_source(source: &str) -> (ResolvedDocument, FileId) {
        let file = FileId(1);
        let tokens = tokenize(source);
        let (statements, _) = parse(&tokens, file);
        let kf = Arc::new(KulFile {
            name: "test.kul".to_string(),
            source: source.to_string(),
            statements,
        });
        let document = Arc::new(Document {
            manifest_name: "kul.yml".to_string(),
            manifest_source: String::new(),
            kul_files: vec![kf],
        });
        let (resolved, _) = resolve(document);
        (resolved, file)
    }

    fn refs(source: &str, id: &str, kind: EntityKind) -> Vec<(usize, usize)> {
        let (resolved, file) = resolve_source(source);
        resolved
            .references_to(file, id, kind)
            .into_iter()
            .map(|s| (s.start, s.end))
            .collect()
    }

    fn idx(source: &str, pat: &str) -> usize {
        source.find(pat).expect("pattern in source")
    }

    #[test]
    fn references_to_person_finds_spouse_positions() {
        let src = "person alice name:\"A\" gender:female\n\
                   person bob name:\"B\" gender:male\n\
                   person carol name:\"C\" gender:female\n\
                   marriage m1 alice bob start:1972\n\
                   marriage m2 alice carol start:2000\n";
        let got = refs(src, "alice", EntityKind::Person);
        assert_eq!(got.len(), 2);

        let m1_alice = idx(src, "marriage m1 alice") + "marriage m1 ".len();
        let m2_alice = idx(src, "marriage m2 alice") + "marriage m2 ".len();
        assert_eq!(got[0], (m1_alice, m1_alice + "alice".len()));
        assert_eq!(got[1], (m2_alice, m2_alice + "alice".len()));
    }

    #[test]
    fn references_to_marriage_finds_birth_and_adoption_refs() {
        let src = "person alice name:\"A\" gender:female\n\
                   person bob name:\"B\" gender:male\n\
                   marriage m alice bob start:1972\n\
                   person kid1 name:\"K1\" gender:other\n  birth m\n\
                   person kid2 name:\"K2\" gender:other\n  adoption m start:2000\n";
        let got = refs(src, "m", EntityKind::Marriage);
        assert_eq!(got.len(), 2);
        assert!(got[0].0 < got[1].0);
    }

    #[test]
    fn references_to_returns_unresolved_refs() {
        let src = "marriage m ghost b start:1972\nperson b name:\"B\" gender:male\n";
        let got = refs(src, "ghost", EntityKind::Person);
        assert_eq!(got.len(), 1);
    }

    #[test]
    fn references_to_excludes_declaration_site() {
        let src = "person alice name:\"A\" gender:female\n\
                   marriage m alice alice start:2000\n";
        let got = refs(src, "alice", EntityKind::Person);
        assert_eq!(got.len(), 2);
        let decl_span = idx(src, "person alice") + "person ".len();
        assert!(got.iter().all(|&(s, _)| s != decl_span));
    }

    #[test]
    fn references_to_no_matches_yields_empty() {
        let src = "person alice name:\"A\" gender:female\n";
        assert!(refs(src, "alice", EntityKind::Person).is_empty());
        assert!(refs(src, "nope", EntityKind::Marriage).is_empty());
    }

    #[test]
    fn references_to_kind_filters_correctly() {
        let src = "person a name:\"A\" gender:female\n\
                   person b name:\"B\" gender:male\n\
                   marriage m a b start:1972\n\
                   person kid name:\"K\" gender:other\n  birth m\n";
        assert!(refs(src, "m", EntityKind::Person).is_empty());
        assert_eq!(refs(src, "m", EntityKind::Marriage).len(), 1);
    }

    #[test]
    fn document_arc_is_shared() {
        let (resolved, _) = resolve_source("person alice name:\"A\" gender:female\n");
        let a = resolved.document_arc();
        let b = resolved.document_arc();
        assert!(Arc::ptr_eq(&a, &b));
    }

    #[test]
    fn person_lookup_returns_none_for_marriage_id() {
        let (resolved, file) = resolve_source(
            "person a name:\"A\" gender:female\n\
             person b name:\"B\" gender:male\n\
             marriage m a b start:2000\n",
        );
        assert!(resolved.person(file, "m").is_none());
        assert!(resolved.marriage(file, "m").is_some());
    }

    #[test]
    fn marriage_lookup_returns_none_for_person_id() {
        let (resolved, file) = resolve_source(
            "person a name:\"A\" gender:female\n\
             person b name:\"B\" gender:male\n\
             marriage m a b start:2000\n",
        );
        assert!(resolved.marriage(file, "a").is_none());
        assert!(resolved.person(file, "a").is_some());
    }

    fn statement_kind_at(
        resolved: &ResolvedDocument,
        file: FileId,
        offset: usize,
    ) -> Option<&'static str> {
        resolved.statement_at(file, offset).map(|s| match s {
            Statement::Person(_) => "person",
            Statement::Marriage(_) => "marriage",
        })
    }

    #[test]
    fn statement_at_before_first_statement_returns_none() {
        // No more `kul 1` line; whitespace before the first statement.
        let src = "  \nperson alice name:\"A\" gender:female\n";
        let (resolved, file) = resolve_source(src);
        assert_eq!(statement_kind_at(&resolved, file, 0), None);
    }

    #[test]
    fn statement_at_inside_a_statement_returns_it() {
        let src = "person alice name:\"A\" gender:female\nmarriage m alice alice start:2000\n";
        let (resolved, file) = resolve_source(src);
        let alice_inside = idx(src, "alice") + 1;
        assert_eq!(
            statement_kind_at(&resolved, file, alice_inside),
            Some("person")
        );
        let marriage_inside = idx(src, "marriage m") + 2;
        assert_eq!(
            statement_kind_at(&resolved, file, marriage_inside),
            Some("marriage")
        );
    }

    #[test]
    fn statement_at_returns_latest_at_or_before_offset() {
        let src = "person alice name:\"A\" gender:female\n";
        let (resolved, file) = resolve_source(src);
        assert_eq!(
            statement_kind_at(&resolved, file, src.len()),
            Some("person")
        );
    }

    #[test]
    fn statement_at_picks_the_most_recent_when_multiple_precede() {
        let src = "person alice name:\"A\" gender:female\n\
                   person bob name:\"B\" gender:male\n\
                   marriage m alice bob start:2000\n";
        let (resolved, file) = resolve_source(src);
        assert_eq!(
            statement_kind_at(&resolved, file, src.len()),
            Some("marriage")
        );
        let after_bob = idx(src, "person bob") + "person bob".len();
        assert_eq!(
            statement_kind_at(&resolved, file, after_bob),
            Some("person")
        );
    }

    #[test]
    fn r01_fires_per_file_not_across_files() {
        // Two `.kul` files each declaring `alice` — no R01 across files.
        let src1 = "person alice name:\"A\" gender:female\n";
        let src2 = "person alice name:\"A\" gender:female\n";
        let kf1 = Arc::new(KulFile {
            name: "a.kul".into(),
            source: src1.into(),
            statements: parse(&tokenize(src1), FileId(1)).0,
        });
        let kf2 = Arc::new(KulFile {
            name: "b.kul".into(),
            source: src2.into(),
            statements: parse(&tokenize(src2), FileId(2)).0,
        });
        let doc = Arc::new(Document {
            manifest_name: "kul.yml".into(),
            manifest_source: String::new(),
            kul_files: vec![kf1, kf2],
        });
        let (_resolved, diags) = resolve(doc);
        assert!(
            !diags.iter().any(|d| d.code == "KUL-R01"),
            "R01 must not fire across files: {diags:#?}"
        );
    }

    #[test]
    fn r01_fires_within_a_single_file() {
        let src = "person alice name:\"A\" gender:female\n\
                   person alice name:\"B\" gender:female\n";
        let (_, _file) = resolve_source(src);
        let kf = Arc::new(KulFile {
            name: "a.kul".into(),
            source: src.into(),
            statements: parse(&tokenize(src), FileId(1)).0,
        });
        let doc = Arc::new(Document {
            manifest_name: "kul.yml".into(),
            manifest_source: String::new(),
            kul_files: vec![kf],
        });
        let (_resolved, diags) = resolve(doc);
        assert!(
            diags.iter().any(|d| d.code == "KUL-R01"),
            "R01 must fire within one file: {diags:#?}"
        );
    }
}
