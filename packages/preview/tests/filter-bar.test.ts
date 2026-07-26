// The filter bar, driven the way a reader drives it: clicking chips, changing
// selects, typing values — with the engine substituted at its one system
// boundary (`CODING_STANDARDS.md` — mock at system boundaries, never internal
// collaborators).
//
// The engine double is a **recorder plus a fixture**, not a re-implementation:
// it answers from an explicit true / unknown table per query, so nothing here
// re-derives a three-valued predicate. `kul-core`'s `filter__*.snap` suites
// own that (PRD-0006, Testing Decisions).

import { readFileSync } from "node:fs";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";

import { type KeyboardPanZoom, mountKeyboardPan } from "../src/controls.js";
import { PREVIEW_BODY_HTML } from "../src/controls.js";
import { createDimRegistry } from "../src/dim.js";
import { KIN_SETS } from "../src/kin-sets.js";
import { DOCKED_TAG_CLASS } from "../src/docked-tag.js";
import type {
    DetailLookupResult,
    DetailTarget,
    ExportedPerson,
    Member,
    Query,
    QueryEnvelope,
    QueryResult,
} from "../src/engine-wire.js";
import type { FilterScope } from "../src/filter.js";
import {
    EVERYONE_LABEL,
    FILTER_REASON_CLASS,
    createFilterBar,
} from "../src/filter-bar.js";
import type { FilterBar } from "../src/filter-bar.js";
import { FILTER_MATCH_CLASS, FILTER_UNCERTAIN_CLASS } from "../src/filter-paint.js";
import { createLocaleController } from "../src/locale.js";
import {
    SYNC_SUSPENDED_HINT,
    createQuerySurface,
} from "../src/query-surface.js";
import type { QuerySurface } from "../src/query-surface.js";
import { DIM_CLASS } from "../src/dim.js";

const SVG = `<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 400 200">
    <g class="kul-card" data-person-id="giulia" data-kind="canonical" data-gender="female">
        <rect x="10" y="10" width="60" height="30"/>
    </g>
    <g class="kul-card" data-person-id="marco" data-kind="canonical" data-gender="male">
        <rect x="110" y="10" width="60" height="30"/>
    </g>
    <g class="kul-card" data-person-id="aldo" data-kind="canonical" data-gender="male">
        <rect x="210" y="10" width="60" height="30"/>
    </g>
    <g class="kul-card" data-person-id="nina" data-kind="canonical" data-gender="female">
        <rect x="310" y="10" width="60" height="30"/>
    </g>
    <path class="kul-edge" data-link-kind="marriage" data-marriage-id="m1" d="M 0 0 L 1 1"/>
</svg>`;

const PEOPLE: Record<string, ExportedPerson> = {
    giulia: { id: "giulia", name: "Giulia", family: "Rossi", gender: "female" },
    marco: { id: "marco", name: "Marco", family: "Rossi", gender: "male" },
    aldo: {
        id: "aldo",
        name: "Aldo",
        gender: "male",
        born: { value: "1942", precision: "year", circa: true },
    },
    nina: { id: "nina", name: "Nina", family: "Bianchi", gender: "female" },
    giuseppe: {
        id: "giuseppe",
        name: "Giuseppe Rossi",
        family: "Rossi",
        gender: "male",
    },
};

const DESCENDANTS: FilterScope = {
    id: "descendants",
    label: "Giuseppe's descendants",
    source: {
        kind: "kinOf",
        anchor: "giuseppe",
        pattern: {
            classification: {
                kind: "lineal",
                role: "descendant",
                generations: { min: 1 },
            },
        },
    },
    personIds: new Set(["giulia", "marco", "aldo"]),
};

/**
 * What one *question* answers: which persons it says `true` for, and which it
 * says `unknown` for. Everyone else is `false`.
 */
type Verdicts = { certain: string[]; unknown: string[] };

/** Nothing matched, nothing was unjudgeable — what an unseeded question answers. */
const NO_ONE: Verdicts = { certain: [], unknown: [] };

/**
 * How the engine double keys its answers: **the predicate list**, and nothing
 * else. A certainty flip and a scope change read the same entry, because the
 * mode is applied on top and the scope filters the answer — but two different
 * `where` lists are two different questions with two different answers, which
 * is the whole property the concurrency suites rest on. A question with no
 * entry answers {@link NO_ONE}, so a superseded evaluation can never come back
 * looking like the one that replaced it.
 */
function whereKey(query: Query): string {
    return JSON.stringify(query.where ?? []);
}

/** The key for the chip `family = <value>`, which is what most suites ask. */
function familyIs(value: string): string {
    return JSON.stringify([{ op: "eq", field: "family", value }]);
}

interface Harness {
    stage: HTMLElement;
    root: HTMLElement;
    flow: HTMLElement;
    floatLayer: HTMLElement;
    bar: FilterBar;
    queries: Query[];
    lookups: DetailTarget[][];
    suspensions: boolean[];
    kin: { current: FilterScope | null };
    /** Question (predicate list) → what it answers. Seeded per test. */
    answers: Map<string, Verdicts>;
    /** Fail every query from here on, as a project that fails its checks does. */
    failing: { value: boolean };
    /**
     * Hold every query from here on. `gates` collects one release per held
     * call, so a test can land two overlapping evaluations in either order.
     */
    holding: { value: boolean };
    gates: Array<() => void>;
}

function harness(overrides: Partial<Verdicts> = {}): Harness {
    const stage = document.createElement("div");
    stage.innerHTML =
        '<div id="flow"></div><div id="root"></div><div id="layer"></div>';
    document.body.appendChild(stage);
    const root = stage.querySelector("#root") as HTMLElement;
    root.innerHTML = SVG;
    const flow = stage.querySelector("#flow") as HTMLElement;
    const floatLayer = stage.querySelector("#layer") as HTMLElement;

    const queries: Query[] = [];
    const lookups: DetailTarget[][] = [];
    const suspensions: boolean[] = [];
    const kin: { current: FilterScope | null } = { current: null };
    // The constructor seeds one question — `family = Rossi`, which is what
    // nearly every suite here asks. Anything else answers `NO_ONE` until a test
    // seeds it by key.
    const answers = new Map<string, Verdicts>([
        [
            familyIs("Rossi"),
            {
                certain: overrides.certain ?? [],
                unknown: overrides.unknown ?? [],
            },
        ],
    ]);
    const failing = { value: false };
    const holding = { value: false };
    const gates: Array<() => void> = [];

    const bar = createFilterBar({
        host: flow,
        root,
        floatLayer,
        dim: createDimRegistry(),
        async run(query) {
            queries.push(query);
            if (holding.value) {
                await new Promise<void>((resolve) => gates.push(resolve));
            }
            if (failing.value) {
                return {
                    ok: false,
                    diagnostics: [
                        {
                            code: "KUL-R03",
                            severity: "error",
                            message: "missing gender",
                            related: [],
                        },
                    ],
                } as QueryEnvelope<QueryResult>;
            }
            const scope =
                query.source.kind === "kinOf"
                    ? DESCENDANTS.personIds
                    : new Set(Object.keys(PEOPLE));
            // Read *after* the gate, and keyed on the question — so a held
            // evaluation answers what it asked, not what the table happens to
            // say when it resolves.
            const verdicts = answers.get(whereKey(query)) ?? NO_ONE;
            const shown =
                query.mode === "includeUncertain"
                    ? [...verdicts.certain, ...verdicts.unknown]
                    : verdicts.certain;
            const personIds = shown.filter((id) => scope.has(id));
            return {
                ok: true,
                result:
                    query.source.kind === "kinOf"
                        ? {
                              kind: "members",
                              members: personIds.map((personId) => ({
                                  personId,
                                  descriptor: {} as never,
                              })),
                          }
                        : { kind: "personIds", personIds },
            } as QueryEnvelope<QueryResult>;
        },
        async lookup(targets) {
            lookups.push(targets);
            return {
                ok: true,
                result: targets.map((target) =>
                    target.kind === "person" && PEOPLE[target.id]
                        ? {
                              kind: "person",
                              person: PEOPLE[target.id],
                              parents: [],
                              marriages: [],
                              children: [],
                          }
                        : null,
                ),
            } as QueryEnvelope<DetailLookupResult>;
        },
        kinScope: () => kin.current,
        setSyncSuspended(suspended) {
            suspensions.push(suspended);
        },
    });

    return {
        stage,
        root,
        flow,
        floatLayer,
        bar,
        queries,
        lookups,
        suspensions,
        kin,
        answers,
        failing,
        holding,
        gates,
    };
}

function settle(): Promise<void> {
    return new Promise((resolve) => setTimeout(resolve, 0));
}

/**
 * The sentence as it reads: keywords, chip labels, the ✕ each condition chip
 * carries, and the tally. An open editor's own controls are not part of the
 * sentence, so they are not read here.
 */
function text(h: Harness): string {
    return Array.from(h.bar.element.children)
        .map((el) => {
            if (!el.classList.contains("kul-filter-chip")) {
                return el.textContent ?? "";
            }
            const label = el.querySelector(".kul-filter-chip-label")?.textContent ?? "";
            return el.querySelector(".kul-filter-chip-remove") ? `${label} ✕` : label;
        })
        .join(" ")
        .trim();
}

function chipLabels(h: Harness): string[] {
    return Array.from(h.bar.element.querySelectorAll(".kul-filter-chip-label")).map(
        (el) => el.textContent ?? "",
    );
}

function click(el: Element | null | undefined): void {
    (el as HTMLElement).dispatchEvent(new MouseEvent("click", { bubbles: true }));
}

function change(el: Element | null | undefined, value: string): void {
    (el as HTMLInputElement | HTMLSelectElement).value = value;
    (el as HTMLElement).dispatchEvent(new Event("change", { bubbles: true }));
}

function editor(h: Harness): HTMLElement {
    return h.bar.element.querySelector(".kul-filter-editor") as HTMLElement;
}

function selects(h: Harness): HTMLSelectElement[] {
    return Array.from(editor(h).querySelectorAll("select"));
}

/** Add a finished condition through the chrome, as a reader would. */
async function addCondition(
    h: Harness,
    field: string,
    op: string,
    value?: string,
): Promise<void> {
    click(h.bar.element.querySelector(".kul-filter-chip-add .kul-filter-chip-label"));
    change(selects(h)[0], field);
    await settle();
    change(selects(h)[1], op);
    await settle();
    if (value !== undefined) {
        const input = editor(h).querySelector("input, select:nth-of-type(3)");
        change(input, value);
        await settle();
    }
}

function tally(h: Harness): string | null {
    return h.bar.element.querySelector(".kul-filter-tally")?.textContent ?? null;
}

beforeEach(() => {
    document.body.innerHTML = "";
});

afterEach(() => {
    document.body.innerHTML = "";
});

describe("the sentence reads as a sentence", () => {
    it("starts with a scope, a +, and a certainty chip — and no `where`", () => {
        const h = harness();
        expect(text(h)).toBe("Showing Everyone 4 ▾ + certain ▾");
        expect(text(h)).not.toContain("where");
    });

    it("spells `where` and `and` as literal text between the chips", async () => {
        const h = harness();
        await addCondition(h, "family", "eq", "Rossi");
        await addCondition(h, "born", "lt", "1950");
        await settle();
        expect(text(h)).toContain(
            "Showing Everyone 4 ▾ where family = Rossi ✕ and born < 1950 ✕",
        );
    });

    it("offers no OR anywhere in the chrome", async () => {
        const h = harness();
        await addCondition(h, "family", "eq", "Rossi");
        click(h.bar.element.querySelector(".kul-filter-chip-add .kul-filter-chip-label"));
        const markup = h.bar.element.innerHTML;
        expect(markup).not.toMatch(/\bor\b/i);
        expect(markup).not.toMatch(/advanced/i);
    });

    it("carries no role, aria-* or focus-visible attribute (ADR-0036)", async () => {
        const h = harness();
        await addCondition(h, "died", "absent");
        click(h.bar.element.querySelector(".kul-filter-chip-scope .kul-filter-chip-label"));
        for (const node of h.bar.element.querySelectorAll("*")) {
            for (const attr of Array.from(node.attributes)) {
                expect(attr.name).not.toBe("role");
                expect(attr.name.startsWith("aria-")).toBe(false);
            }
        }
    });
});

describe("the chip editor", () => {
    it("offers ordering comparisons only once the field takes them", async () => {
        const h = harness();
        click(h.bar.element.querySelector(".kul-filter-chip-add .kul-filter-chip-label"));
        const optionValues = () =>
            Array.from(selects(h)[1].options).map((o) => o.value);
        expect(optionValues()).not.toContain("lt");
        change(selects(h)[0], "born");
        await settle();
        expect(optionValues()).toContain("lt");
        expect(optionValues()).not.toContain("in");
    });

    it("drops an op the new field does not offer", async () => {
        const h = harness();
        await addCondition(h, "family", "in", "Rossi, Bianchi");
        expect(chipLabels(h)[1]).toBe("family ∈ Rossi, Bianchi");
        change(selects(h)[0], "born");
        await settle();
        // `∈` is text-only, so the chip cannot keep it — and the value it
        // carried belongs to the old field, so it goes too.
        expect(chipLabels(h)[1]).toBe("born =");
    });

    it("says what `not recorded` does not mean, where it is offered", async () => {
        const h = harness();
        await addCondition(h, "died", "absent");
        const note = editor(h).querySelector(".kul-filter-note")?.textContent ?? "";
        expect(note).toContain("not about the person");
        expect(note).toContain("living");
        expect(chipLabels(h)[1]).toBe("died not recorded");
    });

    it("does not pan the canvas from inside a chip editor", async () => {
        // The guard the theming slice added: a keystroke from a text-entry
        // host belongs to the field, not to the canvas. Asserted against the
        // real `mountKeyboardPan`, because the bar honours it by *being* made
        // of `<select>` and `<input>` — there is no code in the bar to test.
        const h = harness();
        await addCondition(h, "family", "eq", "Rossi");
        const panZoom: KeyboardPanZoom = {
            panBy: vi.fn(),
            zoomIn: vi.fn(),
            zoomOut: vi.fn(),
            reset: vi.fn(),
        };
        const teardown = mountKeyboardPan(() => panZoom);
        try {
            const controls = Array.from(
                editor(h).querySelectorAll("input, select"),
            );
            expect(controls.length).toBeGreaterThan(0);
            for (const control of controls) {
                for (const key of ["ArrowLeft", "ArrowUp", "+", "-", "0"]) {
                    const event = new KeyboardEvent("keydown", {
                        key,
                        bubbles: true,
                        cancelable: true,
                    });
                    control.dispatchEvent(event);
                    expect(event.defaultPrevented, `${key} in a chip editor`).toBe(
                        false,
                    );
                }
            }
            expect(panZoom.panBy).not.toHaveBeenCalled();
            expect(panZoom.zoomIn).not.toHaveBeenCalled();
            expect(panZoom.zoomOut).not.toHaveBeenCalled();
            expect(panZoom.reset).not.toHaveBeenCalled();
        } finally {
            teardown();
        }
    });

    it("removes a chip when its ✕ is pressed", async () => {
        const h = harness();
        await addCondition(h, "family", "eq", "Rossi");
        await settle();
        click(h.bar.element.querySelector(".kul-filter-chip-remove"));
        await settle();
        expect(text(h)).toBe("Showing Everyone 4 ▾ + certain ▾");
    });
});

describe("the sentence produces the right Query value", () => {
    it("asks both certainty questions, and nothing else, per change", async () => {
        const h = harness();
        await addCondition(h, "family", "eq", "Rossi");
        await settle();
        expect(h.queries).toHaveLength(2);
        expect(h.queries.map((q) => q.mode).sort()).toEqual([
            "certain",
            "includeUncertain",
        ]);
        expect(h.queries[0]).toEqual({
            source: { kind: "allPersons" },
            where: [{ op: "eq", field: "family", value: "Rossi" }],
            mode: "certain",
            projection: "members",
        });
    });

    it("asks nothing for a chip nobody has finished writing", async () => {
        const h = harness();
        click(h.bar.element.querySelector(".kul-filter-chip-add .kul-filter-chip-label"));
        await settle();
        expect(h.queries).toEqual([]);
        expect(h.suspensions).not.toContain(true);
    });
});

describe("the tree under a filter", () => {
    it("paints matches teal, the unjudgeable amber, and recedes the rest", async () => {
        const h = harness({ certain: ["giulia", "marco"], unknown: ["aldo"] });
        await addCondition(h, "family", "eq", "Rossi");
        await settle();
        const cls = (id: string) =>
            h.root
                .querySelector(`[data-person-id="${id}"]`)
                ?.getAttribute("class") ?? "";
        expect(cls("giulia")).toContain(FILTER_MATCH_CLASS);
        expect(cls("aldo")).toContain(FILTER_UNCERTAIN_CLASS);
        expect(cls("aldo")).not.toContain(DIM_CLASS);
        expect(cls("nina")).toContain(DIM_CLASS);
    });

    it("keeps every node and every edge, whatever the filter says", async () => {
        const h = harness({ certain: ["giulia"], unknown: ["aldo"] });
        const before = {
            cards: h.root.querySelectorAll(".kul-card").length,
            edges: h.root.querySelectorAll(".kul-edge").length,
        };
        await addCondition(h, "family", "eq", "Rossi");
        await settle();
        expect({
            cards: h.root.querySelectorAll(".kul-card").length,
            edges: h.root.querySelectorAll(".kul-edge").length,
        }).toEqual(before);
        h.bar.reset();
        await settle();
        expect({
            cards: h.root.querySelectorAll(".kul-card").length,
            edges: h.root.querySelectorAll(".kul-edge").length,
        }).toEqual(before);
    });

    it("discloses what certain mode dropped", async () => {
        const h = harness({ certain: ["giulia", "marco"], unknown: ["aldo"] });
        await addCondition(h, "family", "eq", "Rossi");
        await settle();
        expect(tally(h)).toBe("2 of 4 · 1 dimmed · 1 can't say");
    });

    it("moves the unjudgeable into the shown count when the chip flips", async () => {
        const h = harness({ certain: ["giulia", "marco"], unknown: ["aldo"] });
        await addCondition(h, "family", "eq", "Rossi");
        await settle();
        click(h.bar.element.querySelector(".kul-filter-chip-mode .kul-filter-chip-label"));
        change(selects(h)[0], "includeUncertain");
        await settle();
        expect(tally(h)).toBe("3 of 4 · 1 dimmed · 1 can't say, included");
        expect(chipLabels(h)[chipLabels(h).length - 1]).toBe("certain + can't say ▾");
        // Still amber, still nameable: including them does not un-say that the
        // engine could not judge them.
        const aldo = h.root.querySelector('[data-person-id="aldo"]') as Element;
        expect(aldo.getAttribute("class")).toContain(FILTER_UNCERTAIN_CLASS);
    });
});

describe("composition with a painted kin set", () => {
    it("offers the kin set as a scope only while one is painted", async () => {
        const h = harness();
        click(h.bar.element.querySelector(".kul-filter-chip-scope .kul-filter-chip-label"));
        expect(Array.from(selects(h)[0].options).map((o) => o.value)).toEqual([
            "everyone",
        ]);
        expect(editor(h).textContent).toContain("Paint a kin set");

        // The editor is still open, and a redraw re-renders it in place.
        h.kin.current = DESCENDANTS;
        h.bar.repaint();
        await settle();
        expect(Array.from(selects(h)[0].options).map((o) => o.value)).toEqual([
            "everyone",
            "descendants",
        ]);
    });

    it("re-evaluates against the kin set, not the whole project", async () => {
        const h = harness({ certain: ["giulia", "marco", "nina"], unknown: [] });
        h.kin.current = DESCENDANTS;
        await addCondition(h, "family", "eq", "Rossi");
        await settle();
        expect(tally(h)).toBe("3 of 4 · 1 dimmed");

        h.queries.length = 0;
        click(h.bar.element.querySelector(".kul-filter-chip-scope .kul-filter-chip-label"));
        change(selects(h)[0], "descendants");
        await settle();

        // The source is the kin set's own — the engine composes `kinOf` with
        // the same `where`, rather than the surface intersecting two answers.
        expect(h.queries.map((q) => q.source)).toEqual([
            DESCENDANTS.source,
            DESCENDANTS.source,
        ]);
        // Nina matched the predicate but is outside the set, so she is neither
        // counted nor lit.
        expect(tally(h)).toBe("2 of 3 in Giuseppe's descendants · 1 dimmed");
        expect(
            h.root.querySelector('[data-person-id="nina"]')?.getAttribute("class"),
        ).not.toContain(FILTER_MATCH_CLASS);
    });

    it("falls back to Everyone when the painted set goes away", async () => {
        const h = harness({ certain: ["giulia"], unknown: [] });
        h.kin.current = DESCENDANTS;
        await addCondition(h, "family", "eq", "Rossi");
        click(h.bar.element.querySelector(".kul-filter-chip-scope .kul-filter-chip-label"));
        change(selects(h)[0], "descendants");
        await settle();
        expect(chipLabels(h)[0]).toBe("Giuseppe's descendants 3 ▾");

        h.kin.current = null;
        h.bar.repaint();
        await settle();
        expect(chipLabels(h)[0]).toBe(`${EVERYONE_LABEL} 4 ▾`);
        expect(h.bar.state().scopeId).toBe("everyone");
    });
});

describe("an active filter suspends editor sync", () => {
    it("registers as soon as the reader finishes a chip, and lifts on reset", async () => {
        const h = harness({ certain: ["giulia"], unknown: [] });
        expect(h.suspensions).not.toContain(true);
        await addCondition(h, "family", "eq", "Rossi");
        await settle();
        expect(h.suspensions).toContain(true);

        h.suspensions.length = 0;
        h.bar.reset();
        await settle();
        expect(h.suspensions).toEqual([false]);
    });

    it("does not register for a chip the reader has not finished", async () => {
        // Not the same as the `+` test above: this *commits* — the op select
        // fires `evaluate` — and still asks nothing, so the whole path runs
        // with an incomplete sentence and must leave sync alone.
        const h = harness({ certain: ["giulia"], unknown: [] });
        click(h.bar.element.querySelector(".kul-filter-chip-add .kul-filter-chip-label"));
        change(selects(h)[0], "born");
        await settle();
        change(selects(h)[1], "lt");
        await settle();
        expect(chipLabels(h)[1]).toBe("born <");
        expect(h.queries).toEqual([]);
        expect(h.suspensions).not.toContain(true);

        // Finishing it does register.
        change(editor(h).querySelector("input"), "1950");
        await settle();
        expect(h.suspensions).toContain(true);
    });

    it("lifts on dispose, so a torn-down bar leaves sync held down by nobody", async () => {
        const h = harness({ certain: ["giulia"], unknown: [] });
        await addCondition(h, "family", "eq", "Rossi");
        await settle();
        h.suspensions.length = 0;
        h.bar.dispose();
        expect(h.suspensions).toEqual([false]);
        expect(h.root.querySelectorAll("." + FILTER_MATCH_CLASS)).toHaveLength(0);
        expect(h.flow.querySelector(".kul-filter-bar")).toBeNull();
    });
});

describe("the can't-say whisper", () => {
    async function hovering(): Promise<Harness> {
        const h = harness({ certain: ["giulia"], unknown: ["aldo"] });
        await addCondition(h, "family", "eq", "Rossi");
        await settle();
        return h;
    }

    function whisper(h: Harness): string | null {
        return h.floatLayer.querySelector(".kul-filter-reason")?.textContent ?? null;
    }

    it("whispers the reason under the card, in the docked-tag grammar", async () => {
        const h = await hovering();
        h.bar.handleHover(h.root.querySelector('[data-person-id="aldo"] rect'));
        await settle();
        const tag = h.floatLayer.querySelector("." + DOCKED_TAG_CLASS);
        expect(tag).not.toBeNull();
        // The variant marker earns its keep only if it is actually on the tag:
        // it is the one thing that tells this whisper from the lens's pill,
        // which docks into the same layer with the same box.
        expect(tag?.classList.contains(FILTER_REASON_CLASS)).toBe(true);
        expect(whisper(h)).toBe("can't say — family not recorded");
    });

    it("reads the record through the batched lookup, never off the card", async () => {
        const h = await hovering();
        h.lookups.length = 0;
        h.bar.handleHover(h.root.querySelector('[data-person-id="aldo"] rect'));
        await settle();
        expect(h.lookups).toEqual([[{ kind: "person", id: "aldo" }]]);
    });

    it("names only conditions the engine was actually asked", async () => {
        // A chip the reader has not finished asks nothing, so naming it as a
        // candidate would explain the verdict with a predicate that never ran
        // (ADR-0045; `filter.ts` says the same about `askedConditions`).
        const h = await hovering();
        // A second chip, on a *different* field so its leak would be visible,
        // and left unfinished.
        click(h.bar.element.querySelector(".kul-filter-chip-add .kul-filter-chip-label"));
        change(selects(h)[0], "given");
        await settle();
        expect(chipLabels(h)[2]).toBe("given =");
        expect(PEOPLE.aldo.given).toBeUndefined();

        h.bar.handleHover(h.root.querySelector('[data-person-id="aldo"] rect'));
        await settle();
        // One clause, from the one chip the engine was asked — never
        // "· given not recorded" from a predicate that never ran.
        expect(whisper(h)).toBe("can't say — family not recorded");
    });

    it("says nothing about a card the filter could judge", async () => {
        const h = await hovering();
        h.bar.handleHover(h.root.querySelector('[data-person-id="giulia"] rect'));
        await settle();
        expect(whisper(h)).toBeNull();
        expect(h.lookups.some((l) => l[0]?.kind === "person" && l[0].id === "giulia")).toBe(
            false,
        );
    });

    it("takes the whisper down when the pointer leaves the card", async () => {
        const h = await hovering();
        h.bar.handleHover(h.root.querySelector('[data-person-id="aldo"] rect'));
        await settle();
        h.bar.handleHover(h.root.querySelector('[data-person-id="giulia"] rect'));
        expect(whisper(h)).toBeNull();
    });

    it("keeps it up while the pointer is on the whisper itself", async () => {
        const h = await hovering();
        h.bar.handleHover(h.root.querySelector('[data-person-id="aldo"] rect'));
        await settle();
        const reason = h.floatLayer.querySelector(".kul-filter-reason") as Element;
        h.bar.handleHover(reason);
        expect(whisper(h)).not.toBeNull();
    });

    it("asks once however many times the pointer fires over one card", async () => {
        const h = await hovering();
        h.lookups.length = 0;
        const aldo = h.root.querySelector('[data-person-id="aldo"] rect');
        // Pointer movement is per pixel; the lookup must not be.
        for (let i = 0; i < 5; i++) {
            h.bar.handleHover(aldo);
        }
        await settle();
        expect(h.lookups).toHaveLength(1);
    });

    it("asks once per person per answer, and re-asks after a new one", async () => {
        const h = await hovering();
        h.lookups.length = 0;
        const aldo = h.root.querySelector('[data-person-id="aldo"] rect');
        h.bar.handleHover(aldo);
        await settle();
        h.bar.handleHover(null);
        h.bar.handleHover(aldo);
        await settle();
        expect(h.lookups).toHaveLength(1);

        // A repaint is a new answer about a possibly-new project, so the
        // record it explained is no longer known to be the record.
        h.bar.repaint();
        await settle();
        h.bar.handleHover(aldo);
        await settle();
        expect(h.lookups).toHaveLength(2);
    });
});

describe("concurrent gestures", () => {
    it("discards a superseded evaluation rather than painting it", async () => {
        // Two evaluations overlap and the *earlier* one answers last. The three
        // questions answer three different things, so a stale answer on the
        // tree is a different tally rather than the same one — which is the
        // only way this suite can fail when the generation guard is gone.
        const h = harness({ certain: ["giulia"], unknown: [] });
        h.answers.set(familyIs("Bianchi"), {
            certain: ["giulia", "marco"],
            unknown: [],
        });
        h.answers.set(familyIs("Verdi"), { certain: ["nina"], unknown: [] });
        await addCondition(h, "family", "eq", "Rossi");
        await settle();
        expect(tally(h)).toBe("1 of 4 · 3 dimmed");

        h.holding.value = true;
        change(editor(h).querySelector("input"), "Bianchi");
        await settle();
        const bianchi = h.gates.splice(0);
        expect(bianchi).toHaveLength(2);

        change(editor(h).querySelector("input"), "Verdi");
        await settle();
        const verdi = h.gates.splice(0);
        expect(verdi).toHaveLength(2);

        // The later question answers first, the earlier one second.
        for (const release of [...verdi, ...bianchi]) {
            release();
        }
        await settle();
        await settle();

        // Verdi's answer, not Bianchi's: one match, not two.
        expect(chipLabels(h)[1]).toBe("family = Verdi");
        expect(tally(h)).toBe("1 of 4 · 3 dimmed");
        expect(
            h.root.querySelector('[data-person-id="nina"]')?.getAttribute("class"),
        ).toContain(FILTER_MATCH_CLASS);
        expect(
            h.root.querySelector('[data-person-id="giulia"]')?.getAttribute("class"),
        ).toContain(DIM_CLASS);
    });

    it("discards a render's re-ask that a chip edit has already superseded", async () => {
        const h = harness({ certain: ["giulia"], unknown: [] });
        h.answers.set(familyIs("Bianchi"), {
            certain: ["nina", "marco"],
            unknown: [],
        });
        await addCondition(h, "family", "eq", "Rossi");
        await settle();

        h.holding.value = true;
        h.bar.repaint();
        await settle();
        const fromRender = h.gates.splice(0);
        expect(fromRender).toHaveLength(2);

        change(editor(h).querySelector("input"), "Bianchi");
        await settle();
        const fromEdit = h.gates.splice(0);
        expect(fromEdit).toHaveLength(2);

        for (const release of [...fromEdit, ...fromRender]) {
            release();
        }
        await settle();
        await settle();

        // The edit's answer, not the render's re-ask of the old question.
        expect(tally(h)).toBe("2 of 4 · 2 dimmed");
        expect(
            h.root.querySelector('[data-person-id="nina"]')?.getAttribute("class"),
        ).toContain(FILTER_MATCH_CLASS);
        expect(
            h.root.querySelector('[data-person-id="giulia"]')?.getAttribute("class"),
        ).toContain(DIM_CLASS);
    });

    it("drops a reason whose filter moved on under it", async () => {
        const h = harness({ certain: ["giulia"], unknown: ["aldo"] });
        await addCondition(h, "family", "eq", "Rossi");
        await settle();
        const aldo = h.root.querySelector('[data-person-id="aldo"] rect');
        h.bar.handleHover(aldo);
        // The filter changes before the record comes back.
        h.bar.repaint();
        await settle();
        expect(h.floatLayer.querySelector(".kul-filter-reason")).toBeNull();
    });

    it("drops a reason whose pointer moved on under it", async () => {
        const h = harness({ certain: ["giulia"], unknown: ["aldo"] });
        await addCondition(h, "family", "eq", "Rossi");
        await settle();
        h.bar.handleHover(h.root.querySelector('[data-person-id="aldo"] rect'));
        h.bar.handleHover(null);
        await settle();
        expect(h.floatLayer.querySelector(".kul-filter-reason")).toBeNull();
    });
});

describe("a project that fails its checks", () => {
    it("keeps the sentence and drops the paint", async () => {
        const h = harness({ certain: ["giulia"], unknown: [] });
        await addCondition(h, "family", "eq", "Rossi");
        await settle();
        expect(h.root.querySelectorAll("." + FILTER_MATCH_CLASS).length).toBe(1);

        h.failing.value = true;
        h.bar.repaint();
        await settle();
        // The popover carries the diagnostics (ADR-0009); a stale verdict on
        // the tree would be worse than no verdict.
        expect(h.root.querySelectorAll("." + FILTER_MATCH_CLASS).length).toBe(0);
        expect(tally(h)).toBeNull();
        expect(chipLabels(h)[1]).toBe("family = Rossi");
    });
});

// -- The stylesheet seam ------------------------------------------------
//
// jsdom resolves neither custom-property substitution nor layout, so a green
// DOM test proves nothing about whether the sentence fits its bar. These read
// the application sheet as text, the same shape `tokens.test.ts` and
// `crates/kul-svg/tests/visual.rs` already use.

const appSheet = readFileSync(
    join(dirname(fileURLToPath(import.meta.url)), "..", "src", "preview.css"),
    "utf8",
);

function ruleFor(selector: string): string {
    const at = appSheet.indexOf(selector + " {");
    expect(at, `no rule for ${selector}`).toBeGreaterThan(-1);
    return appSheet.slice(at, appSheet.indexOf("}", at));
}

describe("the chip sentence cannot print through its own chrome", () => {
    it("wraps the sentence rather than letting it overflow", () => {
        const bar = ruleFor(".kul-filter-bar");
        expect(bar).toContain("flex-wrap: wrap");
        expect(bar).not.toContain("overflow-x");
    });

    it("caps a chip and removes the floor that would refuse the cap", () => {
        // `min-width: 0` is the load-bearing half: a flex item's automatic
        // minimum size is its min-content width, which for a long unbroken
        // value exceeds the cap, so the item will not shrink to it until that
        // floor is removed. `overflow-wrap` decides how the text breaks inside
        // the resulting width — per spec it does not reduce min-content size,
        // so it cannot make a `max-width` bind on its own.
        expect(ruleFor(".kul-filter-chip")).toContain(
            "max-width: var(--kul-filter-chip-max-width)",
        );
        const label = ruleFor(".kul-filter-chip-label");
        expect(label).toContain("min-width: 0");
        expect(label).toContain("overflow-wrap: break-word");
    });

    it("bounds the chip editor and lets its controls shrink inside it", () => {
        const editor = ruleFor(".kul-filter-editor");
        expect(editor).toContain("width: var(--kul-filter-editor-width)");
        expect(editor).toContain("max-width: 100%");
        expect(editor).toContain("box-sizing: border-box");
        const controls = ruleFor(".kul-filter-editor select,\n.kul-filter-editor input");
        expect(controls).toContain("min-width: 0");
    });

    it("lets the tally and the reason wrap rather than truncate", () => {
        for (const selector of [".kul-filter-tally", ".kul-filter-reason"]) {
            const rule = ruleFor(selector);
            expect(rule).toContain("min-width: 0");
            expect(rule).toContain("overflow-wrap: break-word");
            expect(rule).not.toContain("text-overflow");
        }
    });
});


/**
 * One sibling of Giuseppe, as the engine describes them. Real enough to phrase:
 * the kin list glosses every member it is handed, so a hollow descriptor would
 * take the phrasing layer down rather than the surface under test.
 */
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

/** Which catalogue set a kin `Query` spells, by matching its pattern. */
function setIdOf(query: Query): string | null {
    return (
        KIN_SETS.find(
            (set) =>
                JSON.stringify(set.pattern) ===
                JSON.stringify(
                    query.source.kind === "kinOf" ? query.source.pattern : null,
                ),
        )?.id ?? null
    );
}

// -- The hover fan-out, at the surface that owns it ----------------------
//
// `handleCanvasHover` has two consumers that dock a tag under the *same* card,
// so it is the one place the placement question ADR-0044 handed to #303 is
// answered. Driven through `createQuerySurface`, because the answer is the
// composition's, not the bar's.

/**
 * The same picture plus Giuseppe, who is the anchor Explore kin needs and who
 * has no business inflating the `Everyone` count the bar's own suites assert.
 */
const SURFACE_SVG = SVG.replace(
    "</svg>",
    `<g class="kul-card" data-person-id="giuseppe" data-kind="canonical" data-gender="male">
        <rect x="410" y="10" width="60" height="30"/>
    </g></svg>`,
);

function surfaceHarness(): {
    stage: HTMLElement;
    root: HTMLElement;
    surface: QuerySurface;
} {
    const stage = document.createElement("div");
    stage.innerHTML = PREVIEW_BODY_HTML;
    document.body.appendChild(stage);
    const root = stage.querySelector("#root") as HTMLElement;
    root.innerHTML = SURFACE_SVG;
    const surface = createQuerySurface({
        root,
        floatDock: stage.querySelector("#kul-region-float-dock") as HTMLElement,
        floatLayer: stage.querySelector("#kul-region-float") as HTMLElement,
        flowRegion: stage.querySelector("#kul-region-flow") as HTMLElement,
        notifyRegion: stage.querySelector("#kul-region-notify") as HTMLElement,
        adapter: { onRevealRequest: () => {} },
        async lookup(targets) {
            return {
                ok: true,
                result: targets.map((target) =>
                    target.kind === "person" && PEOPLE[target.id]
                        ? {
                              kind: "person" as const,
                              person: PEOPLE[target.id],
                              parents: [],
                              marriages: [],
                              children: [],
                          }
                        : null,
                ),
            };
        },
        async runKinQuery(query) {
            // Giuseppe's siblings are Giulia and Marco. Every other set is
            // empty, so exactly one row is clickable and one paint is
            // reachable.
            const members =
                setIdOf(query) === "siblings"
                    ? [siblingOf("giulia", "female"), siblingOf("marco", "male")]
                    : [];
            return query.projection === "count"
                ? {
                      ok: true,
                      result: { kind: "count" as const, count: members.length },
                  }
                : { ok: true, result: { kind: "members" as const, members } };
        },
        async runQuery(query) {
            // Giulia matches; Aldo has no family recorded, so he is the
            // engine's `unknown` and the only amber card.
            const shown =
                query.mode === "includeUncertain"
                    ? ["giulia", "aldo"]
                    : ["giulia"];
            return {
                ok: true,
                result: { kind: "personIds" as const, personIds: shown },
            };
        },
        async resolve() {
            return { ok: true, result: { relationships: [] } };
        },
        locale: createLocaleController(null),
        getPanZoom: () => null,
        applySyncHighlight: () => {},
    });
    return { stage, root, surface };
}

describe("one gesture, one whisper", () => {

    async function filtered(): Promise<ReturnType<typeof surfaceHarness>> {
        const h = surfaceHarness();
        const bar = h.stage.querySelector("#kul-filter-bar") as HTMLElement;
        click(bar.querySelector(".kul-filter-chip-add .kul-filter-chip-label"));
        const editor = bar.querySelector(".kul-filter-editor") as HTMLElement;
        change(editor.querySelector("input"), "Rossi");
        await settle();
        return h;
    }

    function reason(h: { stage: HTMLElement }): string | null {
        return (
            h.stage.querySelector(".kul-filter-reason")?.textContent ?? null
        );
    }

    afterEach(() => {
        document.body.innerHTML = "";
    });

    it("whispers the can't-say reason while nothing is selected", async () => {
        const h = await filtered();
        h.surface.handleCanvasHover(
            h.root.querySelector('[data-person-id="aldo"] rect'),
        );
        await settle();
        expect(reason(h)).toBe("can't say — family not recorded");
    });

    it("stands the reason down while a person is selected", async () => {
        const h = await filtered();
        h.surface.selection.select({ kind: "person", id: "giulia" });
        await settle();
        h.surface.handleCanvasHover(
            h.root.querySelector('[data-person-id="aldo"] rect'),
        );
        await settle();
        // The lens is armed and its pill docks under this very card; two tags
        // under one card would be two answers to one gesture (ADR-0045).
        expect(reason(h)).toBeNull();
    });

    it("takes an open reason off screen when a selection arms the lens", async () => {
        const h = await filtered();
        const aldo = h.root.querySelector('[data-person-id="aldo"] rect');
        h.surface.handleCanvasHover(aldo);
        await settle();
        expect(reason(h)).not.toBeNull();

        h.surface.selection.select({ kind: "person", id: "giulia" });
        await settle();
        // Told to stand down, not merely not called: the next pointer move is
        // what would otherwise leave a stale reason up.
        h.surface.handleCanvasHover(aldo);
        await settle();
        expect(reason(h)).toBeNull();
    });

    it("keeps the reason available under an edge selection, which arms no lens", async () => {
        const h = await filtered();
        h.surface.selection.select({ kind: "marriage", id: "m1" });
        await settle();
        h.surface.handleCanvasHover(
            h.root.querySelector('[data-person-id="aldo"] rect'),
        );
        await settle();
        expect(reason(h)).toBe("can't say — family not recorded");
    });
});

describe("filtering inside a painted kin set, through the real chrome", () => {
    afterEach(() => {
        document.body.innerHTML = "";
    });

    /** Select Giuseppe, open Explore kin, paint his siblings. */
    async function paintSiblings(h: {
        stage: HTMLElement;
        root: HTMLElement;
        surface: QuerySurface;
    }): Promise<void> {
        h.surface.selection.select({ kind: "person", id: "giuseppe" });
        await settle();
        click(h.stage.querySelector(".kul-kin-header"));
        await settle();
        const row = Array.from(h.stage.querySelectorAll(".kul-kin-row")).find(
            (candidate) =>
                candidate.querySelector(".kul-kin-label")?.textContent ===
                "Siblings",
        );
        click(row);
        await settle();
    }

    function bar(h: { stage: HTMLElement }): HTMLElement {
        return h.stage.querySelector("#kul-filter-bar") as HTMLElement;
    }

    function scopeOptions(h: { stage: HTMLElement }): string[] {
        click(
            bar(h).querySelector(".kul-filter-chip-scope .kul-filter-chip-label"),
        );
        const select = bar(h).querySelector(
            ".kul-filter-editor select",
        ) as HTMLSelectElement;
        return Array.from(select.options).map((option) => option.textContent ?? "");
    }

    it("offers the painted set as a scope, named as the reader sees it", async () => {
        const h = surfaceHarness();
        expect(scopeOptions(h)).toEqual(["Everyone"]);
        // Close the editor again so the next open is not a toggle-shut.
        click(
            bar(h).querySelector(".kul-filter-chip-scope .kul-filter-chip-label"),
        );

        await paintSiblings(h);
        expect(scopeOptions(h)).toEqual([
            "Everyone",
            "Giuseppe Rossi's siblings",
        ]);
    });

    it("counts and paints against the painted set, not the project", async () => {
        const h = surfaceHarness();
        await paintSiblings(h);
        const select = (() => {
            click(
                bar(h).querySelector(
                    ".kul-filter-chip-scope .kul-filter-chip-label",
                ),
            );
            return bar(h).querySelector(
                ".kul-filter-editor select",
            ) as HTMLSelectElement;
        })();
        change(select, "siblings");
        await settle();

        // Add a condition inside that scope.
        click(bar(h).querySelector(".kul-filter-chip-add .kul-filter-chip-label"));
        change(bar(h).querySelector(".kul-filter-editor input"), "Rossi");
        await settle();

        // Two people in the painted set; the harness answers `giulia` certain
        // and `aldo` unknown, and Aldo is not in the set — so he is neither
        // counted nor painted amber.
        expect(
            bar(h).querySelector(".kul-filter-tally")?.textContent,
        ).toBe("1 of 2 in Giuseppe Rossi's siblings · 1 dimmed");
        expect(
            h.root.querySelector('[data-person-id="aldo"]')?.getAttribute("class"),
        ).not.toContain(FILTER_UNCERTAIN_CLASS);
    });
});

describe("the post-render hook, and coexistence with kin paint", () => {
    afterEach(() => {
        document.body.innerHTML = "";
    });

    function bar(h: { stage: HTMLElement }): HTMLElement {
        return h.stage.querySelector("#kul-filter-bar") as HTMLElement;
    }

    /** Apply `family = Rossi` through the bar the surface built. */
    async function filter(h: { stage: HTMLElement }): Promise<void> {
        click(bar(h).querySelector(".kul-filter-chip-add .kul-filter-chip-label"));
        change(
            (bar(h).querySelector(".kul-filter-editor") as HTMLElement).querySelector(
                "input",
            ),
            "Rossi",
        );
        await settle();
    }

    function classOf(root: HTMLElement, id: string): string {
        return root.querySelector(`[data-person-id="${id}"]`)?.getAttribute("class") ?? "";
    }

    it("puts the paint back on the SVG a render replaced", async () => {
        const h = surfaceHarness();
        await filter(h);
        expect(classOf(h.root, "giulia")).toContain(FILTER_MATCH_CLASS);
        expect(h.root.querySelectorAll(".kul-filter-uncertain-badge")).toHaveLength(1);

        // A render swaps the SVG out from under every piece of query paint.
        // The new picture carries none of it, and the id-keyed dim is the only
        // thing `apply` alone would restore — the classes and the `?` are not.
        h.root.innerHTML = SURFACE_SVG;
        expect(classOf(h.root, "giulia")).not.toContain(FILTER_MATCH_CLASS);

        h.surface.repaintQueryChrome();
        await settle();

        expect(classOf(h.root, "giulia")).toContain(FILTER_MATCH_CLASS);
        expect(classOf(h.root, "aldo")).toContain(FILTER_UNCERTAIN_CLASS);
        expect(h.root.querySelectorAll(".kul-filter-uncertain-badge")).toHaveLength(1);
        expect(classOf(h.root, "nina")).toContain(DIM_CLASS);
        expect(bar(h).querySelector(".kul-filter-tally")?.textContent).toBe(
            "1 of 5 · 3 dimmed · 1 can't say",
        );
    });

    it("never lets a kin set recede an unjudgeable card", async () => {
        // The case ADR-0045 treats as a requirement rather than an edge: both
        // paints on screen at once. Aldo is not one of Giuseppe's siblings, so
        // kin paint dims him — and he is the filter's can't-say, which must
        // never recede. The exemption is what settles it (S1).
        const h = surfaceHarness();
        h.surface.selection.select({ kind: "person", id: "giuseppe" });
        await settle();
        click(h.stage.querySelector(".kul-kin-header"));
        await settle();
        const row = Array.from(h.stage.querySelectorAll(".kul-kin-row")).find(
            (candidate) =>
                candidate.querySelector(".kul-kin-label")?.textContent === "Siblings",
        );
        click(row);
        await settle();
        expect(classOf(h.root, "aldo")).toContain(DIM_CLASS);

        await filter(h);
        expect(classOf(h.root, "aldo")).toContain(FILTER_UNCERTAIN_CLASS);
        expect(classOf(h.root, "aldo")).not.toContain(DIM_CLASS);
        // Nina is neither kin nor a match, so both sources dim her and she
        // stays dimmed — the exemption lifts one person, not the union.
        expect(classOf(h.root, "nina")).toContain(DIM_CLASS);

        // Dropping the filter hands Aldo back to the kin dim.
        h.surface.clearFilter();
        await settle();
        expect(classOf(h.root, "aldo")).toContain(DIM_CLASS);
    });
});

describe("Esc means what the sync hint says", () => {
    afterEach(() => {
        document.body.innerHTML = "";
    });

    function press(): void {
        window.dispatchEvent(
            new KeyboardEvent("keydown", { key: "Escape", bubbles: true }),
        );
    }

    function bar(h: { stage: HTMLElement }): HTMLElement {
        return h.stage.querySelector("#kul-filter-bar") as HTMLElement;
    }

    function hint(h: { stage: HTMLElement }): string | null {
        return h.stage.querySelector(".kul-sync-hint")?.textContent ?? null;
    }

    async function filter(h: { stage: HTMLElement }): Promise<void> {
        click(bar(h).querySelector(".kul-filter-chip-add .kul-filter-chip-label"));
        change(
            (bar(h).querySelector(".kul-filter-editor") as HTMLElement).querySelector(
                "input",
            ),
            "Rossi",
        );
        await settle();
    }

    it("clears a filter with no selection anywhere", async () => {
        // The hint reads "Editor sync paused · Esc to resume" the moment *any*
        // reason holds sync down, so a filter-only suspension with a
        // selection-only Esc would put that sentence on screen with no key
        // behind it (ADR-0045).
        const h = surfaceHarness();
        await filter(h);
        expect(h.surface.selection.current).toBeNull();
        expect(hint(h)).toBe(SYNC_SUSPENDED_HINT);

        press();
        await settle();

        expect(bar(h).textContent).not.toContain("where");
        expect(hint(h)).toBeNull();
        expect(
            h.root.querySelector('[data-person-id="giulia"]')?.getAttribute("class"),
        ).not.toContain(FILTER_MATCH_CLASS);
    });

    it("clears a selection and a filter together when both hold sync down", async () => {
        const h = surfaceHarness();
        await filter(h);
        h.surface.selection.select({ kind: "person", id: "giuseppe" });
        await settle();
        expect(hint(h)).toBe(SYNC_SUSPENDED_HINT);

        press();
        await settle();

        expect(h.surface.selection.current).toBeNull();
        expect(bar(h).textContent).not.toContain("where");
        expect(hint(h)).toBeNull();
    });

    it("does nothing when nothing is holding sync down", async () => {
        const h = surfaceHarness();
        const before = bar(h).textContent;
        press();
        await settle();
        expect(bar(h).textContent).toBe(before);
        expect(hint(h)).toBeNull();
    });
});
