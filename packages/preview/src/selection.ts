// The query selection — what the reader has clicked, and the vocabulary every
// other piece of query chrome reads it through.
//
// ADR-0035 widened #276's "single selection, always" from persons to entities:
// person cards, marriage edges and adoption edges are all selectable, and
// clicking another entity *moves* the one selection rather than opening a
// second. Esc or a canvas click clears it.
//
// A selection is addressed exactly as a detail target is (ADR-0037) — a person
// id, a marriage id, or an adoption's `(childId, marriageId)` pair — so what
// the reader clicked reaches the engine without translation and the chrome
// carries one address vocabulary rather than two. That is also why an adoption
// is a pair here: the rendered adoption edge already carries both halves
// (ADR-0021), and `RevealTarget` cannot address an adoption at all.
//
// Reading `data-*` off the SVG in this module is *hit-testing* — deciding what
// was clicked and which nodes paint. Panel content never comes from here; it
// comes from the batched lookup alone (ADR-0035), which is why the two live in
// different modules.

import type { DetailTarget } from "./engine-wire.js";
import { selectBoundNodes } from "./result-binding.js";

/**
 * The one selected entity. Structurally a {@link DetailTarget}: the address of
 * a selection and the address of a detail lookup are the same value.
 */
export type EntitySelection = DetailTarget;

/** Class the selected entity's nodes wear. Painted with the reserved query violet. */
export const SELECTION_CLASS = "kul-query-selected";

/**
 * The person a selection is anchored on, or `null` for an edge selection.
 *
 * The hover lens needs a person ego and a kin set needs a person anchor
 * (ADR-0035), so both ask this question rather than re-deriving it. An edge
 * answers `null` — it is a waypoint, and the way onward is the panel's spouse
 * or child row, which selects a person.
 */
export function selectionAnchorPerson(
    selection: EntitySelection | null,
): string | null {
    return selection && selection.kind === "person" ? selection.id : null;
}

/** True when both values address the same entity (or both are absent). */
export function isSameSelection(
    a: EntitySelection | null,
    b: EntitySelection | null,
): boolean {
    if (a === null || b === null) {
        return a === b;
    }
    if (a.kind !== b.kind) {
        return false;
    }
    if (a.kind === "adoption" && b.kind === "adoption") {
        return a.childId === b.childId && a.marriageId === b.marriageId;
    }
    return (
        (a as { id: string }).id === (b as { id: string }).id
    );
}

/**
 * What a click on `target` selects, or `null` when it landed on the canvas
 * rather than on an entity. A ghost card selects its person like the canonical
 * card does — a selection is about a person, not about a card (ADR-0034).
 *
 * Birth edges are deliberately inert: they carry `data-marriage-id` too, so
 * the edge branch keys on `data-link-kind` rather than on the bare attribute.
 */
export function selectionAtElement(
    target: Element | null,
): EntitySelection | null {
    if (!target || typeof target.closest !== "function") {
        return null;
    }
    const card = target.closest("[data-person-id]");
    if (card) {
        const id = card.getAttribute("data-person-id") ?? "";
        return id ? { kind: "person", id } : null;
    }
    const edge = target.closest(
        '[data-link-kind="marriage"], [data-link-kind="adoption"]',
    );
    if (!edge) {
        return null;
    }
    const marriageId = edge.getAttribute("data-marriage-id") ?? "";
    if (edge.getAttribute("data-link-kind") === "marriage") {
        return marriageId ? { kind: "marriage", id: marriageId } : null;
    }
    const childId = edge.getAttribute("data-child-id") ?? "";
    return marriageId && childId
        ? { kind: "adoption", childId, marriageId }
        : null;
}

/**
 * Every node in `root` the selection paints, in document order.
 *
 * Persons and marriages reuse the result binding (ADR-0034) — a person lights
 * their canonical card *and* every ghost. An adoption is narrower than the
 * binding's `parenthood` variant on purpose: that variant matches a child's
 * birth and adoption edges alike, and a selected adoption is one specific link.
 * Ids are author-written and compared as values, never spliced into a selector.
 */
export function selectionNodes(
    root: ParentNode,
    selection: EntitySelection,
): Element[] {
    switch (selection.kind) {
        case "person":
            return selectBoundNodes(root, {
                kind: "person",
                personId: selection.id,
            });
        case "marriage":
            return selectBoundNodes(root, {
                kind: "marriage",
                marriageId: selection.id,
            });
        case "adoption":
            return Array.from(
                root.querySelectorAll(
                    '[data-link-kind="adoption"][data-child-id][data-marriage-id]',
                ),
            ).filter(
                (node) =>
                    node.getAttribute("data-child-id") === selection.childId &&
                    node.getAttribute("data-marriage-id") === selection.marriageId,
            );
    }
}

/** Strip the selection paint. Stateless, like the sync highlight it must never co-paint with. */
export function clearSelectionPaint(root: ParentNode): void {
    root.querySelectorAll("." + SELECTION_CLASS).forEach((node) => {
        node.classList.remove(SELECTION_CLASS);
    });
}

/**
 * Repaint `root` for `selection`. Stateless: prior paint is stripped first, so
 * calling this after a render re-applies the selection onto the fresh SVG.
 */
export function paintSelection(
    root: ParentNode,
    selection: EntitySelection | null,
): void {
    clearSelectionPaint(root);
    if (!selection) {
        return;
    }
    for (const node of selectionNodes(root, selection)) {
        node.classList.add(SELECTION_CLASS);
    }
}

/** Notified on every selection change, with the new selection (`null` on clear). */
export type SelectionListener = (selection: EntitySelection | null) => void;

/**
 * The selection seam. One value, three entity kinds, and a subscription every
 * query surface hangs off — the detail panel today, the kin list, the lens and
 * the filter bar as they land.
 *
 * Both mutators are idempotent: re-selecting the entity that is already
 * selected, or clearing when nothing is selected, notifies nobody. That is what
 * lets a listener treat every notification as a real transition and refetch
 * unconditionally.
 */
export interface SelectionStore {
    /** The selected entity, or `null`. */
    readonly current: EntitySelection | null;
    /** Move the selection onto `next`. */
    select(next: EntitySelection): void;
    /** Drop the selection. */
    clear(): void;
    /** Subscribe to changes; the returned function unsubscribes. */
    subscribe(listener: SelectionListener): () => void;
}

export function createSelectionStore(): SelectionStore {
    let current: EntitySelection | null = null;
    const listeners = new Set<SelectionListener>();

    function emit(): void {
        // Snapshot: a listener may unsubscribe (or subscribe) while notifying.
        for (const listener of [...listeners]) {
            listener(current);
        }
    }

    return {
        get current() {
            return current;
        },
        select(next) {
            if (isSameSelection(current, next)) {
                return;
            }
            current = next;
            emit();
        },
        clear() {
            if (current === null) {
                return;
            }
            current = null;
            emit();
        },
        subscribe(listener) {
            listeners.add(listener);
            return () => {
                listeners.delete(listener);
            };
        },
    };
}
