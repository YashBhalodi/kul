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
import type { QueryEnvelope, QueryResult } from "../src/engine-wire.js";
import type { ProjectSnapshot, QueryEngine } from "../src/engine.js";
import { PREVIEW_BODY_HTML } from "../src/controls.js";
import { KIN_SETS } from "../src/kin-sets.js";
import { DIM_CLASS } from "../src/dim.js";
import { RESULT_CLASS } from "../src/kin-paint.js";
import { SELECTION_CLASS } from "../src/selection.js";
import { createLocaleController } from "../src/locale.js";
import { mountPreview } from "../src/mount.js";
import {
    LENS_DIM_EXEMPTION,
    createQuerySurface,
} from "../src/query-surface.js";
import type { QuerySurface } from "../src/query-surface.js";
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
 * A latch the test opens by hand. Holding an answer is the only way to exercise
 * the windows ADR-0043 designs for — rows are clickable before counts land, and
 * a second row is clickable before the first one's members do — and those
 * windows are what the guards exist to survive.
 */
interface Gate {
    wait(): Promise<void>;
    open(): void;
    held: boolean;
}

function gate(): Gate {
    let release: () => void = () => {};
    const opened = new Promise<void>((resolve) => {
        release = resolve;
    });
    const self: Gate = {
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
 * Latches keyed by something the test names — the sweep's anchor, or the set a
 * row asked about. Keyed rather than shared because "released out of order" is
 * the case the guards are for, and one shared latch resolves everything in the
 * order the calls were made, which is exactly the order that cannot fail.
 */
interface Gates {
    /** Wait on `key`'s latch. Opens on demand when the test is not holding. */
    wait(key: string): Promise<void>;
    /** Release `key`'s latch. */
    open(key: string): void;
    /** Hold everything until released by key. */
    holdAll: boolean;
}

function gates(holdAll: boolean): Gates {
    const held = new Map<string, Gate>();
    function latch(key: string): Gate {
        let existing = held.get(key);
        if (!existing) {
            existing = gate();
            held.set(key, existing);
        }
        return existing;
    }
    return {
        holdAll,
        wait(key) {
            return this.holdAll ? latch(key).wait() : Promise.resolve();
        },
        open(key) {
            latch(key).open();
        },
    };
}

export interface EngineBehaviour {
    /** Latches for the per-anchor count sweep, keyed by anchor id. */
    counts: Gates;
    /** Latches for a row's members answer, keyed by kin-set id. */
    members: Gates;
    /** When true, every `count` query answers the error arm (ADR-0009). */
    failCounts: boolean;
    /** Per-anchor count override, so two anchors can disagree. */
    countFor?: (anchor: string, setId: string) => number;
}

/**
 * Engine double. `members` answers come from `membersBySet`; any set without an
 * entry is genuinely empty, which is what the honest-emptiness case needs. The
 * `count` projection answers the same set's size unless `countFor` says
 * otherwise, so a count and a paint can never disagree in these tests for the
 * wrong reason.
 */
function fakeEngine(
    membersBySet: Record<string, Member[]>,
    behaviour: EngineBehaviour,
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
                const anchor = query.source.kind === "kinOf" ? query.source.anchor : "";
                const setId = setIdOf(query) ?? "";
                const members = membersFor(query, membersBySet);
                if (query.projection === "count") {
                    await behaviour.counts.wait(anchor);
                    if (behaviour.failCounts) {
                        return {
                            ok: false,
                            diagnostics: [
                                {
                                    code: "KUL-R03",
                                    severity: "error",
                                    message: "person `x` is missing required field `gender`",
                                    related: [],
                                },
                            ],
                        };
                    }
                    const count = behaviour.countFor
                        ? behaviour.countFor(anchor, setId)
                        : members.length;
                    return { ok: true, result: { kind: "count", count } };
                }
                await behaviour.members.wait(setId);
                return { ok: true, result: { kind: "members", members } };
            },
            async runQuery() {
                // The filter bar is not what these suites drive; an empty
                // answer keeps the engine double complete without adding one.
                return { ok: true, result: { kind: "personIds", personIds: [] } };
            },
            async queryResolve() {
                // The hover lens is not what these suites drive; a tie-free
                // answer keeps the engine double complete without adding one.
                return { ok: true, result: { relationships: [] } };
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
    options: {
        holdCounts?: boolean;
        holdMembers?: boolean;
        failCounts?: boolean;
        countFor?: (anchor: string, setId: string) => number;
    } = {},
): {
    container: HTMLElement;
    handle: PreviewHandle;
    asked: Asked;
    engine: EngineBehaviour;
} {
    const container = document.createElement("div");
    document.body.appendChild(container);
    const adapter: HostAdapter = { onRevealRequest: () => {} };
    const behaviour: EngineBehaviour = {
        counts: gates(options.holdCounts ?? false),
        members: gates(options.holdMembers ?? false),
        failCounts: options.failCounts ?? false,
        countFor: options.countFor,
    };
    const { engine, asked } = fakeEngine(membersBySet, behaviour);
    const handle = mountPreview(container, adapter, { engine });
    handle.render(SVG, PROJECT);
    return { container, handle, asked, engine: behaviour };
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

/** Distinct person ids wearing the result glow, in document order. */
function litPersonIds(container: HTMLElement): string[] {
    const ids = Array.from(container.querySelectorAll("." + RESULT_CLASS)).map(
        (node) => node.getAttribute("data-person-id") ?? "",
    );
    return [...new Set(ids)];
}

function activeRowLabels(container: HTMLElement): string[] {
    return Array.from(container.querySelectorAll(".kul-kin-row-active")).map(
        (row) => rowText(row as HTMLElement).label,
    );
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
        const { container, engine } = mount(
            { siblings: [siblingOf("elena", "female")] },
            { holdCounts: true },
        );
        await explore(container);
        expect(counts(container).every((count) => count === "·")).toBe(true);

        click(rowFor(container, "Siblings"));
        await settle();
        expect(container.querySelectorAll("." + RESULT_CLASS)).toHaveLength(3);

        engine.counts.open("giuseppe");
        await settle();
        expect(counts(container)).not.toContain("·");
        expect(rowText(rowFor(container, "Siblings")).count).toBe("1");
        // …and the paint the reader made in the meantime is still up.
        expect(container.querySelectorAll("." + RESULT_CLASS)).toHaveLength(3);
    });

    it("does not paint the outgoing anchor's numbers onto the new one's list", async () => {
        // The two anchors must *disagree*, or a stale sweep landing is
        // indistinguishable from the right one landing. Giuseppe answers 1
        // everywhere, Marco answers 7, and Giuseppe's sweep is released last.
        const { container, engine } = mount(
            {},
            {
                holdCounts: true,
                countFor: (anchor) => (anchor === "giuseppe" ? 1 : 7),
            },
        );
        await explore(container);
        click(container.querySelector('[data-person-id="marco"] rect'));
        await settle();

        engine.counts.open("marco");
        await settle();
        expect(counts(container)).toEqual(KIN_SETS.map(() => "7"));

        engine.counts.open("giuseppe");
        await settle();
        // Giuseppe's answers arrived after Marco's and are about somebody the
        // reader is no longer looking at.
        expect(counts(container)).toEqual(KIN_SETS.map(() => "7"));
    });
});

describe("a row's answer is discarded when a later row overtakes it", () => {
    it("keeps the second row's members when the first row's arrive last", async () => {
        // Two rows in flight, released in the order that can go wrong: the
        // reader clicked Siblings, changed their mind and clicked Children, and
        // the abandoned answer comes back afterwards.
        const { container, engine } = mount(
            {
                siblings: [siblingOf("elena", "female")],
                children: [siblingOf("marco", "male")],
            },
            { holdMembers: true },
        );
        await explore(container);
        click(rowFor(container, "Siblings"));
        click(rowFor(container, "Children"));
        await settle();
        expect(container.querySelectorAll("." + RESULT_CLASS)).toHaveLength(0);

        engine.members.open("children");
        await settle();
        expect(litPersonIds(container)).toEqual(["marco"]);
        expect(activeRowLabels(container)).toEqual(["Children"]);

        engine.members.open("siblings");
        await settle();
        // Siblings' answer arrived last and must lose: it is about a question
        // the reader let go of, and Elena owns three cards that would light.
        expect(litPersonIds(container)).toEqual(["marco"]);
        expect(activeRowLabels(container)).toEqual(["Children"]);
    });
});

describe("a sweep that could not answer is retried, not remembered", () => {
    it("re-issues after a failing project is fixed", async () => {
        // ADR-0009's error arm is what a document with a validation error
        // yields, which is the ordinary state mid-edit. A memo held across that
        // failure left every row reading `·` for as long as the person stayed
        // selected — surviving the fix and the re-render after it.
        const { container, engine, asked } = mount(
            { siblings: [siblingOf("elena", "female")] },
            { failCounts: true },
        );
        await explore(container);
        expect(counts(container)).toEqual(KIN_SETS.map(() => "·"));
        const failed = asked.kin.filter((q) => q.projection === "count").length;
        expect(failed).toBe(KIN_SETS.length);

        engine.failCounts = false;
        // Close and reopen — the same gesture a reader makes, with no change of
        // selection, which was the only escape before.
        click(container.querySelector(".kul-kin-header"));
        await settle();
        click(container.querySelector(".kul-kin-header"));
        await settle();

        expect(asked.kin.filter((q) => q.projection === "count")).toHaveLength(
            failed + KIN_SETS.length,
        );
        expect(rowText(rowFor(container, "Siblings")).count).toBe("1");
        expect(counts(container)).not.toContain("·");
    });

    it("shows whatever did answer, and keeps asking for the rest", async () => {
        // A partial sweep is not a failed one: the numbers that landed are the
        // engine's, and the rows that did not show the same placeholder they
        // show before any answer lands.
        let allow = true;
        const { container, asked } = mount(
            {},
            {
                countFor: (_anchor, setId) => {
                    if (setId !== "siblings" && allow) {
                        throw new Error("not this one");
                    }
                    return 4;
                },
            },
        );
        // The double throws rather than returning an error arm, which
        // `mount.ts` turns into the popover and a `null` answer.
        await explore(container);
        expect(rowText(rowFor(container, "Siblings")).count).toBe("4");
        expect(counts(container)).toContain("·");

        allow = false;
        const partial = asked.kin.filter((q) => q.projection === "count").length;
        click(container.querySelector(".kul-kin-header"));
        await settle();
        click(container.querySelector(".kul-kin-header"));
        await settle();
        expect(
            asked.kin.filter((q) => q.projection === "count").length,
        ).toBeGreaterThan(partial);
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

// The two members `mount.ts` never calls. `createQuerySurface` is the seam the
// slices that compose *inside* the surface use, so they are driven here rather
// than through the mounted chrome — and an injected `runKinQuery` is the only
// way to make one reject, which the mount deliberately never lets happen.

function surface(options: {
    runKinQuery(query: Query): Promise<QueryEnvelope<QueryResult> | null>;
}): { root: HTMLElement; surface: QuerySurface; stage: HTMLElement } {
    const stage = document.createElement("div");
    stage.innerHTML = PREVIEW_BODY_HTML;
    document.body.appendChild(stage);
    const root = stage.querySelector("#root") as HTMLElement;
    root.innerHTML = SVG;
    const built = createQuerySurface({
        root,
        floatDock: stage.querySelector("#kul-region-float-dock") as HTMLElement,
        floatLayer: stage.querySelector("#kul-region-float") as HTMLElement,
        notifyRegion: stage.querySelector("#kul-region-notify") as HTMLElement,
        adapter: { onRevealRequest: () => {} },
        async lookup(targets) {
            return { ok: true, result: targets.map(detailFor) };
        },
        // The filter bar is #303's surface, not this one's. A host without a
        // flow region gets no bar, which keeps these suites driving exactly
        // what they are about.
        flowRegion: null,
        async runQuery() {
            return { ok: true, result: { kind: "personIds" as const, personIds: [] } };
        },
        runKinQuery: options.runKinQuery,
        async resolve() {
            return { ok: true, result: { relationships: [] } };
        },
        locale: createLocaleController(null),
        getPanZoom: () => null,
        applySyncHighlight: () => {},
    });
    return { root, surface: built, stage };
}

describe("a rejecting kin query leaves no claim behind", () => {
    it("lets the sweep be re-issued after the throw stops", async () => {
        // `mount.ts` turns a throw into the error popover and a `null` answer,
        // so this path is unreachable through the mounted chrome — but the
        // surface takes `runKinQuery` as an option and must not strand the memo
        // for a host that hands it one which rejects.
        let throwing = true;
        const asked: Query[] = [];
        const { stage, surface: built } = surface({
            async runKinQuery(query) {
                asked.push(query);
                if (throwing) {
                    throw new Error("engine module went away");
                }
                return { ok: true, result: { kind: "count", count: 2 } };
            },
        });
        built.selection.select({ kind: "person", id: "giuseppe" });
        await settle();
        click(stage.querySelector(".kul-kin-header"));
        await settle();
        expect(asked).toHaveLength(KIN_SETS.length);
        expect(counts(stage)).toEqual(KIN_SETS.map(() => "·"));

        throwing = false;
        click(stage.querySelector(".kul-kin-header"));
        await settle();
        click(stage.querySelector(".kul-kin-header"));
        await settle();
        expect(asked).toHaveLength(KIN_SETS.length * 2);
        expect(counts(stage)).toEqual(KIN_SETS.map(() => "2"));
        built.dispose();
    });
});

describe("setDimExemption lifts the dim for a live read", () => {
    it("un-dims the traced persons and puts them back when it is withdrawn", async () => {
        const { root, stage, surface: built } = surface({
            async runKinQuery(query) {
                return query.projection === "count"
                    ? { ok: true, result: { kind: "count", count: 1 } }
                    : {
                          ok: true,
                          result: {
                              kind: "members",
                              members: [siblingOf("elena", "female")],
                          },
                      };
            },
        });
        built.selection.select({ kind: "person", id: "giuseppe" });
        await settle();
        click(stage.querySelector(".kul-kin-header"));
        await settle();
        click(rowFor(stage, "Siblings"));
        await settle();
        const dimmed = () =>
            Array.from(root.querySelectorAll("." + DIM_CLASS)).map((node) =>
                node.getAttribute("data-person-id"),
            );
        expect(dimmed()).toEqual(["marco"]);

        built.setDimExemption(LENS_DIM_EXEMPTION, ["marco"]);
        expect(dimmed()).toEqual([]);
        // The answer itself is untouched — an exemption lifts the dim, it does
        // not change who the engine said was kin.
        expect(litPersonIds(stage)).toEqual(["elena"]);

        built.setDimExemption(LENS_DIM_EXEMPTION, null);
        expect(dimmed()).toEqual(["marco"]);
        built.dispose();
    });
});
