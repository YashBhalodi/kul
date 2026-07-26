// The Explore-kin list, as a value.
//
// One flat scrollable list of every kin set, with the engine's count beside
// each. No grouping — #276's resolution rejected the five-group taxonomy — and
// no results list anywhere: clicking a row paints the tree, which is where the
// answer is read.
//
// The module is pure (no DOM), like `detail-panel.ts`'s model half, so the two
// things worth proving about a row are provable at a value seam: that its count
// is the engine's, and that its gloss is phrased in the active pack.
//
// PHRASING PROVENANCE. A row's gloss is the distinct terms of the members the
// engine returned for that row, phrased through the language pack. `queryKin`
// hands back a full `RelationshipDescriptor` per member, which is exactly what
// `phrase` consumes, so a term costs no lookup of its own (#292 Finding 6).
// Only the painted row has members in hand, so only the painted row glosses —
// an unpainted row has nothing to phrase, and inventing a term for it would be
// the surface asserting something the engine was never asked.

import type { Member } from "./engine-wire.js";
import { phrase } from "./phrasing/index.js";
import type { LanguagePack } from "./phrasing/index.js";
import { KIN_SETS } from "./kin-sets.js";

/** Header the list hangs off, and the control that opens it. */
export const KIN_LIST_TITLE = "Explore kin";

/**
 * What the list knows right now. Counts arrive per set as the engine answers,
 * so a set with no entry has simply not been answered yet — which the row
 * renders as a placeholder rather than as a zero. A zero is an answer and must
 * never be faked (the epic's honesty stance — #296).
 */
export interface KinListState {
    /** The person the questions are about. */
    anchorId: string;
    /** Whether the reader has opened the list. It stays open as they walk. */
    open: boolean;
    /** Engine `count` projections, by set id. */
    counts: ReadonlyMap<string, number>;
    /** The set whose answer is painted on the tree, if any. */
    activeSetId: string | null;
    /** The painted set's members, as the engine returned them. */
    activeMembers: ReadonlyArray<Member>;
}

/** One rendered row. */
export interface KinRowModel {
    id: string;
    label: string;
    /** The engine's count, or `null` while the answer is still in flight. */
    count: number | null;
    /** True for the row whose answer is painted. */
    active: boolean;
    /**
     * Distinct kinship terms of this row's members, in member order, phrased in
     * the active pack. Empty except on the painted row (see the module note).
     */
    terms: string[];
}

export interface KinListModel {
    title: string;
    open: boolean;
    rows: KinRowModel[];
}

/**
 * The list for one anchor, phrased in one pack.
 *
 * Rows are the catalogue in its declared order, always all of them: a set with
 * zero members is a row reading `0`, never a row that disappears. "No cousins"
 * is an answer and it is shown as one.
 */
export function buildKinList(
    state: KinListState,
    pack: LanguagePack,
): KinListModel {
    return {
        title: KIN_LIST_TITLE,
        open: state.open,
        rows: KIN_SETS.map((set) => {
            const active = state.activeSetId === set.id;
            return {
                id: set.id,
                label: set.label,
                count: state.counts.get(set.id) ?? null,
                active,
                terms: active ? distinctTerms(state.activeMembers, pack) : [],
            };
        }),
    };
}

/**
 * The terms of a member list, deduplicated, in member order.
 *
 * Deduplicated because a row is a *set* of ties, not a roll-call: four siblings
 * read "brother · sister", not "brother · brother · sister · sister". Order is
 * the engine's pinned member order, so the gloss is deterministic.
 */
function distinctTerms(
    members: ReadonlyArray<Member>,
    pack: LanguagePack,
): string[] {
    const seen = new Set<string>();
    for (const member of members) {
        const text = phrase(member.descriptor, pack).text;
        if (text) {
            seen.add(text);
        }
    }
    return [...seen];
}

/**
 * What the quiet toast says when a kin set comes back empty.
 *
 * The anchor is named from the display name the batched detail lookup already
 * supplied (ADR-0037) — the panel is holding that answer, so the sentence costs
 * no second provenance path and no second call.
 */
export function emptyKinMessage(setLabel: string, anchorName: string): string {
    return (
        `No ${setLabel.toLowerCase()} recorded for ${anchorName}. ` +
        "Nothing is guessed — absence stays absence."
    );
}
