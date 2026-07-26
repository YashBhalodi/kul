import { describe, expect, it } from "vitest";

import type { LanguagePack, PackEntry } from "../../src/phrasing/pack.js";
import { phrase } from "../../src/phrasing/phrase.js";
import { collateral, descriptorOf, down, up } from "./fixtures.js";
import { enumerateDescriptors, type EnumerationBounds } from "./enumerate.js";
import { findEntryConflicts } from "./pack-audit.js";

/**
 * The conflict scan is only worth having if it fires. This suite tests the
 * detector against packs with a *known* defect — and, more importantly, pins
 * the enumeration property the detector depends on.
 *
 * The shape below is the one `gu` will write (#299): a seniority-marked pair
 * (*moṭā bhāī* / *nāno bhāī*) over an unmarked term. Every key where seniority
 * is `elder` or `younger` is won outright by a marked entry, so a defect
 * between two *unmarked* entries is invisible unless the enumeration also
 * reaches `seniority: unknown` — which is the value the engine emits whenever a
 * birth date is missing, i.e. most of the time.
 */
const BOUNDS: EnumerationBounds = {
    maxUp: 2,
    maxDown: 2,
    maxAcross: 1,
    maxHops: 3,
    genderMode: "product",
};

const HOPS = {
    up: { male: "bapa", female: "ma", other: "vali" },
    down: { male: "dikro", female: "dikri", other: "santan" },
    across: { male: "pati", female: "patni", other: "jodidar" },
} as const;

const MARKED: PackEntry[] = [
    {
        when: {
            classification: "collateral",
            cousinDegree: 0,
            removed: 0,
            alterGender: "male",
            seniority: "elder",
        },
        term: "mota bhai",
    },
    {
        when: {
            classification: "collateral",
            cousinDegree: 0,
            removed: 0,
            alterGender: "male",
            seniority: "younger",
        },
        term: "nano bhai",
    },
];

/** Two unmarked entries at equal specificity that disagree. */
const DISAGREEING: PackEntry[] = [
    {
        when: { classification: "collateral", cousinDegree: 0, alterGender: "male" },
        term: "bhai",
    },
    {
        when: { classification: "collateral", removed: 0, alterGender: "male" },
        term: "bandhu",
    },
];

/** The same pair, agreeing — a legitimate many-to-one collapse. */
const AGREEING: PackEntry[] = DISAGREEING.map((entry) => ({ ...entry, term: "bhai" }));

function packOf(entries: PackEntry[]): LanguagePack {
    return {
        code: "xx",
        label: "fixture",
        entries,
        affixes: [],
        hops: HOPS,
        genitive: "{possessor}-no {possessed}",
    };
}

const sibling = descriptorOf({
    classification: collateral(1, 1),
    sharing: "full",
    side: "both",
    seniority: "unknown",
    apexSeniority: "unknown",
    path: [up("male"), down("male")],
});

describe("the entry-conflict scan", () => {
    it("catches a defect that only surfaces at `seniority: unknown`", () => {
        const pack = packOf([...MARKED, ...DISAGREEING]);
        const conflicts = findEntryConflicts(pack, enumerateDescriptors(BOUNDS, [pack]));
        expect(conflicts.join("\n")).toContain("bandhu | bhai");
    });

    it("is not fooled by the marked entries shadowing the defect", () => {
        // Sanity: at a *known* seniority the marked entry wins outright, so the
        // defect is genuinely invisible there. This is what made the hole
        // dangerous rather than merely theoretical.
        const pack = packOf([...MARKED, ...DISAGREEING]);
        const elder = { ...sibling, seniority: "elder" as const };
        expect(phrase(elder, pack).text).toBe("mota bhai");
        expect(phrase(sibling, pack).text).toBe("bandhu");
    });

    it("passes a tie whose entries agree", () => {
        const pack = packOf([...MARKED, ...AGREEING]);
        expect(findEntryConflicts(pack, enumerateDescriptors(BOUNDS, [pack]))).toEqual([]);
        expect(phrase(sibling, pack).text).toBe("bhai");
    });

    it("enumerates `seniority: unknown` for a pack that keys seniority", () => {
        const pack = packOf([...MARKED, ...AGREEING]);
        const seniorities = new Set(
            enumerateDescriptors(BOUNDS, [pack])
                .filter((d) => d.path.length > 0)
                .map((d) => d.seniority),
        );
        expect(seniorities).toContain("unknown");
        expect(seniorities).toContain("elder");
    });
});
