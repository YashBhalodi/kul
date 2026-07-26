// The docked-tag mechanism, exercised **without the lens** — which is the case
// that forced it out of `hover-lens.ts` in the first place. #303's can't-say
// reason whispers under a card with no selection anywhere and on a trigger of
// its own (PRD-0006, ADR-0044), so the placement primitive has to work with no
// query, no selection store and no phrasing in sight.

import { afterEach, beforeEach, describe, expect, it } from "vitest";

import { PREVIEW_BODY_HTML } from "../src/controls.js";
import { DOCKED_TAG_CLASS, openDockedTag } from "../src/docked-tag.js";

const SVG = `<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 400 200">
    <g class="kul-card" data-person-id="giulia" data-kind="canonical"><rect/></g>
    <g class="kul-card" data-person-id="marco" data-kind="canonical"><rect/></g>
</svg>`;

interface Stage {
    layer: HTMLElement;
    card(id: string): Element;
}

/**
 * Give a card a definite screen box; jsdom lays nothing out on its own.
 *
 * The *tag's* own box stays 0×0 for the same reason, which means `halfWidth` is
 * always 0 here and the horizontal viewport clamp is a no-op in every case
 * below. The clamp is therefore **not** covered by this suite and cannot be
 * from this stack — the same limit `tokens.test.ts` records for computed
 * styles. What is covered is that the tag docks under its anchor and follows it.
 */
function stubBox(
    card: Element,
    box: { left: number; bottom: number; width: number },
): void {
    card.getBoundingClientRect = () =>
        ({
            left: box.left,
            right: box.left + box.width,
            top: box.bottom,
            bottom: box.bottom,
            width: box.width,
            height: 0,
            x: box.left,
            y: box.bottom,
            toJSON: () => ({}),
        }) as DOMRect;
}

function stage(): Stage {
    const host = document.createElement("div");
    host.innerHTML = PREVIEW_BODY_HTML;
    document.body.appendChild(host);
    const root = host.querySelector("#root") as HTMLElement;
    root.innerHTML = SVG;
    return {
        layer: host.querySelector("#kul-region-float") as HTMLElement,
        card: (id) => root.querySelector(`[data-person-id="${id}"]`) as Element,
    };
}

function note(text: string): HTMLElement {
    const el = document.createElement("span");
    el.textContent = text;
    return el;
}

beforeEach(() => {
    document.body.innerHTML = "";
});

afterEach(() => {
    document.body.innerHTML = "";
});

describe("a tag docks under the element it is about", () => {
    it("opens on the float layer, carrying the grammar's class and its variant", () => {
        const s = stage();
        const tag = openDockedTag({
            layer: s.layer,
            anchor: s.card("marco"),
            content: [note("can't say — family not recorded")],
            variant: "kul-filter-reason",
        });
        // The layer itself, not its edge dock: entity-anchored chrome is the
        // second placement ADR-0042 carved, and the dock is where the details
        // panel lives.
        expect(tag.element.parentElement).toBe(s.layer);
        expect(s.layer.querySelector("#kul-region-float-dock")?.children).toHaveLength(
            0,
        );
        expect(tag.element.classList.contains(DOCKED_TAG_CLASS)).toBe(true);
        expect(tag.element.classList.contains("kul-filter-reason")).toBe(true);
        expect(tag.element.textContent).toBe("can't say — family not recorded");
    });

    it("needs no selection, no query and no phrasing", () => {
        // Stated as a test because it is the requirement that split the module.
        const s = stage();
        const tag = openDockedTag({
            layer: s.layer,
            anchor: s.card("giulia"),
            content: [note("anything at all")],
        });
        expect(tag.element.isConnected).toBe(true);
        expect(tag.element.className).toBe(DOCKED_TAG_CLASS);
    });

    it("positions under the anchor's measured box, centred on it", () => {
        const s = stage();
        const card = s.card("marco");
        stubBox(card, { left: 100, bottom: 60, width: 40 });
        const tag = openDockedTag({
            layer: s.layer,
            anchor: card,
            content: [note("x")],
        });
        expect(tag.element.style.top).toBe("60px");
        expect(tag.element.style.left).toBe("120px");
    });

    it("follows the anchor when it moves, on demand", () => {
        // What keeps a whisper docked while the reader drags the canvas under
        // it: the consumer re-anchors, the mechanism re-measures.
        const s = stage();
        const card = s.card("marco");
        stubBox(card, { left: 100, bottom: 60, width: 40 });
        const tag = openDockedTag({
            layer: s.layer,
            anchor: card,
            content: [note("x")],
        });
        stubBox(card, { left: 300, bottom: 180, width: 40 });
        tag.reanchor();
        expect(tag.element.style.top).toBe("180px");
        expect(tag.element.style.left).toBe("320px");
    });

    it("answers for its own content, so hovering it is not leaving", () => {
        const s = stage();
        const inner = note("hover me");
        const tag = openDockedTag({
            layer: s.layer,
            anchor: s.card("marco"),
            content: [inner],
        });
        expect(tag.contains(inner)).toBe(true);
        expect(tag.contains(tag.element)).toBe(true);
        expect(tag.contains(s.card("marco"))).toBe(false);
        expect(tag.contains(null)).toBe(false);
    });

    it("closes idempotently and stops following once closed", () => {
        const s = stage();
        const card = s.card("marco");
        stubBox(card, { left: 100, bottom: 60, width: 40 });
        const tag = openDockedTag({
            layer: s.layer,
            anchor: card,
            content: [note("x")],
        });
        tag.close();
        tag.close();
        expect(tag.element.isConnected).toBe(false);
        expect(s.layer.querySelector("." + DOCKED_TAG_CLASS)).toBeNull();

        stubBox(card, { left: 999, bottom: 999, width: 40 });
        tag.reanchor();
        expect(tag.element.style.top).toBe("60px");
    });

    it("survives an anchor with no box rather than throwing", () => {
        // A detached or non-graphical anchor is a plain no-op: the tag opens
        // where CSS puts it instead of nowhere.
        const s = stage();
        const bare = document.createElement("div");
        (bare as { getBoundingClientRect?: unknown }).getBoundingClientRect =
            undefined;
        const tag = openDockedTag({
            layer: s.layer,
            anchor: bare,
            content: [note("x")],
        });
        expect(tag.element.style.top).toBe("");
        expect(() => tag.reanchor()).not.toThrow();
    });
});
