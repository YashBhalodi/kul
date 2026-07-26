// The filter, drawn on the tree — and, above everything else, the tree
// surviving it.
//
// Two seams are exercised here, because jsdom can see one of them and not the
// other. What it *can* see is structure: which classes land on which nodes, and
// that the picture keeps every node and every edge it had. What it cannot see
// is a box or a computed custom property, so the three paint states being
// visually distinct is asserted at the **stylesheet text**, the same shape
// `tokens.test.ts` and `crates/kul-svg/tests/visual.rs` already use.

import { readFileSync } from "node:fs";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";
import { describe, expect, it } from "vitest";

import { DIM_CLASS, createDimRegistry } from "../src/dim.js";
import {
    FILTER_DIM_SOURCE,
    FILTER_MATCH_CLASS,
    FILTER_UNCERTAIN_CLASS,
    UNCERTAIN_BADGE_CLASS,
    clearFilterPaint,
    paintFilterResults,
    uncertainPersonAt,
} from "../src/filter-paint.js";

const SRC = join(dirname(fileURLToPath(import.meta.url)), "..", "src");
const appSheet = readFileSync(join(SRC, "preview.css"), "utf8");
const themeSheet = readFileSync(join(SRC, "preview-themes.css"), "utf8");

// Two persons with a ghost each, plus three edges, so "every card a person
// owns" and "no edge moves" are both observable.
const SVG = `<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 400 200">
    <g class="kul-card" data-person-id="aldo" data-kind="canonical" data-gender="male">
        <rect x="10" y="10" width="60" height="30"/>
    </g>
    <g class="kul-card" data-person-id="aldo" data-kind="ghost" data-ghost-reason="pastMarriage">
        <rect x="10" y="90" width="60" height="30"/>
    </g>
    <g class="kul-card" data-person-id="sofia" data-kind="canonical" data-gender="female">
        <rect x="110" y="10" width="60" height="30"/>
    </g>
    <g class="kul-card" data-person-id="nina" data-kind="canonical" data-gender="female">
        <rect x="210" y="10" width="60" height="30"/>
    </g>
    <path class="kul-edge" data-link-kind="marriage" data-marriage-id="m1" d="M 0 0 L 1 1"/>
    <path class="kul-edge" data-link-kind="birth" data-marriage-id="m1" data-child-id="nina" d="M 0 0 L 1 1"/>
    <path class="kul-edge" data-link-kind="adoption" data-marriage-id="m1" data-child-id="sofia" d="M 0 0 L 1 1"/>
</svg>`;

function tree(): HTMLElement {
    const root = document.createElement("div");
    root.innerHTML = SVG;
    return root;
}

function answer(matched: string[], cantSay: string[], dimmed: string[]) {
    return {
        matched: new Set(matched),
        cantSay: new Set(cantSay),
        dimmed: new Set(dimmed),
    };
}

function census(root: ParentNode): { cards: number; edges: number } {
    return {
        cards: root.querySelectorAll(".kul-card").length,
        edges: root.querySelectorAll(".kul-edge").length,
    };
}

/** Every filter state the paint can be in, including the way back out. */
const STATES: Array<[string, ReturnType<typeof answer> | null]> = [
    ["nothing filtered", null],
    ["a plain answer", answer(["aldo"], [], ["sofia", "nina"])],
    ["everything unjudgeable", answer([], ["aldo", "sofia", "nina"], [])],
    ["nothing matched", answer([], [], ["aldo", "sofia", "nina"])],
    ["everything matched", answer(["aldo", "sofia", "nina"], [], [])],
    ["all three states at once", answer(["aldo"], ["sofia"], ["nina"])],
];

describe("every filter state leaves the tree intact", () => {
    it("keeps the node and edge count unchanged, in every state", () => {
        const root = tree();
        const dim = createDimRegistry();
        const before = census(root);
        expect(before).toEqual({ cards: 4, edges: 3 });
        for (const [, state] of STATES) {
            paintFilterResults(root, state, dim);
            expect(census(root)).toEqual(before);
        }
        // …and back out again, from the busiest state.
        clearFilterPaint(root, dim);
        expect(census(root)).toEqual(before);
    });

    it("never hides a node or an edge, in every state", () => {
        const root = tree();
        const dim = createDimRegistry();
        for (const [name, state] of STATES) {
            paintFilterResults(root, state, dim);
            for (const node of root.querySelectorAll(".kul-card, .kul-edge")) {
                const el = node as HTMLElement;
                expect(
                    [
                        el.hasAttribute("hidden"),
                        el.style.display === "none",
                        el.style.visibility === "hidden",
                    ],
                    name,
                ).toEqual([false, false, false]);
            }
        }
    });

    it("leaves every edge unpainted — the answer is about persons", () => {
        const root = tree();
        paintFilterResults(root, answer(["aldo"], ["sofia"], ["nina"]), createDimRegistry());
        for (const edge of root.querySelectorAll(".kul-edge")) {
            expect(edge.getAttribute("class")).toBe("kul-edge");
        }
    });
});

describe("the three states land on the right cards", () => {
    it("paints every card a matching person owns, ghosts included", () => {
        const root = tree();
        paintFilterResults(root, answer(["aldo"], [], ["sofia", "nina"]), createDimRegistry());
        const lit = Array.from(root.querySelectorAll("." + FILTER_MATCH_CLASS));
        expect(lit).toHaveLength(2);
        expect(lit.map((n) => n.getAttribute("data-kind")).sort()).toEqual([
            "canonical",
            "ghost",
        ]);
    });

    it("marks an unjudgeable person amber and rides a ? on each of their cards", () => {
        const root = tree();
        paintFilterResults(root, answer([], ["aldo"], ["sofia", "nina"]), createDimRegistry());
        expect(root.querySelectorAll("." + FILTER_UNCERTAIN_CLASS)).toHaveLength(2);
        const badges = Array.from(root.querySelectorAll("." + UNCERTAIN_BADGE_CLASS));
        expect(badges).toHaveLength(2);
        expect(badges[0].textContent).toBe("?");
    });

    it("does not dim an unjudgeable person — that is the silent drop", () => {
        const root = tree();
        paintFilterResults(root, answer(["nina"], ["aldo"], ["sofia"]), createDimRegistry());
        const aldo = root.querySelector(
            '[data-person-id="aldo"][data-kind="canonical"]',
        ) as Element;
        expect(aldo.classList.contains(DIM_CLASS)).toBe(false);
        expect(aldo.classList.contains(FILTER_UNCERTAIN_CLASS)).toBe(true);
        const sofia = root.querySelector('[data-person-id="sofia"]') as Element;
        expect(sofia.classList.contains(DIM_CLASS)).toBe(true);
    });

    it("publishes its dim through the shared registry, under its own source", () => {
        const root = tree();
        const dim = createDimRegistry();
        paintFilterResults(root, answer(["aldo"], [], ["sofia", "nina"]), dim);
        expect([...dim.dimmedPersonIds()].sort()).toEqual(["nina", "sofia"]);

        // A second source dims more, never less: the union is what the class
        // means, and withdrawing one source cannot un-dim the other's.
        dim.set("kin", ["aldo"]);
        dim.apply(root);
        expect([...dim.dimmedPersonIds()].sort()).toEqual(["aldo", "nina", "sofia"]);
        clearFilterPaint(root, dim);
        expect([...dim.dimmedPersonIds()]).toEqual(["aldo"]);
    });

    it("repaints statelessly onto a replaced picture", () => {
        const root = tree();
        const dim = createDimRegistry();
        const state = answer(["aldo"], ["sofia"], ["nina"]);
        paintFilterResults(root, state, dim);
        root.innerHTML = SVG;
        paintFilterResults(root, state, dim);
        expect(root.querySelectorAll("." + FILTER_MATCH_CLASS)).toHaveLength(2);
        expect(root.querySelectorAll("." + UNCERTAIN_BADGE_CLASS)).toHaveLength(1);
        expect(root.querySelectorAll("." + DIM_CLASS)).toHaveLength(1);
    });

    it("adds no second ? when the same answer is painted twice", () => {
        const root = tree();
        const dim = createDimRegistry();
        const state = answer([], ["sofia"], ["aldo", "nina"]);
        paintFilterResults(root, state, dim);
        paintFilterResults(root, state, dim);
        expect(root.querySelectorAll("." + UNCERTAIN_BADGE_CLASS)).toHaveLength(1);
    });

    it("names the unjudgeable person under the pointer, and nobody else", () => {
        const root = tree();
        paintFilterResults(root, answer(["nina"], ["sofia"], ["aldo"]), createDimRegistry());
        const sofiaRect = root.querySelector('[data-person-id="sofia"] rect');
        expect(uncertainPersonAt(sofiaRect)?.personId).toBe("sofia");
        expect(
            uncertainPersonAt(root.querySelector('[data-person-id="nina"] rect')),
        ).toBeNull();
        expect(uncertainPersonAt(null)).toBeNull();
    });
});

describe("the dim source is the shared one, not a class of this module's own", () => {
    it("names a source rather than owning the class", () => {
        expect(FILTER_DIM_SOURCE).toBe("filter");
        expect(FILTER_MATCH_CLASS).not.toBe(DIM_CLASS);
        expect(FILTER_UNCERTAIN_CLASS).not.toBe(DIM_CLASS);
    });
});

// ── The stylesheet seam ──────────────────────────────────────────────────
//
// jsdom resolves neither custom-property substitution nor layout, so a green
// DOM test proves nothing about how any of this renders. These read the sheets
// as text.

function ruleFor(selector: string): string {
    const at = appSheet.indexOf(selector + " {");
    expect(at, `no rule for ${selector}`).toBeGreaterThan(-1);
    return appSheet.slice(at, appSheet.indexOf("}", at));
}

describe("the three paint states are three distinct paints", () => {
    it("sources each state from a different tier-1 reserved value", () => {
        const match = ruleFor(".kul-card.kul-filter-match > rect");
        const uncertain = ruleFor(".kul-card.kul-filter-uncertain > rect");
        const dimmed = ruleFor(".kul-card.kul-query-dim");
        expect(match).toContain("var(--kul-filter-match-outline-color)");
        expect(uncertain).toContain("var(--kul-filter-uncertain-outline-color)");
        expect(dimmed).toContain("var(--kul-query-dim-opacity)");

        // Each alias resolves to a *different* tier-1 name, so no two states
        // can collapse onto one paint by a theme override.
        const alias = (name: string) =>
            themeSheet.match(new RegExp(`${name}:\\s*var\\((--kul-[a-z0-9-]+)\\)`))?.[1];
        const sources = [
            alias("--kul-filter-match-outline-color"),
            alias("--kul-filter-uncertain-outline-color"),
            alias("--kul-query-dim-opacity"),
        ];
        expect(sources).toEqual([
            "--kul-hue-query-result",
            "--kul-hue-query-uncertain",
            "--kul-dim-alpha",
        ]);
        expect(new Set(sources).size).toBe(3);
    });

    it("distinguishes can't-say by line style and a glyph, not by hue alone", () => {
        // A hue difference alone is one dimension, and high-contrast themes are
        // exactly where one dimension is not enough.
        expect(ruleFor(".kul-card.kul-filter-match > rect")).toContain("solid");
        expect(ruleFor(".kul-card.kul-filter-uncertain > rect")).toContain("dashed");
        expect(ruleFor(".kul-filter-uncertain-badge")).toContain(
            "var(--kul-filter-uncertain-badge-fill)",
        );
    });

    it("lets can't-say win over match, whatever order they are applied in", () => {
        expect(appSheet.indexOf(".kul-card.kul-filter-match > rect")).toBeLessThan(
            appSheet.indexOf(".kul-card.kul-filter-uncertain > rect"),
        );
    });

    it("never hides anything, at the stylesheet level either", () => {
        for (const selector of [
            ".kul-card.kul-filter-match > rect",
            ".kul-card.kul-filter-uncertain > rect",
            ".kul-card.kul-query-dim",
            ".kul-filter-uncertain-badge",
        ]) {
            const rule = ruleFor(selector);
            expect(rule).not.toMatch(/display\s*:\s*none/);
            expect(rule).not.toMatch(/visibility\s*:\s*hidden/);
            expect(rule).not.toMatch(/content-visibility/);
        }
    });
});
