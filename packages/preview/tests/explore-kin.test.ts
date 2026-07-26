// "Explore kin", driven the way a reader drives it: through the mounted
// chrome, with the engine substituted at its one system boundary
// (`CODING_STANDARDS.md` — mock at system boundaries, never internal
// collaborators). The engine double records every question it was asked, which
// is how the cost claims below are asserted rather than eyeballed.

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

import type {
    DetailTarget,
    EntityDetail,
    Member,
    Query,
} from "../src/engine-wire.js";
import type { ProjectSnapshot, QueryEngine } from "../src/engine.js";
import { KIN_SETS } from "../src/kin-sets.js";
import { DIM_CLASS } from "../src/dim.js";
import { RESULT_CLASS } from "../src/kin-paint.js";
import { SELECTION_CLASS } from "../src/selection.js";
import { mountPreview } from "../src/mount.js";
import type { HostAdapter, PreviewHandle } from "../src/types.js";

const PROJECT: ProjectSnapshot = {
    files: [{ name: "family.kul", source: "" }],
    manifest: { kul: "0.1" },
};

/**
 * Giuseppe is the anchor. Elena owns three cards — canonical plus two
 * past-intimacy ghosts — and is the multi-card member. Marco is nobody's kin
 * here and is the card that must dim.
 */
const SVG = `<svg xmlns="http://www.w3.org/2000/svg">
  <g class="kul-card" data-person-id="giuseppe" data-kind="canonical" data-gender="male"><rect/></g>
  <g class="kul-card" data-person-id="elena" data-kind="canonical" data-gender="female"><rect/></g>
  <g class="kul-card" data-person-id="elena" data-kind="ghost" data-ghost-reason="past-birth" data-gender="female"><rect/></g>
  <g class="kul-card" data-person-id="elena" data-kind="ghost" data-ghost-reason="past-marriage" data-gender="female"><rect/></g>
  <g class="kul-card" data-person-id="marco" data-kind="canonical" data-gender="male"><rect/></g>
  <path class="kul-edge" data-link-kind="marriage" data-marriage-id="m1"/>
</svg>`;

const GIUSEPPE = { id: "giuseppe", name: "Giuseppe Rossi", gender: "male" };
const MARCO = { id: "marco", name: "Marco Rossi", gender: "male" };

/** A full-sibling descriptor: what `queryKin` hands back per member. */
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

interface Asked {
    details: DetailTarget[][];
    kin: Query[];
}

/** The batched lookup's answer for one target. */
function detailFor(target: DetailTarget): EntityDetail | null {
    if (target.kind === "marriage") {
        return {
            kind: "marriage",
            marriage: { id: "m1", spouses: ["giuseppe", "marco"] },
            spouses: [GIUSEPPE, MARCO],
            children: [],
        };
    }
    if (target.kind !== "person") {
        return null;
    }
    const person = target.id === "giuseppe" ? GIUSEPPE : MARCO;
    return {
        kind: "person",
        person,
        parents: [],
        marriages: [],
        children: [],
    };
}

/**
 * A latch the test opens by hand. Holding the count sweep is the only way to
 * exercise the window ADR-0043 designs for — rows are clickable before their
 * counts land — which is the window the sweep must survive.
 */
function gate(): { wait(): Promise<void>; open(): void; held: boolean } {
    let release: () => void = () => {};
    const opened = new Promise<void>((resolve) => {
        release = resolve;
    });
    const self = {
        held: true,
        wait: () => (self.held ? opened : Promise.resolve()),
        open() {
            self.held = false;
            release();
        },
    };
    return self;
}

/**
 * Engine double. `members` answers come from `membersBySet`; any set without an
 * entry is genuinely empty, which is what the honest-emptiness case needs. The
 * `count` projection answers the same set's size, so a count and a paint can
 * never disagree in these tests for the wrong reason.
 */
function fakeEngine(
    membersBySet: Record<string, Member[]>,
    counts: ReturnType<typeof gate>,
): {
    engine: QueryEngine;
    asked: Asked;
} {
    const asked: Asked = { details: [], kin: [] };
    return {
        asked,
        engine: {
            async queryDetail(_project, targets) {
                asked.details.push(targets);
                return { ok: true, result: targets.map(detailFor) };
            },
            async queryKin(_project, query) {
                asked.kin.push(query);
                if (query.projection === "count") {
                    await counts.wait();
                }
                const members = membersFor(query, membersBySet);
                return query.projection === "count"
                    ? { ok: true, result: { kind: "count", count: members.length } }
                    : { ok: true, result: { kind: "members", members } };
            },
            get isLoaded() {
                return true;
            },
        },
    };
}

/** Which catalogue row a Query value came from, by its pattern. */
function setIdOf(query: Query): string | undefined {
    if (query.source.kind !== "kinOf") {
        return undefined;
    }
    const pattern = JSON.stringify(query.source.pattern);
    return KIN_SETS.find((set) => JSON.stringify(set.pattern) === pattern)?.id;
}

function membersFor(query: Query, table: Record<string, Member[]>): Member[] {
    const id = setIdOf(query);
    return (id && table[id]) || [];
}

function mount(
    membersBySet: Record<string, Member[]> = {},
    options: { holdCounts?: boolean } = {},
): {
    container: HTMLElement;
    handle: PreviewHandle;
    asked: Asked;
    counts: ReturnType<typeof gate>;
} {
    const container = document.createElement("div");
    document.body.appendChild(container);
    const adapter: HostAdapter = { onRevealRequest: () => {} };
    const counts = gate();
    if (!options.holdCounts) {
        counts.open();
    }
    const { engine, asked } = fakeEngine(membersBySet, counts);
    const handle = mountPreview(container, adapter, { engine });
    handle.render(SVG, PROJECT);
    return { container, handle, asked, counts };
}

function click(el: Element | null): void {
    (el as Element).dispatchEvent(new MouseEvent("click", { bubbles: true }));
}

/** Let every in-flight engine answer resolve and the panel redraw. */
function settle(): Promise<void> {
    return new Promise((resolve) => setTimeout(resolve, 0));
}

function kinRows(container: HTMLElement): HTMLElement[] {
    return Array.from(container.querySelectorAll(".kul-kin-row"));
}

function counts(container: HTMLElement): string[] {
    return kinRows(container).map((row) => rowText(row).count);
}

function rowText(row: HTMLElement): { label: string; terms: string; count: string } {
    return {
        label: row.querySelector(".kul-kin-label")?.textContent ?? "",
        terms: row.querySelector(".kul-kin-terms")?.textContent ?? "",
        count: row.querySelector(".kul-kin-count")?.textContent ?? "",
    };
}

function rowFor(container: HTMLElement, label: string): HTMLElement {
    const row = kinRows(container).find(
        (candidate) => rowText(candidate).label === label,
    );
    if (!row) {
        throw new Error(`no kin row labelled ${label}`);
    }
    return row;
}

/** Select Giuseppe and open the Explore-kin list. */
async function explore(container: HTMLElement): Promise<void> {
    click(container.querySelector('[data-person-id="giuseppe"] rect'));
    await settle();
    click(container.querySelector(".kul-kin-header"));
    await settle();
}

beforeEach(() => {
    document.body.innerHTML = "";
});

afterEach(() => {
    document.body.innerHTML = "";
});

describe("the Explore-kin list", () => {
    it("is one flat list of every kin set, in one order, with no grouping", async () => {
        const { container } = mount();
        await explore(container);
        expect(kinRows(container).map((row) => rowText(row).label)).toEqual(
            KIN_SETS.map((set) => set.label),
        );
        // One list element, not one per category.
        expect(container.querySelectorAll(".kul-kin-list")).toHaveLength(1);
    });

    it("shows the engine's count projection for every row", async () => {
        const { container, asked } = mount({
            siblings: [siblingOf("elena", "female"), siblingOf("marco", "male")],
        });
        await explore(container);
        expect(rowText(rowFor(container, "Siblings")).count).toBe("2");
        // Every count on screen came from a `count`-projected Query value the
        // engine evaluated — never a length this chrome derived.
        const counted = asked.kin.filter((query) => query.projection === "count");
        expect(counted).toHaveLength(KIN_SETS.length);
        expect(asked.kin.filter((q) => q.projection === "members")).toHaveLength(0);
    });

    it("stays open as the selection walks, and re-asks about the new anchor", async () => {
        const { container, asked } = mount();
        await explore(container);
        const before = asked.kin.filter((q) => q.projection === "count").length;

        click(container.querySelector('[data-person-id="marco"] rect'));
        await settle();
        // Still open — the reader opened it, and the list is a property of the
        // reader, not of the selection. The counts are Marco's, not Giuseppe's.
        expect(kinRows(container).map((row) => rowText(row).label)).toEqual(
            KIN_SETS.map((set) => set.label),
        );
        const after = asked.kin.filter((q) => q.projection === "count");
        expect(after).toHaveLength(before + KIN_SETS.length);
        expect(
            after.slice(before).every(
                (q) => q.source.kind === "kinOf" && q.source.anchor === "marco",
            ),
        ).toBe(true);
    });

    it("offers no list on an edge selection — an edge is a waypoint", async () => {
        const { container } = mount();
        await explore(container);
        expect(kinRows(container)).not.toHaveLength(0);
        click(container.querySelector('[data-link-kind="marriage"]'));
        await settle();
        // The marriage panel is open, so this is the list's absence and not the
        // panel's: a kin set needs a person anchor (ADR-0035).
        expect(container.querySelector(".kul-query-panel-kicker")?.textContent).toBe(
            "Marriage",
        );
        expect(kinRows(container)).toHaveLength(0);
        expect(container.querySelector(".kul-kin-header")).toBeNull();
    });
});

describe("clicking a row paints the answer on the tree", () => {
    it("lights every card a matched person owns, ghosts included", async () => {
        const { container } = mount({
            siblings: [siblingOf("elena", "female")],
        });
        await explore(container);
        click(rowFor(container, "Siblings"));
        await settle();
        const lit = Array.from(container.querySelectorAll("." + RESULT_CLASS));
        expect(lit).toHaveLength(3);
        expect(lit.map((node) => node.getAttribute("data-kind")).sort()).toEqual([
            "canonical",
            "ghost",
            "ghost",
        ]);
    });

    it("dims the cards outside the answer and leaves the anchor lit", async () => {
        const { container } = mount({
            siblings: [siblingOf("elena", "female")],
        });
        await explore(container);
        click(rowFor(container, "Siblings"));
        await settle();
        const dimmed = Array.from(container.querySelectorAll("." + DIM_CLASS)).map(
            (node) => node.getAttribute("data-person-id"),
        );
        expect(dimmed).toEqual(["marco"]);
        const anchor = container.querySelector(
            '[data-person-id="giuseppe"]',
        ) as Element;
        expect(anchor.classList.contains(DIM_CLASS)).toBe(false);
        expect(anchor.classList.contains(SELECTION_CLASS)).toBe(true);
    });

    it("survives a render, because the SVG every paint was on has been replaced", async () => {
        const { container, handle } = mount({
            siblings: [siblingOf("elena", "female")],
        });
        await explore(container);
        click(rowFor(container, "Siblings"));
        await settle();
        handle.render(SVG, PROJECT);
        expect(container.querySelectorAll("." + RESULT_CLASS)).toHaveLength(3);
        expect(container.querySelectorAll("." + DIM_CLASS)).toHaveLength(1);
        expect(container.querySelectorAll("." + SELECTION_CLASS)).toHaveLength(1);
    });

    it("drops the paint when the selection moves — an answer is about one anchor", async () => {
        const { container } = mount({
            siblings: [siblingOf("elena", "female")],
        });
        await explore(container);
        click(rowFor(container, "Siblings"));
        await settle();
        click(container.querySelector("svg"));
        await settle();
        expect(container.querySelectorAll("." + RESULT_CLASS)).toHaveLength(0);
        expect(container.querySelectorAll("." + DIM_CLASS)).toHaveLength(0);
    });
});

describe("an empty kin set is an answer, not a failure", () => {
    it("renders a plain zero, toasts quietly, and paints nothing", async () => {
        const { container } = mount({ siblings: [] });
        await explore(container);
        expect(rowText(rowFor(container, "First cousins")).count).toBe("0");

        click(rowFor(container, "First cousins"));
        await settle();

        // Scoped to the notify region: the toast joins a region by being
        // appended, and landing anywhere else would be a widget inventing its
        // own placement (ADR-0038).
        const toast = container.querySelector("#kul-region-notify > .kul-toast");
        expect(toast?.textContent).toBe(
            "No first cousins recorded for Giuseppe Rossi. " +
                "Nothing is guessed — absence stays absence.",
        );
        // Named from the detail answer the panel already holds — no second
        // lookup was made to learn the anchor's name.
        expect(container.querySelectorAll("." + RESULT_CLASS)).toHaveLength(0);
    });

    it("is not modal and not an error: no popover, no blocked next question", async () => {
        const { container } = mount({
            siblings: [siblingOf("elena", "female")],
        });
        await explore(container);
        click(rowFor(container, "First cousins"));
        await settle();
        const popover = container.querySelector("#kul-error-popover") as HTMLElement;
        expect(popover.hidden).toBe(true);
        expect(popover.textContent).toBe("");

        click(rowFor(container, "Siblings"));
        await settle();
        expect(container.querySelectorAll("." + RESULT_CLASS)).toHaveLength(3);
        // One absence at a time: the answered question replaces the toast.
        expect(container.querySelector("#kul-region-notify > .kul-toast")).toBeNull();
    });
});

describe("the count sweep survives whatever the reader does while it is in flight", () => {
    it("still lands when a row is painted before it resolves", async () => {
        // ADR-0043 makes rows clickable before their counts land and prices the
        // sweep at ≈215 ms, so this window is the designed path, not an edge
        // case. Sharing one generation counter between the sweep and the paint
        // threw the whole list away here.
        const { container, counts: gated } = mount(
            { siblings: [siblingOf("elena", "female")] },
            { holdCounts: true },
        );
        await explore(container);
        expect(counts(container).every((count) => count === "·")).toBe(true);

        click(rowFor(container, "Siblings"));
        await settle();
        expect(container.querySelectorAll("." + RESULT_CLASS)).toHaveLength(3);

        gated.open();
        await settle();
        expect(counts(container)).not.toContain("·");
        expect(rowText(rowFor(container, "Siblings")).count).toBe("1");
        // …and the paint the reader made in the meantime is still up.
        expect(container.querySelectorAll("." + RESULT_CLASS)).toHaveLength(3);
    });

    it("is discarded when the anchor moved out from under it", async () => {
        const { container, counts: gated, asked } = mount({}, { holdCounts: true });
        await explore(container);
        click(container.querySelector('[data-person-id="marco"] rect'));
        await settle();
        gated.open();
        await settle();
        // Two sweeps were issued and both resolved; the counts on screen are
        // the second anchor's, and the first anchor's answers were dropped.
        const anchors = asked.kin
            .filter((query) => query.projection === "count")
            .map((query) => (query.source.kind === "kinOf" ? query.source.anchor : ""));
        expect(new Set(anchors)).toEqual(new Set(["giuseppe", "marco"]));
        expect(counts(container)).not.toContain("·");
    });
});

describe("a row the reader has not been given a count for shows no number", () => {
    it("renders a placeholder, never a zero", async () => {
        const { container } = mount({}, { holdCounts: true });
        await explore(container);
        expect(kinRows(container)).toHaveLength(KIN_SETS.length);
        expect(counts(container)).toEqual(KIN_SETS.map(() => "·"));
        expect(counts(container)).not.toContain("0");
    });
});

describe("letting go of a painted answer", () => {
    it("clears the paint when the reader closes the list", async () => {
        // The list is the only thing on screen that says which question the
        // teal answers, so a closed list must not leave one up (ADR-0043).
        const { container } = mount({ siblings: [siblingOf("elena", "female")] });
        await explore(container);
        click(rowFor(container, "Siblings"));
        await settle();
        expect(container.querySelectorAll("." + RESULT_CLASS)).toHaveLength(3);

        click(container.querySelector(".kul-kin-header"));
        await settle();
        expect(kinRows(container)).toHaveLength(0);
        expect(container.querySelectorAll("." + RESULT_CLASS)).toHaveLength(0);
        expect(container.querySelectorAll("." + DIM_CLASS)).toHaveLength(0);
        // The selection is untouched: closing the list is not clearing query mode.
        expect(container.querySelectorAll("." + SELECTION_CLASS)).toHaveLength(1);
    });

    it("clears the paint when the reader re-clicks the painted row", async () => {
        const { container, asked } = mount({
            siblings: [siblingOf("elena", "female")],
        });
        await explore(container);
        click(rowFor(container, "Siblings"));
        await settle();
        const membersAsked = asked.kin.filter((q) => q.projection === "members").length;

        click(rowFor(container, "Siblings"));
        await settle();
        expect(container.querySelectorAll("." + RESULT_CLASS)).toHaveLength(0);
        expect(container.querySelectorAll("." + DIM_CLASS)).toHaveLength(0);
        expect(container.querySelectorAll("." + SELECTION_CLASS)).toHaveLength(1);
        // Letting go asks the engine nothing.
        expect(asked.kin.filter((q) => q.projection === "members")).toHaveLength(
            membersAsked,
        );
        // And the counts are untouched, so the list is still answering.
        expect(rowText(rowFor(container, "Siblings")).count).toBe("1");
    });

    it("keeps the counts when the list is closed and reopened", async () => {
        const { container, asked } = mount({
            siblings: [siblingOf("elena", "female")],
        });
        await explore(container);
        const swept = asked.kin.filter((q) => q.projection === "count").length;
        click(container.querySelector(".kul-kin-header"));
        await settle();
        click(container.querySelector(".kul-kin-header"));
        await settle();
        expect(rowText(rowFor(container, "Siblings")).count).toBe("1");
        expect(asked.kin.filter((q) => q.projection === "count")).toHaveLength(swept);
    });
});

describe("what opening the list costs", () => {
    it("adds no batched detail lookup at all, however many rows it has", async () => {
        const { container, asked } = mount();
        click(container.querySelector('[data-person-id="giuseppe"] rect'));
        await settle();
        const afterSelect = asked.details.length;
        expect(afterSelect).toBe(1);

        click(container.querySelector(".kul-kin-header"));
        await settle();
        // Thirteen rows, and the batched operation was not asked again: row
        // labels are chrome and the anchor's display name is the answer the
        // panel is already holding (ADR-0037, ADR-0043).
        expect(kinRows(container)).toHaveLength(KIN_SETS.length);
        expect(asked.details).toHaveLength(afterSelect);
    });

    it("costs one members query per painted row, whatever the answer's size", async () => {
        const many = Array.from({ length: 200 }, (_, i) =>
            siblingOf(`p${i}`, i % 2 === 0 ? "male" : "female"),
        );
        const { container, asked } = mount({ descendants: many });
        await explore(container);
        const before = asked.kin.filter((q) => q.projection === "members").length;
        click(rowFor(container, "Descendants"));
        await settle();
        const after = asked.kin.filter((q) => q.projection === "members");
        expect(after).toHaveLength(before + 1);
        expect(asked.details).toHaveLength(1);
    });

    it("memoizes the sweep, so re-opening the list re-asks nothing", async () => {
        const { container, asked } = mount();
        await explore(container);
        const counted = asked.kin.filter((q) => q.projection === "count").length;
        click(container.querySelector(".kul-kin-header"));
        click(container.querySelector(".kul-kin-header"));
        await settle();
        expect(asked.kin.filter((q) => q.projection === "count")).toHaveLength(
            counted,
        );
    });
});

describe("phrased row labels follow the locale", () => {
    it("glosses the painted row with its members' terms and re-phrases on toggle", async () => {
        const { container, handle } = mount({
            siblings: [siblingOf("elena", "female"), siblingOf("marco", "male")],
        });
        await explore(container);
        click(rowFor(container, "Siblings"));
        await settle();
        expect(rowText(rowFor(container, "Siblings")).terms).toBe("sister · brother");

        // The toggle's existing `refresh()` subscription is the whole wire —
        // no second subscription, no re-query.
        const before = handle.locale.pack().code;
        handle.locale.select("gu");
        expect(handle.locale.pack().code).not.toBe(before);
        expect(rowText(rowFor(container, "Siblings")).terms).toBe("બહેન · ભાઈ");
        expect(
            rowFor(container, "Siblings").querySelector(".kul-kin-terms")?.getAttribute("lang"),
        ).toBe("gu");
    });

    it("glosses only the painted row — an unasked set has nothing to phrase", async () => {
        const { container } = mount({
            siblings: [siblingOf("elena", "female")],
        });
        await explore(container);
        click(rowFor(container, "Siblings"));
        await settle();
        const glossed = kinRows(container).filter(
            (row) => rowText(row).terms !== "",
        );
        expect(glossed.map((row) => rowText(row).label)).toEqual(["Siblings"]);
    });

    it("keeps the row's own name in English — a label is chrome, a term is not", async () => {
        const { container, handle } = mount();
        await explore(container);
        handle.locale.select("gu");
        expect(kinRows(container).map((row) => rowText(row).label)).toEqual(
            KIN_SETS.map((set) => set.label),
        );
    });
});

describe("kin paint and the editor-sync highlight never co-paint", () => {
    it("strips an inbound sync highlight that arrives while a set is painted", async () => {
        const { container, handle } = mount({
            siblings: [siblingOf("elena", "female")],
        });
        await explore(container);
        click(rowFor(container, "Siblings"));
        await settle();
        handle.highlightEntity({ id: "marco", kind: "person" });
        expect(container.querySelectorAll(".kul-selected")).toHaveLength(0);
        expect(container.querySelectorAll("." + RESULT_CLASS)).toHaveLength(3);
    });

    it("replays the held highlight once the selection — and the paint — is gone", async () => {
        const { container, handle } = mount({
            siblings: [siblingOf("elena", "female")],
        });
        handle.highlightEntity({ id: "marco", kind: "person" });
        expect(container.querySelectorAll(".kul-selected")).toHaveLength(1);
        await explore(container);
        click(rowFor(container, "Siblings"));
        await settle();
        expect(container.querySelectorAll(".kul-selected")).toHaveLength(0);

        click(container.querySelector("svg"));
        await settle();
        expect(container.querySelectorAll("." + RESULT_CLASS)).toHaveLength(0);
        expect(container.querySelectorAll(".kul-selected")).toHaveLength(1);
    });
});
