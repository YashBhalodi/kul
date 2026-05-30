//! Semantic analysis: multi-file [`Document`] → [`ResolvedDocument`], the
//! deep query module the validator and cycle-detector talk through.
//!
//! Owns its [`Document`] via `Arc<Document>` (ADR-0007) and a
//! project-wide id index. Per-id queries are project-wide per ADR-0015
//! (the file boundary is organizational, not semantic). `_in(file)`
//! variants restrict iteration to one file; byte-offset queries
//! (`node_at`, `statement_at`) keep their file parameter.

use std::collections::HashMap;
use std::sync::Arc;

use crate::ast::{Document, Ident, MarriageStmt, PersonStmt, Statement};
use crate::diagnostic::{Diagnostic, fspan};
use crate::span::{ByteSpan, FileId, FileSpan};

/// Kind of top-level entity an id names.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EntityKind {
    Person,
    Marriage,
}

impl EntityKind {
    #[must_use]
    pub fn as_str(self) -> &'static str {
        match self {
            EntityKind::Person => "person",
            EntityKind::Marriage => "marriage",
        }
    }
}

/// Project-wide id index entry returned by [`ResolvedDocument::entity`].
/// Built on demand; `id` borrow tied to `&self`.
#[derive(Debug, Clone, Copy)]
pub struct EntityRef<'a> {
    pub kind: EntityKind,
    pub id: &'a Ident,
    /// File the declaration lives in.
    pub file: FileId,
}

impl EntityRef<'_> {
    /// The declaration's id span as a project-wide [`FileSpan`].
    #[must_use]
    pub fn span(&self) -> FileSpan {
        fspan(self.file, self.id.span)
    }
}

/// Internal id index entry: `(kind, file, statement_idx)`. Stored
/// non-borrowed so `ResolvedDocument` can own its [`Document`].
#[derive(Debug, Clone, Copy)]
struct ResolvedEntity {
    kind: EntityKind,
    file: FileId,
    statement_idx: usize,
}

/// Multi-file Kul project with semantic information attached. Owns its
/// [`Document`] via `Arc<Document>` so cloning is a refcount bump.
#[derive(Debug, Clone)]
pub struct ResolvedDocument {
    document: Arc<Document>,
    /// Project-wide id index; R01 fires on collision so every entry is
    /// uniquely owned.
    entities: HashMap<String, ResolvedEntity>,
}

/// Directed edge `child → parent`. `link_span`/`link_file` point to the
/// child's `birth`/`adoption` ref (in the child's file, which may
/// differ from the parent's).
#[derive(Debug, Clone)]
pub struct ParentLink<'a> {
    pub parent: &'a PersonStmt,
    pub link_span: ByteSpan,
    pub link_file: FileId,
    pub kind: ParentLinkKind,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ParentLinkKind {
    Bio,
    Adoption,
}

impl ResolvedDocument {
    /// The underlying parsed [`Document`].
    #[must_use]
    pub fn document(&self) -> &Document {
        &self.document
    }

    /// Clone the shared [`Arc<Document>`] (refcount bump, no allocation).
    #[must_use]
    pub fn document_arc(&self) -> Arc<Document> {
        Arc::clone(&self.document)
    }

    /// Every top-level statement in every `.kul` file, in source order,
    /// file-by-file.
    pub fn statements(&self) -> impl Iterator<Item = &Statement> + '_ {
        self.document
            .kul_files
            .iter()
            .flat_map(|f| f.statements.iter())
    }

    /// Statements in `file`, in source order. Empty when `file` is the
    /// manifest or out of range.
    pub fn statements_in(&self, file: FileId) -> impl Iterator<Item = &Statement> + '_ {
        self.document
            .kul_file(file)
            .into_iter()
            .flat_map(|f| f.statements.iter())
    }

    /// Every `person` in the project, in source order.
    pub fn persons(&self) -> impl Iterator<Item = &PersonStmt> + '_ {
        self.statements().filter_map(|s| match s {
            Statement::Person(p) => Some(p),
            _ => None,
        })
    }

    /// Persons in `file`, in source order.
    pub fn persons_in(&self, file: FileId) -> impl Iterator<Item = &PersonStmt> + '_ {
        self.statements_in(file).filter_map(|s| match s {
            Statement::Person(p) => Some(p),
            _ => None,
        })
    }

    /// Every `marriage` in the project, in source order.
    pub fn marriages(&self) -> impl Iterator<Item = &MarriageStmt> + '_ {
        self.statements().filter_map(|s| match s {
            Statement::Marriage(m) => Some(m),
            _ => None,
        })
    }

    /// Marriages in `file`, in source order.
    pub fn marriages_in(&self, file: FileId) -> impl Iterator<Item = &MarriageStmt> + '_ {
        self.statements_in(file).filter_map(|s| match s {
            Statement::Marriage(m) => Some(m),
            _ => None,
        })
    }

    /// Look up a person by id (project-wide). `None` if absent or the id
    /// names a marriage.
    #[must_use]
    pub fn person(&self, id: &str) -> Option<&PersonStmt> {
        self.person_with_file(id).map(|(_, p)| p)
    }

    /// Like [`Self::person`] but also returns the declaring [`FileId`].
    #[must_use]
    pub fn person_with_file(&self, id: &str) -> Option<(FileId, &PersonStmt)> {
        let entity = self.entity_record(id)?;
        if entity.kind != EntityKind::Person {
            return None;
        }
        let kf = self.document.kul_file(entity.file)?;
        match &kf.statements[entity.statement_idx] {
            Statement::Person(p) => Some((entity.file, p)),
            Statement::Marriage(_) => unreachable!(
                "entity index claims kind=Person but statement is a marriage; resolve() invariant broken"
            ),
        }
    }

    /// Look up a marriage by id (project-wide). Same `None` rules as
    /// [`Self::person`].
    #[must_use]
    pub fn marriage(&self, id: &str) -> Option<&MarriageStmt> {
        self.marriage_with_file(id).map(|(_, m)| m)
    }

    /// Like [`Self::marriage`] but also returns the declaring [`FileId`].
    #[must_use]
    pub fn marriage_with_file(&self, id: &str) -> Option<(FileId, &MarriageStmt)> {
        let entity = self.entity_record(id)?;
        if entity.kind != EntityKind::Marriage {
            return None;
        }
        let kf = self.document.kul_file(entity.file)?;
        match &kf.statements[entity.statement_idx] {
            Statement::Marriage(m) => Some((entity.file, m)),
            Statement::Person(_) => unreachable!(
                "entity index claims kind=Marriage but statement is a person; resolve() invariant broken"
            ),
        }
    }

    /// Look up an entity by id regardless of kind.
    #[must_use]
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

    /// The latest top-level statement whose `span.start` is at-or-before
    /// `byte_offset` in `file`. Not strict span-containment — a cursor on
    /// a blank line below a `person` still returns the `Person`. Callers
    /// that need top-level-vs-indented context refine via the current
    /// line's indent.
    #[must_use]
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

    /// Declared spouses of a marriage, in declaration order. Unresolved
    /// spouses are skipped (R02 reports them).
    pub fn spouses_of<'a>(
        &'a self,
        marriage: &'a MarriageStmt,
    ) -> impl Iterator<Item = &'a PersonStmt> + 'a {
        [&marriage.spouse_a, &marriage.spouse_b]
            .into_iter()
            .filter_map(move |ident| self.person(&ident.name))
    }

    /// Every reference site for `id` anywhere in the project, in source
    /// order. `kind` selects the positions (spouse positions for a person,
    /// `birth`/`adoption` refs for a marriage). The declaration site is
    /// NOT included. Unresolved references with a matching name are still
    /// returned so rename/find-references work on partial documents.
    #[must_use]
    pub fn references_to(&self, id: &str, kind: EntityKind) -> Vec<FileSpan> {
        let mut out = Vec::new();
        for (file, kf) in self.document.kul_files() {
            for stmt in &kf.statements {
                match (kind, stmt) {
                    (EntityKind::Person, Statement::Marriage(m)) => {
                        if m.spouse_a.name == id {
                            out.push(FileSpan::new(file, m.spouse_a.span));
                        }
                        if m.spouse_b.name == id {
                            out.push(FileSpan::new(file, m.spouse_b.span));
                        }
                    }
                    (EntityKind::Marriage, Statement::Person(p)) => {
                        if let Some(b) = &p.birth
                            && b.marriage_ref.name == id
                        {
                            out.push(FileSpan::new(file, b.marriage_ref.span));
                        }
                        for a in &p.adoptions {
                            if a.marriage_ref.name == id {
                                out.push(FileSpan::new(file, a.marriage_ref.span));
                            }
                        }
                    }
                    _ => {}
                }
            }
        }
        out
    }

    /// Biological + adoptive parents of `person`, in source order. Each
    /// link's `link_file` is the child's file. Unresolved refs are
    /// skipped (R02 reports them).
    #[must_use]
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

    /// File containing a person's declaration; falls back to
    /// [`FileId::MANIFEST`] only to keep the type total.
    fn file_of_person(&self, person: &PersonStmt) -> FileId {
        self.entity_record(&person.id.name)
            .map(|e| e.file)
            .unwrap_or(FileId::MANIFEST)
    }
}

/// Build the project-wide id index for `document`, emitting R01
/// (duplicate ids) inline as the index is populated. The R01 primary is
/// on the second declaration; a related-span points to the first. All
/// other rules live in [`crate::validator`].
#[must_use]
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
        let kf = Arc::new(KulFile::new("test.kul", source, statements));
        let document = Arc::new(Document::new("kul.yml", vec![kf]));
        let (resolved, _) = resolve(document);
        (resolved, file)
    }

    fn refs(source: &str, id: &str, kind: EntityKind) -> Vec<(usize, usize)> {
        let (resolved, _file) = resolve_source(source);
        resolved
            .references_to(id, kind)
            .into_iter()
            .map(|s| (s.span.start, s.span.end))
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
    fn references_to_walks_every_file_in_the_project() {
        let src_a = "person alice name:\"A\" gender:female\n\
                     person bob name:\"B\" gender:male\n\
                     marriage m_self alice bob start:1980\n";
        let src_b = "person carol name:\"C\" gender:female\n\
                     marriage m_cross alice carol start:2000\n";
        let tokens_a = tokenize(src_a);
        let tokens_b = tokenize(src_b);
        let file_a = FileId(1);
        let file_b = FileId(2);
        let (stmts_a, _) = parse(&tokens_a, file_a);
        let (stmts_b, _) = parse(&tokens_b, file_b);
        let document = Arc::new(Document::new(
            "kul.yml",
            vec![
                Arc::new(KulFile::new("a.kul", src_a, stmts_a)),
                Arc::new(KulFile::new("b.kul", src_b, stmts_b)),
            ],
        ));
        let (resolved, _) = resolve(document);

        let got: Vec<FileId> = resolved
            .references_to("alice", EntityKind::Person)
            .into_iter()
            .map(|fs| fs.file)
            .collect();
        assert_eq!(got, vec![file_a, file_b]);
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
        let src1 = "person alice name:\"A1\" gender:female\n";
        let src2 = "person alice name:\"A2\" gender:female\n";
        let kf1 = Arc::new(KulFile::new(
            "a.kul",
            src1,
            parse(&tokenize(src1), FileId(1)).0,
        ));
        let kf2 = Arc::new(KulFile::new(
            "b.kul",
            src2,
            parse(&tokenize(src2), FileId(2)).0,
        ));
        let doc = Arc::new(Document::new("kul.yml", vec![kf1, kf2]));
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
        let kf = Arc::new(KulFile::new(
            "a.kul",
            src,
            parse(&tokenize(src), FileId(1)).0,
        ));
        let doc = Arc::new(Document::new("kul.yml", vec![kf]));
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
        let kf1 = Arc::new(KulFile::new(
            "a.kul",
            src1,
            parse(&tokenize(src1), FileId(1)).0,
        ));
        let kf2 = Arc::new(KulFile::new(
            "b.kul",
            src2,
            parse(&tokenize(src2), FileId(2)).0,
        ));
        let doc = Arc::new(Document::new("kul.yml", vec![kf1, kf2]));
        let (resolved, diags) = resolve(doc);
        assert!(diags.is_empty(), "no R01 expected: {diags:#?}");
        assert!(resolved.person("alice").is_some());
        assert!(resolved.person("bob").is_some());
        assert!(resolved.marriage("m").is_some());
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
        let kf1 = Arc::new(KulFile::new(
            "a.kul",
            src1,
            parse(&tokenize(src1), FileId(1)).0,
        ));
        let kf2 = Arc::new(KulFile::new(
            "b.kul",
            src2,
            parse(&tokenize(src2), FileId(2)).0,
        ));
        let doc = Arc::new(Document::new("kul.yml", vec![kf1, kf2]));
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
