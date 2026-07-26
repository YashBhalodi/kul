import { describe, expect, it } from "vitest";

import type { LanguagePack } from "../../src/phrasing/pack.js";
import { EN } from "../../src/phrasing/packs/en.js";
import { phrase } from "../../src/phrasing/phrase.js";
import { across, collateral, descriptorOf, down, lineal, up } from "./fixtures.js";

/** A pack with a hop lexicon and a genitive pattern but no entries at all. */
const NO_TERMS: LanguagePack = {
    code: "xx",
    label: "fixture",
    entries: [],
    affixes: [],
    hops: {
        up: { male: "father", female: "mother", other: "parent" },
        down: { male: "son", female: "daughter", other: "child" },
        across: { male: "husband", female: "wife", other: "spouse" },
    },
    genitive: {
        male: "{possessor}-no {possessed}",
        female: "{possessor}-ni {possessed}",
        other: "{possessor}-nu {possessed}",
    },
};

describe("the compositional fallback", () => {
    it("takes the longest lexicalized prefix, not the first one it can find", () => {
        // `up·up·down·down·across` — a first cousin's wife. The two-hop prefix
        // lexicalizes too ("grandfather"), and a flat walk from ego would
        // spell all five hops. Neither is the answer.
        const cousinsWife = descriptorOf({
            classification: collateral(2, 2),
            affinity: "inLaw",
            side: "maternal",
            path: [up("female"), up("male"), down("female"), down("male"), across("female")],
        });
        const result = phrase(cousinsWife, EN);
        expect(result).toEqual({
            text: "first cousin's wife",
            kind: "composed",
            hopCount: 1,
        });
        expect(result.text).not.toContain("grandfather");
    });

    it("keeps walking back when the longest prefix has no term", () => {
        // `across·up·up` — a spouse's grandparent. The two-hop prefix is the
        // first one English names, so the head is "mother-in-law".
        const spousesGrandparent = descriptorOf({
            classification: lineal("ancestor", 2),
            affinity: "inLaw",
            path: [across("male"), up("female"), up("male")],
        });
        expect(phrase(spousesGrandparent, EN)).toEqual({
            text: "mother-in-law's father",
            kind: "composed",
            hopCount: 1,
        });
    });

    it("reports the tail length as `hopCount`, so chrome can size the slot", () => {
        const distant = descriptorOf({
            classification: collateral(5, 5),
            side: "maternal",
            path: [
                up("female"),
                up("male"),
                up("male"),
                up("male"),
                up("male"),
                down("male"),
                down("male"),
                down("male"),
                down("male"),
                down("female"),
            ],
        });
        // Fourth cousins are past English's ordinals, so the head is the
        // nearest term the language does have and one hop is spelled.
        const result = phrase(distant, EN);
        expect(result).toEqual({
            text: "third cousin once removed's daughter",
            kind: "composed",
            hopCount: 1,
        });
        expect(result.text.split("'s").length - 1).toBe(result.hopCount);
    });

    it("spells the whole backbone when a pack lexicalizes nothing", () => {
        const sibling = descriptorOf({
            classification: collateral(1, 1),
            sharing: "full",
            side: "both",
            path: [up("male"), down("female")],
        });
        expect(phrase(sibling, NO_TERMS)).toEqual({
            text: "father-ni daughter",
            kind: "composed",
            hopCount: 2,
        });
    });

    it("selects the genitive form by the possessed noun's gender", () => {
        const nephew = descriptorOf({
            classification: collateral(1, 2),
            side: "both",
            path: [up("female"), down("male"), down("other")],
        });
        expect(phrase(nephew, NO_TERMS).text).toBe("mother-no son-nu child");
    });

    it("marks a lexical phrase with zero hops and a composed one with at least one", () => {
        const mother = descriptorOf({
            classification: lineal("ancestor", 1),
            path: [up("female")],
        });
        expect(phrase(mother, EN)).toEqual({ text: "mother", kind: "lexical", hopCount: 0 });
        expect(phrase(mother, NO_TERMS)).toEqual({
            text: "mother",
            kind: "composed",
            hopCount: 1,
        });
    });
});
