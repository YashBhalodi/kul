// The selection seam (ADR-0035, ADR-0042), at the module boundary: the store's
// transitions, the person-anchor rule three later slices read, hit-testing over
// the rendered vocabulary, and the paint.

import { describe, expect, it, vi } from "vitest";

import {
    type EntitySelection,
    SELECTION_CLASS,
    createSelectionStore,
    isSameSelection,
    paintSelection,
    selectionAnchorPerson,
    selectionAtElement,
    selectionNodes,
} from "../src/selection.js";

const SVG = `<svg xmlns="http://www.w3.org/2000/svg">
    <g class="kul-card" data-person-id="giulia" data-kind="canonical" data-gender="female">
        <rect/><text class="kul-label-name">Giulia</text>
    </g>
    <g class="kul-card" data-person-id="giulia" data-kind="ghost" data-ghost-reason="pastMarriage"><rect/></g>
    <g class="kul-card" data-person-id="marco" data-kind="canonical" data-gender="male"><rect/></g>
    <path class="kul-edge" data-link-kind="marriage" data-marriage-id="m1" data-host-id="giulia" data-joining-id="marco" d="M 0 0 L 1 1"/>
    <path class="kul-edge" data-link-kind="birth" data-marriage-id="m1" data-child-id="dalisay" d="M 0 0 L 1 1"/>
    <path class="kul-edge" data-link-kind="adoption" data-marriage-id="m1" data-child-id="dalisay" d="M 0 0 L 1 1"/>
</svg>`;

function picture(): HTMLElement {
    const root = document.createElement("div");
    root.innerHTML = SVG;
    return root;
}

function node(root: HTMLElement, selector: string): Element {
    return root.querySelector(selector) as Element;
}

describe("the selection store", () => {
    it("starts with nothing selected", () => {
        expect(createSelectionStore().current).toBeNull();
    });

    it("moves the selection rather than adding a second one", () => {
        const store = createSelectionStore();
        const seen: Array<EntitySelection | null> = [];
        store.subscribe((selection) => seen.push(selection));
        store.select({ kind: "person", id: "giulia" });
        store.select({ kind: "marriage", id: "m1" });
        expect(store.current).toEqual({ kind: "marriage", id: "m1" });
        expect(seen).toEqual([
            { kind: "person", id: "giulia" },
            { kind: "marriage", id: "m1" },
        ]);
    });

    it("selects all three entity kinds, adoptions by their (child, marriage) pair", () => {
        const store = createSelectionStore();
        store.select({ kind: "adoption", childId: "dalisay", marriageId: "m1" });
        expect(store.current).toEqual({
            kind: "adoption",
            childId: "dalisay",
            marriageId: "m1",
        });
    });

    it("treats re-selecting the selected entity as a no-op", () => {
        const store = createSelectionStore();
        const listener = vi.fn();
        store.select({ kind: "person", id: "giulia" });
        store.subscribe(listener);
        store.select({ kind: "person", id: "giulia" });
        expect(listener).not.toHaveBeenCalled();
    });

    it("notifies with null on clear, and not again when already clear", () => {
        const store = createSelectionStore();
        const seen: Array<EntitySelection | null> = [];
        store.select({ kind: "person", id: "giulia" });
        store.subscribe((selection) => seen.push(selection));
        store.clear();
        store.clear();
        expect(seen).toEqual([null]);
        expect(store.current).toBeNull();
    });

    it("stops notifying an unsubscribed listener", () => {
        const store = createSelectionStore();
        const listener = vi.fn();
        store.subscribe(listener)();
        store.select({ kind: "person", id: "giulia" });
        expect(listener).not.toHaveBeenCalled();
    });
});

describe("the person anchor", () => {
    it("is the person for a person selection", () => {
        expect(selectionAnchorPerson({ kind: "person", id: "giulia" })).toBe(
            "giulia",
        );
    });

    it("is absent for an edge selection — the lens needs an ego and a kin set an anchor", () => {
        expect(selectionAnchorPerson({ kind: "marriage", id: "m1" })).toBeNull();
        expect(
            selectionAnchorPerson({
                kind: "adoption",
                childId: "dalisay",
                marriageId: "m1",
            }),
        ).toBeNull();
        expect(selectionAnchorPerson(null)).toBeNull();
    });
});

describe("selection identity", () => {
    it("distinguishes two adoptions of one child under different marriages", () => {
        expect(
            isSameSelection(
                { kind: "adoption", childId: "dalisay", marriageId: "m1" },
                { kind: "adoption", childId: "dalisay", marriageId: "m2" },
            ),
        ).toBe(false);
    });

    it("distinguishes a person from a marriage that share an id", () => {
        expect(
            isSameSelection({ kind: "person", id: "x" }, { kind: "marriage", id: "x" }),
        ).toBe(false);
    });
});

describe("what a click selects", () => {
    it("selects the person behind a canonical card", () => {
        const root = picture();
        expect(
            selectionAtElement(node(root, '[data-person-id="giulia"][data-kind="canonical"] rect')),
        ).toEqual({ kind: "person", id: "giulia" });
    });

    it("selects the person behind a ghost card — a selection is about a person, not a card", () => {
        const root = picture();
        expect(
            selectionAtElement(node(root, '[data-person-id="giulia"][data-kind="ghost"]')),
        ).toEqual({ kind: "person", id: "giulia" });
    });

    it("selects a marriage from its bar", () => {
        const root = picture();
        expect(selectionAtElement(node(root, '[data-link-kind="marriage"]'))).toEqual({
            kind: "marriage",
            id: "m1",
        });
    });

    it("selects an adoption as the (child, marriage) pair its edge already carries", () => {
        const root = picture();
        expect(selectionAtElement(node(root, '[data-link-kind="adoption"]'))).toEqual({
            kind: "adoption",
            childId: "dalisay",
            marriageId: "m1",
        });
    });

    it("leaves birth edges inert even though they carry a marriage id", () => {
        const root = picture();
        expect(selectionAtElement(node(root, '[data-link-kind="birth"]'))).toBeNull();
    });

    it("reads a click on the canvas as no selection", () => {
        const root = picture();
        expect(selectionAtElement(node(root, "svg"))).toBeNull();
        expect(selectionAtElement(null)).toBeNull();
    });
});

describe("selection paint", () => {
    it("lights a person's canonical card and every ghost", () => {
        const root = picture();
        paintSelection(root, { kind: "person", id: "giulia" });
        expect(root.querySelectorAll("." + SELECTION_CLASS)).toHaveLength(2);
    });

    it("lights the selected adoption edge only, never the child's birth edge", () => {
        const root = picture();
        const nodes = selectionNodes(root, {
            kind: "adoption",
            childId: "dalisay",
            marriageId: "m1",
        });
        expect(nodes).toHaveLength(1);
        expect(nodes[0].getAttribute("data-link-kind")).toBe("adoption");
    });

    it("is stateless — repainting strips the prior selection first", () => {
        const root = picture();
        paintSelection(root, { kind: "person", id: "giulia" });
        paintSelection(root, { kind: "marriage", id: "m1" });
        expect(root.querySelectorAll("." + SELECTION_CLASS)).toHaveLength(1);
        paintSelection(root, null);
        expect(root.querySelectorAll("." + SELECTION_CLASS)).toHaveLength(0);
    });
});
