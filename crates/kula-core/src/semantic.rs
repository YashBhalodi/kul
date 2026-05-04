//! Semantic analysis: turns a [`Document`] into a [`ResolvedDocument`].
//!
//! Phase 2 fills this module out across multiple slices. For the tracer
//! bullet (#7) the resolved document is a thin pass-through: an index of
//! declared persons keyed by id. Reference resolution, ID-uniqueness, and
//! cross-entity queries land in #8 and #9.

use std::collections::HashMap;

use crate::ast::{Document, PersonStmt, Statement};
use crate::diagnostic::Diagnostic;

/// A document with semantic information attached.
#[derive(Debug, Clone)]
pub struct ResolvedDocument<'a> {
    pub document: &'a Document,
    pub persons: HashMap<&'a str, &'a PersonStmt>,
}

pub fn resolve(document: &Document) -> (ResolvedDocument<'_>, Vec<Diagnostic>) {
    let mut persons = HashMap::new();
    for stmt in &document.statements {
        match stmt {
            Statement::Person(p) => {
                persons.entry(p.id.name.as_str()).or_insert(p);
            }
        }
    }
    (ResolvedDocument { document, persons }, Vec::new())
}
