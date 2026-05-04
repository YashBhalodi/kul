//! Validator: runs spec rules against a [`ResolvedDocument`].
//!
//! Each rule is a small function `rule_NN(...) -> Vec<Diagnostic>` taking a
//! resolved document and returning diagnostics. The top-level [`validate`]
//! is the composition of every implemented rule. Slices ship one rule at a
//! time across Phase 2.

use crate::ast::{MarriageFieldKind, MarriageStmt, PersonFieldKind, Statement};
use crate::diagnostic::Diagnostic;
use crate::semantic::ResolvedDocument;

pub fn validate(resolved: &ResolvedDocument<'_>) -> Vec<Diagnostic> {
    let mut diagnostics = Vec::new();
    diagnostics.extend(rule_03_required_fields(resolved));
    diagnostics.extend(rule_04_self_marriage(resolved));
    diagnostics
}

/// Rule 3 — required fields missing.
///
/// A `person` MUST have `name` and `gender`.
/// A `marriage` MUST have `start`. (The two spouses are positional and
/// enforced by the grammar — a missing spouse is a parse error.)
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
            Statement::Marriage(m) => {
                let has_start = m
                    .fields
                    .iter()
                    .any(|f| matches!(f.kind, MarriageFieldKind::Start(_)));
                if !has_start {
                    out.push(Diagnostic::error(
                        "KULA-R03",
                        format!("marriage `{}` is missing required field `start`", m.id.name),
                        m.id.span,
                    ));
                }
            }
        }
    }
    out
}

/// Rule 4 — self-marriage. A marriage's two spouse identifiers must be
/// distinct.
pub fn rule_04_self_marriage(resolved: &ResolvedDocument<'_>) -> Vec<Diagnostic> {
    let mut out = Vec::new();
    for stmt in &resolved.document.statements {
        if let Statement::Marriage(m) = stmt {
            if m.spouse_a.name == m.spouse_b.name {
                out.push(self_marriage_diagnostic(m));
            }
        }
    }
    out
}

fn self_marriage_diagnostic(m: &MarriageStmt) -> Diagnostic {
    Diagnostic::error(
        "KULA-R04",
        format!(
            "marriage `{}` has the same person `{}` as both spouses; spouses must be distinct",
            m.id.name, m.spouse_a.name
        ),
        m.spouse_b.span,
    )
    .with_related(m.spouse_a.span, "first spouse")
}
