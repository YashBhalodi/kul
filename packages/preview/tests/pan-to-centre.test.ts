// Pan-to-centre with a panel floating over the canvas (ADR-0035 refined by
// ADR-0036): a panel-driven walk must never park a card behind the panel, so
// centring targets the *visible* region. Verified with the panel on both sides
// — nothing in the geometry may assume which edge it hugs.

import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";

const panCalls: Array<{ x: number; y: number }> = [];

vi.mock("svg-pan-zoom", () => {
    const make = () => ({
        getPan: () => ({ x: 0, y: 0 }),
        getZoom: () => 1,
        getSizes: () => ({ width: 800, height: 600, realZoom: 1 }),
        pan: (p: { x: number; y: number }) => {
            panCalls.push(p);
        },
        panBy: vi.fn(),
        zoom: vi.fn(),
        zoomIn: vi.fn(),
        zoomOut: vi.fn(),
        reset: vi.fn(),
        destroy: vi.fn(),
    });
    return { default: vi.fn(() => make()) };
});

import type {
    DetailLookupResult,
    DetailTarget,
    EntityDetail,
    QueryEnvelope,
} from "../src/engine-wire.js";
import type { ProjectSnapshot, QueryEngine } from "../src/engine.js";
import { visibleCentre } from "../src/highlight.js";
import { mountPreview } from "../src/mount.js";
import type { HostAdapter } from "../src/types.js";

const VIEWPORT = { width: 800, height: 600 };

describe("the visible centre", () => {
    it("is the viewport centre when nothing floats over the canvas", () => {
        expect(visibleCentre(VIEWPORT, null)).toEqual({ x: 400, y: 300 });
    });

    it("shifts left when the panel hugs the right edge", () => {
        expect(
            visibleCentre(VIEWPORT, { left: 500, right: 800, top: 0, bottom: 600 }),
        ).toEqual({ x: 250, y: 300 });
    });

    it("shifts right when the panel hugs the left edge", () => {
        expect(
            visibleCentre(VIEWPORT, { left: 0, right: 300, top: 0, bottom: 600 }),
        ).toEqual({ x: 550, y: 300 });
    });

    it("keeps the vertical centre — the panel is a side panel, not a banner", () => {
        expect(
            visibleCentre(VIEWPORT, { left: 500, right: 800, top: 100, bottom: 200 })
                .y,
        ).toBe(300);
    });

    it("takes the wider free strip when the panel floats mid-canvas", () => {
        // 300px free on the left, 400px on the right.
        expect(
            visibleCentre(VIEWPORT, { left: 300, right: 400, top: 0, bottom: 600 }),
        ).toEqual({ x: 600, y: 300 });
    });

    it("falls back to the raw centre when the panel leaves no visible region", () => {
        expect(
            visibleCentre(VIEWPORT, { left: 0, right: 800, top: 0, bottom: 600 }),
        ).toEqual({ x: 400, y: 300 });
    });

    it("ignores a box that does not overlap the viewport at all", () => {
        expect(
            visibleCentre(VIEWPORT, { left: 900, right: 1200, top: 0, bottom: 600 }),
        ).toEqual({ x: 400, y: 300 });
    });
});

// --- The same rule, through the mounted chrome ---------------------------

const PROJECT: ProjectSnapshot = {
    files: [{ name: "family.kul", source: "# fixture\n" }],
    manifest: { kul: "0.1" },
};

const SVG = `<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 400 200">
    <g class="kul-card" data-person-id="giulia" data-kind="canonical" data-gender="female"><rect x="100" y="50" width="60" height="30"/></g>
    <g class="kul-card" data-person-id="marco" data-kind="canonical" data-gender="male"><rect x="200" y="50" width="60" height="30"/></g>
    <g class="kul-card" data-person-id="marco" data-kind="ghost"><rect x="0" y="0" width="60" height="30"/></g>
    <path class="kul-edge" data-link-kind="marriage" data-marriage-id="m1" data-host-id="giulia" data-joining-id="marco" d="M 0 0 L 1 1"/>
</svg>`;

const GIULIA = { id: "giulia", name: "Giulia Rossi", gender: "female" };
const MARCO = { id: "marco", name: "Marco Rossi", gender: "male" };
const M1 = { id: "m1", spouses: ["giulia", "marco"] as [string, string] };

function detailFor(target: DetailTarget): EntityDetail | null {
    if (target.kind === "marriage") {
        return {
            kind: "marriage",
            marriage: M1,
            spouses: [GIULIA, MARCO],
            children: [],
        };
    }
    if (target.kind === "person") {
        return {
            kind: "person",
            person: target.id === "marco" ? MARCO : GIULIA,
            parents: [],
            marriages: [],
            children: [],
        };
    }
    return null;
}

const engine: QueryEngine = {
    async queryDetail(_project, targets) {
        return {
            ok: true,
            result: targets.map(detailFor),
        } as QueryEnvelope<DetailLookupResult>;
    },
    get isLoaded() {
        return true;
    },
};

const adapter: HostAdapter = { onRevealRequest: () => {} };

function settle(): Promise<void> {
    return new Promise((resolve) => setTimeout(resolve, 0));
}

function click(el: Element | null): void {
    (el as Element).dispatchEvent(new MouseEvent("click", { bubbles: true }));
}

/** jsdom implements neither `getBBox` nor layout, so both are supplied here. */
function stubBBox(el: Element, box: DOMRect | Record<string, number>): void {
    (el as unknown as { getBBox(): unknown }).getBBox = () => box;
}

function stubRect(el: Element, box: Record<string, number>): void {
    (el as unknown as { getBoundingClientRect(): unknown }).getBoundingClientRect =
        () => box;
}

let rafSpy: ReturnType<typeof vi.spyOn> | null = null;

beforeEach(() => {
    panCalls.length = 0;
    document.body.innerHTML = "";
    // Run the tween to completion in one frame so the assertion is about the
    // pan the walk aimed at, not about an intermediate eased position.
    rafSpy = vi
        .spyOn(window, "requestAnimationFrame")
        .mockImplementation((cb: FrameRequestCallback) => {
            cb(performance.now() + 10_000);
            return 1;
        });
});

afterEach(() => {
    rafSpy?.mockRestore();
    document.body.innerHTML = "";
});

/**
 * Open the marriage panel, place it at `panelBox`, then walk to Marco through
 * the panel's spouse row. Marco's card is centred on (230, 65) in user units,
 * and the stubbed viewport is 800×600 at realZoom 1.
 */
async function walkToMarco(
    panelBox: Record<string, number> | null,
): Promise<{ x: number; y: number }> {
    const container = document.createElement("div");
    document.body.appendChild(container);
    const handle = mountPreview(container, adapter, { engine });
    handle.render(SVG, PROJECT);
    stubBBox(
        container.querySelector(
            '[data-person-id="marco"][data-kind="canonical"]',
        ) as Element,
        { x: 200, y: 50, width: 60, height: 30 },
    );
    click(container.querySelector('[data-link-kind="marriage"]'));
    await settle();
    if (panelBox) {
        stubRect(container.querySelector(".kul-query-panel") as Element, panelBox);
    }
    const marco = Array.from(
        container.querySelectorAll(".kul-query-panel-person"),
    ).find((el) => el.textContent === "Marco Rossi");
    click(marco ?? container.querySelector(".kul-query-panel-person"));
    return panCalls[panCalls.length - 1];
}

describe("walking the family from the panel", () => {
    it("centres the card in the raw viewport when the panel occupies no strip", async () => {
        // jsdom lays nothing out, so the panel measures to a zero box — the
        // same answer the geometry gives when no panel is open at all.
        // 400 − 230 = 170, 300 − 65 = 235.
        expect(await walkToMarco(null)).toEqual({ x: 170, y: 235 });
    });

    it("centres it clear of a panel hugging the right edge", async () => {
        // Visible strip [0, 500] → centre 250; 250 − 230 = 20.
        expect(
            await walkToMarco({ left: 500, right: 800, top: 0, bottom: 600 }),
        ).toEqual({ x: 20, y: 235 });
    });

    it("centres it clear of a panel hugging the left edge", async () => {
        // Visible strip [300, 800] → centre 550; 550 − 230 = 320.
        expect(
            await walkToMarco({ left: 0, right: 300, top: 0, bottom: 600 }),
        ).toEqual({ x: 320, y: 235 });
    });

    it("measures the panel against the SVG's own origin, not the viewport's", async () => {
        // The stage is not always flush with the window — a flow-region filter
        // bar (#303) pushes the canvas down, and the extension's pane has its
        // own offset. The panel's viewport box must be translated into the
        // pan/zoom viewport's local pixels or the free strip is measured in the
        // wrong coordinate system.
        const container = document.createElement("div");
        document.body.appendChild(container);
        const handle = mountPreview(container, adapter, { engine });
        handle.render(SVG, PROJECT);
        stubBBox(
            container.querySelector(
                '[data-person-id="marco"][data-kind="canonical"]',
            ) as Element,
            { x: 200, y: 50, width: 60, height: 30 },
        );
        // SVG origin at (200, 40); panel at viewport [700, 1000] → local
        // [500, 800] → visible strip [0, 500] → centre 250 → 250 − 230 = 20.
        stubRect(container.querySelector("svg") as Element, {
            left: 200,
            top: 40,
            right: 1000,
            bottom: 640,
        });
        click(container.querySelector('[data-link-kind="marriage"]'));
        await settle();
        stubRect(container.querySelector(".kul-query-panel") as Element, {
            left: 700,
            right: 1000,
            top: 40,
            bottom: 640,
        });
        const marco = Array.from(
            container.querySelectorAll(".kul-query-panel-person"),
        ).find((el) => el.textContent === "Marco Rossi");
        click(marco ?? null);
        expect(panCalls[panCalls.length - 1]).toEqual({ x: 20, y: 235 });
    });

    it("steers the ghost badge's jump around the panel too", async () => {
        // Two navigation paths reach the canonical card; both must avoid the
        // panel, or the older one silently parks a card behind it.
        const container = document.createElement("div");
        document.body.appendChild(container);
        const handle = mountPreview(container, adapter, { engine });
        handle.render(SVG, PROJECT);
        stubBBox(
            container.querySelector(
                '[data-person-id="marco"][data-kind="canonical"]',
            ) as Element,
            { x: 200, y: 50, width: 60, height: 30 },
        );
        click(container.querySelector('[data-link-kind="marriage"]'));
        await settle();
        stubRect(container.querySelector(".kul-query-panel") as Element, {
            left: 500,
            right: 800,
            top: 0,
            bottom: 600,
        });
        panCalls.length = 0;

        click(container.querySelector(".kul-ghost-badge"));

        // Same answer as the panel-driven walk: visible strip [0, 500].
        expect(panCalls[panCalls.length - 1]).toEqual({ x: 20, y: 235 });
    });

    it("centres the canonical card, never a ghost", async () => {
        const container = document.createElement("div");
        document.body.appendChild(container);
        const handle = mountPreview(container, adapter, { engine });
        handle.render(SVG, PROJECT);
        // Only the ghost can measure; if the walk picked it, a pan would fire.
        stubBBox(
            container.querySelector(
                '[data-person-id="marco"][data-kind="ghost"]',
            ) as Element,
            { x: 0, y: 0, width: 60, height: 30 },
        );
        click(container.querySelector('[data-link-kind="marriage"]'));
        await settle();
        const marco = Array.from(
            container.querySelectorAll(".kul-query-panel-person"),
        ).find((el) => el.textContent === "Marco Rossi");
        click(marco ?? null);
        expect(panCalls).toEqual([]);
    });
});
