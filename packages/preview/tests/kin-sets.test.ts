// The kin-set catalogue and the list it renders, at their value seams.
//
// The catalogue is a set of `Query` values the engine evaluates, not a set of
// membership rules this package owns (ADR-0025, ADR-0043) — so what is worth
// asserting here is that it *stays* the engine's set, and that a row says only
// what it has been told.

import { readFileSync } from "node:fs";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";
import { describe, expect, it } from "vitest";

import type { Member } from "../src/engine-wire.js";
import { buildKinList, emptyKinMessage } from "../src/kin-list.js";
import { KIN_SETS, kinQuery, kinSetById } from "../src/kin-sets.js";
import { EN, packFor } from "../src/phrasing/index.js";

const CRATES = join(
    dirname(fileURLToPath(import.meta.url)),
    "..",
    "..",
    "..",
    "crates",
    "kul-core",
);
const SUGAR = join(CRATES, "src", "query", "sugar.rs");
const CATALOGUE_SNAPSHOT = join(
    CRATES,
    "tests",
    "snapshots",
    "kin_catalogue__catalogue_patterns.snap",
);

/** `aunts_uncles_of` → `auntsUncles`. */
function idOfSugar(name: string): string {
    return name
        .replace(/_of$/, "")
        .replace(/_([a-z])/g, (_, letter: string) => letter.toUpperCase());
}

/**
 * The `KinPattern` the engine itself builds for each named sugar, keyed by
 * sugar name.
 *
 * Read out of `crates/kul-core/tests/snapshots/kin_catalogue__catalogue_patterns.snap`,
 * which `kin_catalogue.rs` writes from `Query::kin_*` and which the same file
 * proves is behaviourally the sugar it claims to be. Nothing in the build links
 * the Rust and the TypeScript — the preview never imports `@kullang/wasm`
 * (ADR-0040) — so this file is the link, exactly as `engine-wire.test.ts` links
 * the wire types.
 *
 * The `.snap` format is an insta front matter block, `---`-delimited, then the
 * payload; the payload here is JSON.
 */
function enginePatterns(): Record<string, unknown> {
    const raw = readFileSync(CATALOGUE_SNAPSHOT, "utf8");
    const payload = raw.slice(raw.indexOf("---", raw.indexOf("---") + 3) + 3);
    return JSON.parse(payload) as Record<string, unknown>;
}

describe("the catalogue is the engine's named kin sets", () => {
    it("offers a row for every sugar the engine names, and no others", () => {
        // Two independent sources, because they fail differently: the snapshot
        // is exhaustive only over what `kin_catalogue.rs` lists, and this regex
        // is exhaustive over what `sugar.rs` declares. A sugar added to the
        // engine and to neither must not pass. `[<(]` because a sugar without a
        // lifetime generic is spelled `pub fn x_of(` — the earlier `<`-only
        // pattern read straight past it.
        const declared = Array.from(
            readFileSync(SUGAR, "utf8").matchAll(/^pub fn ([a-z_]+_of)\s*[<(]/gm),
        ).map((match) => idOfSugar(match[1]));
        const ids = [...KIN_SETS.map((set) => set.id)].sort();
        expect(ids).toEqual([...declared].sort());
        expect(ids).toEqual(Object.keys(enginePatterns()).map(idOfSugar).sort());
    });

    it("asks the pattern the engine builds, field for field", () => {
        // The claim `kin-sets.ts` makes in prose — "every pattern below is the
        // expansion sugar.rs documents" — asserted rather than trusted. This is
        // the lint ADR-0043 leans on when it rejects the sweep alternative on
        // the grounds that a hand-copied matcher drifts silently.
        const engine = enginePatterns();
        for (const set of KIN_SETS) {
            const sugar = Object.keys(engine).find((name) => idOfSugar(name) === set.id);
            expect(sugar, `no engine pattern for ${set.id}`).toBeDefined();
            expect(set.pattern, `${set.id} drifted from ${sugar}`).toEqual(
                engine[sugar as string],
            );
        }
    });

    it("addresses each set by its own id", () => {
        for (const set of KIN_SETS) {
            expect(kinSetById(set.id)).toBe(set);
        }
        expect(kinSetById("nope")).toBeUndefined();
    });
});

describe("a kin query is one Query value, two projections apart", () => {
    const siblings = kinSetById("siblings")!;

    it("anchors on the person and carries the set's pattern", () => {
        expect(kinQuery(siblings, "giuseppe", "count")).toEqual({
            source: {
                kind: "kinOf",
                anchor: "giuseppe",
                pattern: {
                    classification: {
                        kind: "collateral",
                        up: { min: 1, max: 1 },
                        down: { min: 1, max: 1 },
                    },
                },
            },
            projection: "count",
        });
    });

    it("differs between the count and the paint in one field only", () => {
        const counted = kinQuery(siblings, "giuseppe", "count");
        const painted = kinQuery(siblings, "giuseppe", "members");
        expect({ ...counted, projection: "members" }).toEqual(painted);
    });

    it("leaves ancestry unbounded, exactly as `ancestors_of` does", () => {
        const query = kinQuery(kinSetById("ancestors")!, "giuseppe", "count");
        expect(query.source).toEqual({
            kind: "kinOf",
            anchor: "giuseppe",
            pattern: {
                classification: {
                    kind: "lineal",
                    role: "ancestor",
                    generations: { min: 1 },
                },
            },
        });
    });

    it("opens the affinal budget for the sets that need it, and only those", () => {
        const affinal = KIN_SETS.filter(
            (set) => set.pattern.affinity !== undefined || set.pattern.affinalHops,
        ).map((set) => set.id);
        expect(affinal).toEqual([
            "spouses",
            "inLaws",
            "stepParents",
            "stepSiblings",
            "stepChildren",
        ]);
    });
});

function sibling(alterId: string, alterGender: "male" | "female"): Member {
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

const BASE = {
    anchorId: "giuseppe",
    open: true,
    activeSetId: null,
    activeMembers: [],
};

describe("the list model says only what it has been told", () => {
    it("renders every set, always, so a zero is a row and not a gap", () => {
        const model = buildKinList(
            { ...BASE, counts: new Map([["siblings", 0]]) },
            EN,
        );
        expect(model.rows).toHaveLength(KIN_SETS.length);
        expect(model.rows.find((row) => row.id === "siblings")?.count).toBe(0);
    });

    it("distinguishes an unanswered count from a zero", () => {
        const model = buildKinList({ ...BASE, counts: new Map() }, EN);
        expect(model.rows.every((row) => row.count === null)).toBe(true);
    });

    it("glosses the painted row with its members' distinct terms, in order", () => {
        const model = buildKinList(
            {
                ...BASE,
                counts: new Map([["siblings", 3]]),
                activeSetId: "siblings",
                activeMembers: [
                    sibling("elena", "female"),
                    sibling("marco", "male"),
                    sibling("lucia", "female"),
                ],
            },
            EN,
        );
        const row = model.rows.find((r) => r.id === "siblings");
        expect(row?.active).toBe(true);
        expect(row?.terms).toEqual(["sister", "brother"]);
        expect(model.rows.filter((r) => r.terms.length)).toHaveLength(1);
    });

    it("phrases the gloss in the active pack", () => {
        const state = {
            ...BASE,
            counts: new Map([["siblings", 1]]),
            activeSetId: "siblings",
            activeMembers: [sibling("marco", "male")],
        };
        expect(buildKinList(state, EN).rows.find((r) => r.id === "siblings")?.terms).toEqual(
            ["brother"],
        );
        expect(
            buildKinList(state, packFor("gu")!).rows.find((r) => r.id === "siblings")
                ?.terms,
        ).toEqual(["ભાઈ"]);
    });

    it("carries the reader's open state rather than deciding it", () => {
        expect(buildKinList({ ...BASE, open: false, counts: new Map() }, EN).open).toBe(
            false,
        );
    });
});

describe("the empty-set message", () => {
    it("names the set and the anchor, and guesses nothing", () => {
        expect(emptyKinMessage("First cousins", "Giuseppe Rossi")).toBe(
            "No first cousins recorded for Giuseppe Rossi. " +
                "Nothing is guessed — absence stays absence.",
        );
    });
});
