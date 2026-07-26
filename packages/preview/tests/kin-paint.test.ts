// Result ↔ card binding for an answered kin set (ADR-0034, ADR-0043), and the
// dim that recedes everything else.
//
// The rule most easily got wrong is the binding: a result is about a *person*,
// so **every card that person owns lights** — the canonical card and every ghost
// ADR-0019 emitted for a past intimacy. A result that vanished in the past
// family whose edges its ghost anchors would be a result the reader cannot see.
//
// The dim is a shared registry rather than this module's own class, because
// #303's filter is a second source and two `opacity` values on the same card
// would multiply. Union semantics are asserted here while there is one real
// consumer, which is the point of building it now.

import { describe, expect, it } from "vitest";

import { DIM_CLASS, createDimRegistry } from "../src/dim.js";
import type { DimRegistry } from "../src/dim.js";
import {
    KIN_DIM_SOURCE,
    RESULT_CLASS,
    clearKinPaint,
    paintKinResults,
} from "../src/kin-paint.js";

/**
 * Elena owns three cards — a canonical one and two past-intimacy ghosts — and
 * is the multi-card case #301 names. Giuseppe is the anchor; Marco is nobody's
 * answer and is the card that must dim.
 */
const SVG = `<svg xmlns="http://www.w3.org/2000/svg">
  <g class="kul-card" data-person-id="giuseppe" data-kind="canonical" data-gender="male"><rect/></g>
  <g class="kul-card" data-person-id="elena" data-kind="canonical" data-gender="female"><rect/></g>
  <g class="kul-card" data-person-id="elena" data-kind="ghost" data-ghost-reason="past-birth" data-gender="female"><rect/></g>
  <g class="kul-card" data-person-id="elena" data-kind="ghost" data-ghost-reason="past-marriage" data-gender="female"><rect/></g>
  <g class="kul-card" data-person-id="marco" data-kind="canonical" data-gender="male"><rect/></g>
  <path class="kul-edge" data-link-kind="marriage" data-marriage-id="m1"/>
</svg>`;

function tree(): { root: HTMLElement; dim: DimRegistry } {
    const root = document.createElement("div");
    root.innerHTML = SVG;
    return { root, dim: createDimRegistry() };
}

function idsWith(root: ParentNode, cls: string): string[] {
    return Array.from(root.querySelectorAll("." + cls)).map(
        (node) => node.getAttribute("data-person-id") ?? "",
    );
}

const litIds = (root: ParentNode) => idsWith(root, RESULT_CLASS);
const dimmedIds = (root: ParentNode) => idsWith(root, DIM_CLASS);

describe("an answered kin set paints every card its members own", () => {
    it("lights three cards from one result when a person has two ghosts", () => {
        const { root, dim } = tree();
        paintKinResults(root, { anchorId: "giuseppe", personIds: ["elena"] }, dim);
        expect(litIds(root)).toEqual(["elena", "elena", "elena"]);
    });

    it("dims every card outside the answer, and never the anchor", () => {
        const { root, dim } = tree();
        paintKinResults(root, { anchorId: "giuseppe", personIds: ["elena"] }, dim);
        expect(dimmedIds(root)).toEqual(["marco"]);
    });

    it("never lights the anchor, even if an answer names it", () => {
        // The engine excludes an anchor from its own answers; this module says
        // so itself rather than trusting that (ADR-0043).
        const { root, dim } = tree();
        paintKinResults(
            root,
            { anchorId: "giuseppe", personIds: ["giuseppe", "elena"] },
            dim,
        );
        expect(litIds(root)).toEqual(["elena", "elena", "elena"]);
        expect(dimmedIds(root)).toEqual(["marco"]);
    });

    it("dims edges never — the answer is drawn on the picture, not over it", () => {
        const { root, dim } = tree();
        paintKinResults(root, { anchorId: "giuseppe", personIds: ["elena"] }, dim);
        const edge = root.querySelector(".kul-edge") as Element;
        expect(edge.classList.contains(DIM_CLASS)).toBe(false);
    });

    it("paints nothing for an empty answer, and still recedes the rest", () => {
        const { root, dim } = tree();
        paintKinResults(root, { anchorId: "giuseppe", personIds: [] }, dim);
        expect(litIds(root)).toEqual([]);
        expect(dimmedIds(root).sort()).toEqual(["elena", "elena", "elena", "marco"]);
    });

    it("is stateless, so a repaint after a render re-applies the same answer", () => {
        const { root, dim } = tree();
        paintKinResults(root, { anchorId: "giuseppe", personIds: ["marco"] }, dim);
        paintKinResults(root, { anchorId: "giuseppe", personIds: ["elena"] }, dim);
        expect(litIds(root)).toEqual(["elena", "elena", "elena"]);
        expect(dimmedIds(root)).toEqual(["marco"]);
    });

    it("clears both halves together", () => {
        const { root, dim } = tree();
        paintKinResults(root, { anchorId: "giuseppe", personIds: ["elena"] }, dim);
        clearKinPaint(root, dim);
        expect(litIds(root)).toEqual([]);
        expect(dimmedIds(root)).toEqual([]);
    });

    it("treats a null answer as no question asked", () => {
        const { root, dim } = tree();
        paintKinResults(root, { anchorId: "giuseppe", personIds: ["elena"] }, dim);
        paintKinResults(root, null, dim);
        expect(litIds(root)).toEqual([]);
        expect(dimmedIds(root)).toEqual([]);
    });

    it("ignores a member the picture holds no card for", () => {
        const { root, dim } = tree();
        paintKinResults(root, { anchorId: "giuseppe", personIds: ["nobody"] }, dim);
        expect(litIds(root)).toEqual([]);
    });
});

describe("the dim has one owner and unions its sources", () => {
    it("keeps a second source's dim when the kin paint is cleared", () => {
        // The shape #303 walks into: a render calls the kin repaint, which
        // clears the kin paint. If that stripped the class outright, the filter
        // would silently un-dim on every render.
        const { root, dim } = tree();
        dim.set("filter", ["marco"]);
        paintKinResults(root, { anchorId: "giuseppe", personIds: ["elena"] }, dim);
        clearKinPaint(root, dim);
        expect(dimmedIds(root)).toEqual(["marco"]);
    });

    it("dims a person if any source dims them, and applies the class once", () => {
        const { root, dim } = tree();
        dim.set("filter", ["elena"]);
        paintKinResults(root, { anchorId: "giuseppe", personIds: ["marco"] }, dim);
        // Marco is the answer, so kin does not dim him; Elena is dimmed by the
        // filter and by kin at once, and wears the class exactly once.
        expect(dimmedIds(root).sort()).toEqual(["elena", "elena", "elena"]);
        for (const node of root.querySelectorAll("." + DIM_CLASS)) {
            expect(node.getAttribute("class")).toBe("kul-card " + DIM_CLASS);
        }
    });

    it("lifts the dim on the persons a live read is tracing", () => {
        // The lens wins over ambient context: a sky trace through a dimmed
        // person must not render at the dim's alpha (ADR-0043).
        const { root, dim } = tree();
        paintKinResults(root, { anchorId: "giuseppe", personIds: ["elena"] }, dim);
        expect(dimmedIds(root)).toEqual(["marco"]);
        dim.exempt("lens-trace", ["marco"]);
        dim.apply(root);
        expect(dimmedIds(root)).toEqual([]);
        dim.exempt("lens-trace", null);
        dim.apply(root);
        expect(dimmedIds(root)).toEqual(["marco"]);
    });

    it("keeps the exemption across a repaint, so a trace survives a render", () => {
        const { root, dim } = tree();
        dim.exempt("lens-trace", ["marco"]);
        paintKinResults(root, { anchorId: "giuseppe", personIds: ["elena"] }, dim);
        expect(dimmedIds(root)).toEqual([]);
    });

    it("withdraws a source with null and reports the union it is holding", () => {
        const { root, dim } = tree();
        dim.set(KIN_DIM_SOURCE, ["marco"]);
        dim.set("filter", ["elena"]);
        expect([...dim.dimmedPersonIds()].sort()).toEqual(["elena", "marco"]);
        dim.set("filter", null);
        dim.apply(root);
        expect(dimmedIds(root)).toEqual(["marco"]);
    });

    it("reset drops every source and strips the class", () => {
        const { root, dim } = tree();
        dim.set("filter", ["elena"]);
        dim.set(KIN_DIM_SOURCE, ["marco"]);
        dim.apply(root);
        dim.reset(root);
        expect(dimmedIds(root)).toEqual([]);
        expect([...dim.dimmedPersonIds()]).toEqual([]);
    });
});
