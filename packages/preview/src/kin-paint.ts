// Kin paint — an answered kin-set query, drawn on the tree.
//
// The tree *is* the result (#276's resolution, PRD-0006): there is no list of
// matched people beside the canvas, so this paint is the whole readout. It is
// stateless — prior paint is stripped before anything is applied, so calling
// `paintKinResults` after a render re-applies the same answer onto the fresh
// SVG.
//
// **Every card a matched person owns lights**, canonical and ghost alike. That
// is ADR-0034's binding rule — a result is about a *person*, not about a card —
// and it is the acceptance criterion this module exists to hold: a person with
// two past-intimacy ghosts lights three cards from one result. The binding
// itself is `selectBoundNodes`, unchanged and shared with the selection.
//
// The **anchor is neither lit nor dimmed**. Not dimmed, because the card the
// question is about must not be the faintest thing on screen; not lit, because
// it wears the selection violet and a card cannot be both the question and the
// answer. The engine excludes an anchor from its own answers, so the filter
// below is belt-and-braces — but the rule is this module's to state rather than
// the engine's to be trusted for (ADR-0043).
//
// Non-matching **cards** dim; edges never do. Dimming the edges would recede the
// picture the answer is drawn on, and #277's standing rule is only that paint
// never *hides*. The dim itself belongs to `dim.ts`, which owns the class for
// every source: this module publishes a set of person ids and nothing more.

import type { DimRegistry } from "./dim.js";
import { selectBoundNodes } from "./result-binding.js";

/** Class a matched person's cards wear. Painted with the reserved query teal. */
export const RESULT_CLASS = "kul-query-result";

/** This paint's handle in the shared {@link DimRegistry}. */
export const KIN_DIM_SOURCE = "kin";

/** One answered kin set: whose question it was, and who answered it. */
export interface KinAnswer {
    anchorId: string;
    personIds: ReadonlyArray<string>;
}

function stripResults(root: ParentNode): void {
    for (const node of root.querySelectorAll("." + RESULT_CLASS)) {
        node.classList.remove(RESULT_CLASS);
    }
}

/** Strip the result glow and withdraw this source's dim. */
export function clearKinPaint(root: ParentNode, dim: DimRegistry): void {
    stripResults(root);
    dim.set(KIN_DIM_SOURCE, null);
    dim.apply(root);
}

/**
 * Repaint `root` for one answered kin set.
 *
 * A `null` answer clears the paint. An **empty** answer is not the same thing:
 * it paints nothing and recedes everything but the anchor, which is what an
 * honest zero looks like on a tree (the toast says so in words).
 */
export function paintKinResults(
    root: ParentNode,
    answer: KinAnswer | null,
    dim: DimRegistry,
): void {
    stripResults(root);
    if (!answer) {
        clearKinPaint(root, dim);
        return;
    }
    const lit = new Set(answer.personIds);
    lit.delete(answer.anchorId);
    for (const personId of lit) {
        for (const node of selectBoundNodes(root, { kind: "person", personId })) {
            node.classList.add(RESULT_CLASS);
        }
    }
    const dimmed = new Set<string>();
    for (const card of root.querySelectorAll("[data-person-id]")) {
        const id = card.getAttribute("data-person-id") ?? "";
        if (id && id !== answer.anchorId && !lit.has(id)) {
            dimmed.add(id);
        }
    }
    dim.set(KIN_DIM_SOURCE, dimmed);
    dim.apply(root);
}
