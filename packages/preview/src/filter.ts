// The attribute filter, as a value.
//
// Everything about *what the reader asked* lives here, and nothing about how
// it is drawn: the chip sentence's vocabulary, the `Query` value that sentence
// spells, and the arithmetic behind the tally. A pure module — no DOM — so the
// two properties #303 has to prove (the sentence produces the right `Query`,
// the tally adds up) are provable at this seam rather than through chrome.
//
// The vocabulary is a **subset** of the engine's, chosen rather than mirrored:
//
// - **Conjunction only, and the sentence says so.** `where` is a list of
//   conditions the engine ANDs. There is no disjunction in this module — no OR
//   node, no group, no precedence — because OR is permanently out of engine
//   scope (ADR-0025), and a surface that modelled it would advertise something
//   the engine will never do. The `and` between chips is literal text for the
//   same reason: the grammar shows the restriction instead of documenting it.
// - **Ops are field-aware.** Ordering comparisons are offered only for the two
//   date fields, and `∈` only for the free-text ones. The engine would accept
//   `gender < male`; it is meaningless, so the sentence cannot say it.
// - **`id` is not offered.** It is a person's address, not an attribute of
//   them — the reader filters on what a record *says*, and every other field
//   here is something an author wrote (ADR-0045).
//
// Certainty is part of the value, not a view option: `mode` rides inside the
// `Query`, which is why it is a chip in the sentence rather than a toggle
// beside it.

import type {
    FilterMode,
    PersonField,
    Predicate,
    Query,
    QuerySource,
} from "./engine-wire.js";

/** The person fields the sentence can name. `id` is deliberately absent. */
export type FilterField = "name" | "family" | "given" | "gender" | "born" | "died";

/**
 * What kind of value a field holds — which is what decides its op menu.
 * `text` is free text an author typed, `enum` a fixed vocabulary, `date` a
 * date literal with interval semantics.
 */
export type FieldKind = "text" | "enum" | "date";

/** The ops the sentence can spell. A subset of the engine's `Predicate` ops. */
export type FilterOp =
    | "eq"
    | "neq"
    | "in"
    | "lt"
    | "lte"
    | "gt"
    | "gte"
    | "present"
    | "absent";

/** The fields, in the order the field picker offers them. */
export const FILTER_FIELDS: ReadonlyArray<FilterField> = [
    "name",
    "family",
    "given",
    "gender",
    "born",
    "died",
];

const FIELD_KIND: Record<FilterField, FieldKind> = {
    name: "text",
    family: "text",
    given: "text",
    gender: "enum",
    born: "date",
    died: "date",
};

/** The `gender:` vocabulary the language defines. Not free text. */
export const GENDER_VALUES: ReadonlyArray<string> = ["male", "female", "other"];

export function fieldKind(field: FilterField): FieldKind {
    return FIELD_KIND[field];
}

/**
 * The ops offered for `field`.
 *
 * Ordering comparisons appear only on the date fields and `∈` only on the text
 * ones; equality and the two presence ops are universal. The menu *is* the
 * restriction — there is no op the sentence can spell that this list omits.
 */
export function opsFor(field: FilterField): ReadonlyArray<FilterOp> {
    switch (fieldKind(field)) {
        case "text":
            return ["eq", "neq", "in", "present", "absent"];
        case "enum":
            return ["eq", "neq", "present", "absent"];
        case "date":
            return ["eq", "neq", "lt", "lte", "gt", "gte", "present", "absent"];
    }
}

/** True when `op` takes no value — the two presence predicates. */
export function isPresenceOp(op: FilterOp): boolean {
    return op === "present" || op === "absent";
}

/**
 * One chip: a field, an op, and the value the reader typed.
 *
 * `value` is a single string even for `∈`, which takes a comma-separated list.
 * The split happens at the boundary where the `Query` is built, so the chip
 * round-trips exactly what was typed and an editor never has to reassemble it.
 * A presence op ignores `value` entirely.
 */
export interface FilterCondition {
    field: FilterField;
    op: FilterOp;
    value: string;
}

/**
 * The population a filter runs over, and the `Query` source that names it.
 *
 * Two of them exist. `everyone` is the project; a **kin scope** is a painted
 * kin set, which is what makes "which of Giuseppe's descendants were born
 * before 1950" one question instead of two. `personIds` is the *unfiltered*
 * population — the tally's denominator and the set the dim is computed
 * against — and it is supplied rather than derived, because only the surface
 * that painted the kin set knows who is in it.
 */
export interface FilterScope {
    /** Stable handle for the scope chip's menu. */
    id: string;
    /** How the chip and the tally name it — "Everyone", "Giuseppe's descendants". */
    label: string;
    /** The `Query` source it spells. */
    source: QuerySource;
    /** Everyone in it, before any condition applies. */
    personIds: ReadonlySet<string>;
}

/** The whole sentence: a scope, a conjunction of conditions, and a mode. */
export interface FilterState {
    scopeId: string;
    conditions: ReadonlyArray<FilterCondition>;
    mode: FilterMode;
}

/** The empty sentence: everyone, nothing asked, certain. */
export const EMPTY_FILTER: FilterState = {
    scopeId: "everyone",
    conditions: [],
    mode: "certain",
};

/** The scope chip's default entry. Its population is the picture's. */
export const EVERYONE_SCOPE_ID = "everyone";

/**
 * True when a chip says something the engine can evaluate.
 *
 * A chip is born the moment `+` is pressed and is edited afterwards, so a
 * half-written one is a normal state rather than an error. `family = ` is not
 * the question "who has an empty family name" — it is a question nobody has
 * finished asking, and answering it would paint a confident zero.
 */
export function isCompleteCondition(condition: FilterCondition): boolean {
    if (isPresenceOp(condition.op)) {
        return true;
    }
    if (condition.op === "in") {
        return parseInValues(condition.value).length > 0;
    }
    return condition.value.trim().length > 0;
}

/** The conditions that actually reach the engine: the complete ones. */
export function askedConditions(state: FilterState): FilterCondition[] {
    return state.conditions.filter(isCompleteCondition);
}

/**
 * A filter is active iff it asks something.
 *
 * A scope on its own is not a filter — it is a kin set someone else painted —
 * and neither is a certainty mode with nothing to be certain about, nor a chip
 * nobody has finished writing. This is what "an active filter suspends editor
 * sync" is measured against, so it has to mean *the reader asked a question*,
 * not *the bar exists*.
 */
export function isFilterActive(state: FilterState): boolean {
    return askedConditions(state).length > 0;
}

/** The engine's field name for one of ours. They coincide; the cast is the point. */
function personField(field: FilterField): PersonField {
    return field;
}

/**
 * Split an `∈` value into its members: comma-separated, trimmed, empties
 * dropped. Matching stays exact and case-sensitive inside the engine (ADR-0025
 * pins that permanently), so this only decides where one value ends.
 */
export function parseInValues(value: string): string[] {
    return value
        .split(",")
        .map((part) => part.trim())
        .filter((part) => part.length > 0);
}

/** One chip as the engine's `Predicate`. */
export function toPredicate(condition: FilterCondition): Predicate {
    const field = personField(condition.field);
    switch (condition.op) {
        case "present":
        case "absent":
            return { op: condition.op, field };
        case "in":
            return { op: "in", field, values: parseInValues(condition.value) };
        default:
            return { op: condition.op, field, value: condition.value };
    }
}

/**
 * The `Query` the sentence spells, in one certainty mode.
 *
 * `projection` is always `members`: the result kind is fixed by (source,
 * projection), so an `allPersons` source answers `personIds` and a `kinOf` one
 * answers `members`, and both carry the ids the paint needs. `count` would
 * answer the tally's numerator and nothing else, which would cost a third call
 * for a number the id set already has.
 *
 * `sort` is never set. The tree keeps its canonical layout order and only a
 * list can express an order; the preview has none (#296, Out of Scope).
 *
 * `mode` is a parameter rather than read off `state`, because the surface asks
 * **both** questions on every evaluation: the difference between the two
 * answers *is* the unjudgeable set. `state.mode` decides which of the two the
 * reader is shown, not which of them is asked.
 */
export function buildFilterQuery(
    state: FilterState,
    scope: FilterScope,
    mode: FilterMode,
): Query {
    return {
        source: scope.source,
        where: askedConditions(state).map(toPredicate),
        mode,
        projection: "members",
    };
}

/**
 * How an op reads in the sentence.
 *
 * The presence ops read as words rather than symbols because they are the two
 * that make a claim about the *record* instead of about a value, and `died ∅`
 * would hide that. "not recorded" is also exactly what it means — never
 * "living", which is the fact the data does not carry (#296 story 26).
 */
export function opLabel(op: FilterOp): string {
    switch (op) {
        case "eq":
            return "=";
        case "neq":
            return "≠";
        case "in":
            return "∈";
        case "lt":
            return "<";
        case "lte":
            return "≤";
        case "gt":
            return ">";
        case "gte":
            return "≥";
        case "present":
            return "recorded";
        case "absent":
            return "not recorded";
    }
}

/**
 * One chip's text: "family = Rossi", "born < 1950", "died not recorded" — and
 * "family =" for one the reader has not finished.
 */
export function conditionLabel(condition: FilterCondition): string {
    const parts = [condition.field, opLabel(condition.op)];
    if (!isPresenceOp(condition.op)) {
        // A chip nobody has finished writing reads as far as it has been
        // written — "family =" — rather than trailing an empty value.
        const value =
            condition.op === "in"
                ? parseInValues(condition.value).join(", ")
                : condition.value.trim();
        if (value) {
            parts.push(value);
        }
    }
    return parts.join(" ");
}

/**
 * The line under the presence ops, shown wherever they are offered.
 *
 * A presence predicate is decidable, which makes it read more confident than
 * it is: the record either carries a death date or it does not, and neither
 * answer says whether the person is alive. The surface says so where the
 * predicate is offered rather than in a doc nobody opens (#277).
 */
export const PRESENCE_DISCLOSURE =
    "“not recorded” is a fact about the record, not about the person — died not recorded is not “living”.";

/**
 * The three groups a filter splits its scope into. They **partition** it:
 * every person in scope is in exactly one, whatever the certainty mode.
 *
 * This is the *one* place the split is computed. The tally counts it and the
 * paint draws it, so the number the reader is told and the colour they are
 * shown can never come from two different arithmetics.
 */
export interface FilterPartition {
    /** Persons every condition answered **true** for — the teal ones. */
    matched: ReadonlySet<string>;
    /** Persons the conjunction answered **unknown** for — the amber ones. */
    cantSay: ReadonlySet<string>;
    /** Persons the conjunction answered **false** for — the receded ones. */
    dimmed: ReadonlySet<string>;
}

/**
 * Partition a scope from the two answers the engine gave.
 *
 * `certainIds` is the `certain`-mode result and `inclusiveIds` the
 * `includeUncertain` one, so the unjudgeable are exactly the difference — the
 * engine's own three-valued verdict, read off two of its answers rather than
 * re-derived here. Both are intersected with the scope, because a `kinOf`
 * source answers about its own set and what is counted has to agree with what
 * is painted.
 */
export function partitionScope(
    population: ReadonlySet<string>,
    certainIds: Iterable<string>,
    inclusiveIds: Iterable<string>,
): FilterPartition {
    const matched = new Set<string>();
    for (const id of certainIds) {
        if (population.has(id)) {
            matched.add(id);
        }
    }
    const cantSay = new Set<string>();
    for (const id of inclusiveIds) {
        if (population.has(id) && !matched.has(id)) {
            cantSay.add(id);
        }
    }
    const dimmed = new Set<string>();
    for (const id of population) {
        if (!matched.has(id) && !cantSay.has(id)) {
            dimmed.add(id);
        }
    }
    return { matched, cantSay, dimmed };
}

/**
 * A counted {@link FilterPartition}.
 *
 * `matched + cantSay + dimmed === total`. What the mode changes is `shown` —
 * `certain` shows the matched alone, `includeUncertain` shows the matched plus
 * the unjudgeable — and never a count, so the disclosure certain mode owes the
 * reader is the same number whichever way they are looking (#296 story 25).
 */
export interface FilterTally {
    /** Everyone in the scope. */
    total: number;
    matched: number;
    cantSay: number;
    dimmed: number;
    /** Persons the active mode shows: `matched`, or `matched + cantSay`. */
    shown: number;
}

/** Count one partition under one certainty mode. */
export function computeTally(
    partition: FilterPartition,
    mode: FilterMode,
): FilterTally {
    const matched = partition.matched.size;
    const cantSay = partition.cantSay.size;
    const dimmed = partition.dimmed.size;
    return {
        total: matched + cantSay + dimmed,
        matched,
        cantSay,
        dimmed,
        shown: mode === "certain" ? matched : matched + cantSay,
    };
}

/**
 * The tally in words: *"3 of 10 · 6 dimmed · 1 can't say"*.
 *
 * A kin scope names itself right after the count — *"3 of 4 in Giuseppe's
 * descendants"* — because a bare "3 of 4" against a ten-person tree reads as a
 * bug. The can't-say clause is **never** dropped when it is non-zero: certain
 * mode's whole disclosure obligation is that number, and `includeUncertain`
 * marks it "included" so the same number never means two things.
 *
 * Zero-valued clauses are dropped, so an unambiguous answer reads as one.
 */
export function tallyText(
    tally: FilterTally,
    mode: FilterMode,
    scopeLabel: string | null,
): string {
    const scope = scopeLabel ? ` in ${scopeLabel}` : "";
    const parts = [`${tally.shown} of ${tally.total}${scope}`];
    if (tally.dimmed > 0) {
        parts.push(`${tally.dimmed} dimmed`);
    }
    if (tally.cantSay > 0) {
        parts.push(
            mode === "certain"
                ? `${tally.cantSay} can't say`
                : `${tally.cantSay} can't say, included`,
        );
    }
    return parts.join(" · ");
}

/** How the certainty chip reads in the sentence. */
export function modeLabel(mode: FilterMode): string {
    return mode === "certain" ? "certain" : "certain + can't say";
}
