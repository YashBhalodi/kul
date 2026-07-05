//! **Attribute filtering, sort, and count** — the permanently-modest third
//! query capability (PRD 0005, ADR-0025). Filter persons by field
//! predicates, sort deterministically, and count; every capability composes
//! onto an `allPersons` source or the output of a kin-set traversal.
//!
//! The filter surface is **deliberately small and pinned** — no substring, no
//! regex, no OR, no cross-field or cross-entity predicates, no date
//! arithmetic. Those live on the exported JSON, in consumer code. What lives
//! here is the honest core: exact string/id/gender matching, three-valued
//! interval-date predicates, presence tests, deterministic sort, and count.
//!
//! # Three-valued predicate evaluation
//!
//! Every predicate evaluates to [`Bool3`] — `True`, `False`, or `Unknown` —
//! and the [certainty mode](FilterMode) decides which rows survive. This is
//! the same honesty the descriptor's seniority fields carry: the engine never
//! asserts what the data cannot support.
//!
//! **Strings, ids, gender** (`eq` / `neq` / `in`): exact, case-sensitive,
//! codepoint comparison — no substring, no regex, **permanently** (`family`
//! is a first-class field, so cohort queries need neither; fuzzy/locale
//! matching is app UX). A **missing** field compares as `Unknown` (we cannot
//! assert a family we never recorded is or is not "Sharma").
//!
//! | recorded | `eq v` | `neq v` | `in {…}` |
//! |----------|--------|---------|----------|
//! | `== v` (∈) | True | False | True |
//! | `!= v` (∉) | False | True | False |
//! | missing | Unknown | Unknown | Unknown |
//!
//! **Dates** (`born` / `died`; the literal is a Kul date — partial `1950` /
//! `1950-04` and circa `~1950` allowed): three-valued against the recorded
//! value's closed interval (partial = the whole period; circa = ±5 years),
//! reusing the toolchain's single date machinery ([`DateLit`] interval bounds;
//! the same bounds [`before_strict`](crate::date::before_strict) is built on —
//! there is no second comparison).
//!
//! - `lt` / `lte` / `gt` / `gte`: **True** iff the comparison holds under
//!   *every* interpretation of both intervals; **False** iff under *none*;
//!   otherwise **Unknown**.
//! - `eq`: **True** iff every interpretation of the recorded value falls
//!   within the literal's period (interval containment — `born eq 1950` reads
//!   "certainly born within 1950"); **False** iff the intervals are disjoint;
//!   otherwise **Unknown**.
//! - `neq`: the mirror — True iff disjoint, False iff contained, else Unknown.
//! - A **missing** date compares as **Unknown**.
//!
//! | recorded vs `1950` | `lt 1950` | `gt 1949` | `gt 1950` | `eq 1950` | `neq 1950` |
//! |--------------------|-----------|-----------|-----------|-----------|------------|
//! | `1949` | True | False | False | False | True |
//! | `1950` | False | True | Unknown | True | False |
//! | `1951` | False | True | True | False | True |
//! | `1950-04` (⊂ 1950) | False | True | Unknown | True | False |
//! | missing | Unknown | Unknown | Unknown | Unknown | Unknown |
//!
//! **`present` / `absent`**: two-valued, always decidable (is the field
//! recorded).
//!
//! **Conjunction**: `where` is AND-only (**no OR, permanently** — OR is two
//! queries and a set union in consumer code). Three-valued AND: any `False`
//! ⇒ `False`; else any `Unknown` ⇒ `Unknown`; else `True`.
//!
//! **Certainty mode**: [`FilterMode::Certain`] (default) keeps only rows
//! evaluating `True`; [`FilterMode::IncludeUncertain`] keeps `True` **and**
//! `Unknown` rows (for gap-finding researchers hunting fuzzy records).

use std::cmp::Ordering;

use serde::{Deserialize, Serialize};
#[cfg(feature = "tsify")]
use tsify::Tsify;

use crate::ast::PersonStmt;
use crate::date::{CalendarDay, DateLit, parse_date};
use crate::span::ByteSpan;

/// A person field a predicate or sort key names. `born` / `died` are date
/// fields (three-valued interval predicates); the rest are string fields
/// (exact, case-sensitive). `id` is always present; every other field may be
/// absent.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "tsify", derive(Tsify))]
#[serde(rename_all = "camelCase")]
pub enum PersonField {
    Id,
    Name,
    Family,
    Given,
    Gender,
    Born,
    Died,
}

impl PersonField {
    /// Whether this is a date field (`born` / `died`) — the fields ordering
    /// predicates accept and the ones `eq`/`neq` read as interval containment.
    #[must_use]
    pub fn is_date(self) -> bool {
        matches!(self, PersonField::Born | PersonField::Died)
    }

    /// The field's spelling, for diagnostics and the CLI grammar.
    #[must_use]
    pub fn as_str(self) -> &'static str {
        match self {
            PersonField::Id => "id",
            PersonField::Name => "name",
            PersonField::Family => "family",
            PersonField::Given => "given",
            PersonField::Gender => "gender",
            PersonField::Born => "born",
            PersonField::Died => "died",
        }
    }
}

/// One attribute predicate on a person's own field. **Internally tagged on
/// `op`** so TypeScript consumers get a discriminated union to `switch` on.
///
/// Only ever tests the person's *own* fields — never combines fields (no age
/// arithmetic, no "alive in 1985"), never reaches across entities (no
/// "spouse's family"). Compound questions decompose in *consumer* code.
///
/// Set **non**-membership (`∉`) is not a variant: it is a conjunction of
/// `Neq` predicates (`x ∉ {A,B}` ≡ `x≠A ∧ x≠B`), which `where`'s AND-only
/// composition expresses directly.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "tsify", derive(Tsify))]
#[serde(tag = "op", rename_all = "camelCase")]
pub enum Predicate {
    /// `field == value`. Exact/case-sensitive for string fields; interval
    /// containment (three-valued) for date fields.
    Eq { field: PersonField, value: String },
    /// `field != value` — the mirror of [`Predicate::Eq`].
    Neq { field: PersonField, value: String },
    /// `field < value` — date fields only; `value` is a Kul date literal.
    Lt { field: PersonField, value: String },
    /// `field <= value` — date fields only.
    Lte { field: PersonField, value: String },
    /// `field > value` — date fields only.
    Gt { field: PersonField, value: String },
    /// `field >= value` — date fields only.
    Gte { field: PersonField, value: String },
    /// `field ∈ values` — string fields only; exact/case-sensitive.
    In {
        field: PersonField,
        values: Vec<String>,
    },
    /// The field is recorded. Two-valued, always decidable.
    Present { field: PersonField },
    /// The field is not recorded. Two-valued, always decidable.
    Absent { field: PersonField },
}

/// The direction a [`SortSpec`] orders by. `asc` is the default; **missing
/// values sort last regardless of direction**, and ties are always broken by
/// person id ascending.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[cfg_attr(feature = "tsify", derive(Tsify))]
#[serde(rename_all = "camelCase")]
pub enum SortDirection {
    #[default]
    Asc,
    Desc,
}

/// A single-key sort. Dates order by (lower bound, upper bound); strings, ids,
/// and gender by codepoint; **missing values sort last regardless of
/// direction**; ties always broken by person id ascending — fully
/// deterministic, so snapshots stay stable (ADR-0025).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "tsify", derive(Tsify))]
#[serde(rename_all = "camelCase")]
pub struct SortSpec {
    pub field: PersonField,
    #[serde(default)]
    pub direction: SortDirection,
}

/// The certainty mode a filter runs under (PRD 0005).
///
/// `certain` (the default) keeps only rows a predicate conjunction evaluates
/// **True** for — the engine never asserts what the data doesn't support.
/// `includeUncertain` also keeps **Unknown** rows — for gap-finding
/// researchers hunting fuzzy records.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[cfg_attr(feature = "tsify", derive(Tsify))]
#[serde(rename_all = "camelCase")]
pub enum FilterMode {
    #[default]
    Certain,
    IncludeUncertain,
}

impl FilterMode {
    /// Whether this is the default (`certain`) mode. Lets the [`Query`] value
    /// skip serializing the mode when it is the default, keeping the wire
    /// shape minimal.
    ///
    /// [`Query`]: super::Query
    #[must_use]
    pub fn is_certain(&self) -> bool {
        matches!(self, FilterMode::Certain)
    }
}

/// Three-valued truth: the value every predicate evaluates to.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Bool3 {
    True,
    False,
    Unknown,
}

impl Bool3 {
    /// Three-valued negation: True ↔ False; Unknown is its own mirror. Used to
    /// derive `neq` from `eq` and `absent` from `present`.
    fn not(self) -> Bool3 {
        match self {
            Bool3::True => Bool3::False,
            Bool3::False => Bool3::True,
            Bool3::Unknown => Bool3::Unknown,
        }
    }
}

/// A predicate whose date literal has been parsed and whose op/field
/// compatibility has been checked, so per-person evaluation is total (never
/// errors). Produced once by [`compile_predicates`].
pub(crate) enum Compiled {
    /// Exact string match; `negate` flips it into `neq`.
    StrEq {
        field: PersonField,
        value: String,
        negate: bool,
    },
    /// String set membership.
    StrIn {
        field: PersonField,
        values: Vec<String>,
    },
    /// Date interval containment; `negate` flips it into `neq`.
    DateContains {
        field: PersonField,
        lit: DateLit,
        negate: bool,
    },
    /// Date interval ordering.
    DateOrder {
        field: PersonField,
        lit: DateLit,
        op: OrderOp,
    },
    /// Field presence; `negate` flips it into `absent`.
    Present { field: PersonField, negate: bool },
}

/// The four ordering comparisons, all date-only.
#[derive(Clone, Copy)]
pub(crate) enum OrderOp {
    Lt,
    Lte,
    Gt,
    Gte,
}

/// Compile every predicate once: parse date literals, and reject the two
/// op/field mismatches the type cannot (ordering on a non-date field, `in` on
/// a date field). Returns the offending message on the first bad predicate —
/// the caller wraps it in a typed error so the surfaces (CLI diagnostic, WASM
/// envelope) report *which* predicate is malformed.
///
/// # Errors
///
/// A human-readable message when a date literal is malformed, an ordering
/// operator names a non-date field, or `in` names a date field.
pub(crate) fn compile_predicates(predicates: &[Predicate]) -> Result<Vec<Compiled>, String> {
    predicates.iter().map(compile_one).collect()
}

fn compile_one(pred: &Predicate) -> Result<Compiled, String> {
    match pred {
        Predicate::Eq { field, value } => Ok(if field.is_date() {
            Compiled::DateContains {
                field: *field,
                lit: parse_literal(value)?,
                negate: false,
            }
        } else {
            Compiled::StrEq {
                field: *field,
                value: value.clone(),
                negate: false,
            }
        }),
        Predicate::Neq { field, value } => Ok(if field.is_date() {
            Compiled::DateContains {
                field: *field,
                lit: parse_literal(value)?,
                negate: true,
            }
        } else {
            Compiled::StrEq {
                field: *field,
                value: value.clone(),
                negate: true,
            }
        }),
        Predicate::Lt { field, value } => compile_order(*field, value, OrderOp::Lt),
        Predicate::Lte { field, value } => compile_order(*field, value, OrderOp::Lte),
        Predicate::Gt { field, value } => compile_order(*field, value, OrderOp::Gt),
        Predicate::Gte { field, value } => compile_order(*field, value, OrderOp::Gte),
        Predicate::In { field, values } => {
            if field.is_date() {
                return Err(format!(
                    "`in` set membership is not valid for the date field `{}` \
                     (use </<=/>/>= or eq/neq)",
                    field.as_str()
                ));
            }
            Ok(Compiled::StrIn {
                field: *field,
                values: values.clone(),
            })
        }
        Predicate::Present { field } => Ok(Compiled::Present {
            field: *field,
            negate: false,
        }),
        Predicate::Absent { field } => Ok(Compiled::Present {
            field: *field,
            negate: true,
        }),
    }
}

fn compile_order(field: PersonField, value: &str, op: OrderOp) -> Result<Compiled, String> {
    if !field.is_date() {
        return Err(format!(
            "ordering comparison requires a date field (`born`/`died`), got `{}`",
            field.as_str()
        ));
    }
    Ok(Compiled::DateOrder {
        field,
        lit: parse_literal(value)?,
        op,
    })
}

/// Parse a predicate's date literal, mapping a parse failure to a diagnostic
/// message. The span is synthetic (the literal came from the query, not
/// source), so only the message survives.
fn parse_literal(value: &str) -> Result<DateLit, String> {
    parse_date(value, ByteSpan::new(0, value.len()))
        .map_err(|e| format!("invalid date literal `{value}`: {}", e.message()))
}

/// Whether `person` survives the compiled predicate conjunction under `mode`.
/// Three-valued AND (any `False` ⇒ excluded; else any `Unknown` ⇒ excluded in
/// `certain`, kept in `includeUncertain`; else kept). An empty conjunction is
/// `True` — every person survives.
pub(crate) fn passes(person: &PersonStmt, predicates: &[Compiled], mode: FilterMode) -> bool {
    let mut any_unknown = false;
    for pred in predicates {
        match eval(person, pred) {
            Bool3::False => return false,
            Bool3::Unknown => any_unknown = true,
            Bool3::True => {}
        }
    }
    match mode {
        FilterMode::Certain => !any_unknown,
        FilterMode::IncludeUncertain => true,
    }
}

fn eval(person: &PersonStmt, pred: &Compiled) -> Bool3 {
    match pred {
        Compiled::StrEq {
            field,
            value,
            negate,
        } => {
            let base = match person_string(person, *field) {
                Some(s) if &s == value => Bool3::True,
                Some(_) => Bool3::False,
                None => Bool3::Unknown,
            };
            if *negate { base.not() } else { base }
        }
        Compiled::StrIn { field, values } => match person_string(person, *field) {
            Some(s) if values.iter().any(|v| v == &s) => Bool3::True,
            Some(_) => Bool3::False,
            None => Bool3::Unknown,
        },
        Compiled::DateContains { field, lit, negate } => {
            let base = match person_date(person, *field) {
                Some(recorded) => date_contains(recorded, lit),
                None => Bool3::Unknown,
            };
            if *negate { base.not() } else { base }
        }
        Compiled::DateOrder { field, lit, op } => match person_date(person, *field) {
            Some(recorded) => date_order(recorded, lit, *op),
            None => Bool3::Unknown,
        },
        Compiled::Present { field, negate } => {
            let base = if person_present(person, *field) {
                Bool3::True
            } else {
                Bool3::False
            };
            if *negate { base.not() } else { base }
        }
    }
}

/// `eq`/`neq` on a date: True iff the recorded interval is wholly contained in
/// the literal's period; False iff they are disjoint; else Unknown.
fn date_contains(recorded: &DateLit, lit: &DateLit) -> Bool3 {
    let (rl, ru) = (recorded.lower_bound(), recorded.upper_bound());
    let (ll, lu) = (lit.lower_bound(), lit.upper_bound());
    if ll <= rl && ru <= lu {
        Bool3::True
    } else if ru < ll || lu < rl {
        Bool3::False
    } else {
        Bool3::Unknown
    }
}

/// Ordering on a date: True iff the comparison holds under *every*
/// interpretation of both closed intervals; False iff under *none*; else
/// Unknown. Expressed entirely on the [`DateLit`] interval bounds — the same
/// machinery `before_strict` is built on, no second comparison.
fn date_order(recorded: &DateLit, lit: &DateLit, op: OrderOp) -> Bool3 {
    let (rl, ru) = (recorded.lower_bound(), recorded.upper_bound());
    let (ll, lu) = (lit.lower_bound(), lit.upper_bound());
    let (holds_all, holds_none) = match op {
        // recorded < literal
        OrderOp::Lt => (ru < ll, rl >= lu),
        // recorded <= literal
        OrderOp::Lte => (ru <= ll, rl > lu),
        // recorded > literal
        OrderOp::Gt => (rl > lu, ru <= ll),
        // recorded >= literal
        OrderOp::Gte => (rl >= lu, ru < ll),
    };
    if holds_all {
        Bool3::True
    } else if holds_none {
        Bool3::False
    } else {
        Bool3::Unknown
    }
}

/// The person's value for a string field (`None` when absent). `id` is always
/// present; `gender` renders to its canonical token.
fn person_string(person: &PersonStmt, field: PersonField) -> Option<String> {
    match field {
        PersonField::Id => Some(person.id.name.clone()),
        PersonField::Name => person.name().map(|v| v.value.clone()),
        PersonField::Family => person.family().map(|v| v.value.clone()),
        PersonField::Given => person.given().map(|v| v.value.clone()),
        PersonField::Gender => person.gender().map(|g| g.value.as_token().to_string()),
        PersonField::Born | PersonField::Died => None,
    }
}

/// The person's recorded date for a date field (`None` for string fields or an
/// absent date).
fn person_date(person: &PersonStmt, field: PersonField) -> Option<&DateLit> {
    match field {
        PersonField::Born => person.born(),
        PersonField::Died => person.died(),
        _ => None,
    }
}

/// Whether the field is recorded (`id` always is).
fn person_present(person: &PersonStmt, field: PersonField) -> bool {
    match field {
        PersonField::Id => true,
        PersonField::Name => person.name().is_some(),
        PersonField::Family => person.family().is_some(),
        PersonField::Given => person.given().is_some(),
        PersonField::Gender => person.gender().is_some(),
        PersonField::Born => person.born().is_some(),
        PersonField::Died => person.died().is_some(),
    }
}

/// A sort key value: a date interval (ordered by lower then upper bound) or a
/// string (codepoint order). The two never mix within one sort — the field is
/// fixed — so the derived `Ord` (which would order `Str < Date`) is never
/// exercised across variants.
#[derive(PartialEq, Eq, PartialOrd, Ord)]
enum SortKey {
    Str(String),
    Date(CalendarDay, CalendarDay),
}

fn sort_key(person: &PersonStmt, field: PersonField) -> Option<SortKey> {
    if field.is_date() {
        person_date(person, field).map(|d| SortKey::Date(d.lower_bound(), d.upper_bound()))
    } else {
        person_string(person, field).map(SortKey::Str)
    }
}

/// Compare two persons under `spec`. **Missing values sort last regardless of
/// direction**; among present values the direction applies; ties (and two
/// missing values) are always broken by person id ascending. Fully
/// deterministic.
pub(crate) fn sort_compare(a: &PersonStmt, b: &PersonStmt, spec: &SortSpec) -> Ordering {
    let id_tiebreak = || a.id.name.cmp(&b.id.name);
    match (sort_key(a, spec.field), sort_key(b, spec.field)) {
        (Some(ka), Some(kb)) => {
            let base = ka.cmp(&kb);
            let directed = match spec.direction {
                SortDirection::Asc => base,
                SortDirection::Desc => base.reverse(),
            };
            directed.then_with(id_tiebreak)
        }
        // Present sorts before missing in both directions.
        (Some(_), None) => Ordering::Less,
        (None, Some(_)) => Ordering::Greater,
        (None, None) => id_tiebreak(),
    }
}
