// The hover lens, driven the way a reader drives it: a selection, then a
// pointer moving over the picture, with the engine substituted at its one
// system boundary (`CODING_STANDARDS.md` — mock at system boundaries, never
// internal collaborators).
//
// The debounce is exercised through `handleCanvasHover` with Vitest's fake
// timers rather than by waiting on a wall clock: what #302 asks to be proved
// is that a sweep coalesces into one query per settle, and that is a statement
// about the module's seam, not about how fast a machine happens to be.

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
    resolutionPathBindings,
} from "../src/hover-lens.js";
import type { ProjectSnapshot, QueryEngine } from "../src/engine.js";
import { createLocaleController } from "../src/locale.js";
import { createMemoryLocaleStore } from "../src/locale-store.js";
import { mountPreview } from "../src/mount.js";
import type { RelationshipDescriptor } from "../src/phrasing/descriptor.js";
import { createQuerySurface } from "../src/query-surface.js";
import type { QuerySurface } from "../src/query-surface.js";
import type { LocaleController } from "../src/locale.js";
import type { PreviewHandle } from "../src/types.js";

// Three generations plus a spouse, with the edges the lens traces: one
// marriage, two birth edges and one adoption edge. Ghost and canonical cards
// for `marco`, because a tie is about a person and every card of theirs binds.
const SVG = `<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 600 400">
    <g class="kul-card" data-person-id="giulia" data-kind="canonical" data-gender="female"><rect/></g>
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

function harness(): Harness {
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
        async resolve(egoId, alterId) {
            asked.push([egoId, alterId]);
            return { ok: true, result: answer };
        },
        getPanZoom: () => null,
        applySyncHighlight: (ref) => {
            highlightEntity(root, null, ref);
        },
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
