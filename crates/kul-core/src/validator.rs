//! Validator: spec rules against a [`ResolvedDocument`].
//!
//! Each rule is a `rule_NN(...) -> Vec<Diagnostic>`. Rules query through
//! [`ResolvedDocument`]'s typed methods — they never enumerate
//! `document.statements` themselves. Rules iterate files one at a time
//! for source-order grouping, but cross-reference lookups are
//! project-wide (ADR-0015). R13 walks the whole project's parent graph
//! in one pass.

use crate::ast::{EndReason, Gender, Ident, MarriageStmt, PersonStmt, Statement};
use crate::date::{DateLit, before_strict};
use crate::diagnostic::{Diagnostic, detail, fspan};
use crate::lexer::FieldName;
use crate::semantic::{EntityKind, EntityRef, ResolvedDocument};
use crate::span::{ByteSpan, FileId};

pub fn validate(resolved: &ResolvedDocument) -> Vec<Diagnostic> {
    let mut diagnostics = Vec::new();
    for file in resolved.document().kul_file_ids() {
        // R02 first: downstream rules trust that resolved spouse/parent
        // links exist. Diagnostic order R02→R03+ is part of the contract.
        diagnostics.extend(rule_02_unresolved_references(resolved, file));
        diagnostics.extend(rule_03_required_fields(resolved, file));
        diagnostics.extend(rule_04_self_marriage(resolved, file));
        diagnostics.extend(rule_05_end_consistency(resolved, file));
        diagnostics.extend(rule_06_died_before_born(resolved, file));
        diagnostics.extend(rule_07_marriage_end_before_start(resolved, file));
        diagnostics.extend(rule_08_adoption_end_before_start(resolved, file));
        diagnostics.extend(rule_09_marriage_before_spouse_born(resolved, file));
        diagnostics.extend(rule_10_spouse_died_before_marriage(resolved, file));
        diagnostics.extend(rule_11_bio_child_born_before_parent(resolved, file));
        diagnostics.extend(rule_12_adoption_before_adopter_born(resolved, file));
        diagnostics.extend(rule_15_duplicate_field(resolved, file));
    }
    // R13/R14 walk project-wide so cross-file cycles and hubs are
    // reported as single violations (ADR-0015, ADR-0020).
    diagnostics.extend(rule_13_parenthood_cycles(resolved));
    diagnostics.extend(rule_14_polygamy_hub_must_host(resolved));
    diagnostics
}

/// R02 — every spouse / `birth` ref / `adoption` ref must resolve to a
/// declared id of the correct kind. Project-wide (ADR-0015).
pub fn rule_02_unresolved_references(resolved: &ResolvedDocument, file: FileId) -> Vec<Diagnostic> {
    let mut out = Vec::new();
    for stmt in resolved.statements_in(file) {
        match stmt {
            Statement::Person(p) => {
                if let Some(birth) = &p.birth {
                    check_marriage_ref(resolved, file, &birth.marriage_ref, "birth", &mut out);
                }
                for adoption in &p.adoptions {
                    check_marriage_ref(
                        resolved,
                        file,
                        &adoption.marriage_ref,
                        "adoption",
                        &mut out,
                    );
                }
            }
            Statement::Marriage(m) => {
                check_person_ref(resolved, file, &m.spouse_a, &mut out);
                check_person_ref(resolved, file, &m.spouse_b, &mut out);
            }
        }
    }
    out
}

fn check_person_ref(
    resolved: &ResolvedDocument,
    file: FileId,
    ident: &Ident,
    out: &mut Vec<Diagnostic>,
) {
    match resolved.entity(&ident.name) {
        None => {
            out.push(Diagnostic::error(
                "KUL-R02",
                format!(
                    "no person with id `{}` is declared in the project — check for a typo, or add a `person {} …` declaration",
                    ident.name, ident.name
                ),
                fspan(file, ident.span),
            ));
        }
        Some(EntityRef {
            kind: EntityKind::Person,
            ..
        }) => {}
        Some(EntityRef {
            kind: EntityKind::Marriage,
            id: prior,
            file: prior_file,
        }) => {
            out.push(
                Diagnostic::error(
                    "KUL-R02",
                    format!(
                        "`{}` is a marriage, not a person — spouses must reference declared persons",
                        ident.name
                    ),
                    fspan(file, ident.span),
                )
                .with_related(
                    fspan(prior_file, prior.span),
                    format!("`{}` declared as a marriage here", ident.name),
                ),
            );
        }
    }
}

fn check_marriage_ref(
    resolved: &ResolvedDocument,
    file: FileId,
    ident: &Ident,
    role: &str,
    out: &mut Vec<Diagnostic>,
) {
    match resolved.entity(&ident.name) {
        None => {
            out.push(Diagnostic::error(
                "KUL-R02",
                format!(
                    "no marriage with id `{}` is declared in the project — check for a typo, or add a `marriage {} …` declaration (the `{role}` link expects a marriage id)",
                    ident.name, ident.name
                ),
                fspan(file, ident.span),
            ));
        }
        Some(EntityRef {
            kind: EntityKind::Marriage,
            ..
        }) => {}
        Some(EntityRef {
            kind: EntityKind::Person,
            id: prior,
            file: prior_file,
        }) => {
            out.push(
                Diagnostic::error(
                    "KUL-R02",
                    format!(
                        "`{}` is a person, not a marriage — `{role}` links must reference a marriage id",
                        ident.name
                    ),
                    fspan(file, ident.span),
                )
                .with_related(
                    fspan(prior_file, prior.span),
                    format!("`{}` declared as a person here", ident.name),
                ),
            );
        }
    }
}

/// R03 — required fields. `person` needs `name` + `gender`. Marriages
/// have no required fields beyond the positional spouses enforced by the
/// grammar; `start:` is optional (genealogical records may not know it).
pub fn rule_03_required_fields(resolved: &ResolvedDocument, file: FileId) -> Vec<Diagnostic> {
    let mut out = Vec::new();
    for p in resolved.persons_in(file) {
        if !p.has_field(FieldName::Name) {
            out.push(
                Diagnostic::error(
                    "KUL-R03",
                    format!(
                        "person `{}` needs a `name:` field — add `name:\"…\"` to the declaration",
                        p.id.name
                    ),
                    fspan(file, p.id.span),
                )
                .with_detail(detail::R03_MISSING_NAME),
            );
        }
        if !p.has_field(FieldName::Gender) {
            out.push(
                Diagnostic::error(
                    "KUL-R03",
                    format!(
                        "person `{}` needs a `gender:` field — use `gender:male`, `gender:female`, or `gender:other`",
                        p.id.name
                    ),
                    fspan(file, p.id.span),
                )
                .with_detail(detail::R03_MISSING_GENDER),
            );
        }
    }
    out
}

/// R04 — spouses must be distinct.
pub fn rule_04_self_marriage(resolved: &ResolvedDocument, file: FileId) -> Vec<Diagnostic> {
    let mut out = Vec::new();
    for m in resolved.marriages_in(file) {
        if m.spouse_a.name == m.spouse_b.name {
            out.push(self_marriage_diagnostic(file, m));
        }
    }
    out
}

fn self_marriage_diagnostic(file: FileId, m: &MarriageStmt) -> Diagnostic {
    Diagnostic::error(
        "KUL-R04",
        format!(
            "marriage `{}` lists `{}` as both spouses — spouses must be distinct people",
            m.id.name, m.spouse_a.name
        ),
        fspan(file, m.spouse_b.span),
    )
    .with_related(fspan(file, m.spouse_a.span), "first spouse listed here")
}

/// R05 — `end` and `end_reason` are both present or both absent.
/// R05b — `end_reason` value must be in vocabulary (`divorce` in v1).
pub fn rule_05_end_consistency(resolved: &ResolvedDocument, file: FileId) -> Vec<Diagnostic> {
    let mut out = Vec::new();
    for m in resolved.marriages_in(file) {
        match (m.end(), m.end_reason()) {
            (Some(end), None) => {
                out.push(
                    Diagnostic::error(
                        "KUL-R05",
                        format!(
                            "marriage `{}` has an `end:` date but no `end_reason:` — add `end_reason:divorce`",
                            m.id.name
                        ),
                        fspan(file, end.span),
                    )
                    .with_detail(detail::R05_END_WITHOUT_END_REASON),
                );
            }
            (None, Some(reason)) => {
                let field_span = m
                    .fields
                    .iter()
                    .find(|f| matches!(f.kind, crate::ast::MarriageFieldKind::EndReason(_)))
                    .map(|f| f.span)
                    .unwrap_or(reason.span);
                out.push(
                    Diagnostic::error(
                        "KUL-R05",
                        format!(
                            "marriage `{}` has an `end_reason:` but no `end:` date — add an `end:` date or remove this field",
                            m.id.name
                        ),
                        fspan(file, field_span),
                    )
                    .with_detail(detail::R05_END_REASON_WITHOUT_END),
                );
            }
            _ => {}
        }
        if let Some(reason) = m.end_reason()
            && let EndReason::Unknown(raw) = &reason.value
        {
            out.push(Diagnostic::error(
                "KUL-R05b",
                format!("`end_reason:{raw}` isn't recognized — the only value in v1 is `divorce`"),
                fspan(file, reason.span),
            ));
        }
    }
    out
}

/// R06 — `person.died < person.born`.
pub fn rule_06_died_before_born(resolved: &ResolvedDocument, file: FileId) -> Vec<Diagnostic> {
    resolved
        .persons_in(file)
        .filter_map(|p| {
            temporal_violation(
                file,
                "KUL-R06",
                p.died(),
                p.born(),
                Anchor::Earlier,
                || format!("person `{}` died before they were born", p.id.name),
                || "born here".to_owned(),
            )
        })
        .collect()
}

/// R07 — `marriage.end < marriage.start`.
pub fn rule_07_marriage_end_before_start(
    resolved: &ResolvedDocument,
    file: FileId,
) -> Vec<Diagnostic> {
    resolved
        .marriages_in(file)
        .filter_map(|m| {
            temporal_violation(
                file,
                "KUL-R07",
                m.end(),
                m.start(),
                Anchor::Earlier,
                || format!("marriage `{}` ended before it began", m.id.name),
                || "started here".to_owned(),
            )
        })
        .collect()
}

/// R08 — `adoption.end < adoption.start`.
pub fn rule_08_adoption_end_before_start(
    resolved: &ResolvedDocument,
    file: FileId,
) -> Vec<Diagnostic> {
    let mut out = Vec::new();
    for p in resolved.persons_in(file) {
        for adoption in &p.adoptions {
            if let Some(d) = temporal_violation(
                file,
                "KUL-R08",
                adoption.end(),
                adoption.start(),
                Anchor::Earlier,
                || {
                    format!(
                        "adoption of `{}` (by marriage `{}`) ended before it began",
                        p.id.name, adoption.marriage_ref.name
                    )
                },
                || "started here".to_owned(),
            ) {
                out.push(d);
            }
        }
    }
    out
}

/// R09 — `marriage.start < S.born` for either spouse `S`.
pub fn rule_09_marriage_before_spouse_born(
    resolved: &ResolvedDocument,
    file: FileId,
) -> Vec<Diagnostic> {
    let mut out = Vec::new();
    for m in resolved.marriages_in(file) {
        for spouse in resolved.spouses_of(m) {
            if let Some(d) = temporal_violation(
                file,
                "KUL-R09",
                m.start(),
                spouse.born(),
                Anchor::Earlier,
                || {
                    format!(
                        "marriage `{}` started before spouse `{}` was born",
                        m.id.name, spouse.id.name
                    )
                },
                || format!("`{}` born here", spouse.id.name),
            ) {
                out.push(d);
            }
        }
    }
    out
}

/// R10 — `marriage.start > S.died` for either spouse `S`.
pub fn rule_10_spouse_died_before_marriage(
    resolved: &ResolvedDocument,
    file: FileId,
) -> Vec<Diagnostic> {
    let mut out = Vec::new();
    for m in resolved.marriages_in(file) {
        for spouse in resolved.spouses_of(m) {
            if let Some(d) = temporal_violation(
                file,
                "KUL-R10",
                spouse.died(),
                m.start(),
                Anchor::Later,
                || {
                    format!(
                        "marriage `{}` started after spouse `{}` had already died",
                        m.id.name, spouse.id.name
                    )
                },
                || format!("`{}` died here", spouse.id.name),
            ) {
                out.push(d);
            }
        }
    }
    out
}

/// R11 — bio child born before either bio parent.
pub fn rule_11_bio_child_born_before_parent(
    resolved: &ResolvedDocument,
    file: FileId,
) -> Vec<Diagnostic> {
    let mut out = Vec::new();
    for child in resolved.persons_in(file) {
        let Some(birth) = &child.birth else { continue };
        let Some(marriage) = resolved.marriage(&birth.marriage_ref.name) else {
            continue;
        };
        for parent in resolved.spouses_of(marriage) {
            if let Some(d) = temporal_violation(
                file,
                "KUL-R11",
                child.born(),
                parent.born(),
                Anchor::Earlier,
                || {
                    format!(
                        "`{}` was born before their biological parent `{}`",
                        child.id.name, parent.id.name
                    )
                },
                || format!("`{}` born here", parent.id.name),
            ) {
                out.push(d);
            }
        }
    }
    out
}

/// R12 — `adoption.start < P.born` for either adoptive parent `P`.
pub fn rule_12_adoption_before_adopter_born(
    resolved: &ResolvedDocument,
    file: FileId,
) -> Vec<Diagnostic> {
    let mut out = Vec::new();
    for child in resolved.persons_in(file) {
        for adoption in &child.adoptions {
            let Some(marriage) = resolved.marriage(&adoption.marriage_ref.name) else {
                continue;
            };
            for parent in resolved.spouses_of(marriage) {
                if let Some(d) = temporal_violation(
                    file,
                    "KUL-R12",
                    adoption.start(),
                    parent.born(),
                    Anchor::Earlier,
                    || {
                        format!(
                            "adoption of `{}` started before adoptive parent `{}` was born",
                            child.id.name, parent.id.name
                        )
                    },
                    || format!("`{}` born here", parent.id.name),
                ) {
                    out.push(d);
                }
            }
        }
    }
    out
}

#[derive(Copy, Clone)]
enum Anchor {
    Earlier,
    Later,
}

fn temporal_violation(
    file: FileId,
    code: &'static str,
    earlier: Option<&DateLit>,
    later: Option<&DateLit>,
    anchor: Anchor,
    message: impl FnOnce() -> String,
    related_label: impl FnOnce() -> String,
) -> Option<Diagnostic> {
    let earlier = earlier?;
    let later = later?;
    if !before_strict(earlier, later) {
        return None;
    }
    let (primary, related) = match anchor {
        Anchor::Earlier => (earlier, later),
        Anchor::Later => (later, earlier),
    };
    Some(
        Diagnostic::error(code, message(), fspan(file, primary.span))
            .with_related(fspan(file, related.span), related_label()),
    )
}

/// R14 — a polygamy hub (≥2 un-ended marriages) must be the host
/// (first-listed spouse) in every one of those marriages. Fires once per
/// offending marriage where the hub is the joining spouse. ADR-0020.
pub fn rule_14_polygamy_hub_must_host(resolved: &ResolvedDocument) -> Vec<Diagnostic> {
    let mut un_ended_count: std::collections::HashMap<&str, usize> =
        std::collections::HashMap::new();
    for m in resolved.marriages() {
        if m.end().is_some() {
            continue;
        }
        // Skip marriages whose spouses don't both resolve / are equal —
        // R02 / R04 already report those; folding them into the count
        // would cascade those rules into a misleading R14.
        let spouses: Vec<&PersonStmt> = resolved.spouses_of(m).collect();
        if spouses.len() != 2 {
            continue;
        }
        if spouses[0].id.name == spouses[1].id.name {
            continue;
        }
        for spouse in spouses {
            *un_ended_count.entry(spouse.id.name.as_str()).or_insert(0) += 1;
        }
    }
    let mut out = Vec::new();
    for file in resolved.document().kul_file_ids() {
        for m in resolved.marriages_in(file) {
            if m.end().is_some() {
                continue;
            }
            // Only the joining spouse (spouse_b) can violate R14 — host
            // (spouse_a) is host by definition. Skip unresolved spouses
            // (R02 covers those).
            if resolved.person(&m.spouse_a.name).is_none() {
                continue;
            }
            let Some(hub_person) = resolved.person(&m.spouse_b.name) else {
                continue;
            };
            let hub_count = un_ended_count
                .get(hub_person.id.name.as_str())
                .copied()
                .unwrap_or(0);
            if hub_count < 2 {
                continue;
            }
            out.push(polygamy_hub_diagnostic(file, m, hub_person, hub_count));
        }
    }
    out
}

fn polygamy_hub_diagnostic(
    file: FileId,
    m: &MarriageStmt,
    hub: &PersonStmt,
    hub_count: usize,
) -> Diagnostic {
    let pronoun = match hub.gender().map(|g| g.value) {
        Some(Gender::Female) => "she",
        Some(Gender::Male) => "he",
        Some(Gender::Other) | None => "they",
    };
    // "Fix:" shows the hub moved to first position; `...` elides fields.
    let message = format!(
        "{name} has {hub_count} concurrent un-ended marriages; {pronoun} must be the declared host (first spouse) in all of them.\nCurrently: marriage {id} {a} {b} ...\nFix:       marriage {id} {hub_name} {other} ...",
        name = hub.id.name,
        hub_count = hub_count,
        pronoun = pronoun,
        id = m.id.name,
        a = m.spouse_a.name,
        b = m.spouse_b.name,
        hub_name = hub.id.name,
        other = m.spouse_a.name,
    );
    Diagnostic::error("KUL-R14", message, fspan(file, m.id.span))
}

/// R15 — a field may appear at most once per `person`, `marriage`, or
/// `adoption` statement. Accessors take the first occurrence, so a
/// repeated field silently discards later values; each repeat is an
/// error. Anchors at the duplicate occurrence's field name; a
/// related-span points to the first occurrence.
pub fn rule_15_duplicate_field(resolved: &ResolvedDocument, file: FileId) -> Vec<Diagnostic> {
    let mut out = Vec::new();
    for stmt in resolved.statements_in(file) {
        match stmt {
            Statement::Person(p) => {
                duplicate_fields(
                    file,
                    p.fields.iter().map(|f| (f.kind.field_name(), f.name_span)),
                    &mut out,
                );
                for adoption in &p.adoptions {
                    duplicate_fields(
                        file,
                        adoption
                            .fields
                            .iter()
                            .map(|f| (f.kind.field_name(), f.name_span)),
                        &mut out,
                    );
                }
            }
            Statement::Marriage(m) => {
                duplicate_fields(
                    file,
                    m.fields.iter().map(|f| (f.kind.field_name(), f.name_span)),
                    &mut out,
                );
            }
        }
    }
    out
}

/// Emit KUL-R15 for every field name that appears more than once in
/// `fields` (in source order, name span first). At most nine distinct
/// field names exist, so the linear `seen` scan is cheap.
fn duplicate_fields(
    file: FileId,
    fields: impl Iterator<Item = (FieldName, ByteSpan)>,
    out: &mut Vec<Diagnostic>,
) {
    let mut seen: Vec<(FieldName, ByteSpan)> = Vec::new();
    for (name, name_span) in fields {
        if let Some((_, first_span)) = seen.iter().find(|(n, _)| *n == name) {
            out.push(
                Diagnostic::error(
                    "KUL-R15",
                    format!(
                        "field `{}` is set more than once — a field may appear at most once per statement; remove the duplicate",
                        name.as_str()
                    ),
                    fspan(file, name_span),
                )
                .with_related(
                    fspan(file, *first_span),
                    format!("`{}` first set here", name.as_str()),
                ),
            );
        } else {
            seen.push((name, name_span));
        }
    }
}

/// R13 — no person may appear as their own ancestor in the parent graph
/// (bio ∪ adoptive). Cross-file cycles report as a single cycle with
/// per-link related-spans (ADR-0015).
pub fn rule_13_parenthood_cycles(resolved: &ResolvedDocument) -> Vec<Diagnostic> {
    let mut out = Vec::new();
    for cycle in crate::cycles::find_cycles(resolved) {
        let head = *cycle
            .members
            .first()
            .expect("cycle must have at least one member");
        let head_file = resolved
            .entity(&head.id.name)
            .map(|e| e.file)
            .unwrap_or(FileId::MANIFEST);
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
        let mut diag = Diagnostic::error("KUL-R13", message, fspan(head_file, head.id.span));
        for link in cycle.link_spans {
            diag = diag.with_related(fspan(link.file, link.span), "parent link in the cycle");
        }
        out.push(diag);
    }
    out
}
