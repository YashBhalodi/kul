// The filter, drawn on the tree.
//
// **Filtering never hides a node or an edge — it only dims** (#277's standing
// rule, generalised past this feature). Hiding punches holes in the canonical
// layout and leaves marriage stubs and birth edges running into empty space,
// so every filter state leaves the picture structurally identical and changes
// only how it reads. Nothing here removes a node, sets `hidden`, or touches
// `display`.
//
// Three states, because the engine has three answers:
//
//   - **matched** — every condition answered `true`. The reserved teal.
//   - **can't say** — the conjunction answered `unknown`. Amber, dashed, and a
//     `?` riding the card. It is *not* dimmed: an unjudgeable person that
//     receded like a non-match would be exactly the silent drop the disclosure
//     exists to prevent (PRD-0006 story 23).
//   - **dimmed** — answered `false`. The shared dim, and no colour of its own.
//
// The dim class is never touched here: this publishes a set of person ids and
// asks the registry to recompute, so the class keeps one owner and one alpha
// whatever else is painting (ADR-0043). The set is *republished* on every
// paint rather than left standing, which is that module's rule for a source
// whose ids are derived from the cards the picture holds — a swapped-out SVG
// can change who is in it.
// The match glow is this module's **own** class rather than kin paint's: the
// two co-exist whenever a filter runs inside a painted kin set, and two owners
// stripping one class is precisely the failure the dim registry exists to
// prevent (ADR-0045).

import type { DimRegistry } from "./dim.js";
import type { FilterPartition } from "./filter.js";
import { selectBoundNodes } from "./result-binding.js";

/** Class a matching person's cards wear. Painted with the reserved query teal. */
export const FILTER_MATCH_CLASS = "kul-filter-match";

/** Class an unjudgeable person's cards wear. Painted with the reserved amber. */
export const FILTER_UNCERTAIN_CLASS = "kul-filter-uncertain";

/** Class on the `?` glyph this module injects into an unjudgeable card. */
export const UNCERTAIN_BADGE_CLASS = "kul-filter-uncertain-badge";

/** This paint's handle in the shared {@link DimRegistry}. */
export const FILTER_DIM_SOURCE = "filter";

const SVG_NS = "http://www.w3.org/2000/svg";

function strip(root: ParentNode): void {
    for (const node of root.querySelectorAll(
        "." + FILTER_MATCH_CLASS + ", ." + FILTER_UNCERTAIN_CLASS,
    )) {
        node.classList.remove(FILTER_MATCH_CLASS);
        node.classList.remove(FILTER_UNCERTAIN_CLASS);
    }
    for (const badge of root.querySelectorAll("." + UNCERTAIN_BADGE_CLASS)) {
        badge.remove();
    }
}

/**
 * Draw the `?` that rides an unjudgeable card.
 *
 * CSS cannot generate an SVG element (ADR-0016), so the mark is injected the
 * same way the ghost badge is — `createElementNS`, sized off the card's own
 * rect, appended *inside* the card so it travels with it. It sits at the
 * card's leading edge, where the ghost's ↺ is not.
 */
function markUncertain(card: Element): void {
    const rect = card.querySelector("rect");
    if (!rect) {
        return;
    }
    const x = parseFloat(rect.getAttribute("x") ?? "");
    const y = parseFloat(rect.getAttribute("y") ?? "");
    if (!isFinite(x) || !isFinite(y)) {
        return;
    }
    const glyph = document.createElementNS(SVG_NS, "text");
    glyph.setAttribute("class", UNCERTAIN_BADGE_CLASS);
    glyph.setAttribute("x", String(x + 10));
    glyph.setAttribute("y", String(y + 20));
    glyph.setAttribute("text-anchor", "middle");
    glyph.textContent = "?";
    card.appendChild(glyph);
}

/** Strip the filter's paint and withdraw its dim. */
export function clearFilterPaint(root: ParentNode, dim: DimRegistry): void {
    strip(root);
    dim.set(FILTER_DIM_SOURCE, null);
    dim.apply(root);
}

/**
 * Repaint `root` for one evaluated filter. A `null` answer clears it.
 *
 * Stateless: prior paint is stripped first, so calling this after a render
 * re-applies the same answer onto the fresh SVG.
 *
 * Every card a person owns takes their state, canonical and ghost alike —
 * a result is about a person, not a card (ADR-0034) — and edges take nothing
 * at all, because an edge belongs to no single person and dimming the picture's
 * connective tissue would recede the thing the answer is drawn on.
 */
export function paintFilterResults(
    root: ParentNode,
    answer: FilterPartition | null,
    dim: DimRegistry,
): void {
    strip(root);
    if (!answer) {
        clearFilterPaint(root, dim);
        return;
    }
    for (const personId of answer.matched) {
        for (const node of selectBoundNodes(root, { kind: "person", personId })) {
            node.classList.add(FILTER_MATCH_CLASS);
        }
    }
    for (const personId of answer.cantSay) {
        for (const node of selectBoundNodes(root, { kind: "person", personId })) {
            node.classList.add(FILTER_UNCERTAIN_CLASS);
            markUncertain(node);
        }
    }
    dim.set(FILTER_DIM_SOURCE, answer.dimmed);
    dim.apply(root);
}

/**
 * The person whose unjudgeable card `target` is inside, or `null`.
 *
 * The hit-test behind the can't-say whisper: hovering an amber card is what
 * asks for its reason, and only an amber card has one.
 */
export function uncertainPersonAt(target: Element | null): {
    personId: string;
    card: Element;
} | null {
    if (!target || typeof target.closest !== "function") {
        return null;
    }
    const card = target.closest("." + FILTER_UNCERTAIN_CLASS + "[data-person-id]");
    const personId = card?.getAttribute("data-person-id");
    return card && personId ? { personId, card } : null;
}
