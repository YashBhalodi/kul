//! Validator: runs spec rules against a [`ResolvedDocument`].
//!
//! Each rule is a small function `rule_NN(...) -> Vec<Diagnostic>` taking a
//! resolved document and returning diagnostics. The top-level [`validate`]
//! is the composition of every implemented rule. Rules query the document
//! through [`ResolvedDocument`]'s typed methods — they never enumerate
//! `document.statements` themselves.
//!
//! Rules walk `.kul` files one at a time (per-file namespaces, ADR-0014):
//! references resolve only against the same file, so cross-file lookalikes
//! (two files each declaring `alice`) are independent.

use crate::ast::{EndReason, Ident, MarriageStmt, Statement};
use crate::date::{DateLit, before_strict};
use crate::diagnostic::{Diagnostic, detail, fspan};
use crate::lexer::FieldName;
use crate::semantic::{EntityKind, EntityRef, ResolvedDocument};
use crate::span::FileId;

pub fn validate(resolved: &ResolvedDocument) -> Vec<Diagnostic> {
    let mut diagnostics = Vec::new();
    for file in resolved.document().kul_file_ids() {
        // R02 must run first inside each file: it's the source-order pass
        // over raw statements that reports "no entity with this id," and
        // downstream rules already trust that resolved spouse/parent
        // links exist. Keeping it at the top of the per-file walk
        // preserves the diagnostic ordering R02-then-R03+ that callers
        // (and snapshot tests) rely on.
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
        diagnostics.extend(rule_13_parenthood_cycles(resolved, file));
    }
    diagnostics
}

/// Rule 2 — every marriage spouse, `birth` ref, and `adoption` ref must
/// resolve to a declared id of the appropriate kind, *within the same
/// file*. Cross-file references are not supported in v1; the same name
/// appearing in another file does not satisfy resolution here.
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
    match resolved.entity(file, &ident.name) {
        None => {
            out.push(Diagnostic::error(
                "KUL-R02",
                format!(
                    "no person with id `{}` is declared in this file — check for a typo, or add a `person {} …` declaration",
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
    match resolved.entity(file, &ident.name) {
        None => {
            out.push(Diagnostic::error(
                "KUL-R02",
                format!(
                    "no marriage with id `{}` is declared in this file — check for a typo, or add a `marriage {} …` declaration (the `{role}` link expects a marriage id)",
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

/// Rule 3 — required fields missing.
///
/// A `person` MUST have `name` and `gender`.
/// A `marriage` MUST have `start`. (The two spouses are positional and
/// enforced by the grammar — a missing spouse is a parse error.)
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
    for m in resolved.marriages_in(file) {
        if m.start().is_none() {
            out.push(
                Diagnostic::error(
                    "KUL-R03",
                    format!(
                        "marriage `{}` needs a `start:` date — add `start:YYYY` (or a fuller `YYYY-MM-DD`)",
                        m.id.name
                    ),
                    fspan(file, m.id.span),
                )
                .with_detail(detail::R03_MISSING_MARRIAGE_START),
            );
        }
    }
    out
}

/// Rule 4 — self-marriage. A marriage's two spouse identifiers must be
/// distinct.
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

/// Rule 5 — `end` and `end_reason` must both be present or both absent.
/// Rule 5b (KUL-R05b) — `end_reason` value must be in the v1 vocabulary
/// (currently just `divorce`).
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

/// Rule 6 — `person.died < person.born`.
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

/// Rule 7 — `marriage.end < marriage.start`.
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

/// Rule 8 — `adoption.end < adoption.start`.
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

/// Rule 9 — `marriage.start < S.born` for either spouse `S`.
pub fn rule_09_marriage_before_spouse_born(
    resolved: &ResolvedDocument,
    file: FileId,
) -> Vec<Diagnostic> {
    let mut out = Vec::new();
    for m in resolved.marriages_in(file) {
        for spouse in resolved.spouses_of(file, m) {
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

/// Rule 10 — `marriage.start > S.died` for either spouse `S`.
pub fn rule_10_spouse_died_before_marriage(
    resolved: &ResolvedDocument,
    file: FileId,
) -> Vec<Diagnostic> {
    let mut out = Vec::new();
    for m in resolved.marriages_in(file) {
        for spouse in resolved.spouses_of(file, m) {
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

/// Rule 11 — bio child born before either bio parent.
pub fn rule_11_bio_child_born_before_parent(
    resolved: &ResolvedDocument,
    file: FileId,
) -> Vec<Diagnostic> {
    let mut out = Vec::new();
    for child in resolved.persons_in(file) {
        let Some(birth) = &child.birth else { continue };
        let Some(marriage) = resolved.marriage(file, &birth.marriage_ref.name) else {
            continue;
        };
        for parent in resolved.spouses_of(file, marriage) {
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

/// Rule 12 — `adoption.start < P.born` for either adoptive parent `P`.
pub fn rule_12_adoption_before_adopter_born(
    resolved: &ResolvedDocument,
    file: FileId,
) -> Vec<Diagnostic> {
    let mut out = Vec::new();
    for child in resolved.persons_in(file) {
        for adoption in &child.adoptions {
            let Some(marriage) = resolved.marriage(file, &adoption.marriage_ref.name) else {
                continue;
            };
            for parent in resolved.spouses_of(file, marriage) {
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

/// Rule 13 — no person may appear as their own ancestor in the parent
/// graph (union of bio and adoptive parent links). Cycles are scoped per
/// file (per ADR-0014), since parent links resolve only against the same
/// file.
pub fn rule_13_parenthood_cycles(resolved: &ResolvedDocument, file: FileId) -> Vec<Diagnostic> {
    let mut out = Vec::new();
    for cycle in crate::cycles::find_cycles(resolved, file) {
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
        let mut diag = Diagnostic::error("KUL-R13", message, fspan(file, head.id.span));
        for span in cycle.link_spans {
            diag = diag.with_related(fspan(file, span), "parent link in the cycle");
        }
        out.push(diag);
    }
    out
}
