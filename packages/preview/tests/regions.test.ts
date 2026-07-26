import { readFileSync } from "node:fs";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";
import { afterEach, describe, expect, it, vi } from "vitest";

// svg-pan-zoom relies on SVG measurement APIs jsdom does not implement. Stub
// it out before any module imports the dep so the chrome can mount cleanly.
vi.mock("svg-pan-zoom", () => {
    const factory = vi.fn(() => ({
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
    }));
    return { default: factory };
});

import { mountPreview } from "../src/mount.js";
import type { PreviewHandle } from "../src/types.js";

const SAMPLE_SVG = `<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 200 100">
    <g class="kul-card" data-person-id="alice" data-kind="canonical" data-gender="female">
        <rect x="10" y="10" width="60" height="30"/>
        <text class="kul-label-name">Alice</text>
    </g>
</svg>`;

const APP_SHEET = readFileSync(
    join(dirname(fileURLToPath(import.meta.url)), "..", "src", "preview.css"),
    "utf8",
);

function ruleBody(selector: string): string {
    const at = APP_SHEET.indexOf(`\n${selector} {`);
    expect(at, `expected a \`${selector}\` rule`).toBeGreaterThan(-1);
    const open = APP_SHEET.indexOf("{", at);
    return APP_SHEET.slice(open + 1, APP_SHEET.indexOf("}", open));
}

function mount(): { container: HTMLElement; handle: PreviewHandle } {
    const container = document.createElement("div");
    document.body.appendChild(container);
    const handle = mountPreview(container, { onRevealRequest: vi.fn() });
    return { container, handle };
}

function click(container: HTMLElement, selector: string): void {
    (container.querySelector(selector) as HTMLElement).dispatchEvent(
        new MouseEvent("click", { bubbles: true }),
    );
}

afterEach(() => {
    document.body.innerHTML = "";
});

describe("the stage's regions", () => {
    it("mounts one element per region, with the canvas holding #root", () => {
        const { container } = mount();
        const stage = container.querySelector(".kul-stage") as HTMLElement;
        expect(stage).not.toBeNull();
        for (const region of [
            ".kul-region-flow",
            ".kul-region-canvas",
            ".kul-region-overlay",
            ".kul-region-float",
            ".kul-region-notify",
        ]) {
            expect(stage.querySelector(region)?.parentElement).toBe(stage);
        }
        expect(container.querySelector("#root")?.parentElement).toBe(
            container.querySelector(".kul-region-canvas"),
        );
    });

    it("keeps the legend and the error popover open together in one stack", () => {
        const { container, handle } = mount();
        handle.render(SAMPLE_SVG);
        click(container, 'button[data-action="toggle-legend"]');
        handle.showErrors([{ message: "boom", code: "R01" }]);
        click(container, "#kul-error-button");

        const legend = container.querySelector("#kul-legend") as HTMLElement;
        const popover = container.querySelector("#kul-error-popover") as HTMLElement;
        expect(legend.hidden).toBe(false);
        expect(popover.hidden).toBe(false);

        // Both are members of the overlay stack, which lays its children out in
        // sequence — so "both open" can no longer mean "both at the same inset".
        const overlay = container.querySelector(".kul-region-overlay") as HTMLElement;
        expect([...overlay.children].map((el) => el.id)).toEqual([
            "kul-controls",
            "kul-error-popover",
            "kul-legend",
        ]);
    });

    it("declares both float placements, so a later float picks one", () => {
        // ADR-0042: edge-docked chrome joins the dock; entity-anchored chrome
        // (#302's lens pill, positioned by JS against a hovered card) is
        // appended to the layer itself and is unaffected by the dock's flow.
        const { container } = mount();
        const layer = container.querySelector("#kul-region-float") as HTMLElement;
        const dock = container.querySelector(
            "#kul-region-float-dock",
        ) as HTMLElement;
        expect(dock.parentElement).toBe(layer);
        expect(ruleBody(".kul-region-float")).not.toMatch(/display\s*:\s*flex/);
        expect(ruleBody(".kul-region-float-dock")).toMatch(/display\s*:\s*flex/);
    });

    it("leaves placement to the region, not to the chrome inside it", () => {
        const placement = /(^|\s)(position|top|right|bottom|left|inset|z-index)\s*:/;
        for (const selector of [
            ".kul-preview-controls",
            ".kul-error-popover",
            ".kul-preview-legend",
            // The details panel joins the float region and the sync hint the
            // notify region by being appended — neither invents an inset. The
            // quiet toast joins the notify region the same way (ADR-0043).
            ".kul-query-panel",
            ".kul-sync-hint",
            ".kul-toast",
        ]) {
            expect(ruleBody(selector)).not.toMatch(placement);
        }
        // …and no site computes an inset from another widget's dimensions.
        expect(APP_SHEET).not.toContain("--kul-control-inset");
    });
});
