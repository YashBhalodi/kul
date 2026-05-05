//! Semantic analysis: turns a [`Document`] into a [`ResolvedDocument`].
//!
//! [`ResolvedDocument`] is the deep query module the validator and the
//! cycle-detector talk to. It owns the ID index and exposes typed query
//! methods (`persons`, `marriages`, `spouses_of`, `parents_of`); callers
//! never touch the underlying maps. New questions about kinship belong
//! here, not at every rule's call site.

use std::collections::HashMap;

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

/// One entry in the ID index.
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

/// A document with semantic information attached.
///
/// Built by [`resolve`]; consumed by the validator and the cycle-detector.
/// All cross-reference and kinship queries go through methods on this
/// type — callers do not enumerate the underlying maps.
#[derive(Debug, Clone)]
pub struct ResolvedDocument<'a> {
    pub(crate) document: &'a Document,
    /// First-seen entity per id, for unresolved-reference checks.
    pub(crate) entities: HashMap<&'a str, EntityRef<'a>>,
    pub(crate) persons: HashMap<&'a str, &'a PersonStmt>,
    pub(crate) marriages: HashMap<&'a str, &'a MarriageStmt>,
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

impl<'a> ResolvedDocument<'a> {
    /// The underlying parsed [`Document`]. Useful for downstream consumers
    /// that need the raw AST (e.g. a future LSP needs span lookups by
    /// statement); rules inside this crate go through the typed queries
    /// below instead.
    pub fn document(&self) -> &'a Document {
        self.document
    }

    /// Walk every `person` statement in source order.
    pub fn persons(&self) -> impl Iterator<Item = &'a PersonStmt> + '_ {
        self.document.statements.iter().filter_map(|s| match s {
            Statement::Person(p) => Some(p),
            _ => None,
        })
    }

    /// Walk every `marriage` statement in source order.
    pub fn marriages(&self) -> impl Iterator<Item = &'a MarriageStmt> + '_ {
        self.document.statements.iter().filter_map(|s| match s {
            Statement::Marriage(m) => Some(m),
            _ => None,
        })
    }

    /// Look up a person by id.
    pub fn person(&self, id: &str) -> Option<&'a PersonStmt> {
        self.persons.get(id).copied()
    }

    /// Look up a marriage by id.
    pub fn marriage(&self, id: &str) -> Option<&'a MarriageStmt> {
        self.marriages.get(id).copied()
    }

    /// Look up an entity (person or marriage) by id, regardless of kind.
    /// Used by reference-resolution checks.
    pub fn entity(&self, id: &str) -> Option<EntityRef<'a>> {
        self.entities.get(id).copied()
    }

    /// The two declared spouses of a marriage, in declaration order, with
    /// unresolved spouses skipped (rule 2 reports them).
    ///
    /// Returns at most two persons; an empty iterator if both spouses are
    /// unresolved.
    pub fn spouses_of(
        &self,
        marriage: &'a MarriageStmt,
    ) -> impl Iterator<Item = &'a PersonStmt> + '_ {
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
    pub fn parents_of(&self, person: &'a PersonStmt) -> Vec<ParentLink<'a>> {
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
}

pub fn resolve(document: &Document) -> (ResolvedDocument<'_>, Vec<Diagnostic>) {
    let mut entities: HashMap<&str, EntityRef<'_>> = HashMap::new();
    let mut persons = HashMap::new();
    let mut marriages = HashMap::new();
    let mut diagnostics = Vec::new();

    for stmt in &document.statements {
        let (kind, id) = match stmt {
            Statement::Person(p) => (EntityKind::Person, &p.id),
            Statement::Marriage(m) => (EntityKind::Marriage, &m.id),
        };
        let key = id.name.as_str();
        match entities.get(key) {
            Some(prior) => {
                let prior_kind = prior.kind.as_str();
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
                        prior.span(),
                        format!("first declared here as a {prior_kind}"),
                    ),
                );
            }
            None => {
                entities.insert(key, EntityRef { kind, id });
            }
        }
        match stmt {
            Statement::Person(p) => {
                persons.entry(p.id.name.as_str()).or_insert(p);
            }
            Statement::Marriage(m) => {
                marriages.entry(m.id.name.as_str()).or_insert(m);
            }
        }
    }

    let resolved = ResolvedDocument {
        document,
        entities,
        persons,
        marriages,
    };

    diagnostics.extend(rule_02_unresolved_references(&resolved));

    (resolved, diagnostics)
}

/// Rule 2 — every marriage spouse, `birth` ref, and `adoption` ref must
/// resolve to a declared id of the appropriate kind.
///
/// Iterates the raw statement list so diagnostics are emitted in source
/// order; this is the only check that lives inside `semantic` because it
/// runs as part of `resolve` itself.
fn rule_02_unresolved_references(resolved: &ResolvedDocument<'_>) -> Vec<Diagnostic> {
    let mut out = Vec::new();
    for stmt in &resolved.document.statements {
        match stmt {
            Statement::Person(p) => {
                if let Some(birth) = &p.birth {
                    check_marriage_ref(resolved, &birth.marriage_ref, "birth", &mut out);
                }
                for adoption in &p.adoptions {
                    check_marriage_ref(resolved, &adoption.marriage_ref, "adoption", &mut out);
                }
            }
            Statement::Marriage(m) => {
                check_person_ref(resolved, &m.spouse_a, &mut out);
                check_person_ref(resolved, &m.spouse_b, &mut out);
            }
        }
    }
    out
}

fn check_person_ref(resolved: &ResolvedDocument<'_>, ident: &Ident, out: &mut Vec<Diagnostic>) {
    match resolved.entity(&ident.name) {
        None => {
            out.push(Diagnostic::error(
                "KULA-R02",
                format!(
                    "no person with id `{}` is declared in this file — check for a typo, or add a `person {} …` declaration",
                    ident.name, ident.name
                ),
                ident.span,
            ));
        }
        Some(EntityRef {
            kind: EntityKind::Person,
            ..
        }) => {}
        Some(EntityRef {
            kind: EntityKind::Marriage,
            id: prior,
        }) => {
            out.push(
                Diagnostic::error(
                    "KULA-R02",
                    format!(
                        "`{}` is a marriage, not a person — spouses must reference declared persons",
                        ident.name
                    ),
                    ident.span,
                )
                .with_related(prior.span, format!("`{}` declared as a marriage here", ident.name)),
            );
        }
    }
}

fn check_marriage_ref(
    resolved: &ResolvedDocument<'_>,
    ident: &Ident,
    role: &str,
    out: &mut Vec<Diagnostic>,
) {
    match resolved.entity(&ident.name) {
        None => {
            out.push(Diagnostic::error(
                "KULA-R02",
                format!(
                    "no marriage with id `{}` is declared in this file — check for a typo, or add a `marriage {} …` declaration (the `{role}` link expects a marriage id)",
                    ident.name, ident.name
                ),
                ident.span,
            ));
        }
        Some(EntityRef {
            kind: EntityKind::Marriage,
            ..
        }) => {}
        Some(EntityRef {
            kind: EntityKind::Person,
            id: prior,
        }) => {
            out.push(
                Diagnostic::error(
                    "KULA-R02",
                    format!(
                        "`{}` is a person, not a marriage — `{role}` links must reference a marriage id",
                        ident.name
                    ),
                    ident.span,
                )
                .with_related(prior.span, format!("`{}` declared as a person here", ident.name)),
            );
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::lexer::tokenize;
    use crate::parser::parse;

    fn resolve_str(source: &str) -> (Document, Vec<ByteSpan>) {
        let tokens = tokenize(source);
        let (document, _) = parse(&tokens);
        // We move `document` into a slot the caller can hold; then re-resolve
        // for queries.
        (document, Vec::new())
    }

    fn refs(source: &str, id: &str, kind: EntityKind) -> Vec<(usize, usize)> {
        let tokens = tokenize(source);
        let (document, _) = parse(&tokens);
        let (resolved, _) = resolve(&document);
        resolved
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
        let _ = resolve_str(src);
        assert!(refs(src, "m", EntityKind::Person).is_empty());
        // …but as a marriage, it has one ref (the `birth`).
        assert_eq!(refs(src, "m", EntityKind::Marriage).len(), 1);
    }
}
