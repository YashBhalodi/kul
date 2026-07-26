// Selection and the details panel, driven the way a reader drives them:
// through the mounted chrome, with the engine substituted at its one system
// boundary (`CODING_STANDARDS.md` — mock at system boundaries, never internal
// collaborators).

import { readFileSync } from "node:fs";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";
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

import { PREVIEW_BODY_HTML } from "../src/controls.js";
import type {
    DetailLookupResult,
    DetailTarget,
    EntityDetail,
    QueryEnvelope,
} from "../src/engine-wire.js";
import { highlightEntity } from "../src/highlight.js";
import { createLocaleController } from "../src/locale.js";
import type { ProjectSnapshot, QueryEngine } from "../src/engine.js";
import { SYNC_SUSPENDED_HINT, createQuerySurface } from "../src/query-surface.js";
import type { QuerySurface } from "../src/query-surface.js";
import { mountPreview } from "../src/mount.js";
import type { HostAdapter, PreviewHandle, RevealTarget } from "../src/types.js";

const PROJECT: ProjectSnapshot = {
    files: [{ name: "family.kul", source: "# fixture\n" }],
    manifest: { kul: "0.1" },
};

// The labels in the picture deliberately disagree with the engine's names.
// Anything the panel renders that matches "…-ON-CARD" would prove it read the
// SVG; ADR-0035 says it must read the batched answer and nothing else.
const SVG = `<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 400 200">
    <g class="kul-card" data-person-id="giulia" data-kind="canonical" data-gender="female">
        <rect x="100" y="50" width="60" height="30"/>
        <text class="kul-label-name">GIULIA-ON-CARD</text>
    </g>
    <g class="kul-card" data-person-id="marco" data-kind="canonical" data-gender="male">
        <rect x="200" y="50" width="60" height="30"/>
        <text class="kul-label-name">MARCO-ON-CARD</text>
    </g>
    <g class="kul-card" data-person-id="dalisay" data-kind="canonical" data-gender="female">
        <rect x="300" y="120" width="60" height="30"/>
        <text class="kul-label-name">DALISAY-ON-CARD</text>
    </g>
    <path class="kul-edge" data-link-kind="marriage" data-marriage-id="m1" data-host-id="giulia" data-joining-id="marco" d="M 0 0 L 1 1"/>
    <path class="kul-edge" data-link-kind="adoption" data-marriage-id="m1" data-child-id="dalisay" d="M 0 0 L 1 1"/>
</svg>`;

const GIULIA = { id: "giulia", name: "Giulia Rossi", gender: "female" };
const MARCO = { id: "marco", name: "Marco Rossi", gender: "male" };
const DALISAY = { id: "dalisay", name: "Dalisay Reyes", gender: "female" };
const M1 = {
    id: "m1",
    spouses: ["giulia", "marco"] as [string, string],
    start: { value: "1948", precision: "year", circa: false },
};
const ADOPTION_LINK = {
    marriageId: "m1",
    childId: "dalisay",
    kind: "adoptive" as const,
};

function detailFor(target: DetailTarget): EntityDetail | null {
    if (target.kind === "person") {
        const person = { giulia: GIULIA, marco: MARCO, dalisay: DALISAY }[
            target.id
        ];
        return person
            ? {
                  kind: "person",
                  person,
                  parents: [],
                  marriages:
                      person.id === "giulia"
                          ? [{ marriage: M1, spouse: MARCO }]
                          : [],
                  children: [],
              }
            : null;
    }
    if (target.kind === "marriage") {
        return target.id === "m1"
            ? {
                  kind: "marriage",
                  marriage: M1,
                  spouses: [GIULIA, MARCO],
                  children: [],
              }
            : null;
    }
    return {
        kind: "adoption",
        adoption: ADOPTION_LINK,
        child: DALISAY,
        parents: [GIULIA, MARCO],
    };
}

/** Engine double: answers each target from the fixture, in the order asked. */
function fakeEngine(): { engine: QueryEngine; targets: DetailTarget[][] } {
    const targets: DetailTarget[][] = [];
    return {
        engine: {
            async queryDetail(_project, asked) {
                targets.push(asked);
                return {
                    ok: true,
                    result: asked.map(detailFor),
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
                return { ok: true, result: { relationships: [] } };
            },
            get isLoaded() {
                return true;
            },
        },
        targets,
    };
}

function mount(): {
    container: HTMLElement;
    handle: PreviewHandle;
    reveals: RevealTarget[];
    targets: DetailTarget[][];
} {
    const container = document.createElement("div");
    document.body.appendChild(container);
    const reveals: RevealTarget[] = [];
    const adapter: HostAdapter = {
        onRevealRequest(target) {
            reveals.push(target);
        },
    };
    const { engine, targets } = fakeEngine();
    const handle = mountPreview(container, adapter, { engine });
    handle.render(SVG, PROJECT);
    return { container, handle, reveals, targets };
}

function click(el: Element | null): void {
    (el as Element).dispatchEvent(new MouseEvent("click", { bubbles: true }));
}

/** Let the batched lookup resolve and the panel render. */
function settle(): Promise<void> {
    return new Promise((resolve) => setTimeout(resolve, 0));
}

function panel(container: HTMLElement): HTMLElement | null {
    return container.querySelector(".kul-query-panel");
}

function rowNames(container: HTMLElement, section: string): string[] {
    const groups = Array.from(
        container.querySelectorAll(".kul-query-panel-section"),
    );
    const group = groups.find(
        (g) =>
            g.querySelector(".kul-query-panel-section-title")?.textContent ===
            section,
    );
    return Array.from(group?.querySelectorAll(".kul-query-panel-person") ?? []).map(
        (el) => el.textContent ?? "",
    );
}

beforeEach(() => {
    panCalls.length = 0;
    document.body.innerHTML = "";
});

afterEach(() => {
    document.body.innerHTML = "";
});

describe("selecting an entity opens its panel", () => {
    it("asks the batched operation for exactly the selected target", async () => {
        const { container, targets } = mount();
        click(container.querySelector('[data-person-id="giulia"] rect'));
        await settle();
        expect(targets).toEqual([[{ kind: "person", id: "giulia" }]]);
    });

    it("renders the engine's answer, never the label on the card", async () => {
        const { container } = mount();
        click(container.querySelector('[data-person-id="giulia"] rect'));
        await settle();
        const open = panel(container) as HTMLElement;
        expect(open.querySelector(".kul-query-panel-kicker")?.textContent).toBe(
            "Person",
        );
        expect(open.querySelector(".kul-query-panel-title")?.textContent).toBe(
            "Giulia Rossi",
        );
        expect(open.textContent).not.toContain("ON-CARD");
    });

    it("opens the marriage variant from the bar", async () => {
        const { container, targets } = mount();
        click(container.querySelector('[data-link-kind="marriage"]'));
        await settle();
        expect(targets).toEqual([[{ kind: "marriage", id: "m1" }]]);
        expect(
            panel(container)?.querySelector(".kul-query-panel-title")?.textContent,
        ).toBe("Giulia Rossi & Marco Rossi");
    });

    it("opens the adoption variant from the edge, addressed by its pair", async () => {
        const { container, targets } = mount();
        click(container.querySelector('[data-link-kind="adoption"]'));
        await settle();
        expect(targets).toEqual([
            [{ kind: "adoption", childId: "dalisay", marriageId: "m1" }],
        ]);
        expect(
            panel(container)?.querySelector(".kul-query-panel-kicker")?.textContent,
        ).toBe("Adoption");
    });

    it("moves the panel as the selection moves, keeping exactly one open", async () => {
        const { container } = mount();
        click(container.querySelector('[data-person-id="giulia"] rect'));
        await settle();
        click(container.querySelector('[data-link-kind="marriage"]'));
        await settle();
        expect(container.querySelectorAll(".kul-query-panel")).toHaveLength(1);
        expect(
            panel(container)?.querySelector(".kul-query-panel-kicker")?.textContent,
        ).toBe("Marriage");
    });

    it("closes the panel and drops the paint on Esc", async () => {
        const { container } = mount();
        click(container.querySelector('[data-person-id="giulia"] rect'));
        await settle();
        window.dispatchEvent(new KeyboardEvent("keydown", { key: "Escape" }));
        expect(panel(container)).toBeNull();
        expect(container.querySelectorAll(".kul-query-selected")).toHaveLength(0);
    });

    it("closes the panel on a canvas click", async () => {
        const { container } = mount();
        click(container.querySelector('[data-person-id="giulia"] rect'));
        await settle();
        click(container.querySelector("svg"));
        expect(panel(container)).toBeNull();
    });

    it("survives a render — the mode boundary is #304's, not this slice's", async () => {
        const { container, handle } = mount();
        click(container.querySelector('[data-person-id="giulia"] rect'));
        await settle();
        handle.render(SVG, PROJECT);
        expect(container.querySelectorAll(".kul-query-selected")).toHaveLength(1);
    });
});

describe("an edge is a waypoint, not a dead end", () => {
    it("moves the selection to a person when a marriage panel's spouse row is clicked", async () => {
        const { container, targets } = mount();
        click(container.querySelector('[data-link-kind="marriage"]'));
        await settle();
        expect(rowNames(container, "Spouses")).toEqual([
            "Giulia Rossi",
            "Marco Rossi",
        ]);

        const marco = Array.from(
            container.querySelectorAll(".kul-query-panel-person"),
        ).find((el) => el.textContent === "Marco Rossi");
        click(marco ?? null);
        await settle();

        expect(targets[1]).toEqual([{ kind: "person", id: "marco" }]);
        expect(
            panel(container)?.querySelector(".kul-query-panel-kicker")?.textContent,
        ).toBe("Person");
        expect(
            container.querySelector('[data-person-id="marco"]')?.classList.contains(
                "kul-query-selected",
            ),
        ).toBe(true);
    });

    it("moves the selection to the child from an adoption panel's child row", async () => {
        const { container, targets } = mount();
        click(container.querySelector('[data-link-kind="adoption"]'));
        await settle();
        expect(rowNames(container, "Child")).toEqual(["Dalisay Reyes"]);
        click(container.querySelector(".kul-query-panel-person"));
        await settle();
        expect(targets[1]).toEqual([{ kind: "person", id: "dalisay" }]);
    });
});

describe("reveal-in-editor lives on the panel header", () => {
    it("posts the person's entity id", async () => {
        const { container, reveals } = mount();
        click(container.querySelector('[data-person-id="giulia"] rect'));
        await settle();
        click(container.querySelector(".kul-query-panel-reveal"));
        expect(reveals).toEqual([{ kind: "entity", id: "giulia" }]);
    });

    it("posts the marriage's entity id — the seam now works for edges too", async () => {
        const { container, reveals } = mount();
        click(container.querySelector('[data-link-kind="marriage"]'));
        await settle();
        click(container.querySelector(".kul-query-panel-reveal"));
        expect(reveals).toEqual([{ kind: "entity", id: "m1" }]);
    });

    it("targets the child for an adoption, which has no entity id of its own", async () => {
        const { container, reveals } = mount();
        click(container.querySelector('[data-link-kind="adoption"]'));
        await settle();
        click(container.querySelector(".kul-query-panel-reveal"));
        expect(reveals).toEqual([{ kind: "entity", id: "dalisay" }]);
    });
});

// `createQuerySurface` is the composition root later slices add chrome inside,
// so the two levers they need are exercised at its own seam. `lookup` and
// `applySyncHighlight` are the module's declared inputs, not internal
// collaborators reached around: the second is the shipped `highlightEntity`.
function surfaceHarness(): {
    stage: HTMLElement;
    root: HTMLElement;
    surface: QuerySurface;
    targets: DetailTarget[][];
} {
    const stage = document.createElement("div");
    stage.innerHTML = PREVIEW_BODY_HTML;
    document.body.appendChild(stage);
    const root = stage.querySelector("#root") as HTMLElement;
    root.innerHTML = SVG;
    const targets: DetailTarget[][] = [];
    const surface = createQuerySurface({
        root,
        floatDock: stage.querySelector("#kul-region-float-dock") as HTMLElement,
        floatLayer: stage.querySelector("#kul-region-float") as HTMLElement,
        notifyRegion: stage.querySelector("#kul-region-notify") as HTMLElement,
        adapter: { onRevealRequest: () => {} },
        async lookup(asked) {
            targets.push(asked);
            return {
                ok: true,
                result: asked.map(detailFor),
            } as QueryEnvelope<DetailLookupResult>;
        },
        async runKinQuery() {
            return { ok: true, result: { kind: "count" as const, count: 0 } };
        },
        async resolve() {
            return { ok: true, result: { relationships: [] } };
        },
        locale: createLocaleController(null),
        getPanZoom: () => null,
        applySyncHighlight: (ref) => {
            highlightEntity(root, null, ref);
        },
    });
    return { stage, root, surface, targets };
}

describe("the open panel can be redrawn without re-asking the engine", () => {
    it("refreshes from the answer it is already showing", async () => {
        const { stage, root, surface, targets } = surfaceHarness();
        surface.handleCanvasClick(
            root.querySelector('[data-person-id="giulia"] rect'),
        );
        await settle();
        const before = panel(stage);

        surface.refresh();

        const after = panel(stage);
        expect(after).not.toBe(before);
        expect(after?.querySelector(".kul-query-panel-title")?.textContent).toBe(
            "Giulia Rossi",
        );
        // One lookup, two draws — the lever exists so a locale flip or a
        // re-phrased row costs no engine call.
        expect(targets).toHaveLength(1);
    });

    it("is a no-op with no panel open", () => {
        const { stage, surface } = surfaceHarness();
        expect(() => surface.refresh()).not.toThrow();
        expect(panel(stage)).toBeNull();
    });

    it("is what the locale toggle drives, so a language flip re-reads the panel", async () => {
        // ADR-0041's toggle re-reads everything on screen; the open panel is on
        // screen. Today it redraws identical words — the panel's vocabulary is
        // chrome and stays English (#276 point 6) — so what this pins is the
        // wiring, which is what makes #301's phrased rows a data change.
        const { container, targets } = mount();
        click(container.querySelector('[data-person-id="giulia"] rect'));
        await settle();
        const before = panel(container);
        click(container.querySelector("#kul-locale-toggle"));
        expect(panel(container)).not.toBe(before);
        expect(targets).toHaveLength(1);
    });

    it("resurrects nothing when the locale flips after dispose", async () => {
        // `mountPreview`'s dispose releases the locale subscription. That
        // release has no DOM consequence to assert — dispose closes the panel
        // first, so a leaked subscription would call `refresh()` into a closed
        // panel and change nothing; what it prevents is the controller
        // retaining a dead surface, which is a memory property. What *is*
        // observable, and what this pins, is that a post-dispose flip neither
        // throws nor brings the panel back.
        const { container, handle } = mount();
        click(container.querySelector('[data-person-id="giulia"] rect'));
        await settle();
        const toggle = container.querySelector("#kul-locale-toggle") as HTMLElement;

        handle.dispose();

        expect(() => click(toggle)).not.toThrow();
        expect(panel(container)).toBeNull();
        expect(container.querySelectorAll(".kul-query-selected")).toHaveLength(0);
    });

    it("leaves no query paint or hint behind on dispose", async () => {
        const { container, handle } = mount();
        click(container.querySelector('[data-person-id="giulia"] rect'));
        await settle();
        expect(container.querySelectorAll(".kul-query-selected")).toHaveLength(1);
        expect(container.querySelector(".kul-sync-hint")).not.toBeNull();

        handle.dispose();

        expect(container.querySelectorAll(".kul-query-selected")).toHaveLength(0);
        expect(container.querySelector(".kul-sync-hint")).toBeNull();
        expect(panel(container)).toBeNull();
    });

    it("forgets the answer once the panel closes", async () => {
        const { stage, root, surface } = surfaceHarness();
        surface.handleCanvasClick(
            root.querySelector('[data-person-id="giulia"] rect'),
        );
        await settle();
        surface.clearSelection();
        surface.refresh();
        expect(panel(stage)).toBeNull();
    });
});

describe("sync suspension is a set of reasons, not a selection test", () => {
    it("suspends on a reason of its own, with no selection anywhere", () => {
        const { stage, root, surface } = surfaceHarness();
        surface.setSyncSuspended("filter", true);
        surface.syncHighlight({ id: "giulia", kind: "person" });
        expect(root.querySelectorAll(".kul-selected")).toHaveLength(0);
        expect(
            stage.querySelector("#kul-region-notify .kul-sync-hint"),
        ).not.toBeNull();
    });

    it("keeps sync down while any reason remains, and resumes when the last lifts", async () => {
        const { stage, root, surface } = surfaceHarness();
        surface.setSyncSuspended("filter", true);
        surface.syncHighlight({ id: "giulia", kind: "person" });
        surface.handleCanvasClick(
            root.querySelector('[data-person-id="marco"] rect'),
        );
        await settle();

        // Dropping the selection leaves the other reason holding sync down.
        surface.clearSelection();
        expect(root.querySelectorAll(".kul-selected")).toHaveLength(0);
        expect(
            stage.querySelector("#kul-region-notify .kul-sync-hint"),
        ).not.toBeNull();

        surface.setSyncSuspended("filter", false);
        expect(root.querySelectorAll(".kul-selected")).toHaveLength(1);
        expect(stage.querySelector("#kul-region-notify .kul-sync-hint")).toBeNull();
    });

    it("shows one hint however many reasons are registered", () => {
        const { stage, surface } = surfaceHarness();
        surface.setSyncSuspended("filter", true);
        surface.setSyncSuspended("something-else", true);
        expect(stage.querySelectorAll(".kul-sync-hint")).toHaveLength(1);
    });
});

describe("accessibility is a stated non-goal for new query chrome (ADR-0036)", () => {
    const APP_SHEET = readFileSync(
        join(dirname(fileURLToPath(import.meta.url)), "..", "src", "preview.css"),
        "utf8",
    );

    it("gives the panel and the hint no role and no aria-* attribute", async () => {
        const { container } = mount();
        click(container.querySelector('[data-person-id="giulia"] rect'));
        await settle();
        const chrome = [
            ...container.querySelectorAll(".kul-query-panel, .kul-query-panel *"),
            ...container.querySelectorAll(".kul-sync-hint"),
        ];
        expect(chrome.length).toBeGreaterThan(0);
        for (const el of chrome) {
            const names = el.getAttributeNames();
            expect(names).not.toContain("role");
            expect(names.filter((name) => name.startsWith("aria-"))).toEqual([]);
        }
    });

    it("declares no :focus-visible rule for query chrome", () => {
        const focusRules = APP_SHEET.replace(/\/\*[\s\S]*?\*\//g, "")
            .split("}")
            .map((block) => block.split("{")[0])
            .filter((selector) => selector.includes(":focus-visible"));
        expect(focusRules.length).toBeGreaterThan(0);
        for (const selector of focusRules) {
            expect(selector).not.toContain("kul-query");
            expect(selector).not.toContain("kul-sync-hint");
        }
    });

    it("leaves the chrome that already had accessibility untouched", () => {
        const { container } = mount();
        // Nothing here is licence to break what works: the controls, the legend
        // and the error popover keep every attribute they have.
        expect(
            container.querySelector("#kul-controls")?.getAttribute("role"),
        ).toBe("group");
        expect(
            container.querySelector("#kul-legend")?.getAttribute("aria-label"),
        ).toBe("Diagram legend");
        expect(
            container
                .querySelector('button[data-action="toggle-legend"]')
                ?.getAttribute("aria-pressed"),
        ).toBe("false");
        expect(APP_SHEET).toContain(".kul-control-btn:focus-visible");
    });
});

describe("editor sync suspends while a selection exists", () => {
    it("paints the sync highlight while nothing is selected", () => {
        const { container, handle } = mount();
        handle.highlightEntity({ id: "giulia", kind: "person" });
        expect(container.querySelectorAll(".kul-selected")).toHaveLength(1);
    });

    it("holds an inbound highlight arriving after the selection", async () => {
        const { container, handle } = mount();
        click(container.querySelector('[data-person-id="giulia"] rect'));
        await settle();
        handle.highlightEntity({ id: "marco", kind: "person" });
        expect(container.querySelectorAll(".kul-selected")).toHaveLength(0);
        expect(container.querySelectorAll(".kul-query-selected")).toHaveLength(1);
    });

    it("strips a highlight that was already painted when the selection takes over", async () => {
        // The normal VSCode path: the editor cursor is almost always on some
        // entity, so a sync highlight is usually already up when the reader
        // clicks. Suspending has to strip it, not merely stop painting.
        const { container, handle } = mount();
        handle.highlightEntity({ id: "giulia", kind: "person" });
        expect(container.querySelectorAll(".kul-selected")).toHaveLength(1);
        click(container.querySelector('[data-person-id="marco"] rect'));
        await settle();
        expect(container.querySelectorAll(".kul-selected")).toHaveLength(0);
        expect(container.querySelectorAll(".kul-query-selected")).toHaveLength(1);
    });

    it("replays the already-painted highlight when Esc resumes sync", async () => {
        const { container, handle } = mount();
        handle.highlightEntity({ id: "giulia", kind: "person" });
        click(container.querySelector('[data-person-id="marco"] rect'));
        await settle();
        window.dispatchEvent(new KeyboardEvent("keydown", { key: "Escape" }));
        expect(
            container.querySelector('[data-person-id="giulia"]')?.classList.contains(
                "kul-selected",
            ),
        ).toBe(true);
        expect(container.querySelectorAll(".kul-query-selected")).toHaveLength(0);
    });

    it("shows the hint in the notify region for exactly as long as a selection exists", async () => {
        const { container } = mount();
        const notify = container.querySelector("#kul-region-notify") as HTMLElement;
        expect(notify.querySelector(".kul-sync-hint")).toBeNull();
        click(container.querySelector('[data-person-id="giulia"] rect'));
        await settle();
        expect(notify.querySelector(".kul-sync-hint")?.textContent).toBe(
            SYNC_SUSPENDED_HINT,
        );
        window.dispatchEvent(new KeyboardEvent("keydown", { key: "Escape" }));
        expect(notify.querySelector(".kul-sync-hint")).toBeNull();
    });

    it("replays the held highlight when Esc resumes sync", async () => {
        const { container, handle } = mount();
        click(container.querySelector('[data-person-id="giulia"] rect'));
        await settle();
        handle.highlightEntity({ id: "marco", kind: "person" });
        window.dispatchEvent(new KeyboardEvent("keydown", { key: "Escape" }));
        expect(
            container.querySelector('[data-person-id="marco"]')?.classList.contains(
                "kul-selected",
            ),
        ).toBe(true);
    });
});
