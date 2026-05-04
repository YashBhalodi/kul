//! Validator: runs spec rules against a [`ResolvedDocument`].
//!
//! Each rule is a small function `rule_NN(...) -> Vec<Diagnostic>` taking a
//! resolved document and returning diagnostics. The top-level [`validate`]
//! is the composition of every implemented rule. Slices ship one rule at a
//! time across Phase 2.

use crate::ast::{
    AdoptionFieldKind, AdoptionSub, EndReason, MarriageFieldKind, MarriageStmt, PersonFieldKind,
    PersonStmt, Statement,
};
use crate::date::{DateLit, before_strict};
use crate::diagnostic::Diagnostic;
use crate::semantic::ResolvedDocument;
use crate::span::ByteSpan;

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

/// Rule 13 — no person may appear as their own ancestor in the parent graph
/// (union of bio and adoptive parent links).
pub fn rule_13_parenthood_cycles(resolved: &ResolvedDocument<'_>) -> Vec<Diagnostic> {
    let mut out = Vec::new();
    for cycle in crate::cycles::find_cycles(resolved) {
        let head = cycle
            .members
            .first()
            .copied()
            .expect("cycle must have at least one member");
        let head_person = resolved
            .person(head)
            .expect("cycle member is a declared person");
        let chain = if cycle.members.len() == 1 {
            format!("`{head}` is their own ancestor")
        } else {
            let mut parts: Vec<String> = cycle.members.iter().map(|m| format!("`{m}`")).collect();
            parts.push(format!("`{head}`"));
            format!("parent cycle: {}", parts.join(" → "))
        };
        let mut diag = Diagnostic::error(
            "KULA-R13",
            format!("parenthood cycle detected — {chain}"),
            head_person.id.span,
        );
        for span in cycle.link_spans {
            diag = diag.with_related(span, "parent link in this cycle");
        }
        out.push(diag);
    }
    out
}

fn person_born(p: &PersonStmt) -> Option<&DateLit> {
    p.fields.iter().find_map(|f| match &f.kind {
        PersonFieldKind::Born(d) => Some(d),
        _ => None,
    })
}

fn person_died(p: &PersonStmt) -> Option<&DateLit> {
    p.fields.iter().find_map(|f| match &f.kind {
        PersonFieldKind::Died(d) => Some(d),
        _ => None,
    })
}

fn marriage_start(m: &MarriageStmt) -> Option<&DateLit> {
    m.fields.iter().find_map(|f| match &f.kind {
        MarriageFieldKind::Start(d) => Some(d),
        _ => None,
    })
}

fn adoption_start(a: &AdoptionSub) -> Option<&DateLit> {
    a.fields.iter().find_map(|f| match &f.kind {
        AdoptionFieldKind::Start(d) => Some(d),
        _ => None,
    })
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

/// Rule 5 — `end` and `end_reason` must both be present or both absent.
/// Rule 5b (KULA-R05b) — `end_reason` value must be in the v1 vocabulary
/// (currently just `divorce`).
pub fn rule_05_end_consistency(resolved: &ResolvedDocument<'_>) -> Vec<Diagnostic> {
    let mut out = Vec::new();
    for stmt in &resolved.document.statements {
        if let Statement::Marriage(m) = stmt {
            let mut end_span: Option<ByteSpan> = None;
            let mut end_reason_span: Option<ByteSpan> = None;
            let mut end_reason_field_span: Option<ByteSpan> = None;
            let mut end_reason_value: Option<&EndReason> = None;
            for field in &m.fields {
                match &field.kind {
                    MarriageFieldKind::End(d) => end_span = Some(d.span),
                    MarriageFieldKind::EndReason(v) => {
                        end_reason_span = Some(v.span);
                        end_reason_field_span = Some(field.span);
                        end_reason_value = Some(&v.value);
                    }
                    MarriageFieldKind::Start(_) => {}
                }
            }
            match (end_span, end_reason_span) {
                (Some(end), None) => {
                    out.push(Diagnostic::error(
                        "KULA-R05",
                        format!(
                            "marriage `{}` has `end` without `end_reason`; add `end_reason:divorce`",
                            m.id.name
                        ),
                        end,
                    ));
                }
                (None, Some(_)) => {
                    let span = end_reason_field_span.expect("set with reason span");
                    out.push(Diagnostic::error(
                        "KULA-R05",
                        format!(
                            "marriage `{}` has `end_reason` without `end`; remove this field or add a matching `end:` date",
                            m.id.name
                        ),
                        span,
                    ));
                }
                _ => {}
            }
            if let (Some(EndReason::Unknown(raw)), Some(span)) = (end_reason_value, end_reason_span)
            {
                out.push(Diagnostic::error(
                    "KULA-R05b",
                    format!(
                        "`end_reason:{raw}` is not a recognized v1 value; the v1 vocabulary is `divorce`"
                    ),
                    span,
                ));
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

/// Rule 9 — `marriage.start < S.born` for either spouse `S`.
pub fn rule_09_marriage_before_spouse_born(resolved: &ResolvedDocument<'_>) -> Vec<Diagnostic> {
    let mut out = Vec::new();
    for stmt in &resolved.document.statements {
        if let Statement::Marriage(m) = stmt {
            let Some(start) = marriage_start(m) else {
                continue;
            };
            for spouse_id in [&m.spouse_a, &m.spouse_b] {
                let Some(spouse) = resolved.person(&spouse_id.name) else {
                    continue;
                };
                let Some(born) = person_born(spouse) else {
                    continue;
                };
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
    }
    out
}

/// Rule 10 — `marriage.start > S.died` for either spouse `S`.
pub fn rule_10_spouse_died_before_marriage(resolved: &ResolvedDocument<'_>) -> Vec<Diagnostic> {
    let mut out = Vec::new();
    for stmt in &resolved.document.statements {
        if let Statement::Marriage(m) = stmt {
            let Some(start) = marriage_start(m) else {
                continue;
            };
            for spouse_id in [&m.spouse_a, &m.spouse_b] {
                let Some(spouse) = resolved.person(&spouse_id.name) else {
                    continue;
                };
                let Some(died) = person_died(spouse) else {
                    continue;
                };
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
    }
    out
}

/// Rule 11 — bio child born before either bio parent. Parents are the
/// spouses of the marriage referenced by the child's `birth` sub-statement.
pub fn rule_11_bio_child_born_before_parent(resolved: &ResolvedDocument<'_>) -> Vec<Diagnostic> {
    let mut out = Vec::new();
    for stmt in &resolved.document.statements {
        if let Statement::Person(child) = stmt {
            let Some(birth) = &child.birth else {
                continue;
            };
            let Some(child_born) = person_born(child) else {
                continue;
            };
            let Some(marriage) = resolved.marriage(&birth.marriage_ref.name) else {
                continue;
            };
            for parent_id in [&marriage.spouse_a, &marriage.spouse_b] {
                let Some(parent) = resolved.person(&parent_id.name) else {
                    continue;
                };
                let Some(parent_born) = person_born(parent) else {
                    continue;
                };
                if before_strict(child_born, parent_born) {
                    out.push(
                        Diagnostic::error(
                            "KULA-R11",
                            format!(
                                "bio child `{}` was born before parent `{}`",
                                child.id.name, parent.id.name
                            ),
                            child_born.span,
                        )
                        .with_related(parent_born.span, format!("`{}` born here", parent.id.name)),
                    );
                }
            }
        }
    }
    out
}

/// Rule 12 — `adoption.start < P.born` for either adoptive parent `P`.
pub fn rule_12_adoption_before_adopter_born(resolved: &ResolvedDocument<'_>) -> Vec<Diagnostic> {
    let mut out = Vec::new();
    for stmt in &resolved.document.statements {
        if let Statement::Person(child) = stmt {
            for adoption in &child.adoptions {
                let Some(start) = adoption_start(adoption) else {
                    continue;
                };
                let Some(marriage) = resolved.marriage(&adoption.marriage_ref.name) else {
                    continue;
                };
                for parent_id in [&marriage.spouse_a, &marriage.spouse_b] {
                    let Some(parent) = resolved.person(&parent_id.name) else {
                        continue;
                    };
                    let Some(parent_born) = person_born(parent) else {
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
                            .with_related(
                                parent_born.span,
                                format!("`{}` born here", parent.id.name),
                            ),
                        );
                    }
                }
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
