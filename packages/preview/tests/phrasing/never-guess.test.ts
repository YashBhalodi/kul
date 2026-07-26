import { describe, expect, it } from "vitest";

import { subPathKeyOf } from "../../src/phrasing/key.js";
import type { LanguagePack } from "../../src/phrasing/pack.js";
import { EN } from "../../src/phrasing/packs/en.js";
import { phrase } from "../../src/phrasing/phrase.js";
import { collateral, descriptorOf, down, up } from "./fixtures.js";

/**
 * The never-guess rule: an entry is ineligible if any facet it keys is
 * `unknown` on the key, so lookup falls through to the unmarked, less specific
 * entry (ADR-0033).
 *
 * `en` keys neither seniority, so the rule is exercised here against a
 * fixture pack shaped like the Gujarati regions that need it — *moṭā bhāī* vs
 * *bhāī*, *moṭā bāpā* vs *kākā*. Those exact terms arrive with the `gu` pack;
 * what is under test is the machinery they will ride.
 */
const SENIORITY_PACK: LanguagePack = {
    code: "xx",
    label: "fixture",
    entries: [
        {
            when: {
                classification: "collateral",
                cousinDegree: 0,
                removed: 0,
                alterGender: "male",
            },
            term: "bhai",
        },
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
                up: 2,
                down: 1,
                alterGender: "male",
            },
            term: "kaka",
        },
        {
            when: {
                classification: "collateral",
                up: 2,
                down: 1,
                alterGender: "male",
                apexSeniority: "elder",
            },
            term: "mota bapa",
        },
    ],
    affixes: [],
    hops: {
        up: { male: "bapa", female: "ma", other: "vali" },
        down: { male: "dikro", female: "dikri", other: "santan" },
        across: { male: "pati", female: "patni", other: "jodidar" },
    },
    genitive: "{possessor}-no {possessed}",
};

function sibling(seniority: "elder" | "younger" | "unknown") {
    return descriptorOf({
        classification: collateral(1, 1),
        sharing: "full",
        side: "both",
        seniority,
        apexSeniority: seniority,
        path: [up("male"), down("male")],
    });
}

function parentsBrother(apexSeniority: "elder" | "unknown") {
    return descriptorOf({
        classification: collateral(2, 1),
        sharing: "full",
        side: "paternal",
        apexSeniority,
        path: [up("male"), up("male"), down("male")],
    });
}

describe("an unknown facet disqualifies the entry that keys it", () => {
    it("takes the marked term when the deciding facet is known", () => {
        expect(phrase(sibling("elder"), SENIORITY_PACK).text).toBe("mota bhai");
        expect(phrase(parentsBrother("elder"), SENIORITY_PACK).text).toBe("mota bapa");
    });

    it("falls through to the unmarked term when `seniority` is unknown", () => {
        expect(phrase(sibling("unknown"), SENIORITY_PACK).text).toBe("bhai");
    });

    it("falls through to the unmarked term when `apexSeniority` is unknown", () => {
        expect(phrase(parentsBrother("unknown"), SENIORITY_PACK).text).toBe("kaka");
    });

    it("still matches `notApplicable`, which is a value and not a gap", () => {
        const pack: LanguagePack = {
            ...SENIORITY_PACK,
            entries: [
                { when: { classification: "lineal", role: "ancestor" }, term: "elder" },
                {
                    when: { classification: "lineal", role: "ancestor", side: "notApplicable" },
                    term: "parent",
                },
            ],
        };
        const parent = descriptorOf({
            classification: { kind: "lineal", role: "ancestor", generations: 1 },
            path: [up("female")],
        });
        expect(phrase(parent, pack).text).toBe("parent");
    });
});

describe("a sub-path key marks what a hop sequence cannot know", () => {
    const nephewOfOtherGender = descriptorOf({
        classification: collateral(1, 2),
        sharing: "half",
        side: "both",
        seniority: "younger",
        apexSeniority: "younger",
        path: [up("male"), down("male"), down("other")],
    });

    it("marks sharing, both seniorities and the `both`-capable side unknown", () => {
        const key = subPathKeyOf(nephewOfOtherGender, 2);
        expect(key).toMatchObject({
            sharing: "unknown",
            seniority: "unknown",
            apexSeniority: "unknown",
            side: "unknown",
            classification: "collateral",
            affinity: "blood",
            alterGender: "male",
        });
    });

    it("keeps a derivable side rather than blanking it", () => {
        const cousin = descriptorOf({
            classification: collateral(2, 2),
            side: "paternal",
            path: [up("male"), up("female"), down("male"), down("other")],
        });
        expect(subPathKeyOf(cousin, 3).side).toBe("paternal");
    });

    it("disqualifies exactly the entry that would have been wrong", () => {
        // The whole-path descriptor says `sharing: half`, but the prefix
        // `up·down` is a hop sequence — it cannot see the parent sets — so the
        // half-sibling entry is out and the head is the unmarked "brother".
        expect(phrase(nephewOfOtherGender, EN)).toMatchObject({
            text: "brother's child",
            kind: "composed",
            hopCount: 1,
        });
    });
});
