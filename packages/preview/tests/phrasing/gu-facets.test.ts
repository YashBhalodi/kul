import { describe, expect, it } from "vitest";

import type { PhrasingKey } from "../../src/phrasing/key.js";
import type { LanguagePack, PackEntry } from "../../src/phrasing/pack.js";
import { GU } from "../../src/phrasing/packs/gu.js";
import { phrase } from "../../src/phrasing/phrase.js";
import type { RelationshipDescriptor } from "../../src/phrasing/descriptor.js";
import { across, collateral, descriptorOf, down, up } from "./fixtures.js";

/**
 * Which facets in the affinal collision class are **load-bearing**, pinned by
 * removing them.
 *
 * `gu-terms.test.ts` exercises the pack as written, so it can only show that
 * the right term comes out — never that a key is what produced it. The
 * failure mode this file guards is the one that matters: a facet dropped as
 * redundant, and a path quietly answering with a **neighbouring term for a
 * different relationship**. Every row below is a wrong answer, not a coarse
 * one, and one of them fires only when a birth date is missing.
 */
function withoutFacets(
    pack: LanguagePack,
    term: string,
    facets: ReadonlyArray<keyof PhrasingKey>,
): LanguagePack {
    const entries: PackEntry[] = pack.entries.map((entry) => {
        if (entry.term !== term) {
            return entry;
        }
        const when = { ...entry.when };
        for (const facet of facets) {
            delete when[facet];
        }
        return { ...entry, when };
    });
    return { ...pack, entries };
}

/** A husband's brother's wife — *jeṭhāṇī* / *derāṇī*, split by `apexSeniority`. */
const HUSBANDS_BROTHERS_WIFE: RelationshipDescriptor = descriptorOf({
    classification: collateral(1, 1),
    affinity: "inLaw",
    apexSeniority: "unknown",
    path: [across("male"), up("male"), down("male"), across("female")],
});

/** A co-spouse's brother: `across·across·up·down`, two marriages out. */
const CO_SPOUSES_BROTHER: RelationshipDescriptor = descriptorOf({
    classification: collateral(1, 1),
    affinity: "inLaw",
    egoGender: "male",
    path: [across("female"), across("male"), up("male"), down("male")],
});

/** A child's spouse's mother — *vevāṇ*, the shape with an `across` at neither end. */
const CHILDS_PARENT_IN_LAW: RelationshipDescriptor = descriptorOf({
    classification: collateral(1, 1),
    affinity: "inLaw",
    path: [down("male"), across("male"), up("female")],
});

const CASES: ReadonlyArray<{
    name: string;
    descriptor: RelationshipDescriptor;
    entry: string;
    drop: ReadonlyArray<keyof PhrasingKey>;
    wrong: string;
}> = [
    {
        // The sharpest one: `apexSeniority: unknown` is what a missing birth
        // date yields — the most common real descriptor there is — so
        // *jeṭhāṇī* and *derāṇī* are both disqualified and a weakened *naṇand*
        // becomes the winner. A husband's brother's **wife**, answered with
        // the word for his **sister**.
        name: "naṇand without `acrossCount` + `acrossAtEnd` swallows jeṭhāṇī at unknown seniority",
        descriptor: HUSBANDS_BROTHERS_WIFE,
        entry: "નણંદ",
        drop: ["acrossCount", "acrossAtEnd"],
        wrong: "નણંદ",
    },
    {
        name: "sāḷo without `acrossCount` reaches a co-spouse's brother",
        descriptor: CO_SPOUSES_BROTHER,
        entry: "સાળો",
        drop: ["acrossCount"],
        wrong: "સાળો",
    },
    {
        name: "sāḷī without `acrossCount` reaches a co-spouse's sister",
        descriptor: descriptorOf({
            classification: collateral(1, 1),
            affinity: "inLaw",
            egoGender: "male",
            path: [across("female"), across("male"), up("male"), down("female")],
        }),
        entry: "સાળી",
        drop: ["acrossCount"],
        wrong: "સાળી",
    },
    {
        name: "jeṭh without `acrossCount` reaches a co-spouse's elder brother",
        descriptor: descriptorOf({
            classification: collateral(1, 1),
            affinity: "inLaw",
            apexSeniority: "elder",
            path: [across("male"), across("male"), up("male"), down("male")],
        }),
        entry: "જેઠ",
        drop: ["acrossCount"],
        wrong: "જેઠ",
    },
    {
        // *bhābhī* and *vevāṇ* differ only in where the single `across` sits.
        name: "bhābhī without `acrossAtEnd` swallows a child's parent-in-law",
        descriptor: CHILDS_PARENT_IN_LAW,
        entry: "ભાભી",
        drop: ["acrossAtEnd"],
        wrong: "ભાભી",
    },
    {
        name: "banevī without `acrossAtEnd` swallows the same cell from the other side",
        descriptor: descriptorOf({
            classification: collateral(1, 1),
            affinity: "inLaw",
            path: [down("female"), across("female"), up("male")],
        }),
        entry: "બનેવી",
        drop: ["acrossAtEnd"],
        wrong: "બનેવી",
    },
    {
        name: "vevāī without `acrossAtEnd` swallows a sibling's husband",
        descriptor: descriptorOf({
            classification: collateral(1, 1),
            affinity: "inLaw",
            sharing: "full",
            side: "both",
            path: [up("male"), down("male"), across("male")],
        }),
        entry: "વેવાઈ",
        drop: ["acrossAtEnd"],
        wrong: "વેવાઈ",
    },
];

describe("the `gu` pack's affinal facets", () => {
    it.each(CASES.map((c) => [c.name, c] as const))("%s", (_name, testCase) => {
        const weakened = withoutFacets(GU, testCase.entry, testCase.drop);
        expect(phrase(testCase.descriptor, weakened).text).toBe(testCase.wrong);
        // …and the pack as shipped does not say that.
        expect(phrase(testCase.descriptor, GU).text).not.toBe(testCase.wrong);
    });

    it("phrases the husband's brother's wife descriptively when seniority is unknown", () => {
        // The honest answer the never-guess rule leaves: no term, so the
        // fallback spells it. Pinned as text because "not naṇand" alone would
        // pass on any other wrong word.
        expect(phrase(HUSBANDS_BROTHERS_WIFE, GU)).toMatchObject({
            text: "સસરોનો પુત્રની પત્ની",
            kind: "composed",
            hopCount: 2,
        });
    });

    it("leaves the two-marriage chains it has no word for to compose", () => {
        for (const descriptor of [CO_SPOUSES_BROTHER, CHILDS_PARENT_IN_LAW]) {
            expect(phrase(descriptor, GU).text).not.toBe("સાળો");
        }
        expect(phrase(CO_SPOUSES_BROTHER, GU).kind).toBe("composed");
    });
});
