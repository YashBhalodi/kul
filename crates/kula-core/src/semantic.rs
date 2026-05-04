//! Semantic analysis: turns a [`Document`] into a [`ResolvedDocument`].
//!
//! Phase 2 fills this module out across multiple slices:
//!
//! - #7: trivial pass-through.
//! - #8: ID index across persons and marriages; rule 1 (duplicate ID).
//! - #9: reference resolution; rule 2 (unresolved reference).
//! - #11/#13: parent-graph queries.

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
#[derive(Debug, Clone)]
pub struct ResolvedDocument<'a> {
    pub document: &'a Document,
    /// First-seen entity per ID.
    pub entities: HashMap<&'a str, EntityRef<'a>>,
    pub persons: HashMap<&'a str, &'a PersonStmt>,
    pub marriages: HashMap<&'a str, &'a MarriageStmt>,
}

impl<'a> ResolvedDocument<'a> {
    pub fn person(&self, id: &str) -> Option<&'a PersonStmt> {
        self.persons.get(id).copied()
    }

    pub fn marriage(&self, id: &str) -> Option<&'a MarriageStmt> {
        self.marriages.get(id).copied()
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
                diagnostics.push(
                    Diagnostic::error(
                        "KULA-R01",
                        format!(
                            "duplicate id `{}`: this {} re-declares an id already used by a {}",
                            id.name,
                            kind.as_str(),
                            prior.kind.as_str()
                        ),
                        id.span,
                    )
                    .with_related(prior.span(), "prior declaration"),
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
    match resolved.entities.get(ident.name.as_str()) {
        None => {
            out.push(Diagnostic::error(
                "KULA-R02",
                format!(
                    "unresolved reference: spouse `{}` is not a declared person",
                    ident.name
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
                        "wrong-kind reference: spouse `{}` resolves to a marriage, not a person",
                        ident.name
                    ),
                    ident.span,
                )
                .with_related(prior.span, "marriage declared here"),
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
    match resolved.entities.get(ident.name.as_str()) {
        None => {
            out.push(Diagnostic::error(
                "KULA-R02",
                format!(
                    "unresolved reference: {role} marriage `{}` is not a declared marriage",
                    ident.name
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
                        "wrong-kind reference: {role} `{}` resolves to a person, not a marriage",
                        ident.name
                    ),
                    ident.span,
                )
                .with_related(prior.span, "person declared here"),
            );
        }
    }
}
