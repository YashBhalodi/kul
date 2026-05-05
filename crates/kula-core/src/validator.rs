//! Validator: runs spec rules against a [`ResolvedDocument`].
//!
//! Each rule is a small function `rule_NN(...) -> Vec<Diagnostic>` taking a
//! resolved document and returning diagnostics. The top-level [`validate`]
//! is the composition of every implemented rule. Rules query the document
//! through [`ResolvedDocument`]'s typed methods — they never enumerate
//! `document.statements` themselves.

use crate::ast::{EndReason, MarriageStmt};
use crate::date::before_strict;
use crate::diagnostic::Diagnostic;
use crate::lexer::FieldName;
use crate::semantic::ResolvedDocument;

pub fn validate(resolved: &ResolvedDocument<'_>) -> Vec<Diagnostic> {
    let mut diagnostics = Vec::new();
    diagnostics.extend(rule_03_required_fields(resolved));
    diagnostics.extend(rule_04_self_marriage(resolved));
    diagnostics.extend(rule_05_end_consistency(resolved));
    diagnostics.extend(rule_06_died_before_born(resolved));
    diagnostics.extend(rule_07_marriage_end_before_start(resolved));
    diagnostics.extend(rule_08_adoption_end_before_start(resolved));
    diagnostics.extend(rule_09_marriage_before_spouse_born(resolved));
    diagnostics.extend(rule_10_spouse_died_before_marriage(resolved));
    diagnostics.extend(rule_11_bio_child_born_before_parent(resolved));
    diagnostics.extend(rule_12_adoption_before_adopter_born(resolved));
    diagnostics.extend(rule_13_parenthood_cycles(resolved));
    diagnostics
}

/// Rule 3 — required fields missing.
///
/// A `person` MUST have `name` and `gender`.
/// A `marriage` MUST have `start`. (The two spouses are positional and
/// enforced by the grammar — a missing spouse is a parse error.)
pub fn rule_03_required_fields(resolved: &ResolvedDocument<'_>) -> Vec<Diagnostic> {
    let mut out = Vec::new();
    for p in resolved.persons() {
        if !p.has_field(FieldName::Name) {
            out.push(Diagnostic::error(
                "KULA-R03",
                format!(
                    "person `{}` needs a `name:` field — add `name:\"…\"` to the declaration",
                    p.id.name
                ),
                p.id.span,
            ));
        }
        if !p.has_field(FieldName::Gender) {
            out.push(Diagnostic::error(
                "KULA-R03",
                format!(
                    "person `{}` needs a `gender:` field — use `gender:male`, `gender:female`, or `gender:other`",
                    p.id.name
                ),
                p.id.span,
            ));
        }
    }
    for m in resolved.marriages() {
        if m.start().is_none() {
            out.push(Diagnostic::error(
                "KULA-R03",
                format!(
                    "marriage `{}` needs a `start:` date — add `start:YYYY` (or a fuller `YYYY-MM-DD`)",
                    m.id.name
                ),
                m.id.span,
            ));
        }
    }
    out
}

/// Rule 4 — self-marriage. A marriage's two spouse identifiers must be
/// distinct.
pub fn rule_04_self_marriage(resolved: &ResolvedDocument<'_>) -> Vec<Diagnostic> {
    let mut out = Vec::new();
    for m in resolved.marriages() {
        if m.spouse_a.name == m.spouse_b.name {
            out.push(self_marriage_diagnostic(m));
        }
    }
    out
}

fn self_marriage_diagnostic(m: &MarriageStmt) -> Diagnostic {
    Diagnostic::error(
        "KULA-R04",
        format!(
            "marriage `{}` lists `{}` as both spouses — spouses must be distinct people",
            m.id.name, m.spouse_a.name
        ),
        m.spouse_b.span,
    )
    .with_related(m.spouse_a.span, "first spouse listed here")
}

/// Rule 5 — `end` and `end_reason` must both be present or both absent.
/// Rule 5b (KULA-R05b) — `end_reason` value must be in the v1 vocabulary
/// (currently just `divorce`).
pub fn rule_05_end_consistency(resolved: &ResolvedDocument<'_>) -> Vec<Diagnostic> {
    let mut out = Vec::new();
    for m in resolved.marriages() {
        match (m.end(), m.end_reason()) {
            (Some(end), None) => {
                out.push(Diagnostic::error(
                    "KULA-R05",
                    format!(
                        "marriage `{}` has an `end:` date but no `end_reason:` — add `end_reason:divorce`",
                        m.id.name
                    ),
                    end.span,
                ));
            }
            (None, Some(reason)) => {
                // Anchor on the field's full span (including the `end_reason:`
                // keyword), not the value alone, so the suggestion to remove it
                // covers the right text.
                let field_span = m
                    .fields
                    .iter()
                    .find(|f| matches!(f.kind, crate::ast::MarriageFieldKind::EndReason(_)))
                    .map(|f| f.span)
                    .unwrap_or(reason.span);
                out.push(Diagnostic::error(
                    "KULA-R05",
                    format!(
                        "marriage `{}` has an `end_reason:` but no `end:` date — add an `end:` date or remove this field",
                        m.id.name
                    ),
                    field_span,
                ));
            }
            _ => {}
        }
        if let Some(reason) = m.end_reason()
            && let EndReason::Unknown(raw) = &reason.value
        {
            out.push(Diagnostic::error(
                "KULA-R05b",
                format!("`end_reason:{raw}` isn't recognized — the only value in v1 is `divorce`"),
                reason.span,
            ));
        }
    }
    out
}

/// Rule 6 — `person.died < person.born`.
pub fn rule_06_died_before_born(resolved: &ResolvedDocument<'_>) -> Vec<Diagnostic> {
    let mut out = Vec::new();
    for p in resolved.persons() {
        if let (Some(born), Some(died)) = (p.born(), p.died())
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
    out
}

/// Rule 7 — `marriage.end < marriage.start`.
pub fn rule_07_marriage_end_before_start(resolved: &ResolvedDocument<'_>) -> Vec<Diagnostic> {
    let mut out = Vec::new();
    for m in resolved.marriages() {
        if let (Some(start), Some(end)) = (m.start(), m.end())
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
    out
}

/// Rule 8 — `adoption.end < adoption.start`.
pub fn rule_08_adoption_end_before_start(resolved: &ResolvedDocument<'_>) -> Vec<Diagnostic> {
    let mut out = Vec::new();
    for p in resolved.persons() {
        for adoption in &p.adoptions {
            if let (Some(start), Some(end)) = (adoption.start(), adoption.end())
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
    out
}

/// Rule 9 — `marriage.start < S.born` for either spouse `S`.
pub fn rule_09_marriage_before_spouse_born(resolved: &ResolvedDocument<'_>) -> Vec<Diagnostic> {
    let mut out = Vec::new();
    for m in resolved.marriages() {
        let Some(start) = m.start() else { continue };
        for spouse in resolved.spouses_of(m) {
            let Some(born) = spouse.born() else { continue };
            if before_strict(start, born) {
                out.push(
                    Diagnostic::error(
                        "KULA-R09",
                        format!(
                            "marriage `{}` started before spouse `{}` was born",
                            m.id.name, spouse.id.name
                        ),
                        start.span,
                    )
                    .with_related(born.span, format!("`{}` born here", spouse.id.name)),
                );
            }
        }
    }
    out
}

/// Rule 10 — `marriage.start > S.died` for either spouse `S`.
pub fn rule_10_spouse_died_before_marriage(resolved: &ResolvedDocument<'_>) -> Vec<Diagnostic> {
    let mut out = Vec::new();
    for m in resolved.marriages() {
        let Some(start) = m.start() else { continue };
        for spouse in resolved.spouses_of(m) {
            let Some(died) = spouse.died() else { continue };
            if before_strict(died, start) {
                out.push(
                    Diagnostic::error(
                        "KULA-R10",
                        format!(
                            "marriage `{}` started after spouse `{}` had already died",
                            m.id.name, spouse.id.name
                        ),
                        start.span,
                    )
                    .with_related(died.span, format!("`{}` died here", spouse.id.name)),
                );
            }
        }
    }
    out
}

/// Rule 11 — bio child born before either bio parent. Parents are the
/// spouses of the marriage referenced by the child's `birth` sub-statement.
pub fn rule_11_bio_child_born_before_parent(resolved: &ResolvedDocument<'_>) -> Vec<Diagnostic> {
    let mut out = Vec::new();
    for child in resolved.persons() {
        let Some(birth) = &child.birth else { continue };
        let Some(child_born) = child.born() else {
            continue;
        };
        let Some(marriage) = resolved.marriage(&birth.marriage_ref.name) else {
            continue;
        };
        for parent in resolved.spouses_of(marriage) {
            let Some(parent_born) = parent.born() else {
                continue;
            };
            if before_strict(child_born, parent_born) {
                out.push(
                    Diagnostic::error(
                        "KULA-R11",
                        format!(
                            "`{}` was born before their biological parent `{}`",
                            child.id.name, parent.id.name
                        ),
                        child_born.span,
                    )
                    .with_related(parent_born.span, format!("`{}` born here", parent.id.name)),
                );
            }
        }
    }
    out
}

/// Rule 12 — `adoption.start < P.born` for either adoptive parent `P`.
pub fn rule_12_adoption_before_adopter_born(resolved: &ResolvedDocument<'_>) -> Vec<Diagnostic> {
    let mut out = Vec::new();
    for child in resolved.persons() {
        for adoption in &child.adoptions {
            let Some(start) = adoption.start() else {
                continue;
            };
            let Some(marriage) = resolved.marriage(&adoption.marriage_ref.name) else {
                continue;
            };
            for parent in resolved.spouses_of(marriage) {
                let Some(parent_born) = parent.born() else {
                    continue;
                };
                if before_strict(start, parent_born) {
                    out.push(
                        Diagnostic::error(
                            "KULA-R12",
                            format!(
                                "adoption of `{}` started before adoptive parent `{}` was born",
                                child.id.name, parent.id.name
                            ),
                            start.span,
                        )
                        .with_related(parent_born.span, format!("`{}` born here", parent.id.name)),
                    );
                }
            }
        }
    }
    out
}

/// Rule 13 — no person may appear as their own ancestor in the parent graph
/// (union of bio and adoptive parent links).
pub fn rule_13_parenthood_cycles(resolved: &ResolvedDocument<'_>) -> Vec<Diagnostic> {
    let mut out = Vec::new();
    for cycle in crate::cycles::find_cycles(resolved) {
        let head = *cycle
            .members
            .first()
            .expect("cycle must have at least one member");
        let message = if cycle.members.len() == 1 {
            format!(
                "`{}` ends up as their own ancestor — check the `birth` and `adoption` links",
                head.id.name
            )
        } else {
            let mut parts: Vec<String> = cycle
                .members
                .iter()
                .map(|m| format!("`{}`", m.id.name))
                .collect();
            parts.push(format!("`{}`", head.id.name));
            format!(
                "parent-link cycle: {} — one of these `birth` or `adoption` links must be wrong",
                parts.join(" → ")
            )
        };
        let mut diag = Diagnostic::error("KULA-R13", message, head.id.span);
        for span in cycle.link_spans {
            diag = diag.with_related(span, "parent link in the cycle");
        }
        out.push(diag);
    }
    out
}
