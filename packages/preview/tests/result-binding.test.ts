// Result ↔ node binding (ADR-0034). The fixture mirrors what `kul-svg` emits:
// one person owning a canonical card plus two ghosts, a marriage edge whose
// birth edges carry the same `data-marriage-id`, and a child reached by both a
// birth and an adoption link.

import { beforeEach, describe, expect, it } from "vitest";

import { selectBoundNodes } from "../src/result-binding.js";

const SVG = `<svg xmlns="http://www.w3.org/2000/svg">
  <g class="kul-card" data-person-id="giuseppe" data-kind="canonical" data-gender="male"></g>
  <g class="kul-card" data-person-id="giuseppe" data-kind="ghost" data-ghost-reason="pastMarriage"></g>
  <g class="kul-card" data-person-id="giuseppe" data-kind="ghost" data-ghost-reason="pastAdoption"></g>
  <g class="kul-card" data-person-id="maria" data-kind="canonical" data-gender="female"></g>
  <path class="kul-edge" data-link-kind="marriage" data-marriage-id="m1" data-host-id="giuseppe" data-joining-id="maria"/>
  <path class="kul-edge" data-link-kind="marriage" data-marriage-id="m2" data-host-id="giuseppe" data-joining-id="rosa"/>
  <path class="kul-edge" data-link-kind="birth" data-marriage-id="m1" data-child-id="dalisay" data-is-past="false"/>
  <path class="kul-edge" data-link-kind="adoption" data-marriage-id="m2" data-child-id="dalisay" data-is-past="true"/>
  <path class="kul-edge" data-link-kind="birth" data-marriage-id="m1" data-child-id="lucia" data-is-past="false"/>
  <g class="kul-legend">
    <path class="kul-edge" data-link-kind="birth"/>
    <path class="kul-edge" data-link-kind="marriage"/>
  </g>
</svg>`;

let root: HTMLElement;

beforeEach(() => {
    document.body.innerHTML = `<div id="root">${SVG}</div>`;
    root = document.getElementById("root") as HTMLElement;
});

describe("selectBoundNodes — person", () => {
    it("selects the canonical card and every ghost for one person", () => {
        const nodes = selectBoundNodes(root, {
            kind: "person",
            personId: "giuseppe",
        });
        expect(nodes).toHaveLength(3);
        expect(nodes.map((n) => n.getAttribute("data-kind"))).toEqual([
            "canonical",
            "ghost",
            "ghost",
        ]);
    });

    it("selects only the one card of a person who owns no ghosts", () => {
        const nodes = selectBoundNodes(root, { kind: "person", personId: "maria" });
        expect(nodes).toHaveLength(1);
    });

    it("answers an id the picture does not draw with an empty list", () => {
        expect(
            selectBoundNodes(root, { kind: "person", personId: "nobody" }),
        ).toEqual([]);
    });

    it("compares ids as values, so a quoted id cannot widen the match", () => {
        expect(
            selectBoundNodes(root, { kind: "person", personId: 'x"] , [data-kind' }),
        ).toEqual([]);
    });
});

describe("selectBoundNodes — marriage (an `across` hop)", () => {
    it("selects the marriage edge", () => {
        const nodes = selectBoundNodes(root, { kind: "marriage", marriageId: "m1" });
        expect(nodes).toHaveLength(1);
        expect(nodes[0].getAttribute("data-link-kind")).toBe("marriage");
    });

    it("excludes the birth/adoption edges that carry the same marriage id", () => {
        const nodes = selectBoundNodes(root, { kind: "marriage", marriageId: "m1" });
        expect(nodes.every((n) => !n.hasAttribute("data-child-id"))).toBe(true);
    });
});

describe("selectBoundNodes — parenthood (a vertical hop)", () => {
    it("selects both the birth and the adoption edge of a doubly-linked child", () => {
        const nodes = selectBoundNodes(root, {
            kind: "parenthood",
            childId: "dalisay",
        });
        expect(nodes.map((n) => n.getAttribute("data-link-kind")).sort()).toEqual([
            "adoption",
            "birth",
        ]);
    });

    it("selects the single birth edge of a child with one link", () => {
        const nodes = selectBoundNodes(root, {
            kind: "parenthood",
            childId: "lucia",
        });
        expect(nodes).toHaveLength(1);
    });

    it("never selects the legend's child-less swatch edges", () => {
        const nodes = selectBoundNodes(root, {
            kind: "parenthood",
            childId: "dalisay",
        });
        expect(nodes.every((n) => !n.closest(".kul-legend"))).toBe(true);
    });
});
