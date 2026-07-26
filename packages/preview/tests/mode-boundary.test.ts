// The mode boundary: **an edit ends query mode** (ADR-0035, ADR-0046).
//
// Driven through the mounted preview rather than through `QuerySurface`
// directly, because the rule is composed of two things this suite must not
// take on trust: that `mount.ts`'s render path still reaches the one
// post-render hook, and that the hook lets go of every surface at once. A test
// that called `endQueryMode()` by hand would prove the second and assume the
// first — and the first is the wire an edit actually travels.
//
// The engine is substituted at its one system boundary (`CODING_STANDARDS.md`
// — mock at system boundaries, never internal collaborators); the double
// answers from a fixed table and re-derives no kinship or predicate.

import { afterEach, describe, expect, it, vi } from "vitest";

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

import { DIM_CLASS } from "../src/dim.js";
import type {
    DetailTarget,
    EntityDetail,
    ExportedPerson,
    Member,
    Query,
} from "../src/engine-wire.js";
import type { ProjectSnapshot, QueryEngine } from "../src/engine.js";
import {
    FILTER_MATCH_CLASS,
    FILTER_UNCERTAIN_CLASS,
} from "../src/filter-paint.js";
import { KIN_SETS } from "../src/kin-sets.js";
import { RESULT_CLASS } from "../src/kin-paint.js";
import { mountPreview } from "../src/mount.js";
import { SELECTION_CLASS } from "../src/selection.js";
import type { HostAdapter, PreviewHandle } from "../src/types.js";

const PROJECT: ProjectSnapshot = {
    files: [{ name: "family.kul", source: "" }],
    manifest: { kul: "0.1" },
};

/**
 * Giuseppe is the anchor; Giulia and Marco are his siblings and the kin paint's
 * answer; Aldo is neither, so kin paint dims him and the filter cannot judge
 * him — which is what puts all three states on screen at once.
 */
const SVG = `<svg xmlns="http://www.w3.org/2000/svg">
  <g class="kul-card" data-person-id="giuseppe" data-kind="canonical" data-gender="male"><rect/></g>
  <g class="kul-card" data-person-id="giulia" data-kind="canonical" data-gender="female"><rect/></g>
  <g class="kul-card" data-person-id="marco" data-kind="canonical" data-gender="male"><rect/></g>
  <g class="kul-card" data-person-id="aldo" data-kind="canonical" data-gender="male"><rect/></g>
</svg>`;

const PEOPLE: Record<string, ExportedPerson> = {
    giuseppe: { id: "giuseppe", name: "Giuseppe Rossi", family: "Rossi", gender: "male" },
    giulia: { id: "giulia", name: "Giulia Rossi", family: "Rossi", gender: "female" },
    marco: { id: "marco", name: "Marco Rossi", family: "Rossi", gender: "male" },
    // No `family` recorded, so `family = Rossi` is the engine's `unknown`.
    aldo: { id: "aldo", name: "Aldo", gender: "male" },
};

/** A full-sibling descriptor — what `queryKin` hands back per member. */
function siblingOf(alterId: string, alterGender: "male" | "female"): Member {
    return {
        personId: alterId,
        descriptor: {
            egoId: "giuseppe",
            alterId,
            egoGender: "male",
            alterGender,
            classification: {
                kind: "collateral",
                up: 1,
                down: 1,
                cousinDegree: 0,
                removed: 0,
            },
            edgeNature: "blood",
            affinity: "blood",
            sharing: "full",
            side: "both",
            seniority: "unknown",
            apexSeniority: "unknown",
            path: [
                { step: "up", to: "aldo", gender: "male", edge: "bio" },
                { step: "down", to: alterId, gender: alterGender, edge: "bio" },
            ],
        },
    };
}

const SIBLINGS = [siblingOf("giulia", "female"), siblingOf("marco", "male")];

/** Which catalogue row a Query value came from, by its pattern. */
function setIdOf(query: Query): string | undefined {
    if (query.source.kind !== "kinOf") {
        return undefined;
    }
    const pattern = JSON.stringify(query.source.pattern);
    return KIN_SETS.find((set) => JSON.stringify(set.pattern) === pattern)?.id;
}

function detailFor(target: DetailTarget): EntityDetail | null {
    if (target.kind !== "person" || !PEOPLE[target.id]) {
        return null;
    }
    return {
        kind: "person",
        person: PEOPLE[target.id],
        parents: [],
        marriages: [],
        children: [],
    };
}

const engine: QueryEngine = {
    async queryDetail(_project, targets) {
        return { ok: true, result: targets.map(detailFor) };
    },
    async queryKin(_project, query) {
        const members = setIdOf(query) === "siblings" ? SIBLINGS : [];
        return query.projection === "count"
            ? { ok: true, result: { kind: "count", count: members.length } }
            : { ok: true, result: { kind: "members", members } };
    },
    async runQuery(_project, query) {
        // Giulia matches `family = Rossi`; Aldo has no family recorded, so he
        // is the engine's `unknown` and the only amber card.
        return {
            ok: true,
            result: {
                kind: "personIds",
                personIds:
                    query.mode === "includeUncertain"
                        ? ["giulia", "aldo"]
                        : ["giulia"],
            },
        };
    },
    async queryResolve() {
        return { ok: true, result: { relationships: [] } };
    },
    get isLoaded() {
        return true;
    },
};

function mount(): { container: HTMLElement; handle: PreviewHandle } {
    const container = document.createElement("div");
    document.body.appendChild(container);
    const adapter: HostAdapter = { onRevealRequest: () => {} };
    const handle = mountPreview(container, adapter, { engine });
    handle.render(SVG, PROJECT);
    return { container, handle };
}

function click(el: Element | null): void {
    (el as Element).dispatchEvent(new MouseEvent("click", { bubbles: true }));
}

function change(el: Element | null, value: string): void {
    (el as HTMLInputElement | HTMLSelectElement).value = value;
    (el as HTMLElement).dispatchEvent(new Event("change", { bubbles: true }));
}

function settle(): Promise<void> {
    return new Promise((resolve) => setTimeout(resolve, 0));
}

function panel(container: HTMLElement): Element | null {
    return container.querySelector(".kul-query-panel");
}

function tally(container: HTMLElement): string | null {
    return container.querySelector(".kul-filter-tally")?.textContent ?? null;
}

/** The conditions the sentence currently spells — one label per `✕` chip. */
function conditions(container: HTMLElement): string[] {
    return Array.from(
        container.querySelectorAll("#kul-filter-bar .kul-filter-chip"),
    )
        .filter((chip) => chip.querySelector(".kul-filter-chip-remove"))
        .map((chip) => chip.querySelector(".kul-filter-chip-label")?.textContent ?? "");
}

function hint(container: HTMLElement): Element | null {
    return container.querySelector("#kul-region-notify .kul-sync-hint");
}

/**
 * Put all three query states on screen at once, plus a held editor-sync
 * highlight, and assert every one of them is really there before the boundary
 * is exercised. The assertions in here are load-bearing: without them a test
 * that "clears" would pass against a surface that never painted anything.
 */
async function queryingHard(): Promise<{
    container: HTMLElement;
    handle: PreviewHandle;
}> {
    const { container, handle } = mount();

    // The editor cursor is on Marco. Sync is live, so it paints.
    handle.highlightEntity({ id: "marco", kind: "person" });
    expect(container.querySelectorAll(".kul-selected")).toHaveLength(1);

    // A selection: panel, and sync suspends (stripping the sync highlight).
    click(container.querySelector('[data-person-id="giuseppe"] rect'));
    await settle();
    expect(panel(container)).not.toBeNull();
    expect(container.querySelectorAll("." + SELECTION_CLASS)).toHaveLength(1);
    expect(container.querySelectorAll(".kul-selected")).toHaveLength(0);

    // A painted kin set: Giulia and Marco teal, Aldo dimmed.
    click(container.querySelector(".kul-kin-header"));
    await settle();
    const row = Array.from(container.querySelectorAll(".kul-kin-row")).find(
        (candidate) =>
            candidate.querySelector(".kul-kin-label")?.textContent === "Siblings",
    );
    click(row ?? null);
    await settle();
    expect(container.querySelectorAll("." + RESULT_CLASS)).toHaveLength(2);
    expect(container.querySelectorAll("." + DIM_CLASS).length).toBeGreaterThan(0);

    // An active filter: `family = Rossi`, with Aldo as the can't-say.
    const bar = container.querySelector("#kul-filter-bar") as HTMLElement;
    click(bar.querySelector(".kul-filter-chip-add .kul-filter-chip-label"));
    change(
        (bar.querySelector(".kul-filter-editor") as HTMLElement).querySelector("input"),
        "Rossi",
    );
    await settle();
    expect(container.querySelectorAll("." + FILTER_MATCH_CLASS).length).toBeGreaterThan(0);
    expect(container.querySelectorAll("." + FILTER_UNCERTAIN_CLASS)).toHaveLength(1);
    expect(tally(container)).not.toBeNull();
    expect(conditions(container)).toEqual(["family = Rossi"]);
    expect(hint(container)).not.toBeNull();

    return { container, handle };
}

describe("an edit ends query mode", () => {
    afterEach(() => {
        document.body.innerHTML = "";
    });

    it("clears the selection, the kin paint and the filter in one render", async () => {
        const { container, handle } = await queryingHard();

        // The edit. A render is how one reaches the webview (ADR-0046).
        handle.render(SVG, PROJECT);
        await settle();

        // The selection and everything hanging off it.
        expect(container.querySelectorAll("." + SELECTION_CLASS)).toHaveLength(0);
        expect(panel(container)).toBeNull();

        // The kin paint, and the dim it published.
        expect(container.querySelectorAll("." + RESULT_CLASS)).toHaveLength(0);
        expect(container.querySelectorAll("." + DIM_CLASS)).toHaveLength(0);

        // The filter: paint, badge, tally and the sentence itself.
        expect(container.querySelectorAll("." + FILTER_MATCH_CLASS)).toHaveLength(0);
        expect(container.querySelectorAll("." + FILTER_UNCERTAIN_CLASS)).toHaveLength(0);
        expect(container.querySelectorAll(".kul-filter-uncertain-badge")).toHaveLength(0);
        expect(tally(container)).toBeNull();
        expect(conditions(container)).toEqual([]);
    });

    it("resumes editor sync on its own, with no gesture from the reader", async () => {
        const { container, handle } = await queryingHard();

        handle.render(SVG, PROJECT);
        await settle();

        // Both suspension reasons lifted together, so the hint comes down and
        // the highlight the editor last asked for is replayed. Nobody pressed
        // Esc: that is what "the mode boundary costs no gesture" means.
        expect(hint(container)).toBeNull();
        expect(
            container.querySelector('[data-person-id="marco"]')?.classList.contains(
                "kul-selected",
            ),
        ).toBe(true);
    });

    it("takes only query state, leaving the reader's standing choices alone", async () => {
        // Ending query mode is not a reset of the preview. The Explore-kin
        // list's open/closed state belongs to the reader and outlives every
        // anchor (ADR-0043), exactly as it does on Esc and on a canvas click —
        // so the next selection opens straight back onto the list.
        const { container, handle } = await queryingHard();
        handle.render(SVG, PROJECT);
        await settle();

        click(container.querySelector('[data-person-id="giuseppe"] rect'));
        await settle();
        expect(container.querySelectorAll(".kul-kin-row").length).toBe(
            KIN_SETS.length,
        );
    });

    it("is the same exit Esc is", async () => {
        // Two ways out of query mode, one behaviour — which is why they are
        // one function. If they diverged, "Esc to resume" and an edit would
        // leave the reader in two different places.
        const { container } = await queryingHard();
        window.dispatchEvent(
            new KeyboardEvent("keydown", { key: "Escape", bubbles: true }),
        );
        await settle();

        expect(container.querySelectorAll("." + SELECTION_CLASS)).toHaveLength(0);
        expect(panel(container)).toBeNull();
        expect(container.querySelectorAll("." + RESULT_CLASS)).toHaveLength(0);
        expect(container.querySelectorAll("." + DIM_CLASS)).toHaveLength(0);
        expect(container.querySelectorAll("." + FILTER_MATCH_CLASS)).toHaveLength(0);
        expect(tally(container)).toBeNull();
        expect(hint(container)).toBeNull();
    });

    it("does not re-ask the filter against the project the render brought", async () => {
        // The rejected alternative, asserted as an absence: refetching per
        // render was measured as affordable and turned down, because no query
        // artefact outlives the source it was computed from (ADR-0035). If the
        // hook re-asked, this render would cost two more engine calls.
        const asked: Query[] = [];
        const container = document.createElement("div");
        document.body.appendChild(container);
        const handle = mountPreview(
            container,
            { onRevealRequest: () => {} },
            {
                engine: {
                    ...engine,
                    async runQuery(project, query) {
                        asked.push(query);
                        return engine.runQuery(project, query);
                    },
                },
            },
        );
        handle.render(SVG, PROJECT);

        const bar = container.querySelector("#kul-filter-bar") as HTMLElement;
        click(bar.querySelector(".kul-filter-chip-add .kul-filter-chip-label"));
        change(
            (bar.querySelector(".kul-filter-editor") as HTMLElement).querySelector(
                "input",
            ),
            "Rossi",
        );
        await settle();
        // Both certainty questions, once — the difference between them is the
        // unjudgeable set (ADR-0045).
        expect(asked).toHaveLength(2);

        handle.render(SVG, PROJECT);
        await settle();
        expect(asked).toHaveLength(2);
    });
});
