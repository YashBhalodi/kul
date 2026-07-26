// The kin-set catalogue — the thirteen questions "Explore kin" can ask, each
// spelled as the engine's own {@link Query} value.
//
// PROVENANCE. Every pattern below is the expansion `crates/kul-core/src/query/
// sugar.rs` documents for the named sugar of the same name (`parents_of`,
// `children_of`, … `step_children_of`). **No sugar reaches JavaScript**: the
// WASM surface takes a `Query` value and evaluates it, so a consumer that wants
// "siblings" builds `kinOf(x, collateral up{1,1} down{1,1})` itself. That is
// exactly the role ADR-0025 gives a surface — *the surfaces are thin
// constructors of this value, never second engines* — and no membership rule
// lives here; the engine answers every one of these (ADR-0043).
//
// The copy is **enforced, not trusted**: `crates/kul-core/tests/kin_catalogue.rs`
// snapshots the `KinPattern` the engine itself builds for each sugar (and proves
// each is behaviourally that sugar over the corpus), and
// `packages/preview/tests/kin-sets.test.ts` reads that snapshot and fails when
// anything below has drifted from it.
//
// The list is deliberately **flat and ordered**, with no grouping: #276
// prototyped a five-group taxonomy (Immediate / Lineage / Extended / In-laws /
// Step) and its resolution replaced it with "one flat scrollable list of all
// kin-set queries with per-set counts — no category grouping". The order below
// is the one that resolution names, "Parents … Step-children".

import type { KinPattern, Projection, Query } from "./engine-wire.js";

/** One row of the Explore-kin list. */
export interface KinSet {
    /** Stable identity, used as the row's handle and in tests. */
    id: string;
    /**
     * The row's name, in English.
     *
     * Chrome copy, like every other label in the panel: #276 point 6 keeps
     * chrome English while the locale setting governs kinship *phrasing*. The
     * phrased half of a row is its term gloss, which comes from the members'
     * descriptors and re-phrases with the toggle (ADR-0043).
     */
    label: string;
    /** The engine-side shape of the question, minus the anchor. */
    pattern: KinPattern;
}

/** `{min: n, max: n}` — the range containing exactly `n`. */
function exactly(n: number): { min: number; max: number } {
    return { min: n, max: n };
}

/** `{min: 1}` — at least one generation, unbounded above. */
const FROM_ONE = { min: 1 };

export const KIN_SETS: ReadonlyArray<KinSet> = [
    {
        id: "parents",
        label: "Parents",
        pattern: {
            classification: { kind: "lineal", role: "ancestor", generations: exactly(1) },
        },
    },
    {
        id: "children",
        label: "Children",
        pattern: {
            classification: { kind: "lineal", role: "descendant", generations: exactly(1) },
        },
    },
    {
        id: "siblings",
        label: "Siblings",
        pattern: {
            classification: { kind: "collateral", up: exactly(1), down: exactly(1) },
        },
    },
    {
        id: "spouses",
        label: "Spouses",
        // `any {0,0}` plus exactly one marriage hop: zero vertical displacement
        // is what makes this the spouse and not the spouse's kin, and the
        // `affinalHops` bound is what opens the affinal budget at all.
        pattern: {
            classification: { kind: "any", maxUp: 0, maxDown: 0 },
            affinalHops: exactly(1),
        },
    },
    {
        id: "ancestors",
        label: "Ancestors",
        pattern: {
            classification: { kind: "lineal", role: "ancestor", generations: FROM_ONE },
        },
    },
    {
        id: "descendants",
        label: "Descendants",
        pattern: {
            classification: { kind: "lineal", role: "descendant", generations: FROM_ONE },
        },
    },
    {
        id: "auntsUncles",
        label: "Aunts & uncles",
        pattern: {
            classification: { kind: "collateral", up: exactly(2), down: exactly(1) },
        },
    },
    {
        id: "niecesNephews",
        label: "Nieces & nephews",
        pattern: {
            classification: { kind: "collateral", up: exactly(1), down: exactly(2) },
        },
    },
    {
        id: "cousins",
        label: "First cousins",
        // `cousins_of(x, degree, removed)` is the general form; the list offers
        // the one cell a reader means by "cousins". Second cousins and removals
        // are the same Query with different numbers, and the list does not
        // enumerate them — thirteen rows that each answer a question a reader
        // asks beats forty that spell a lattice.
        pattern: {
            classification: {
                kind: "collateralByDegree",
                degree: exactly(1),
                removed: exactly(0),
            },
        },
    },
    {
        id: "inLaws",
        label: "In-laws",
        pattern: {
            classification: { kind: "any", maxUp: 2, maxDown: 2 },
            affinity: "inLaw",
        },
    },
    {
        id: "stepParents",
        label: "Step-parents",
        pattern: {
            classification: { kind: "lineal", role: "ancestor", generations: exactly(1) },
            affinity: "step",
        },
    },
    {
        id: "stepSiblings",
        label: "Step-siblings",
        pattern: {
            classification: { kind: "collateral", up: exactly(1), down: exactly(1) },
            affinity: "step",
        },
    },
    {
        id: "stepChildren",
        label: "Step-children",
        pattern: {
            classification: { kind: "lineal", role: "descendant", generations: exactly(1) },
            affinity: "step",
        },
    },
];

/** The set with this id, or `undefined`. */
export function kinSetById(id: string): KinSet | undefined {
    return KIN_SETS.find((set) => set.id === id);
}

/**
 * The Query value for one set against one anchor.
 *
 * The projection is the caller's, and the two callers want different ones:
 * opening the list asks for `count` (the engine's own count, never a client
 * side length), and painting a row asks for `members` (ids to bind, descriptors
 * to phrase). Same value, same evaluator, one field apart (ADR-0043).
 */
export function kinQuery(
    set: KinSet,
    anchor: string,
    projection: Projection,
): Query {
    return {
        source: { kind: "kinOf", anchor, pattern: set.pattern },
        projection,
    };
}
