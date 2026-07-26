// The attribute filter as a value: the `Query` the chip sentence spells, the
// arithmetic behind the tally, and the words each of them reads as.
//
// Three-valued predicate correctness is **not** re-tested here.
// `crates/kul-core`'s `filter__*.snap` suites own it (#296, Testing
// Decisions); what this file holds is that the preview asks the right
// question and adds up the answer honestly.

import { describe, expect, it } from "vitest";

import type { ExportedPerson, FilterMode, QuerySource } from "../src/engine-wire.js";
import {
    EMPTY_FILTER,
    FILTER_FIELDS,
    type FilterCondition,
    type FilterScope,
    type FilterState,
    PRESENCE_DISCLOSURE,
    buildFilterQuery,
    computeTally,
    conditionLabel,
    partitionScope,
    askedConditions,
    isCompleteCondition,
    isFilterActive,
    modeLabel,
    opsFor,
    parseInValues,
    tallyText,
    toPredicate,
} from "../src/filter.js";
import { CANT_SAY_LEAD, cantSayReason } from "../src/filter-reason.js";

const EVERYONE: FilterScope = {
    id: "everyone",
    label: "Everyone",
    source: { kind: "allPersons" },
    personIds: new Set(["a", "b", "c"]),
};

const KIN_SOURCE: QuerySource = {
    kind: "kinOf",
    anchor: "giuseppe",
    pattern: {
        classification: {
            kind: "lineal",
            role: "descendant",
            generations: { min: 1 },
        },
    },
};

const DESCENDANTS: FilterScope = {
    id: "descendants",
    label: "Giuseppe's descendants",
    source: KIN_SOURCE,
    personIds: new Set(["a", "b", "c", "d"]),
};

function state(
    conditions: FilterCondition[],
    mode: FilterMode = "certain",
): FilterState {
    return { scopeId: "everyone", conditions, mode };
}

describe("the op menu is field-aware", () => {
    it("offers ordering comparisons on the date fields only", () => {
        for (const field of FILTER_FIELDS) {
            const ordering = opsFor(field).filter((op) =>
                ["lt", "lte", "gt", "gte"].includes(op),
            );
            expect(ordering.length > 0).toBe(field === "born" || field === "died");
        }
    });

    it("offers ∈ on the free-text fields only", () => {
        const withIn = FILTER_FIELDS.filter((f) => opsFor(f).includes("in"));
        expect(withIn).toEqual(["name", "family", "given"]);
    });

    it("offers both presence predicates on every field", () => {
        for (const field of FILTER_FIELDS) {
            expect(opsFor(field)).toContain("present");
            expect(opsFor(field)).toContain("absent");
        }
    });

    it("never offers a disjunction — the grammar has no OR to spell", () => {
        const every = FILTER_FIELDS.flatMap((f) => [...opsFor(f)]);
        expect(every.filter((op) => /or/i.test(op))).toEqual([]);
    });
});

describe("the chip sentence produces the right Query value", () => {
    it("ANDs its conditions under an allPersons source", () => {
        const filter = state([
            { field: "family", op: "eq", value: "Rossi" },
            { field: "born", op: "lt", value: "1950" },
        ]);
        expect(buildFilterQuery(filter, EVERYONE, "certain")).toEqual({
            source: { kind: "allPersons" },
            where: [
                { op: "eq", field: "family", value: "Rossi" },
                { op: "lt", field: "born", value: "1950" },
            ],
            mode: "certain",
            projection: "members",
        });
    });

    it("composes the same conditions onto a painted kin set", () => {
        const filter = state([{ field: "born", op: "lt", value: "1950" }]);
        const query = buildFilterQuery(filter, DESCENDANTS, "includeUncertain");
        expect(query.source).toEqual(KIN_SOURCE);
        expect(query.where).toEqual([
            { op: "lt", field: "born", value: "1950" },
        ]);
        expect(query.mode).toBe("includeUncertain");
    });

    it("never sets sort — the tree has no order to express", () => {
        const query = buildFilterQuery(
            state([{ field: "family", op: "eq", value: "Rossi" }]),
            EVERYONE,
            "certain",
        );
        expect(query.sort).toBeUndefined();
    });

    it("splits ∈ into values and drops empties", () => {
        expect(parseInValues(" Rossi , Bianchi ,, ")).toEqual(["Rossi", "Bianchi"]);
        expect(
            toPredicate({ field: "family", op: "in", value: "Rossi, Bianchi" }),
        ).toEqual({ op: "in", field: "family", values: ["Rossi", "Bianchi"] });
    });

    it("drops the value from a presence predicate", () => {
        expect(
            toPredicate({ field: "died", op: "absent", value: "ignored" }),
        ).toEqual({ op: "absent", field: "died" });
    });
});

describe("a filter is active only when it asks something", () => {
    it("is inactive when empty", () => {
        expect(isFilterActive(EMPTY_FILTER)).toBe(false);
    });

    it("is inactive for a scope or a mode alone", () => {
        expect(
            isFilterActive({ ...EMPTY_FILTER, scopeId: "descendants" }),
        ).toBe(false);
        expect(
            isFilterActive({ ...EMPTY_FILTER, mode: "includeUncertain" }),
        ).toBe(false);
    });

    it("is active with one condition", () => {
        expect(
            isFilterActive(state([{ field: "family", op: "eq", value: "Rossi" }])),
        ).toBe(true);
    });

    it("does not count a chip nobody has finished writing", () => {
        const half = state([{ field: "family", op: "eq", value: "  " }]);
        expect(isCompleteCondition(half.conditions[0])).toBe(false);
        expect(isFilterActive(half)).toBe(false);
        expect(conditionLabel(half.conditions[0])).toBe("family =");
    });

    it("counts a presence predicate, which needs no value", () => {
        const presence = state([{ field: "died", op: "absent", value: "" }]);
        expect(isFilterActive(presence)).toBe(true);
    });

    it("keeps an unfinished chip out of the Query it would otherwise widen", () => {
        const mixed = state([
            { field: "family", op: "eq", value: "Rossi" },
            { field: "given", op: "eq", value: "" },
        ]);
        expect(askedConditions(mixed)).toHaveLength(1);
        expect(buildFilterQuery(mixed, EVERYONE, "certain").where).toEqual([
            { op: "eq", field: "family", value: "Rossi" },
        ]);
    });
});

describe("the tally arithmetic", () => {
    const population = new Set(["p1", "p2", "p3", "p4", "p5"]);

    /** The two answers the engine gives, counted the way the bar counts them. */
    function count(
        pop: ReadonlySet<string>,
        certainIds: string[],
        inclusiveIds: string[],
        mode: FilterMode,
    ) {
        return computeTally(partitionScope(pop, certainIds, inclusiveIds), mode);
    }

    it("splits the scope into the three sets the paint draws", () => {
        // The same partition the tally counts is the one the tree is painted
        // from, so a number and a colour can never disagree.
        const partition = partitionScope(
            population,
            ["p1", "p2"],
            ["p1", "p2", "p3"],
        );
        expect([...partition.matched].sort()).toEqual(["p1", "p2"]);
        expect([...partition.cantSay]).toEqual(["p3"]);
        expect([...partition.dimmed].sort()).toEqual(["p4", "p5"]);
    });

    it("partitions the scope in certain mode", () => {
        const tally = count(population, ["p1", "p2"], ["p1", "p2", "p3"], "certain");
        expect(tally).toEqual({
            total: 5,
            matched: 2,
            cantSay: 1,
            dimmed: 2,
            shown: 2,
        });
        expect(tally.matched + tally.cantSay + tally.dimmed).toBe(tally.total);
    });

    it("moves only `shown` when the certainty chip flips", () => {
        const certain = count(population, ["p1", "p2"], ["p1", "p2", "p3"], "certain");
        const inclusive = count(
            population,
            ["p1", "p2"],
            ["p1", "p2", "p3"],
            "includeUncertain",
        );
        expect(inclusive.shown).toBe(3);
        expect({ ...inclusive, shown: certain.shown }).toEqual(certain);
    });

    it("counts against the scope, not the project", () => {
        // The engine answered about four people; only three are in scope.
        const scoped = new Set(["p1", "p2", "p3"]);
        const tally = count(scoped, ["p1", "p9"], ["p1", "p9", "p2"], "certain");
        expect(tally).toEqual({
            total: 3,
            matched: 1,
            cantSay: 1,
            dimmed: 1,
            shown: 1,
        });
    });

    it("has nothing to disclose when every row was judged", () => {
        const tally = count(population, ["p1"], ["p1"], "certain");
        expect(tally.cantSay).toBe(0);
        expect(tally.dimmed).toBe(4);
    });
});

describe("the tally in words", () => {
    const population = new Set(["p1", "p2", "p3", "p4", "p5"]);
    const tally = computeTally(
        partitionScope(population, ["p1", "p2"], ["p1", "p2", "p3"]),
        "certain",
    );

    it("discloses what certain mode dropped", () => {
        expect(tallyText(tally, "certain", null)).toBe(
            "2 of 5 · 2 dimmed · 1 can't say",
        );
    });

    it("marks the same number included when the mode changes", () => {
        const inclusive = computeTally(
            partitionScope(population, ["p1", "p2"], ["p1", "p2", "p3"]),
            "includeUncertain",
        );
        expect(tallyText(inclusive, "includeUncertain", null)).toBe(
            "3 of 5 · 2 dimmed · 1 can't say, included",
        );
    });

    it("names the scope it counted against", () => {
        expect(tallyText(tally, "certain", "Giuseppe's descendants")).toBe(
            "2 of 5 in Giuseppe's descendants · 2 dimmed · 1 can't say",
        );
    });

    it("drops a clause that would read as zero", () => {
        const clean = computeTally(
            partitionScope(population, [...population], [...population]),
            "certain",
        );
        expect(tallyText(clean, "certain", null)).toBe("5 of 5");
    });
});

describe("how the sentence reads", () => {
    it("spells a value condition", () => {
        expect(conditionLabel({ field: "family", op: "eq", value: "Rossi" })).toBe(
            "family = Rossi",
        );
        expect(conditionLabel({ field: "born", op: "lt", value: "1950" })).toBe(
            "born < 1950",
        );
        expect(
            conditionLabel({ field: "family", op: "in", value: "Rossi,Bianchi" }),
        ).toBe("family ∈ Rossi, Bianchi");
    });

    it("spells a presence predicate as a claim about the record", () => {
        expect(conditionLabel({ field: "died", op: "absent", value: "" })).toBe(
            "died not recorded",
        );
    });

    it("never lets `not recorded` read as a fact about the person", () => {
        expect(PRESENCE_DISCLOSURE).toContain("not about the person");
        expect(PRESENCE_DISCLOSURE).toContain("living");
    });

    it("names certainty as part of the query, both ways", () => {
        expect(modeLabel("certain")).toBe("certain");
        expect(modeLabel("includeUncertain")).toBe("certain + can't say");
    });
});

describe("why the filter could not judge someone", () => {
    const aldo: ExportedPerson = {
        id: "aldo",
        name: "Aldo",
        gender: "male",
        born: { value: "1942", precision: "year", circa: true },
    };
    const sofia: ExportedPerson = {
        id: "sofia",
        name: "Sofia",
        family: "Rossi",
        gender: "female",
        born: { value: "1943", precision: "year", circa: false },
    };
    const crisp: ExportedPerson = {
        id: "crisp",
        name: "Crisp",
        family: "Rossi",
        gender: "female",
        born: { value: "1950-06-15", precision: "day", circa: false },
    };

    it("names a field the author never recorded", () => {
        expect(
            cantSayReason(aldo, [{ field: "family", op: "eq", value: "Rossi" }]),
        ).toBe("can't say — family not recorded");
    });

    it("names a circa date and the comparison it straddles", () => {
        expect(
            cantSayReason(aldo, [{ field: "born", op: "lt", value: "1945" }]),
        ).toBe(
            "can't say — ~1942 is approximate (±5y) — straddles born < 1945",
        );
    });

    it("names a partial date the same way — fuzziness is not only absence", () => {
        expect(
            cantSayReason(sofia, [{ field: "born", op: "lt", value: "1943-06" }]),
        ).toBe(
            "can't say — 1943 is year-precision — straddles born < 1943-06",
        );
    });

    it("names a wide literal when the record itself is crisp", () => {
        expect(
            cantSayReason(crisp, [{ field: "born", op: "lt", value: "1950" }]),
        ).toBe(
            "can't say — born < 1950 spans a whole year — 1950-06-15 falls inside it",
        );
    });

    it("never blames a presence predicate — those are decidable on any record", () => {
        expect(
            cantSayReason(aldo, [
                { field: "died", op: "absent", value: "" },
                { field: "family", op: "eq", value: "Rossi" },
            ]),
        ).toBe("can't say — family not recorded");
    });

    it("names every candidate when more than one could be the culprit", () => {
        expect(
            cantSayReason(aldo, [
                { field: "family", op: "eq", value: "Rossi" },
                { field: "born", op: "lt", value: "1945" },
            ]),
        ).toBe(
            "can't say — family not recorded · ~1942 is approximate (±5y) — straddles born < 1945",
        );
    });

    it("degrades to the bare lead rather than inventing a reason", () => {
        expect(
            cantSayReason(crisp, [{ field: "family", op: "eq", value: "Rossi" }]),
        ).toBe(CANT_SAY_LEAD);
    });
});
