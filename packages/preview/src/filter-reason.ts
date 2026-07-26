// Why the filter could not judge this person.
//
// The engine answers in **sets**: a `certain` result and an `includeUncertain`
// one, whose difference is exactly the persons its three-valued conjunction
// answered `unknown` for. It does not say which condition went unknown, or
// why, and asking it to would mean a new operation and a new wire shape for a
// string (ADR-0045).
//
// So the reason is derived here — and the line this module does not cross is
// what makes that safe. **It never re-decides a truth value.** The engine has
// already ruled the person unjudgeable; this module only reads the *record*
// and names what about it is unjudgeable-shaped:
//
//   - a field the predicate needs that the author never recorded, and
//   - a recorded date whose interval is wider than a day, or a literal whose
//     interval is, so the comparison has interpretations that disagree.
//
// Both are structural facts, visible without evaluating anything: is the field
// there, and is either interval a point. Every route to `unknown` in
// `filter.rs` passes through one of them, so the candidates this finds always
// contain the real culprit. When exactly one condition is a candidate, it *is*
// the culprit and the reason names the comparison it straddles; when several
// are, all of them are named and none is claimed to be the one.
//
// A three-valued re-implementation was rejected outright: `kul-core`'s
// `filter__*.snap` suites own predicate correctness, and a second evaluator in
// the preview would be a second answer that could disagree with the paint
// beside it (PRD-0006, Testing Decisions).

import type { ExportedDate, ExportedPerson } from "./engine-wire.js";
import {
    type FilterCondition,
    type FilterField,
    conditionLabel,
    fieldKind,
    isPresenceOp,
} from "./filter.js";

/** The opening of every whisper, so the paint and the words agree. */
export const CANT_SAY_LEAD = "can't say";

/** The recorded value of one filterable field, or `undefined`. */
function recorded(
    person: ExportedPerson,
    field: FilterField,
): string | ExportedDate | undefined {
    switch (field) {
        case "name":
            return person.name;
        case "family":
            return person.family;
        case "given":
            return person.given;
        case "gender":
            return person.gender;
        case "born":
            return person.born;
        case "died":
            return person.died;
    }
}

/** A date as the author wrote it: the circa marker back on the front. */
function dateText(date: ExportedDate): string {
    return (date.circa ? "~" : "") + date.value;
}

/**
 * How wide the recorded value's interval is, in words, or `null` when it is a
 * single day and therefore comparable without ambiguity.
 *
 * Circa outranks precision: `~1942-03-04` is a day-precision date whose
 * interval is ten years wide, so the circa note is the one that explains it.
 * The ±5 years is the engine's own reading of `~` (`filter.rs`), stated here
 * because the reader is being told why a comparison could not be made.
 */
function widthNote(date: ExportedDate): string | null {
    if (date.circa) {
        return "approximate (±5y)";
    }
    if (date.precision === "year") {
        return "year-precision";
    }
    if (date.precision === "month") {
        return "month-precision";
    }
    return null;
}

/** The same question about the literal the reader typed into the chip. */
function literalWidthNote(value: string): string | null {
    if (value.startsWith("~")) {
        return "approximate (±5y)";
    }
    if (/^\d{4}$/.test(value)) {
        return "a whole year";
    }
    if (/^\d{4}-\d{2}$/.test(value)) {
        return "a whole month";
    }
    return null;
}

/**
 * Why `condition` may be the unjudgeable one for `person`, or `null` when the
 * record decides it either way.
 */
function candidateClause(
    person: ExportedPerson,
    condition: FilterCondition,
): string | null {
    // Presence predicates are decidable on any record: the field is there or
    // it is not. They can never be the culprit, and saying so keeps them out
    // of an ambiguous multi-clause reason.
    if (isPresenceOp(condition.op)) {
        return null;
    }
    const value = recorded(person, condition.field);
    if (value === undefined || value === "") {
        return `${condition.field} not recorded`;
    }
    if (fieldKind(condition.field) !== "date") {
        // A recorded string is compared exactly and case-sensitively, which is
        // two-valued (ADR-0025).
        return null;
    }
    const date = value as ExportedDate;
    const recordedNote = widthNote(date);
    if (recordedNote) {
        return `${dateText(date)} is ${recordedNote} — straddles ${conditionLabel(condition)}`;
    }
    const literalNote = literalWidthNote(condition.value);
    if (literalNote) {
        return `${conditionLabel(condition)} spans ${literalNote} — ${dateText(date)} falls inside it`;
    }
    return null;
}

/**
 * The whisper for one unjudgeable person: *"can't say — family not
 * recorded"*, *"can't say — ~1942 is approximate (±5y) — straddles born <
 * 1945"*.
 *
 * With no candidate — which the engine's verdict says should not happen, and
 * which a record shape nobody anticipated could still produce — it degrades to
 * the bare lead rather than inventing an explanation.
 */
export function cantSayReason(
    person: ExportedPerson,
    conditions: ReadonlyArray<FilterCondition>,
): string {
    const clauses: string[] = [];
    for (const condition of conditions) {
        const clause = candidateClause(person, condition);
        if (clause && !clauses.includes(clause)) {
            clauses.push(clause);
        }
    }
    if (clauses.length === 0) {
        return CANT_SAY_LEAD;
    }
    return `${CANT_SAY_LEAD} — ${clauses.join(" · ")}`;
}
