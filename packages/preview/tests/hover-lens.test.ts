// The hover lens, driven the way a reader drives it: a selection, then a
// pointer moving over the picture, with the engine substituted at its one
// system boundary (`CODING_STANDARDS.md` — mock at system boundaries, never
// internal collaborators).
//
// The debounce is exercised through `handleCanvasHover` with Vitest's fake
// timers rather than by waiting on a wall clock: what #302 asks to be proved
// is that a sweep coalesces into one query per settle, and that is a statement
// about the module's seam, not about how fast a machine happens to be.

import { readFileSync } from "node:fs";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";

import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";

vi.mock("svg-pan-zoom", () => {
    const make = () => ({
        getPan: () => ({ x: 0, y: 0 }),
        getZoom: () => 1,
        getSizes: () => ({ width: 800, height: 600, realZoom: 1 }),
        pan: vi.fn(),
        panBy: vi.fn(),
        zoom: vi.fn(),
        zoomIn: vi.fn(),
        zoomOut: vi.fn(),
        reset: vi.fn(),
        destroy: vi.fn(),
    });
    return { default: vi.fn(() => make()) };
});

import { PREVIEW_BODY_HTML } from "../src/controls.js";
import type {
    DetailLookupResult,
    DetailTarget,
    EntityDetail,
    QueryEnvelope,
    ResolveResult,
} from "../src/engine-wire.js";
import { highlightEntity } from "../src/highlight.js";
import {
    LENS_DEBOUNCE_MS,
    LENS_PATH_CLASS,
    NOT_RELATED_DISCONNECTED,
    NOT_RELATED_WITHIN_BOUNDS,
    VIEWPOINT_TITLE,
    createHoverLens,
    emptinessWhisper,
    resolutionPathBindings,
    tracedPersonIds,
} from "../src/hover-lens.js";
import { DIM_CLASS } from "../src/dim.js";
import { KIN_SETS } from "../src/kin-sets.js";
import { createSelectionStore } from "../src/selection.js";
import type { ProjectSnapshot, QueryEngine } from "../src/engine.js";
import { createLocaleController } from "../src/locale.js";
import { createMemoryLocaleStore } from "../src/locale-store.js";
import { mountPreview } from "../src/mount.js";
import type { RelationshipDescriptor } from "../src/phrasing/descriptor.js";
import { PACKS } from "../src/phrasing/packs/index.js";
import { createQuerySurface } from "../src/query-surface.js";
import type { QuerySurface, QuerySurfaceOptions } from "../src/query-surface.js";
import type { LocaleController } from "../src/locale.js";
import type { PreviewHandle } from "../src/types.js";

// Three generations plus a spouse, with the edges the lens traces: one
// marriage, two birth edges and one adoption edge. `marco` and `giulia` each
// own a canonical card *and* a ghost, because a tie is about a person and every
// card of theirs binds — which the id gate and the re-anchor rule both turn on.
const SVG = `<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 600 400">
    <g class="kul-card" data-person-id="giulia" data-kind="canonical" data-gender="female"><rect/></g>
    <g class="kul-card" data-person-id="giulia" data-kind="ghost" data-gender="female"><rect/></g>
    <g class="kul-card" data-person-id="marco" data-kind="canonical" data-gender="male"><rect/></g>
    <g class="kul-card" data-person-id="marco" data-kind="ghost" data-gender="male"><rect/></g>
    <g class="kul-card" data-person-id="luca" data-kind="canonical" data-gender="male"><rect/></g>
    <g class="kul-card" data-person-id="dalisay" data-kind="canonical" data-gender="female"><rect/></g>
    <path class="kul-edge" data-link-kind="marriage" data-marriage-id="m1" d="M 0 0 L 1 1"/>
    <path class="kul-edge" data-link-kind="birth" data-marriage-id="m1" data-child-id="luca" d="M 0 0 L 1 1"/>
    <path class="kul-edge" data-link-kind="birth" data-marriage-id="m1" data-child-id="marco" d="M 0 0 L 1 1"/>
    <path class="kul-edge" data-link-kind="adoption" data-marriage-id="m1" data-child-id="dalisay" d="M 0 0 L 1 1"/>
</svg>`;

const PERSON_DETAIL: EntityDetail = {
    kind: "person",
    person: { id: "giulia", name: "Giulia", gender: "female" },
    parents: [],
    marriages: [],
    children: [],
};

/** ego → their father's brother: *kākā* in `gu`, *uncle* in `en`. */
function uncleOf(egoId: string, alterId: string): RelationshipDescriptor {
    return {
        egoId,
        alterId,
        egoGender: "female",
        alterGender: "male",
        classification: { kind: "collateral", up: 2, down: 1, cousinDegree: 0, removed: 1 },
        edgeNature: "blood",
        affinity: "blood",
        sharing: "full",
        side: "paternal",
        seniority: "unknown",
        apexSeniority: "unknown",
        path: [
            { step: "up", to: "father", gender: "male", edge: "bio" },
            { step: "up", to: "grandfather", gender: "male", edge: "bio" },
            { step: "down", to: alterId, gender: "male", edge: "bio" },
        ],
    };
}

/** ego → their spouse's father: *sasro* in `gu`. A one-`across` affinal tie. */
function fatherInLawOf(egoId: string, alterId: string): RelationshipDescriptor {
    return {
        egoId,
        alterId,
        egoGender: "female",
        alterGender: "male",
        classification: { kind: "lineal", role: "ancestor", generations: 1 },
        edgeNature: "blood",
        affinity: "inLaw",
        sharing: "notApplicable",
        side: "notApplicable",
        seniority: "unknown",
        apexSeniority: "notApplicable",
        path: [
            { step: "across", to: "marco", gender: "male", marriage: "m1", status: "ongoing" },
            { step: "up", to: alterId, gender: "male", edge: "bio" },
        ],
    };
}

/**
 * A five-hop tie Gujarati has no word for — a first cousin's son on the
 * mother's-brother line — so it composes into a genitive chain off *māmā*.
 */
function unlexicalized(egoId: string, alterId: string): RelationshipDescriptor {
    return {
        egoId,
        alterId,
        egoGender: "female",
        alterGender: "male",
        classification: { kind: "collateral", up: 2, down: 3, cousinDegree: 1, removed: 1 },
        edgeNature: "blood",
        affinity: "blood",
        sharing: "full",
        side: "maternal",
        seniority: "unknown",
        apexSeniority: "unknown",
        path: [
            { step: "up", to: "mother", gender: "female", edge: "bio" },
            { step: "up", to: "grandmother", gender: "female", edge: "bio" },
            { step: "down", to: "uncle", gender: "male", edge: "bio" },
            { step: "down", to: "cousin", gender: "male", edge: "bio" },
            { step: "down", to: alterId, gender: "male", edge: "bio" },
        ],
    };
}

interface Harness {
    stage: HTMLElement;
    root: HTMLElement;
    surface: QuerySurface;
    locale: LocaleController;
    /** One entry per `queryResolve` the surface actually made. */
    asked: Array<[string, string]>;
}

let answer: ResolveResult = { relationships: [] };

function harness(options: Partial<QuerySurfaceOptions> = {}): Harness {
    const stage = document.createElement("div");
    stage.innerHTML = PREVIEW_BODY_HTML;
    document.body.appendChild(stage);
    const root = stage.querySelector("#root") as HTMLElement;
    root.innerHTML = SVG;
    const asked: Array<[string, string]> = [];
    const locale = createLocaleController(null, createMemoryLocaleStore());
    const surface = createQuerySurface({
        root,
        floatDock: stage.querySelector("#kul-region-float-dock") as HTMLElement,
        floatLayer: stage.querySelector("#kul-region-float") as HTMLElement,
        locale,
        notifyRegion: stage.querySelector("#kul-region-notify") as HTMLElement,
        adapter: { onRevealRequest: () => {} },
        async lookup(targets: DetailTarget[]) {
            return {
                ok: true,
                result: targets.map(() => PERSON_DETAIL),
            } as QueryEnvelope<DetailLookupResult>;
        },
        // Explore kin is #301's surface, not this one's; an empty count keeps
        // the surface complete without painting anything. The one suite that
        // *does* need a kin paint — where the trace has to outrank its dim —
        // overrides it.
        async runKinQuery() {
            return { ok: true, result: { kind: "count" as const, count: 0 } };
        },
        async resolve(egoId, alterId) {
            asked.push([egoId, alterId]);
            return { ok: true, result: answer };
        },
        getPanZoom: () => null,
        applySyncHighlight: (ref) => {
            highlightEntity(root, null, ref);
        },
        ...options,
    });
    return { stage, root, surface, locale, asked };
}

function cardOf(root: HTMLElement, id: string, kind = "canonical"): Element {
    return root.querySelector(
        `[data-person-id="${id}"][data-kind="${kind}"]`,
    ) as Element;
}

/**
 * Move the pointer onto a card, the way the mount's `pointermove` handler
 * reports it: over the card's `rect`, not over its group.
 */
function hover(h: Harness, id: string, kind = "canonical"): void {
    h.surface.handleCanvasHover(cardOf(h.root, id, kind).querySelector("rect"));
}

/**
 * Give one card a definite screen box. jsdom lays nothing out, so the pill's
 * anchoring is only observable if the cards it anchors to have boxes that
 * differ — which is exactly what distinguishes two cards of one person.
 */
function stubBox(card: Element, box: { left: number; bottom: number }): void {
    card.getBoundingClientRect = () =>
        ({
            left: box.left,
            right: box.left,
            top: box.bottom,
            bottom: box.bottom,
            width: 0,
            height: 0,
            x: box.left,
            y: box.bottom,
            toJSON: () => ({}),
        }) as DOMRect;
}

/** Let the trailing edge fire and the resolution promise resolve. */
async function settle(): Promise<void> {
    await vi.advanceTimersByTimeAsync(LENS_DEBOUNCE_MS);
    await vi.advanceTimersByTimeAsync(0);
}

function pill(stage: HTMLElement): HTMLElement | null {
    return stage.querySelector(".kul-lens");
}

function terms(stage: HTMLElement): string[] {
    return Array.from(stage.querySelectorAll(".kul-lens-term")).map(
        (el) => el.textContent ?? "",
    );
}

function pathNodes(root: HTMLElement): string[] {
    return Array.from(root.querySelectorAll("." + LENS_PATH_CLASS)).map(
        (node) =>
            node.getAttribute("data-person-id") ??
            `${node.getAttribute("data-link-kind")}:${
                node.getAttribute("data-child-id") ??
                node.getAttribute("data-marriage-id")
            }`,
    );
}

beforeEach(() => {
    vi.useFakeTimers();
    answer = { relationships: [] };
    document.body.innerHTML = "";
});

afterEach(() => {
    vi.useRealTimers();
    document.body.innerHTML = "";
});

describe("the lens reads only where a question was asked", () => {
    it("stays silent while nothing is selected", async () => {
        const h = harness();
        const { stage, root, asked } = h;
        hover(h, "marco");
        await settle();
        expect(asked).toEqual([]);
        expect(pill(stage)).toBeNull();
    });

    it("resolves the hovered person against the selected one", async () => {
        const h = harness();
        const { root, surface, asked } = h;
        answer = { relationships: [uncleOf("giulia", "marco")] };
        surface.selection.select({ kind: "person", id: "giulia" });
        hover(h, "marco");
        await settle();
        expect(asked).toEqual([["giulia", "marco"]]);
    });

    it("never resolves the selected person against themselves", async () => {
        const h = harness();
        const { stage, root, surface, asked } = h;
        surface.selection.select({ kind: "person", id: "giulia" });
        hover(h, "giulia");
        await settle();
        expect(asked).toEqual([]);
        expect(pill(stage)).toBeNull();
    });

    it("never resolves a *ghost* of the selected person either", async () => {
        // The gate is on the person id, not on the card, so a selected
        // person's ghost is as much themselves as their canonical card is.
        const h = harness();
        const { stage, surface, asked } = h;
        surface.selection.select({ kind: "person", id: "giulia" });
        hover(h, "giulia", "ghost");
        await settle();
        expect(asked).toEqual([]);
        expect(pill(stage)).toBeNull();
    });

    it("stays silent for an edge selection, which has no person ego", async () => {
        const h = harness();
        const { root, surface, asked } = h;
        surface.selection.select({ kind: "marriage", id: "m1" });
        hover(h, "marco");
        await settle();
        expect(asked).toEqual([]);
    });

    it("reads from a ghost card too — a tie is about a person, not a card", async () => {
        const h = harness();
        const { root, surface, asked } = h;
        answer = { relationships: [uncleOf("giulia", "marco")] };
        surface.selection.select({ kind: "person", id: "giulia" });
        hover(h, "marco", "ghost");
        await settle();
        expect(asked).toEqual([["giulia", "marco"]]);
    });

    it("takes itself down when the selection moves", async () => {
        const h = harness();
        const { stage, root, surface } = h;
        answer = { relationships: [uncleOf("giulia", "marco")] };
        surface.selection.select({ kind: "person", id: "giulia" });
        hover(h, "marco");
        await settle();
        expect(pill(stage)).not.toBeNull();

        // The ego just changed, so the tie on screen is about the person who
        // *was* selected.
        surface.selection.select({ kind: "person", id: "luca" });
        expect(pill(stage)).toBeNull();
        expect(pathNodes(root)).toEqual([]);
    });
});

describe("a fast sweep costs one query, not one per pixel", () => {
    it("coalesces every card crossed into a single query at the settle", async () => {
        const h = harness();
        const { root, surface, asked } = h;
        answer = { relationships: [uncleOf("giulia", "marco")] };
        surface.selection.select({ kind: "person", id: "giulia" });

        // A sweep: five cards crossed, plus repeated moves inside the last one.
        for (const id of ["marco", "luca", "dalisay", "marco", "luca"]) {
            hover(h, id);
            await vi.advanceTimersByTimeAsync(LENS_DEBOUNCE_MS / 4);
        }
        hover(h, "luca");
        hover(h, "luca");
        expect(asked).toEqual([]);

        await settle();
        expect(asked).toEqual([["giulia", "luca"]]);
    });

    it("re-arms when the pointer crosses to another card of the same person", async () => {
        // The dedupe key is the **card**, not the person id. The pending
        // timeout captured the card it was armed for, so a person-keyed dedupe
        // would let the first timer survive the crossing and dock the pill
        // under the canonical card the pointer had already left. Only the
        // in-flight window is affected — which is precisely the window a
        // 120 ms debounce spends most of its life in.
        const h = harness();
        const { stage, surface } = h;
        answer = { relationships: [uncleOf("giulia", "marco")] };
        stubBox(cardOf(h.root, "marco", "canonical"), { left: 100, bottom: 60 });
        stubBox(cardOf(h.root, "marco", "ghost"), { left: 300, bottom: 200 });
        surface.selection.select({ kind: "person", id: "giulia" });

        hover(h, "marco", "canonical");
        await vi.advanceTimersByTimeAsync(LENS_DEBOUNCE_MS / 2);
        hover(h, "marco", "ghost");
        await settle();

        const el = pill(stage) as HTMLElement;
        expect(el.style.top).toBe("200px");
        expect(el.style.left).toBe("300px");
    });

    it("asks again only once the pointer settles somewhere new", async () => {
        const h = harness();
        const { root, surface, asked } = h;
        answer = { relationships: [uncleOf("giulia", "marco")] };
        surface.selection.select({ kind: "person", id: "giulia" });
        hover(h, "marco");
        await settle();
        hover(h, "luca");
        await settle();
        expect(asked).toEqual([
            ["giulia", "marco"],
            ["giulia", "luca"],
        ]);
    });
});

describe("the pill whispers every tie, in the reader's language", () => {
    it("renders one term per descriptor, stacked inline, nothing collapsed", async () => {
        const h = harness();
        const { stage, root, surface } = h;
        answer = {
            relationships: [
                uncleOf("giulia", "marco"),
                fatherInLawOf("giulia", "marco"),
            ],
        };
        surface.selection.select({ kind: "person", id: "giulia" });
        hover(h, "marco");
        await settle();

        expect(terms(stage)).toEqual(["uncle", "father-in-law"]);
        expect(stage.querySelectorAll(".kul-lens-sep")).toHaveLength(1);
    });

    it("carries no role, aria-* or tabindex — the stated non-goal, in the markup", async () => {
        const h = harness();
        const { stage, surface } = h;
        answer = {
            relationships: [
                uncleOf("giulia", "marco"),
                fatherInLawOf("giulia", "marco"),
            ],
        };
        surface.selection.select({ kind: "person", id: "giulia" });
        hover(h, "marco");
        await settle();

        const nodes = [
            pill(stage) as HTMLElement,
            ...Array.from(pill(stage)!.querySelectorAll("*")),
        ];
        for (const node of nodes) {
            const attributes = node.getAttributeNames();
            expect(attributes.filter((name) => name.startsWith("aria-"))).toEqual([]);
            expect(attributes).not.toContain("role");
            expect(attributes).not.toContain("tabindex");
        }
    });

    it("marks the viewpoint the terms are read from", async () => {
        const h = harness();
        const { stage, root, surface } = h;
        answer = { relationships: [uncleOf("giulia", "marco")] };
        surface.selection.select({ kind: "person", id: "giulia" });
        hover(h, "marco");
        await settle();
        const dot = stage.querySelector(".kul-lens-viewpoint") as HTMLElement;
        expect(dot).not.toBeNull();
        expect(dot.title).toBe(VIEWPOINT_TITLE);
    });

    it("phrases in the active locale, one language at a time", async () => {
        const h = harness();
        const { stage, root, surface, locale } = h;
        answer = { relationships: [uncleOf("giulia", "marco")] };
        locale.select("gu");
        surface.selection.select({ kind: "person", id: "giulia" });
        hover(h, "marco");
        await settle();

        const term = stage.querySelector(".kul-lens-term") as HTMLElement;
        expect(term.textContent).toBe("કાકા");
        expect(term.getAttribute("lang")).toBe("gu");
        // Never bilingual stacking: one term, and the English reading is a
        // gloss on hover rather than a second line.
        expect(terms(stage)).toHaveLength(1);
        expect(term.title).toBe("kākā");
    });

    it("re-phrases a pill that is up when the language flips", async () => {
        const h = harness();
        const { stage, root, surface, locale } = h;
        answer = { relationships: [uncleOf("giulia", "marco")] };
        surface.selection.select({ kind: "person", id: "giulia" });
        hover(h, "marco");
        await settle();
        expect(terms(stage)).toEqual(["uncle"]);
        locale.select("gu");
        expect(terms(stage)).toEqual(["કાકા"]);
    });

    it("sizes for a composed chain from the phrasing result, not from the string", async () => {
        const h = harness();
        const { stage, root, surface, locale } = h;
        answer = { relationships: [unlexicalized("giulia", "marco")] };
        locale.select("gu");
        surface.selection.select({ kind: "person", id: "giulia" });
        hover(h, "marco");
        await settle();

        const term = stage.querySelector(".kul-lens-term") as HTMLElement;
        // A tie Gujarati has no word for: the genitive chain the fallback
        // builds off *māmā*, a term it does have, rather than a five-link walk
        // from ego. The pill renders it whole — it never truncates a phrase.
        expect(term.dataset.phraseKind).toBe("composed");
        expect(term.textContent).toBe("મામાનો પુત્રનો પુત્ર");
        expect(term.title).toBe("māmā-no putra-no putra");
        // The slot widens by the chain's own length — two genitive links here,
        // zero for *kākā* — which is what lets one pill hold both without ever
        // measuring or inspecting the string (ADR-0033, ADR-0039).
        expect(term.style.getPropertyValue("--kul-phrase-hops")).toBe("2");
    });
});

describe("the pill contains every phrase it can be handed", () => {
    // jsdom resolves neither custom-property substitution nor layout, so the
    // overflow itself is not reachable from this stack — the same limit
    // `tokens.test.ts` records. What *is* reachable is the pair of facts the
    // sizing model rests on: one about the phrasing result, one about the
    // stylesheet. Both are asserted, because getting either wrong is what put
    // 145px of English through an 85px box.
    const SRC = join(dirname(fileURLToPath(import.meta.url)), "..", "src");
    const appSheet = readFileSync(join(SRC, "preview.css"), "utf8");

    /**
     * Every rule body whose selector mentions the docked tag or the lens's
     * content inside it, comments stripped. Both prefixes, because the
     * containment hazard belongs to the *grammar*: any consumer that puts text
     * in a tag inherits it (ADR-0044).
     */
    const lensRules = [
        ...appSheet
            .replace(/\/\*[\s\S]*?\*\//g, "")
            .matchAll(/((?:\.kul-docked-tag|\.kul-lens)[^{]*)\{([^}]*)\}/g),
    ].map((m) => ({ selector: m[1].trim(), body: m[2] }));

    it("has lexical terms far too long for a hop-derived slot", () => {
        // `hopCount` is `0` for every lexical phrase by construction, so it
        // reports nothing about how wide one is — and lexical terms are not
        // uniformly short. *kākā* is four characters; English lexicalizes
        // *second cousin twice removed*. A slot sized from the hop count
        // therefore cannot govern a lexical term, whatever base it starts from.
        const longest = PACKS.flatMap((pack) => pack.entries)
            .map((entry) => entry.term)
            .reduce((a, b) => (b.length > a.length ? b : a));
        expect(longest.length).toBeGreaterThan(20);
        expect(PACKS.length).toBeGreaterThan(0);
    });

    it("lets every phrase wrap inside the pill instead of pinning it to a line", () => {
        // A flex item's automatic minimum size is its min-content width, and
        // for text that may not wrap that is the *whole string* — so a
        // `white-space: nowrap` phrase silently refuses every `max-width` and
        // paints through the pill's border. `min-width: 0` is what makes a cap
        // bind at all.
        expect(lensRules.length).toBeGreaterThan(0);
        for (const rule of lensRules) {
            expect(`${rule.selector} { ${rule.body} }`).not.toMatch(
                /white-space:\s*nowrap/,
            );
        }
        const relaxed = lensRules
            .filter((rule) => /min-width:\s*0/.test(rule.body))
            .map((rule) => rule.selector);
        expect(relaxed).toContain(".kul-lens-term");
        expect(relaxed).toContain(".kul-lens-empty");
    });

    it("keeps the tag reachable by the pointer, so a gloss can be hovered", () => {
        // `pointer-events: auto` on the tag is what makes "hovering a term
        // gives the fuller gloss" possible at all — the float layer around it
        // is `none`. Losing it would silently break every consumer's gloss,
        // and jsdom cannot see it any other way.
        const tag = lensRules.find((rule) => rule.selector === ".kul-docked-tag");
        expect(tag?.body).toMatch(/pointer-events:\s*auto/);
    });

    it("caps a term by the pill, and only scales a slot for a composed chain", () => {
        const term = lensRules.find((rule) => rule.selector === ".kul-lens-term");
        const composed = lensRules.find((rule) =>
            rule.selector.includes('data-phrase-kind="composed"'),
        );
        // The unqualified rule governs both kinds, so its cap is the pill.
        expect(term?.body).toMatch(/max-width:\s*100%/);
        expect(term?.body).not.toMatch(/--kul-phrase-hops/);
        // The hop count widens a chain's slot and nothing else's, still bounded
        // by the pill — which is the whole of "size from the result, never from
        // the string" that survives contact with a 27-character lexical term.
        expect(composed?.body).toMatch(/--kul-phrase-hops/);
        expect(composed?.body).toMatch(/min\(\s*100%/);
        expect(
            lensRules.some((rule) =>
                rule.selector.includes('data-phrase-kind="lexical"'),
            ),
        ).toBe(false);
    });
});

describe("emptiness stays two answers, never one", () => {
    it("whispers the bounded form when the engine ran out of budget", async () => {
        const h = harness();
        const { stage, root, surface } = h;
        answer = { relationships: [], emptyReason: "noneWithinBounds" };
        surface.selection.select({ kind: "person", id: "giulia" });
        hover(h, "marco");
        await settle();
        expect(pill(stage)?.textContent).toBe(NOT_RELATED_WITHIN_BOUNDS);
    });

    it("says something different when no budget could ever help", async () => {
        const h = harness();
        const { stage, root, surface } = h;
        answer = { relationships: [], emptyReason: "disconnected" };
        surface.selection.select({ kind: "person", id: "giulia" });
        hover(h, "marco");
        await settle();
        expect(pill(stage)?.textContent).toBe(NOT_RELATED_DISCONNECTED);
        expect(NOT_RELATED_DISCONNECTED).not.toBe(NOT_RELATED_WITHIN_BOUNDS);
    });

    it("falls through to the bounded wording when no reason arrived", () => {
        // The engine sets a reason iff the list is empty, so this cannot occur
        // — but the fallthrough has a *direction*, and only one of the two is
        // safe. Claiming "no connection" for an answer that merely ran out of
        // budget asserts more than the engine did, which is the exact failure
        // the `disconnected` / `noneWithinBounds` split exists to prevent
        // (ADR-0028). `kul query rel`'s human output falls the same way.
        expect(emptinessWhisper({ relationships: [] })).toBe(
            NOT_RELATED_WITHIN_BOUNDS,
        );
    });

    it("whispers nothing at all when there is a tie to show", () => {
        expect(
            emptinessWhisper({ relationships: [uncleOf("giulia", "marco")] }),
        ).toBeNull();
        // A reason alongside relationships is not the engine's shape, and the
        // relationships still win: the pill shows terms, never both.
        expect(
            emptinessWhisper({
                relationships: [uncleOf("giulia", "marco")],
                emptyReason: "disconnected",
            }),
        ).toBeNull();
    });

    it("shows no viewpoint dot and no terms when there is no tie", async () => {
        const h = harness();
        const { stage, root, surface } = h;
        answer = { relationships: [], emptyReason: "noneWithinBounds" };
        surface.selection.select({ kind: "person", id: "giulia" });
        hover(h, "marco");
        await settle();
        expect(stage.querySelector(".kul-lens-viewpoint")).toBeNull();
        expect(terms(stage)).toEqual([]);
    });
});

describe("the traced path shows why the answer is what it is", () => {
    it("selects marriages for `across` hops and child edges for vertical ones", () => {
        // Read at the pure seam: which nodes a descriptor names, before any
        // DOM is involved. `up` leaves *from* a child and `down` lands *on*
        // one, and getting that backwards paints a plausible wrong edge.
        expect(resolutionPathBindings(fatherInLawOf("giulia", "sergio"))).toEqual([
            { kind: "marriage", marriageId: "m1" },
            { kind: "person", personId: "marco" },
            { kind: "parenthood", childId: "marco" },
        ]);
    });

    it("paints the connecting persons and edges, but neither endpoint", async () => {
        const h = harness();
        const { root, surface } = h;
        answer = { relationships: [fatherInLawOf("giulia", "luca")] };
        surface.selection.select({ kind: "person", id: "giulia" });
        hover(h, "luca");
        await settle();

        const painted = pathNodes(root);
        // `marco` is the person the path runs through, and both his cards
        // light — canonical and ghost (ADR-0034).
        expect(painted.filter((n) => n === "marco")).toHaveLength(2);
        expect(painted).toContain("marriage:m1");
        expect(painted).toContain("birth:marco");
        // The endpoints are not on the trace: one wears the selection violet,
        // the other is under the pointer.
        expect(painted).not.toContain("giulia");
        expect(painted).not.toContain("luca");
    });

    it("traces every tie when there is more than one", async () => {
        const h = harness();
        const { root, surface } = h;
        answer = {
            relationships: [
                fatherInLawOf("giulia", "luca"),
                uncleOf("giulia", "luca"),
            ],
        };
        surface.selection.select({ kind: "person", id: "giulia" });
        hover(h, "luca");
        await settle();
        // The uncle path's last hop lands on `luca`, drawn as luca's birth
        // edge — a node only the second descriptor reaches.
        expect(pathNodes(root)).toContain("birth:luca");
        expect(pathNodes(root)).toContain("marriage:m1");
    });

    it("comes down with the pill when the pointer leaves the picture", async () => {
        const h = harness();
        const { stage, root, surface } = h;
        answer = { relationships: [fatherInLawOf("giulia", "luca")] };
        surface.selection.select({ kind: "person", id: "giulia" });
        hover(h, "luca");
        await settle();
        expect(pathNodes(root).length).toBeGreaterThan(0);

        surface.handleCanvasHover(null);
        expect(pill(stage)).toBeNull();
        expect(pathNodes(root)).toEqual([]);
    });

    it("keeps reading while the pointer is on its own pill", async () => {
        const h = harness();
        const { stage, root, surface } = h;
        answer = { relationships: [uncleOf("giulia", "marco")] };
        surface.selection.select({ kind: "person", id: "giulia" });
        hover(h, "marco");
        await settle();

        // Moving onto the pill to read a term leaves the canvas; that must not
        // dismiss the thing being read.
        surface.handleCanvasHover(stage.querySelector(".kul-lens-term"));
        expect(pill(stage)).not.toBeNull();
    });
});

describe("a render takes the lens down rather than re-asserting it", () => {
    it("dismisses on the post-render repaint", async () => {
        const h = harness();
        const { stage, root, surface } = h;
        answer = { relationships: [uncleOf("giulia", "marco")] };
        surface.selection.select({ kind: "person", id: "giulia" });
        hover(h, "marco");
        await settle();
        expect(pill(stage)).not.toBeNull();

        // A render replaced the picture the tie was computed against.
        surface.repaintQueryChrome();
        expect(pill(stage)).toBeNull();
    });

    it("abandons a query still in flight when the surface is disposed", async () => {
        const h = harness();
        const { stage, surface } = h;
        answer = { relationships: [uncleOf("giulia", "marco")] };
        surface.selection.select({ kind: "person", id: "giulia" });
        hover(h, "marco");
        surface.dispose();
        await settle();
        expect(pill(stage)).toBeNull();
    });
});

describe("the mounted preview feeds the lens pointer movement", () => {
    const PROJECT: ProjectSnapshot = {
        files: [{ name: "family.kul", source: "# fixture\n" }],
        manifest: { kul: "0.1" },
    };

    function mounted(): { container: HTMLElement; handle: PreviewHandle } {
        const container = document.createElement("div");
        document.body.appendChild(container);
        const engine: QueryEngine = {
            async queryDetail(_project, targets) {
                return {
                    ok: true,
                    result: targets.map(() => PERSON_DETAIL),
                } as QueryEnvelope<DetailLookupResult>;
            },
            async queryKin() {
                return { ok: true, result: { kind: "count" as const, count: 0 } };
            },
            async runQuery() {
                return {
                    ok: true,
                    result: { kind: "personIds" as const, personIds: [] },
                };
            },
            async queryResolve() {
                return {
                    ok: true,
                    result: { relationships: [uncleOf("giulia", "marco")] },
                };
            },
            get isLoaded() {
                return true;
            },
        };
        const handle = mountPreview(
            container,
            { onRevealRequest: () => {} },
            { engine },
        );
        handle.render(SVG, PROJECT);
        return { container, handle };
    }

    it("resolves on a plain pointer move over a card — no click, no mode", async () => {
        const { container, handle } = mounted();
        const root = container.querySelector("#root") as HTMLElement;
        root.querySelector('[data-person-id="giulia"] rect')?.dispatchEvent(
            new MouseEvent("click", { bubbles: true }),
        );
        await vi.advanceTimersByTimeAsync(0);

        root.querySelector('[data-person-id="marco"] rect')?.dispatchEvent(
            new MouseEvent("pointermove", { bubbles: true }),
        );
        await settle();
        expect(container.querySelector(".kul-lens")?.textContent).toContain("uncle");
        handle.dispose();
    });

    it("does not dismiss when the pointer leaves the canvas onto the pill", async () => {
        const { container, handle } = mounted();
        const root = container.querySelector("#root") as HTMLElement;
        root.querySelector('[data-person-id="giulia"] rect')?.dispatchEvent(
            new MouseEvent("click", { bubbles: true }),
        );
        await vi.advanceTimersByTimeAsync(0);
        root.querySelector('[data-person-id="marco"] rect')?.dispatchEvent(
            new MouseEvent("pointermove", { bubbles: true }),
        );
        await settle();

        const term = container.querySelector(".kul-lens-term") as Element;
        root.dispatchEvent(
            new MouseEvent("pointerleave", { relatedTarget: term }),
        );
        expect(container.querySelector(".kul-lens")).not.toBeNull();

        // Leaving onto anything else does take it down.
        root.dispatchEvent(new MouseEvent("pointerleave", { relatedTarget: null }));
        expect(container.querySelector(".kul-lens")).toBeNull();
        handle.dispose();
    });
});

describe("a live read outranks the paint the reader left behind", () => {
    it("names the intermediates and the alter, deduplicated, never the ego", () => {
        // Read at the pure seam: the exemption comes from the backbones, which
        // is what lets it be published without owing a republish after a render
        // (`dim.ts` obliges only sources that read the picture to decide).
        expect(tracedPersonIds([fatherInLawOf("giulia", "sergio")])).toEqual([
            "marco",
            "sergio",
        ]);
        // Two ways of being related routinely share a person; the exemption is
        // a set, not a concatenation.
        const both = tracedPersonIds([
            fatherInLawOf("giulia", "luca"),
            fatherInLawOf("giulia", "luca"),
        ]);
        expect(both).toEqual(["marco", "luca"]);
        expect(both).not.toContain("giulia");
    });

    it("publishes on the settle and withdraws on dismiss", () => {
        const published: Array<ReadonlyArray<string> | null> = [];
        const selection = createSelectionStore();
        const stage = document.createElement("div");
        stage.innerHTML = PREVIEW_BODY_HTML;
        document.body.appendChild(stage);
        const root = stage.querySelector("#root") as HTMLElement;
        root.innerHTML = SVG;
        const lens = createHoverLens({
            root,
            layer: stage.querySelector("#kul-region-float") as HTMLElement,
            selection,
            async resolve() {
                return {
                    ok: true,
                    result: { relationships: [fatherInLawOf("giulia", "luca")] },
                };
            },
            bindPhrase: () => () => {},
            onTrace: (ids) => published.push(ids === null ? null : [...ids]),
        });

        selection.select({ kind: "person", id: "giulia" });
        // A selection change dismisses, but nothing was ever traced — and
        // withdrawing walks every card, so it must not fire on an empty slot.
        expect(published).toEqual([]);

        lens.handleHover(cardOf(root, "luca").querySelector("rect"));
        expect(published).toEqual([]);

        return settle().then(() => {
            expect(published).toEqual([["marco", "luca"]]);
            lens.dismiss();
            expect(published).toEqual([["marco", "luca"], null]);
            // Idempotent: a second dismiss has nothing to withdraw.
            lens.dismiss();
            expect(published).toHaveLength(2);
            lens.dispose();
        });
    });

    it("publishes nothing for an unrelated pair, which traces nobody", async () => {
        // An empty exemption exempts no one, and publishing it would walk every
        // card in the picture to change nothing — then walk it again on the
        // dismiss that withdrew it.
        const published: Array<ReadonlyArray<string> | null> = [];
        const h = harness({
            async resolve() {
                return {
                    ok: true,
                    result: { relationships: [], emptyReason: "noneWithinBounds" },
                };
            },
        });
        const lens = createHoverLens({
            root: h.root,
            layer: h.stage.querySelector("#kul-region-float") as HTMLElement,
            selection: h.surface.selection,
            async resolve() {
                return {
                    ok: true,
                    result: { relationships: [], emptyReason: "noneWithinBounds" },
                };
            },
            bindPhrase: () => () => {},
            onTrace: (ids) => published.push(ids === null ? null : [...ids]),
        });
        h.surface.selection.select({ kind: "person", id: "giulia" });
        lens.handleHover(cardOf(h.root, "marco").querySelector("rect"));
        await settle();
        // The pill is up — the whisper still happened — and nothing was published.
        expect(h.stage.querySelectorAll(".kul-lens-empty").length).toBeGreaterThan(0);
        expect(published).toEqual([]);
        lens.dismiss();
        expect(published).toEqual([]);
        lens.dispose();
    });

    it("lifts the kin dim off the persons the trace runs through", async () => {
        // The end-to-end shape of ADR-0043's rule, through the real surface:
        // paint a kin set that excludes `marco`, then hover a card whose answer
        // runs through him. He is outside the answer and would render at the
        // dim's alpha, which would leave the explanation fainter than the thing
        // it explains.
        const h = harness({
            async runKinQuery(query) {
                return query.projection === "count"
                    ? { ok: true, result: { kind: "count" as const, count: 1 } }
                    : {
                          ok: true,
                          result: {
                              kind: "members" as const,
                              members: [
                                  {
                                      personId: "dalisay",
                                      descriptor: uncleOf("giulia", "dalisay"),
                                  },
                              ],
                          },
                      };
            },
        });
        answer = { relationships: [fatherInLawOf("giulia", "luca")] };
        h.surface.selection.select({ kind: "person", id: "giulia" });
        await settle();
        (h.stage.querySelector(".kul-kin-header") as HTMLElement).click();
        await settle();
        (h.stage.querySelector(".kul-kin-row") as HTMLElement).click();
        await settle();

        const marco = cardOf(h.root, "marco");
        expect(marco.classList.contains(DIM_CLASS)).toBe(true);

        hover(h, "luca");
        await settle();
        expect(marco.classList.contains(DIM_CLASS)).toBe(false);

        // And it comes back the moment the read ends: the dim is what the
        // reader did a moment ago, and it is still true.
        h.surface.handleCanvasHover(null);
        expect(marco.classList.contains(DIM_CLASS)).toBe(true);
        expect(KIN_SETS.length).toBeGreaterThan(0);
    });
});
