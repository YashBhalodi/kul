//! Validator: runs spec rules against a [`ResolvedDocument`].
//!
//! Each rule is a small function `rule_NN(...) -> Vec<Diagnostic>` taking a
//! resolved document and returning diagnostics. The top-level [`validate`]
//! is the composition of every implemented rule. Slices ship one rule at a
//! time across Phase 2.

use crate::ast::{PersonFieldKind, Statement};
use crate::diagnostic::Diagnostic;
use crate::semantic::ResolvedDocument;

pub fn validate(resolved: &ResolvedDocument<'_>) -> Vec<Diagnostic> {
    let mut diagnostics = Vec::new();
    diagnostics.extend(rule_03_required_fields(resolved));
    diagnostics
}

/// Rule 3 — required fields missing on `person` (or `marriage`).
///
/// A `person` MUST have `name` and `gender`. A `marriage` MUST have both
/// spouses and `start`. Marriage support lands with the marriage slice (#8).
pub fn rule_03_required_fields(resolved: &ResolvedDocument<'_>) -> Vec<Diagnostic> {
    let mut out = Vec::new();
    for stmt in &resolved.document.statements {
        match stmt {
            Statement::Person(p) => {
                let mut has_name = false;
                let mut has_gender = false;
                for field in &p.fields {
                    match field.kind {
                        PersonFieldKind::Name(_) => has_name = true,
                        PersonFieldKind::Gender(_) => has_gender = true,
                    }
                }
                if !has_name {
                    out.push(Diagnostic::error(
                        "KULA-R03",
                        format!("person `{}` is missing required field `name`", p.id.name),
                        p.id.span,
                    ));
                }
                if !has_gender {
                    out.push(Diagnostic::error(
                        "KULA-R03",
                        format!("person `{}` is missing required field `gender`", p.id.name),
                        p.id.span,
                    ));
                }
            }
        }
    }
    out
}
