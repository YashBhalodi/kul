//! Semantic analysis: turns a [`Document`] into a [`ResolvedDocument`].
//!
//! Phase 2 fills this module out across multiple slices:
//!
//! - #7: trivial pass-through.
//! - #8: ID index across persons and marriages; rule 1 (duplicate ID).
//! - #9 onwards: reference resolution, parent graph, etc.

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
    /// First-seen entity per ID. The validator uses this for cross-references.
    pub entities: HashMap<&'a str, EntityRef<'a>>,
    pub persons: HashMap<&'a str, &'a PersonStmt>,
    pub marriages: HashMap<&'a str, &'a MarriageStmt>,
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

    (
        ResolvedDocument {
            document,
            entities,
            persons,
            marriages,
        },
        diagnostics,
    )
}
