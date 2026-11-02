//! The **query seam**: the single public module where all kinship-query
//! capabilities and their result types live.
//!
//! The query engine is a deep module layered over [`ResolvedDocument`]
//! (ADR-0001) — the same checked-project substrate the validator and the
//! language service read. It consumes only `ResolvedDocument`'s public accessors and
//! never re-walks the AST, and it never consumes the exported graph: the
//! export is the escape hatch for consumers who don't use the engine, not
//! the engine's own input.
//!
//! The simplest operations are the id → detail lookups [`person`] and
//! [`marriage`]. They return the **same serialized
//! shapes the export produces** ([`ExportedPerson`] / [`ExportedMarriage`]),
//! single-sourced through `projection::build_one_person` /
//! `projection::build_one_marriage` so a lookup and a whole-graph export can
//! never disagree about a person's shape.
//!
//! **Lookup semantics: absence is the answer.** An unknown id, or an id
//! that names a marriage when a person was asked for (and vice versa),
//! yields `None`. There is no error type at the lookup layer — a lookup
//! asks "is there a person with this id?", and "no" is a complete, honest
//! answer. (A typed unknown-id error exists only where an id is an *input
//! anchor* to a kin-set or relationship question rather than the subject of
//! the question.)
//!
//! [`detail`] is the third member of the same family: one person or marriage
//! answered with its own fields *plus* its relational neighbourhood, because
//! the plain lookups cannot answer a details panel (ADR-0037);
//! [`EntityDetail`] carries the shape.
//!
//! The [`QueryEnvelope`] type is the CLI's contract: `kul query` wraps an
//! answer in it, gated on the project passing checks (strict-on-errors,
//! ADR-0009), and `--format json` prints it (ADR-0024, ADR-0025).

use serde::Serialize;

use crate::CheckResult;
use crate::export::{
    ExportOptions, ExportedDiagnostic, ExportedMarriage, ExportedPerson, export_diagnostics,
};
use crate::projection::{build_one_marriage, build_one_person};
use crate::semantic::ResolvedDocument;

mod descriptor;
mod detail;
mod engine;
mod filter;
mod junction;
mod pattern;
mod resolve;
mod sugar;

pub use descriptor::{
    Affinity, Classification, EdgeNature, Gender, HopEdge, LinealRole, MarriageStatus, PathHop,
    RelationshipDescriptor, Seniority, Sharing, Side,
};
pub use detail::{EntityDetail, LinkedPerson, MarriageTie, detail};
pub use engine::{KinMember, QueryEvalError, evaluate, resolve, run_query};
pub use filter::{FilterMode, PersonField, Predicate, SortDirection, SortSpec};
pub use pattern::{
    IntRange, KinPattern, Member, PatternClassification, Projection, Query, QueryResult,
    QuerySource,
};
pub use resolve::{DEFAULT_MAX_APEX_GENERATIONS, EmptyReason, ResolveConfig, ResolveResult};
pub use sugar::{
    ancestors_of, aunts_uncles_of, children_of, cousins_of, descendants_of, in_laws_of,
    nieces_nephews_of, parents_of, siblings_of, spouses_of, step_children_of, step_parents_of,
    step_siblings_of,
};

/// Look up a person by id. Returns the person in the export shape, or
/// `None` when no person has that id (an unknown id, or an id that names a
/// marriage, is simply not a person). Reads only `ResolvedDocument`'s id
/// index — no AST walk.
#[must_use]
pub fn person(resolved: &ResolvedDocument, id: &str) -> Option<ExportedPerson> {
    resolved
        .person(id)
        .map(|p| build_one_person(p, &ExportOptions::default()))
}

/// Look up a marriage by id. Returns the marriage in the export shape, or
/// `None` when no marriage has that id. Same absence-is-the-answer
/// semantics as [`person`].
#[must_use]
pub fn marriage(resolved: &ResolvedDocument, id: &str) -> Option<ExportedMarriage> {
    resolved
        .marriage(id)
        .map(|m| build_one_marriage(m, &ExportOptions::default()))
}

/// Payload of a person lookup: the person in the export shape, or `null`
/// when the id names no person.
pub type PersonLookupResult = Option<ExportedPerson>;

/// Payload of a marriage lookup: the marriage in the export shape, or
/// `null` when the id names no marriage.
pub type MarriageLookupResult = Option<ExportedMarriage>;

/// The CLI-facing result of a query operation. Mirrors the export
/// envelope: an untagged union discriminated by an `ok` boolean — the ok arm carries the query `result`, the error arm carries
/// the structured `diagnostics` of a project that failed its checks.
///
/// The engine never throws / never panics: a failing project yields the
/// [`QueryEnvelope::Error`] arm, not a partial answer (strict-on-errors,
/// ADR-0009). Generic over the payload `T`, so lookups, kin-set queries and
/// relationship resolution share one envelope.
#[derive(Debug, Clone, Serialize)]
#[serde(untagged)]
pub enum QueryEnvelope<T> {
    /// The project passed its checks; `result` carries the query answer.
    Ok(QueryOk<T>),
    /// The project failed its checks; `diagnostics` carries why.
    Error(QueryError),
}

/// Ok arm of a [`QueryEnvelope`]. `ok` is always `true` (consumer-facing
/// discriminator); `result` is the query payload.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct QueryOk<T> {
    /// Always `true`. Consumer-facing discriminator.
    pub ok: bool,
    /// The query answer (for a lookup: the entity, or `null`).
    pub result: T,
}

/// Error arm of a [`QueryEnvelope`]. `ok` is always `false`;
/// `diagnostics` carries every diagnostic the failing project produced —
/// the same [`ExportedDiagnostic`] shape the check/export surfaces use.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct QueryError {
    /// Always `false`. Consumer-facing discriminator.
    pub ok: bool,
    /// Every diagnostic the validator produced (errors, warnings, notes).
    pub diagnostics: Vec<ExportedDiagnostic>,
}

impl<T> QueryEnvelope<T> {
    /// `true` for the [`QueryEnvelope::Ok`] arm.
    #[must_use]
    pub fn is_ok(&self) -> bool {
        matches!(self, QueryEnvelope::Ok(_))
    }
}

/// Wrap a person lookup in a [`QueryEnvelope`], gated on the project
/// passing its checks. A project with error-severity diagnostics yields
/// the error arm (never a partial answer). This is the single source of
/// the person-lookup contract serialization `kul query person` prints.
#[must_use]
pub fn person_lookup(check: &CheckResult, id: &str) -> QueryEnvelope<PersonLookupResult> {
    if check.has_errors() {
        return QueryEnvelope::Error(QueryError {
            ok: false,
            diagnostics: export_diagnostics(check),
        });
    }
    QueryEnvelope::Ok(QueryOk {
        ok: true,
        result: person(check.resolved(), id),
    })
}

/// Wrap a marriage lookup in a [`QueryEnvelope`], gated on the project
/// passing its checks. Marriage-lookup counterpart to [`person_lookup`].
#[must_use]
pub fn marriage_lookup(check: &CheckResult, id: &str) -> QueryEnvelope<MarriageLookupResult> {
    if check.has_errors() {
        return QueryEnvelope::Error(QueryError {
            ok: false,
            diagnostics: export_diagnostics(check),
        });
    }
    QueryEnvelope::Ok(QueryOk {
        ok: true,
        result: marriage(check.resolved(), id),
    })
}

/// Evaluate a kin-set [`Query`] and wrap the answer in a [`QueryEnvelope`],
/// gated on the project passing its checks (strict-on-errors, ADR-0009).
/// The single source of the kin-query contract serialization the CLI's
/// `kul query kin --format json` path prints.
///
/// Three outcomes, all non-throwing:
/// - project failed its checks → the error arm carries the check diagnostics;
/// - anchor names no person → the error arm carries a single synthesized
///   diagnostic naming the id ([`QueryEvalError::to_diagnostic`]);
/// - otherwise → the ok arm carries the `members` result in the pinned order.
#[must_use]
pub fn query_envelope(check: &CheckResult, query: &Query) -> QueryEnvelope<QueryResult> {
    if check.has_errors() {
        return QueryEnvelope::Error(QueryError {
            ok: false,
            diagnostics: export_diagnostics(check),
        });
    }
    match run_query(check.resolved(), query) {
        Ok(result) => QueryEnvelope::Ok(QueryOk { ok: true, result }),
        Err(err) => QueryEnvelope::Error(QueryError {
            ok: false,
            diagnostics: vec![err.to_diagnostic()],
        }),
    }
}

/// Resolve **all** the ways two persons are related and wrap the answer in a
/// [`QueryEnvelope`], gated on the project passing its checks (strict-on-
/// errors, ADR-0009). The single source of the resolution contract
/// serialization the CLI's `kul query rel --format json` path prints.
///
/// Three outcomes, all non-throwing:
/// - project failed its checks → the error arm carries the check diagnostics;
/// - either id names no person → the error arm carries a single synthesized
///   diagnostic naming the bad id ([`QueryEvalError::to_diagnostic`]);
/// - otherwise → the ok arm carries the [`ResolveResult`] (a possibly-empty
///   relationship list plus its [`EmptyReason`](resolve::EmptyReason)).
///
/// An empty-but-clean result is the **ok** arm — an empty result is an answer,
/// not an error.
#[must_use]
pub fn resolve_relationship(
    check: &CheckResult,
    x: &str,
    y: &str,
    config: &ResolveConfig,
) -> QueryEnvelope<ResolveResult> {
    if check.has_errors() {
        return QueryEnvelope::Error(QueryError {
            ok: false,
            diagnostics: export_diagnostics(check),
        });
    }
    match resolve(check.resolved(), x, y, config) {
        Ok(result) => QueryEnvelope::Ok(QueryOk { ok: true, result }),
        Err(err) => QueryEnvelope::Error(QueryError {
            ok: false,
            diagnostics: vec![err.to_diagnostic()],
        }),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ast::InputFile;

    fn check(source: &str) -> CheckResult {
        let inputs = vec![InputFile::new("test.kul", source)];
        crate::check(&inputs)
    }

    const NUCLEAR: &str = "\
person hiroshi name:\"Hiroshi\" gender:male born:1978
person yuki name:\"Yuki\" gender:female born:1980
marriage m_hiroshi_yuki hiroshi yuki start:2005
";

    #[test]
    fn person_lookup_returns_known_person() {
        let c = check(NUCLEAR);
        let p = person(c.resolved(), "hiroshi").expect("hiroshi is a person");
        assert_eq!(p.id, "hiroshi");
        assert_eq!(p.name, "Hiroshi");
    }

    #[test]
    fn marriage_lookup_returns_known_marriage() {
        let c = check(NUCLEAR);
        let m = marriage(c.resolved(), "m_hiroshi_yuki").expect("marriage exists");
        assert_eq!(m.id, "m_hiroshi_yuki");
        assert_eq!(m.spouses, ["hiroshi".to_string(), "yuki".to_string()]);
    }

    #[test]
    fn unknown_id_is_none() {
        let c = check(NUCLEAR);
        assert!(person(c.resolved(), "nobody").is_none());
        assert!(marriage(c.resolved(), "nobody").is_none());
    }

    #[test]
    fn wrong_kind_id_is_none() {
        let c = check(NUCLEAR);
        // A marriage id asked for as a person, and vice versa: both `None`.
        assert!(person(c.resolved(), "m_hiroshi_yuki").is_none());
        assert!(marriage(c.resolved(), "hiroshi").is_none());
    }

    #[test]
    fn lookup_envelope_ok_carries_result() {
        let c = check(NUCLEAR);
        let env = person_lookup(&c, "hiroshi");
        assert!(env.is_ok());
        let QueryEnvelope::Ok(ok) = env else {
            panic!("expected ok arm");
        };
        assert!(ok.ok);
        assert_eq!(ok.result.expect("some person").id, "hiroshi");
    }

    #[test]
    fn lookup_envelope_ok_carries_null_for_unknown_id() {
        let c = check(NUCLEAR);
        let env = person_lookup(&c, "nobody");
        let QueryEnvelope::Ok(ok) = env else {
            panic!("expected ok arm for a clean project even on unknown id");
        };
        assert!(ok.result.is_none());
    }

    #[test]
    fn failing_project_yields_error_arm() {
        // Missing required `name:` triggers KUL-R03.
        let c = check("person alice gender:female\n");
        let env = person_lookup(&c, "alice");
        assert!(!env.is_ok());
        let QueryEnvelope::Error(err) = env else {
            panic!("expected error arm");
        };
        assert!(!err.ok);
        assert!(err.diagnostics.iter().any(|d| d.code == "KUL-R03"));
    }
}
