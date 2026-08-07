//! Shared wire projection for kinship-native person / marriage /
//! parenthood-link shapes.
//!
//! Owns the per-entity builders that turn resolved AST statements into the
//! [`ExportedPerson`] / [`ExportedMarriage`] / [`ExportedParenthoodLink`]
//! wire types. Both the whole-graph export loop (`export`) and the query
//! lookups (`query::{person, marriage, details}`) call these builders, so a
//! lookup and a whole-graph export can never disagree about an entity's
//! serialized shape.
//!
//! The public wire types themselves stay in [`crate::export`] (ABI-stable
//! `kul_core::export::…` paths). This module produces them; it does not
//! redefine them.

use crate::ast::{AdoptionSub, BirthSub, MarriageStmt, PersonStmt};
use crate::date::DateLit;
use crate::export::{
    ExportOptions, ExportedDate, ExportedMarriage, ExportedParenthoodLink, ExportedPerson,
    ParenthoodLinkKind,
};
use crate::span::ByteSpan;

/// Build the [`ExportedParenthoodLink`] for `child`'s `birth` sub-statement.
/// The single source of a biological link's serialized shape: the graph
/// export loop and the `query` module's batched detail lookup both call this,
/// so the two can never drift (same discipline as [`build_one_person`]).
pub(crate) fn build_one_birth_link(
    child: &PersonStmt,
    birth: &BirthSub,
    options: &ExportOptions,
) -> ExportedParenthoodLink {
    ExportedParenthoodLink {
        marriage_id: birth.marriage_ref.name.clone(),
        child_id: child.id.name.clone(),
        kind: ParenthoodLinkKind::Biological,
        start: None,
        end: None,
        span: span_if(options, birth.span),
    }
}

/// Build the [`ExportedParenthoodLink`] for one of `child`'s `adoption`
/// sub-statements. Adoptive counterpart to [`build_one_birth_link`]; carries
/// the adoption's own `start:` / `end:` dates.
pub(crate) fn build_one_adoption_link(
    child: &PersonStmt,
    adoption: &AdoptionSub,
    options: &ExportOptions,
) -> ExportedParenthoodLink {
    ExportedParenthoodLink {
        marriage_id: adoption.marriage_ref.name.clone(),
        child_id: child.id.name.clone(),
        kind: ParenthoodLinkKind::Adoptive,
        start: adoption.start().map(exported_date),
        end: adoption.end().map(exported_date),
        span: span_if(options, adoption.span),
    }
}

/// Build one [`ExportedPerson`] from a resolved [`PersonStmt`]. The single
/// source of a person's serialized shape: the graph export loop and the
/// `query` module's `person(id)` lookup both call this, so the two can
/// never drift (per the query-engine ADR).
pub(crate) fn build_one_person(p: &PersonStmt, options: &ExportOptions) -> ExportedPerson {
    ExportedPerson {
        id: p.id.name.clone(),
        name: p
            .name()
            .expect("R03 ensures person.name is present")
            .value
            .clone(),
        family: p.family().map(|s| s.value.clone()),
        given: p.given().map(|s| s.value.clone()),
        gender: p
            .gender()
            .expect("R03 ensures person.gender is present")
            .value
            .as_token(),
        born: p.born().map(exported_date),
        died: p.died().map(exported_date),
        span: span_if(options, p.span),
    }
}

/// Build one [`ExportedMarriage`] from a resolved [`MarriageStmt`]. The
/// single source of a marriage's serialized shape: the graph export loop
/// and the `query` module's `marriage(id)` lookup both call this (per the
/// query-engine ADR).
pub(crate) fn build_one_marriage(m: &MarriageStmt, options: &ExportOptions) -> ExportedMarriage {
    ExportedMarriage {
        id: m.id.name.clone(),
        spouses: [m.spouse_a.name.clone(), m.spouse_b.name.clone()],
        start: m.start().map(exported_date),
        end: m.end().map(exported_date),
        end_reason: m.end_reason().map(|er| er.value.as_str().to_string()),
        span: span_if(options, m.span),
    }
}

fn span_if(options: &ExportOptions, span: ByteSpan) -> Option<[usize; 2]> {
    options.with_positions.then_some([span.start, span.end])
}

fn exported_date(d: &DateLit) -> ExportedDate {
    let (value, precision) = match (d.month, d.day) {
        (Some(m), Some(day)) => (format!("{:04}-{:02}-{:02}", d.year, m, day), "day"),
        (Some(m), None) => (format!("{:04}-{:02}", d.year, m), "month"),
        (None, _) => (format!("{:04}", d.year), "year"),
    };
    ExportedDate {
        value,
        precision,
        circa: d.circa,
    }
}
