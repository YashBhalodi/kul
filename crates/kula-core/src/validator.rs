//! Validator: runs spec rules against a [`ResolvedDocument`].
//!
//! Each rule is a small function `rule_NN(...) -> Vec<Diagnostic>` taking a
//! resolved document and returning diagnostics. The top-level [`validate`]
//! is the composition of every implemented rule. Slices ship one rule at a
//! time across Phase 2.

use crate::ast::{AdoptionFieldKind, MarriageFieldKind, MarriageStmt, PersonFieldKind, Statement};
use crate::date::{DateLit, before_strict};
use crate::diagnostic::Diagnostic;
use crate::semantic::ResolvedDocument;

pub fn validate(resolved: &ResolvedDocument<'_>) -> Vec<Diagnostic> {
    let mut diagnostics = Vec::new();
    diagnostics.extend(rule_03_required_fields(resolved));
    diagnostics.extend(rule_04_self_marriage(resolved));
    diagnostics.extend(rule_06_died_before_born(resolved));
    diagnostics.extend(rule_07_marriage_end_before_start(resolved));
    diagnostics.extend(rule_08_adoption_end_before_start(resolved));
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
                        PersonFieldKind::Family(_)
                        | PersonFieldKind::Given(_)
                        | PersonFieldKind::Born(_)
                        | PersonFieldKind::Died(_) => {}
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

/// Rule 6 — `person.died < person.born`.
pub fn rule_06_died_before_born(resolved: &ResolvedDocument<'_>) -> Vec<Diagnostic> {
    let mut out = Vec::new();
    for stmt in &resolved.document.statements {
        if let Statement::Person(p) = stmt {
            let mut born: Option<&DateLit> = None;
            let mut died: Option<&DateLit> = None;
            for field in &p.fields {
                match &field.kind {
                    PersonFieldKind::Born(d) => born = Some(d),
                    PersonFieldKind::Died(d) => died = Some(d),
                    _ => {}
                }
            }
            if let (Some(born), Some(died)) = (born, died)
                && before_strict(died, born)
            {
                out.push(
                    Diagnostic::error(
                        "KULA-R06",
                        format!("person `{}` died before they were born", p.id.name),
                        died.span,
                    )
                    .with_related(born.span, "born here"),
                );
            }
        }
    }
    out
}

/// Rule 7 — `marriage.end < marriage.start`.
pub fn rule_07_marriage_end_before_start(resolved: &ResolvedDocument<'_>) -> Vec<Diagnostic> {
    let mut out = Vec::new();
    for stmt in &resolved.document.statements {
        if let Statement::Marriage(m) = stmt {
            let mut start: Option<&DateLit> = None;
            let mut end: Option<&DateLit> = None;
            for field in &m.fields {
                match &field.kind {
                    MarriageFieldKind::Start(d) => start = Some(d),
                    MarriageFieldKind::End(d) => end = Some(d),
                    MarriageFieldKind::EndReason(_) => {}
                }
            }
            if let (Some(start), Some(end)) = (start, end)
                && before_strict(end, start)
            {
                out.push(
                    Diagnostic::error(
                        "KULA-R07",
                        format!("marriage `{}` ended before it began", m.id.name),
                        end.span,
                    )
                    .with_related(start.span, "started here"),
                );
            }
        }
    }
    out
}

/// Rule 8 — `adoption.end < adoption.start`.
pub fn rule_08_adoption_end_before_start(resolved: &ResolvedDocument<'_>) -> Vec<Diagnostic> {
    let mut out = Vec::new();
    for stmt in &resolved.document.statements {
        if let Statement::Person(p) = stmt {
            for adoption in &p.adoptions {
                let mut start: Option<&DateLit> = None;
                let mut end: Option<&DateLit> = None;
                for field in &adoption.fields {
                    match &field.kind {
                        AdoptionFieldKind::Start(d) => start = Some(d),
                        AdoptionFieldKind::End(d) => end = Some(d),
                    }
                }
                if let (Some(start), Some(end)) = (start, end)
                    && before_strict(end, start)
                {
                    out.push(
                        Diagnostic::error(
                            "KULA-R08",
                            format!(
                                "adoption of `{}` (by marriage `{}`) ended before it began",
                                p.id.name, adoption.marriage_ref.name
                            ),
                            end.span,
                        )
                        .with_related(start.span, "started here"),
                    );
                }
            }
        }
    }
    out
}
