//! Semantic analysis: turns a multi-file [`Document`] into a
//! [`ResolvedDocument`].
//!
//! [`ResolvedDocument`] is the deep query module the validator and the
//! cycle-detector talk to. It owns the [`Document`] (via `Arc<Document>`)
//! and a project-wide id index, and exposes typed query methods (`persons`,
//! `marriages`, `spouses_of`, `parents_of`, `person`, `marriage`); callers
//! never touch the underlying maps. New questions about kinship belong
//! here, not at every rule's call site.
//!
//! # Project-wide namespace (ADR-0015)
//!
//! A Kul project is a directory containing a `kul.yml` manifest plus one
//! or more `.kul` files. Every person- or marriage-id declared in any
//! file of the project is visible from every file — the file boundary is
//! organizational, not semantic. Per-id queries (`person(id)`,
//! `marriage(id)`, `entity(id)`) take only the bare id; the resolver
//! returns the unique declaration regardless of which file owns it.
//!
//! Iteration queries (`persons`, `marriages`, `statements`) walk every
//! file in source order; the `_in(file)` variants restrict to one file
//! (the LSP uses these for per-document symbol listings and the like).
//!
//! `node_at(file, offset)` and `statement_at(file, offset)` keep their
//! file parameter because byte offsets are inherently per-file.
//!
//! [ADR-0015](../../docs/adr/0015-global-project-namespace.md) records
//! the supersession of ADR-0014's "Position B" (per-file namespaces) with
//! this project-wide model.
//!
//! The `Arc<Document>` shape is what lets the LSP cache a single resolved
//! view per project — the borrowed-lifetime alternative was self-
//! referential and forced a re-resolve on every editor request. See
//! [ADR-0007](../../docs/adr/0007-resolved-document-owns-document.md).

use std::collections::HashMap;
use std::sync::Arc;

use crate::ast::{Document, Ident, MarriageStmt, PersonStmt, Statement};
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

/// One entry in the project-wide ID index, returned by
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

/// Stored entry in the resolved id index — `kind`, the owning file, and
/// the position of the corresponding statement in that file's
/// `statements`. Storing the (file, index) pair rather than a borrow is
/// what lets `ResolvedDocument` own its `Document` instead of borrowing
/// into it; query methods rebuild the borrowed [`EntityRef`] view on
/// demand.
#[derive(Debug, Clone, Copy)]
struct ResolvedEntity {
    kind: EntityKind,
    file: FileId,
    statement_idx: usize,
}

/// A multi-file Kul project with semantic information attached.
///
/// Built by [`resolve`]; consumed by the validator and the cycle-detector.
/// All cross-reference and kinship queries go through methods on this
/// type — callers do not enumerate the underlying maps. Owns its
/// [`Document`] via `Arc<Document>`, so it is cheap to clone (refcount
/// bump) and can be cached alongside other artifacts (the LSP document
/// cache holds one per project).
#[derive(Debug, Clone)]
pub struct ResolvedDocument {
    document: Arc<Document>,
    /// Flat project-wide entity index. Every id declared in any `.kul`
    /// file of the project lives here once; R01 fires before insertion on
    /// collision, so a successful insert means the id is uniquely owned.
    entities: HashMap<String, ResolvedEntity>,
}

/// One parent link: a directed edge `child → parent` with the source span
/// where the link is documented (the child's `birth` or `adoption`
/// marriage-ref) and the file containing that span.
#[derive(Debug, Clone)]
pub struct ParentLink<'a> {
    pub parent: &'a PersonStmt,
    pub link_span: ByteSpan,
    /// The file containing `link_span` — the child's owning file, which
    /// may differ from the parent's owning file under project-wide
    /// resolution.
    pub link_file: FileId,
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
    /// enum across the whole project (the export builder, project-wide
    /// validator rules). Use [`Self::statements_in`] for a single-file
    /// iterator.
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

    /// Look up a person by id anywhere in the project. Returns `None` if
    /// no entity has this id, or if one does but it's a marriage. Lookups
    /// are project-wide per ADR-0015.
    pub fn person(&self, id: &str) -> Option<&PersonStmt> {
        let entity = self.entity_record(id)?;
        if entity.kind != EntityKind::Person {
            return None;
        }
        let kf = self.document.kul_file(entity.file)?;
        match &kf.statements[entity.statement_idx] {
            Statement::Person(p) => Some(p),
            Statement::Marriage(_) => unreachable!(
                "entity index claims kind=Person but statement is a marriage; resolve() invariant broken"
            ),
        }
    }

    /// Look up a marriage by id anywhere in the project. Same `None`
    /// rules as [`Self::person`].
    pub fn marriage(&self, id: &str) -> Option<&MarriageStmt> {
        let entity = self.entity_record(id)?;
        if entity.kind != EntityKind::Marriage {
            return None;
        }
        let kf = self.document.kul_file(entity.file)?;
        match &kf.statements[entity.statement_idx] {
            Statement::Marriage(m) => Some(m),
            Statement::Person(_) => unreachable!(
                "entity index claims kind=Marriage but statement is a person; resolve() invariant broken"
            ),
        }
    }

    /// Look up an entity (person or marriage) by id anywhere in the
    /// project, regardless of kind. Used by reference-resolution checks.
    pub fn entity(&self, id: &str) -> Option<EntityRef<'_>> {
        let entity = self.entity_record(id)?;
        let kf = self.document.kul_file(entity.file)?;
        let ident = match &kf.statements[entity.statement_idx] {
            Statement::Person(p) => &p.id,
            Statement::Marriage(m) => &m.id,
        };
        Some(EntityRef {
            kind: entity.kind,
            id: ident,
            file: entity.file,
        })
    }

    fn entity_record(&self, id: &str) -> Option<&ResolvedEntity> {
        self.entities.get(id)
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
    /// unresolved. Resolution is project-wide: a spouse declared in one
    /// file is reachable from a marriage in another (per ADR-0015).
    pub fn spouses_of<'a>(
        &'a self,
        marriage: &'a MarriageStmt,
    ) -> impl Iterator<Item = &'a PersonStmt> + 'a {
        [&marriage.spouse_a, &marriage.spouse_b]
            .into_iter()
            .filter_map(move |ident| self.person(&ident.name))
    }

    /// Every reference site for `id` in `file`, in source order.
    ///
    /// `kind` selects which positions count: spouse positions for a
    /// person, `birth`/`adoption` marriage refs for a marriage. The
    /// declaration site is **not** included — callers that want it (per
    /// LSP's `includeDeclaration` flag) prepend it themselves.
    ///
    /// Reference sites are scoped to `file` because the LSP's per-URI
    /// rename and find-references features iterate one file at a time;
    /// project-wide reference enumeration is handled by the LSP layer by
    /// calling this once per file. Unresolved references whose name
    /// happens to match `id` are still returned, so rename and
    /// find-references both work on partly-broken documents.
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
    /// tagged with the link's source span (and the file containing it)
    /// and the link kind. Unresolved references are skipped (rule 2
    /// reports them). Parent lookups happen project-wide; the link's
    /// file is the child's owning file, which may differ from the
    /// parent's.
    pub fn parents_of<'a>(&'a self, person: &PersonStmt) -> Vec<ParentLink<'a>> {
        let mut out = Vec::new();
        let person_file = self.file_of_person(person);
        if let Some(birth) = &person.birth
            && let Some(marriage) = self.marriage(&birth.marriage_ref.name)
        {
            for parent in self.spouses_of(marriage) {
                out.push(ParentLink {
                    parent,
                    link_span: birth.marriage_ref.span,
                    link_file: person_file,
                    kind: ParentLinkKind::Bio,
                });
            }
        }
        for adoption in &person.adoptions {
            if let Some(marriage) = self.marriage(&adoption.marriage_ref.name) {
                for parent in self.spouses_of(marriage) {
                    out.push(ParentLink {
                        parent,
                        link_span: adoption.marriage_ref.span,
                        link_file: person_file,
                        kind: ParentLinkKind::Adoption,
                    });
                }
            }
        }
        out
    }

    /// The file containing a person's declaration. Falls back to
    /// [`FileId::MANIFEST`] if the person is somehow not in any file
    /// (impossible under [`resolve`]'s invariants); the fallback exists
    /// only to keep the type total.
    fn file_of_person(&self, person: &PersonStmt) -> FileId {
        self.entity_record(&person.id.name)
            .map(|e| e.file)
            .unwrap_or(FileId::MANIFEST)
    }
}

/// Build the project-wide id index for `document` and return any
/// diagnostics that surface during construction. Takes ownership via
/// [`Arc<Document>`] — the returned [`ResolvedDocument`] holds a
/// refcounted handle to the same allocation, so callers can keep their
/// own clone for source-level work.
///
/// Currently emits **rule 01** (duplicate ids) inline as the entity table
/// is populated — the duplicate check is a property of insertion order
/// and belongs in the resolver. R01 fires whenever an id is declared
/// twice anywhere in the project. The primary span is on the second
/// declaration (in file-discovery order, then byte offset within a file);
/// a related-span points to the first declaration. All other spec rules
/// (including rule 02 — unresolved references) live in
/// [`crate::validator`] and run as a separate pass over the
/// [`ResolvedDocument`].
pub fn resolve(document: Arc<Document>) -> (ResolvedDocument, Vec<Diagnostic>) {
    let mut entities: HashMap<String, ResolvedEntity> = HashMap::new();
    let mut diagnostics: Vec<Diagnostic> = Vec::new();

    for (file, kf) in document.kul_files() {
        for (statement_idx, stmt) in kf.statements.iter().enumerate() {
            let (kind, id) = match stmt {
                Statement::Person(p) => (EntityKind::Person, &p.id),
                Statement::Marriage(m) => (EntityKind::Marriage, &m.id),
            };
            match entities.get(id.name.as_str()) {
                Some(prior) => {
                    let prior_kind = prior.kind.as_str();
                    let prior_kf = document
                        .kul_file(prior.file)
                        .expect("prior entity must live in a known file");
                    let prior_ident = match &prior_kf.statements[prior.statement_idx] {
                        Statement::Person(p) => &p.id,
                        Statement::Marriage(m) => &m.id,
                    };
                    let message = if prior.kind == kind {
                        format!(
                            "id `{}` is already used by another {prior_kind} — pick a different id (every id must be unique across the project)",
                            id.name
                        )
                    } else {
                        format!(
                            "id `{}` is already used by a {prior_kind} — pick a different id (every id must be unique across the project)",
                            id.name
                        )
                    };
                    diagnostics.push(
                        Diagnostic::error("KUL-R01", message, fspan(file, id.span)).with_related(
                            fspan(prior.file, prior_ident.span),
                            format!("first declared here as a {prior_kind}"),
                        ),
                    );
                }
                None => {
                    entities.insert(
                        id.name.clone(),
                        ResolvedEntity {
                            kind,
                            file,
                            statement_idx,
                        },
                    );
                }
            }
        }
    }

    let resolved = ResolvedDocument { document, entities };
    (resolved, diagnostics)
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
        let (resolved, _) = resolve_source(
            "person a name:\"A\" gender:female\n\
             person b name:\"B\" gender:male\n\
             marriage m a b start:2000\n",
        );
        assert!(resolved.person("m").is_none());
        assert!(resolved.marriage("m").is_some());
    }

    #[test]
    fn marriage_lookup_returns_none_for_person_id() {
        let (resolved, _) = resolve_source(
            "person a name:\"A\" gender:female\n\
             person b name:\"B\" gender:male\n\
             marriage m a b start:2000\n",
        );
        assert!(resolved.marriage("a").is_none());
        assert!(resolved.person("a").is_some());
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
    fn r01_fires_across_files_with_primary_on_second_decl() {
        // Two `.kul` files each declaring `alice`. Under project-wide
        // resolution (ADR-0015) R01 fires with the primary on the second
        // declaration (file 2) and a related-span on the first (file 1).
        let src1 = "person alice name:\"A1\" gender:female\n";
        let src2 = "person alice name:\"A2\" gender:female\n";
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
        let r01: Vec<_> = diags.iter().filter(|d| d.code == "KUL-R01").collect();
        assert_eq!(r01.len(), 1, "expected one R01: {diags:#?}");
        let d = r01[0];
        let primary = d.primary.unwrap();
        assert_eq!(primary.file, FileId(2), "primary anchored at second file");
        assert_eq!(d.related.len(), 1, "expected one related span");
        assert_eq!(d.related[0].span.file, FileId(1));
    }

    #[test]
    fn r01_fires_within_a_single_file() {
        let src = "person alice name:\"A\" gender:female\n\
                   person alice name:\"B\" gender:female\n";
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

    #[test]
    fn cross_file_reference_resolves() {
        let src1 = "person alice name:\"A\" gender:female\n\
                    person bob name:\"B\" gender:male\n";
        let src2 = "marriage m alice bob start:1972\n";
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
        let (resolved, diags) = resolve(doc);
        assert!(diags.is_empty(), "no R01 expected: {diags:#?}");
        assert!(resolved.person("alice").is_some());
        assert!(resolved.person("bob").is_some());
        assert!(resolved.marriage("m").is_some());
        // Spouses of `m` (in file 2) resolve to persons in file 1.
        let spouses: Vec<_> = resolved
            .spouses_of(resolved.marriage("m").unwrap())
            .map(|p| p.id.name.clone())
            .collect();
        assert_eq!(spouses, vec!["alice".to_string(), "bob".to_string()]);
    }

    #[test]
    fn project_wide_iteration_walks_every_file() {
        let src1 = "person alice name:\"A\" gender:female\n";
        let src2 = "person bob name:\"B\" gender:male\n";
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
        let (resolved, _) = resolve(doc);
        let all: Vec<_> = resolved.persons().map(|p| p.id.name.clone()).collect();
        assert_eq!(all, vec!["alice".to_string(), "bob".to_string()]);
        let in_a: Vec<_> = resolved
            .persons_in(FileId(1))
            .map(|p| p.id.name.clone())
            .collect();
        assert_eq!(in_a, vec!["alice".to_string()]);
        let in_b: Vec<_> = resolved
            .persons_in(FileId(2))
            .map(|p| p.id.name.clone())
            .collect();
        assert_eq!(in_b, vec!["bob".to_string()]);
    }
}
