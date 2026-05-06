//! Semantic analysis: turns a [`Document`] into a [`ResolvedDocument`].
//!
//! [`ResolvedDocument`] is the deep query module the validator and the
//! cycle-detector talk to. It owns the [`Document`] (via `Arc<Document>`) and
//! the id index, and exposes typed query methods (`persons`, `marriages`,
//! `spouses_of`, `parents_of`); callers never touch the underlying maps. New
//! questions about kinship belong here, not at every rule's call site.
//!
//! The `Arc<Document>` shape is what lets the LSP cache a single resolved
//! view per open document — the borrowed-lifetime alternative was
//! self-referential and forced a re-resolve on every editor request. See
//! [ADR-0007](../../docs/adr/0007-resolved-document-owns-document.md).

use std::collections::HashMap;
use std::sync::Arc;

use crate::ast::{Document, Ident, MarriageStmt, PersonStmt, Statement};
use crate::diagnostic::Diagnostic;
use crate::span::ByteSpan;

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

/// One entry in the ID index, returned by [`ResolvedDocument::entity`].
///
/// Built on demand from the underlying statement when the caller asks; the
/// `id` borrow is tied to `&self` of the resolved document.
#[derive(Debug, Clone, Copy)]
pub struct EntityRef<'a> {
    pub kind: EntityKind,
    pub id: &'a Ident,
}

impl EntityRef<'_> {
    pub fn span(&self) -> ByteSpan {
        self.id.span
    }
}

/// Stored entry in the resolved id index — `kind` plus the position of the
/// corresponding statement in `document.statements`. Storing an index rather
/// than a borrow is what lets `ResolvedDocument` own its `Document` instead
/// of borrowing into it; query methods rebuild the borrowed [`EntityRef`]
/// view on demand.
#[derive(Debug, Clone, Copy)]
struct ResolvedEntity {
    kind: EntityKind,
    statement_idx: usize,
}

/// A document with semantic information attached.
///
/// Built by [`resolve`]; consumed by the validator and the cycle-detector.
/// All cross-reference and kinship queries go through methods on this
/// type — callers do not enumerate the underlying maps. Owns its
/// [`Document`] via `Arc<Document>`, so it is cheap to clone (refcount bump)
/// and can be cached alongside other artifacts (the LSP document cache holds
/// one per open URI).
#[derive(Debug, Clone)]
pub struct ResolvedDocument {
    document: Arc<Document>,
    /// First-seen entity per id, by statement index. The keys are the id
    /// names as `String` so the map owns its data (no borrow into
    /// `document`).
    entities: HashMap<String, ResolvedEntity>,
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
    /// The underlying parsed [`Document`]. Useful for downstream consumers
    /// that need the raw AST (e.g. mapping a file offset to a statement);
    /// rules inside this crate go through the typed queries below instead.
    pub fn document(&self) -> &Document {
        &self.document
    }

    /// A clone of the shared [`Arc<Document>`] backing this resolved view —
    /// a refcount bump, no allocation. Useful for callers that want to hold
    /// onto the document alongside the resolved view (the LSP document
    /// cache, downstream tooling) without copying the AST.
    pub fn document_arc(&self) -> Arc<Document> {
        Arc::clone(&self.document)
    }

    /// Walk every top-level statement in source order.
    ///
    /// Use this when a caller wants to dispatch on the typed `Statement`
    /// enum (semantic tokens, the document outline). Kinship questions
    /// belong on the per-kind iterators below.
    pub fn statements(&self) -> impl Iterator<Item = &Statement> + '_ {
        self.document.statements.iter()
    }

    /// Walk every `person` statement in source order.
    pub fn persons(&self) -> impl Iterator<Item = &PersonStmt> + '_ {
        self.statements().filter_map(|s| match s {
            Statement::Person(p) => Some(p),
            _ => None,
        })
    }

    /// Walk every `marriage` statement in source order.
    pub fn marriages(&self) -> impl Iterator<Item = &MarriageStmt> + '_ {
        self.statements().filter_map(|s| match s {
            Statement::Marriage(m) => Some(m),
            _ => None,
        })
    }

    /// Look up a person by id. Returns `None` if no entity has this id, or
    /// if an entity does but it's a marriage.
    pub fn person(&self, id: &str) -> Option<&PersonStmt> {
        let entity = self.entities.get(id)?;
        if entity.kind != EntityKind::Person {
            return None;
        }
        match self.statement_by_index(entity.statement_idx) {
            Statement::Person(p) => Some(p),
            Statement::Marriage(_) => unreachable!(
                "entity index claims kind=Person but statement is a marriage; resolve() invariant broken"
            ),
        }
    }

    /// Look up a marriage by id. Returns `None` if no entity has this id, or
    /// if an entity does but it's a person.
    pub fn marriage(&self, id: &str) -> Option<&MarriageStmt> {
        let entity = self.entities.get(id)?;
        if entity.kind != EntityKind::Marriage {
            return None;
        }
        match self.statement_by_index(entity.statement_idx) {
            Statement::Marriage(m) => Some(m),
            Statement::Person(_) => unreachable!(
                "entity index claims kind=Marriage but statement is a person; resolve() invariant broken"
            ),
        }
    }

    /// Look up an entity (person or marriage) by id, regardless of kind.
    /// Used by reference-resolution checks.
    pub fn entity(&self, id: &str) -> Option<EntityRef<'_>> {
        let entity = self.entities.get(id)?;
        let ident = match self.statement_by_index(entity.statement_idx) {
            Statement::Person(p) => &p.id,
            Statement::Marriage(m) => &m.id,
        };
        Some(EntityRef {
            kind: entity.kind,
            id: ident,
        })
    }

    /// The top-level statement the cursor is sitting in, or just past.
    ///
    /// "Sitting in" meaning the latest statement whose `span.start` is
    /// at-or-before `byte_offset`. This is **not** strict span-containment:
    /// a cursor on a brand-new blank line below a `person` declaration is
    /// past the `PersonStmt.span.end`, but this method still returns the
    /// `Person` — callers that care about top-level vs. indented context
    /// (the completion classifier) refine that with the current line's
    /// indent. Returns `None` only when the cursor is before any statement
    /// (e.g. inside the version line, or in leading whitespace).
    ///
    /// Implementation note: `O(n)` linear scan over `document.statements`.
    /// Statements are kept in source order by the parser, so this is the
    /// last hit while iterating; binary search would also be correct but
    /// hasn't been needed at the perf budget.
    pub fn statement_at(&self, byte_offset: usize) -> Option<&Statement> {
        let mut chosen = None;
        for stmt in &self.document.statements {
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
    /// unresolved.
    pub fn spouses_of<'a>(
        &'a self,
        marriage: &'a MarriageStmt,
    ) -> impl Iterator<Item = &'a PersonStmt> + 'a {
        [&marriage.spouse_a, &marriage.spouse_b]
            .into_iter()
            .filter_map(|ident| self.person(&ident.name))
    }

    /// Every reference site for `id` in the document, in source order.
    ///
    /// `kind` selects which positions count: spouse positions for a person,
    /// `birth`/`adoption` marriage refs for a marriage. The declaration
    /// site is **not** included — callers that want it (per LSP's
    /// `includeDeclaration` flag) prepend it themselves.
    ///
    /// Unresolved references whose name happens to match `id` are still
    /// returned. This means rename and find-references both work on partly-
    /// broken documents (the user is mid-edit), and rule 2's "no person/
    /// marriage with this id" diagnostic is the right place to surface
    /// the missing declaration — not here.
    pub fn references_to(&self, id: &str, kind: EntityKind) -> Vec<ByteSpan> {
        let mut out = Vec::new();
        match kind {
            EntityKind::Person => {
                for m in self.marriages() {
                    if m.spouse_a.name == id {
                        out.push(m.spouse_a.span);
                    }
                    if m.spouse_b.name == id {
                        out.push(m.spouse_b.span);
                    }
                }
            }
            EntityKind::Marriage => {
                for p in self.persons() {
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
    /// are skipped (rule 2 reports them).
    pub fn parents_of<'a>(&'a self, person: &PersonStmt) -> Vec<ParentLink<'a>> {
        let mut out = Vec::new();
        if let Some(birth) = &person.birth
            && let Some(marriage) = self.marriage(&birth.marriage_ref.name)
        {
            for parent in self.spouses_of(marriage) {
                out.push(ParentLink {
                    parent,
                    link_span: birth.marriage_ref.span,
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
                        kind: ParentLinkKind::Adoption,
                    });
                }
            }
        }
        out
    }

    /// Internal: look up the statement at a given index in
    /// `document.statements`. Panics if the index is out of bounds — callers
    /// (`person`/`marriage`/`entity`) only use indices stored by [`resolve`],
    /// which always come from `document.statements.iter().enumerate()`.
    fn statement_by_index(&self, idx: usize) -> &Statement {
        &self.document.statements[idx]
    }
}

/// Build the id index for `document` and return any diagnostics that
/// surface during construction. Takes ownership via [`Arc<Document>`] — the
/// returned [`ResolvedDocument`] holds a refcounted handle to the same
/// allocation, so callers can keep their own clone for source-level work.
///
/// Currently emits **rule 01** (duplicate ids) inline as the entity table is
/// populated — the duplicate check is a property of insertion order and
/// belongs in the resolver. All other spec rules (including rule 02 —
/// unresolved references) live in [`crate::validator`] and run as a
/// separate pass over the [`ResolvedDocument`].
pub fn resolve(document: Arc<Document>) -> (ResolvedDocument, Vec<Diagnostic>) {
    let mut entities: HashMap<String, ResolvedEntity> = HashMap::new();
    let mut diagnostics = Vec::new();

    for (statement_idx, stmt) in document.statements.iter().enumerate() {
        let (kind, id) = match stmt {
            Statement::Person(p) => (EntityKind::Person, &p.id),
            Statement::Marriage(m) => (EntityKind::Marriage, &m.id),
        };
        match entities.get(id.name.as_str()) {
            Some(prior) => {
                let prior_kind = prior.kind.as_str();
                let prior_ident = match &document.statements[prior.statement_idx] {
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
                    Diagnostic::error("KULA-R01", message, id.span).with_related(
                        prior_ident.span,
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

    let resolved = ResolvedDocument { document, entities };
    (resolved, diagnostics)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::lexer::tokenize;
    use crate::parser::parse;

    fn resolve_source(source: &str) -> ResolvedDocument {
        let tokens = tokenize(source);
        let (document, _) = parse(&tokens);
        let (resolved, _) = resolve(Arc::new(document));
        resolved
    }

    fn refs(source: &str, id: &str, kind: EntityKind) -> Vec<(usize, usize)> {
        resolve_source(source)
            .references_to(id, kind)
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
        // Two refs, one per marriage spouse_a slot.
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
        // First is the birth ref (earlier in source), second is the adoption.
        assert!(got[0].0 < got[1].0);
    }

    #[test]
    fn references_to_returns_unresolved_refs() {
        // No declaration of `ghost`, but it is referenced as a spouse.
        // The query still returns it so rename/find-references work mid-edit.
        let src = "marriage m ghost b start:1972\nperson b name:\"B\" gender:male\n";
        let got = refs(src, "ghost", EntityKind::Person);
        assert_eq!(got.len(), 1);
    }

    #[test]
    fn references_to_excludes_declaration_site() {
        let src = "person alice name:\"A\" gender:female\n\
                   marriage m alice alice start:2000\n"; // self-marriage, fine for span counting
        let got = refs(src, "alice", EntityKind::Person);
        // Two spouse positions, NOT the decl id.
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
        // Same id used as marriage; querying as Person finds nothing.
        let src = "person a name:\"A\" gender:female\n\
                   person b name:\"B\" gender:male\n\
                   marriage m a b start:1972\n\
                   person kid name:\"K\" gender:other\n  birth m\n";
        // `m` is a marriage; querying it as a person yields no refs.
        assert!(refs(src, "m", EntityKind::Person).is_empty());
        // …but as a marriage, it has one ref (the `birth`).
        assert_eq!(refs(src, "m", EntityKind::Marriage).len(), 1);
    }

    #[test]
    fn document_arc_is_shared() {
        let resolved = resolve_source("person alice name:\"A\" gender:female\n");
        let a = resolved.document_arc();
        let b = resolved.document_arc();
        // Shared via Arc; both clones point to the same allocation.
        assert!(Arc::ptr_eq(&a, &b));
    }

    #[test]
    fn person_lookup_returns_none_for_marriage_id() {
        // A marriage id should not satisfy a person lookup, even when the
        // entities map has an entry for it.
        let resolved = resolve_source(
            "person a name:\"A\" gender:female\n\
             person b name:\"B\" gender:male\n\
             marriage m a b start:2000\n",
        );
        assert!(resolved.person("m").is_none());
        assert!(resolved.marriage("m").is_some());
    }

    #[test]
    fn marriage_lookup_returns_none_for_person_id() {
        let resolved = resolve_source(
            "person a name:\"A\" gender:female\n\
             person b name:\"B\" gender:male\n\
             marriage m a b start:2000\n",
        );
        assert!(resolved.marriage("a").is_none());
        assert!(resolved.person("a").is_some());
    }

    fn statement_kind_at(resolved: &ResolvedDocument, offset: usize) -> Option<&'static str> {
        resolved.statement_at(offset).map(|s| match s {
            Statement::Person(_) => "person",
            Statement::Marriage(_) => "marriage",
        })
    }

    #[test]
    fn statement_at_before_first_statement_returns_none() {
        let src = "kula 1\nperson alice name:\"A\" gender:female\n";
        let resolved = resolve_source(src);
        // Inside the version line, before any statement.
        assert_eq!(statement_kind_at(&resolved, 0), None);
        assert_eq!(statement_kind_at(&resolved, 5), None);
    }

    #[test]
    fn statement_at_inside_a_statement_returns_it() {
        let src = "person alice name:\"A\" gender:female\nmarriage m alice alice start:2000\n";
        let resolved = resolve_source(src);
        let alice_inside = idx(src, "alice") + 1;
        assert_eq!(statement_kind_at(&resolved, alice_inside), Some("person"));
        let marriage_inside = idx(src, "marriage m") + 2;
        assert_eq!(
            statement_kind_at(&resolved, marriage_inside),
            Some("marriage")
        );
    }

    #[test]
    fn statement_at_returns_latest_at_or_before_offset() {
        // Cursor on a brand-new blank line below a person — past the
        // person's span.end, but `statement_at` still returns the person
        // because no later statement starts before the cursor. The completion
        // classifier relies on this "sitting just past" semantic.
        let src = "person alice name:\"A\" gender:female\n";
        let resolved = resolve_source(src);
        // Offset at end-of-source, well past the last statement.
        assert_eq!(statement_kind_at(&resolved, src.len()), Some("person"));
    }

    #[test]
    fn statement_at_picks_the_most_recent_when_multiple_precede() {
        let src = "person alice name:\"A\" gender:female\n\
                   person bob name:\"B\" gender:male\n\
                   marriage m alice bob start:2000\n";
        let resolved = resolve_source(src);
        // After the marriage, the marriage is the most recent — even though
        // both persons also start before the cursor.
        assert_eq!(statement_kind_at(&resolved, src.len()), Some("marriage"));
        // Mid-document, just after `bob`'s declaration line, the most-recent
        // is bob, not alice.
        let after_bob = idx(src, "person bob") + "person bob".len();
        assert_eq!(statement_kind_at(&resolved, after_bob), Some("person"));
    }
}
