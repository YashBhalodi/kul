import { describe, expect, it } from "vitest";

import type { AffixRule, LanguagePack, PackEntry } from "../../src/phrasing/pack.js";
import { EN } from "../../src/phrasing/packs/en.js";
import { phrase } from "../../src/phrasing/phrase.js";
import { collateral, descriptorOf, down, lineal, up } from "./fixtures.js";

const HOPS = {
    up: { male: "father", female: "mother", other: "parent" },
    down: { male: "son", female: "daughter", other: "child" },
    across: { male: "husband", female: "wife", other: "spouse" },
} as const;

function packOf(entries: PackEntry[], affixes: AffixRule[]): LanguagePack {
    return {
        code: "xx",
        label: "fixture",
        entries,
        affixes,
        hops: HOPS,
        genitive: "{possessor}'s {possessed}",
    };
}

function ancestor(generations: number) {
    return descriptorOf({
        classification: lineal("ancestor", generations),
        side: generations > 1 ? "paternal" : "notApplicable",
        path: Array.from({ length: generations }, () => up("male")),
    });
}

/**
 * Productive morphology, as data: `repeatPer` counts from the floor, `cap`
 * refuses past the language's ceiling, and rule order is derived, never
 * declared.
 */
describe("affix rules", () => {
    it("repeats once per unit at and above the floor", () => {
        expect(phrase(ancestor(1), EN).text).toBe("father");
        expect(phrase(ancestor(2), EN).text).toBe("grandfather");
        expect(phrase(ancestor(3), EN).text).toBe("great-grandfather");
        expect(phrase(ancestor(4), EN).text).toBe("great-great-grandfather");
        expect(phrase(ancestor(6), EN).text).toBe("great-great-great-great-grandfather");
    });

    it("refuses past `cap`, so the fallback fires rather than a coined word", () => {
        // The Gujarati shape: `par-` is productive for exactly one generation
        // past *dādā*, and generation 4 has no term at all.
        const gujaratiShaped = packOf(
            [{ when: { classification: "lineal", role: "ancestor", alterGender: "male" }, term: "dada" }],
            [
                {
                    when: {
                        classification: "lineal",
                        atLeast: { facet: "generations", value: 3 },
                    },
                    affix: "par-",
                    position: "prefix",
                    cap: 3,
                },
            ],
        );
        expect(phrase(ancestor(2), gujaratiShaped).text).toBe("dada");
        expect(phrase(ancestor(3), gujaratiShaped).text).toBe("par-dada");
        // Refusing hands the key to the fallback, which lexicalizes the
        // longest prefix the cap still allows and spells the rest.
        expect(phrase(ancestor(4), gujaratiShaped)).toEqual({
            text: "par-dada's father",
            kind: "composed",
            hopCount: 1,
        });
    });

    it("applies suffixes and prefixes to the same term", () => {
        const pack = packOf(
            [{ when: { classification: "collateral" }, term: "sibling" }],
            [
                { when: { affinity: "inLaw" }, affix: "-in-law", position: "suffix" },
                { when: { endedMarriage: true }, affix: "former ", position: "prefix" },
            ],
        );
        const formerSiblingInLaw = descriptorOf({
            classification: collateral(1, 1),
            affinity: "inLaw",
            path: [up("male"), down("female"), { step: "across", to: "s", gender: "male", marriage: "m", status: "ended" }],
        });
        expect(phrase(formerSiblingInLaw, pack).text).toBe("former sibling-in-law");
    });

    it("nests by threshold, not by declaration order", () => {
        const forwards = packOf(
            [{ when: { classification: "lineal", role: "ancestor" }, term: "father" }],
            [
                {
                    when: { classification: "lineal", atLeast: { facet: "generations", value: 2 } },
                    affix: "grand",
                    position: "prefix",
                },
                {
                    when: { classification: "lineal", atLeast: { facet: "generations", value: 3 } },
                    affix: "great-",
                    position: "prefix",
                    repeatPer: "generations",
                },
            ],
        );
        const backwards = packOf([...forwards.entries], [...forwards.affixes].reverse());
        expect(phrase(ancestor(4), forwards).text).toBe("great-great-grandfather");
        expect(phrase(ancestor(4), backwards).text).toBe("great-great-grandfather");
    });

    it("stays silent when a facet it keys is unknown on the key", () => {
        // The `up·down` prefix of a nephew path: `sharing` is unknowable from
        // hops, so a rule keying it cannot fire on the composed head.
        const pack = packOf(
            [{ when: { classification: "collateral", alterGender: "male" }, term: "brother" }],
            [{ when: { sharing: "half" }, affix: "half-", position: "prefix" }],
        );
        const nephew = descriptorOf({
            classification: collateral(1, 2),
            sharing: "half",
            path: [up("male"), down("male"), down("other")],
        });
        expect(phrase(nephew, pack).text).toBe("brother's child");
    });
});

describe("entry precedence", () => {
    it("takes the most specific match regardless of declaration order", () => {
        const coarse: PackEntry = {
            when: { classification: "collateral", alterGender: "male" },
            term: "cousin",
        };
        const specific: PackEntry = {
            when: {
                classification: "collateral",
                alterGender: "male",
                side: "maternal",
                cousinDegree: 1,
            },
            term: "masiyai",
        };
        const cousin = descriptorOf({
            classification: collateral(2, 2),
            side: "maternal",
            path: [up("female"), up("male"), down("female"), down("male")],
        });
        expect(phrase(cousin, packOf([coarse, specific], [])).text).toBe("masiyai");
        expect(phrase(cousin, packOf([specific, coarse], [])).text).toBe("masiyai");
    });

    it("is deterministic — not order-dependent — even for a tie", () => {
        const a: PackEntry = { when: { classification: "lineal" }, term: "alpha" };
        const b: PackEntry = { when: { classification: "lineal" }, term: "beta" };
        const parent = descriptorOf({
            classification: lineal("ancestor", 1),
            path: [up("female")],
        });
        expect(phrase(parent, packOf([a, b], [])).text).toBe(
            phrase(parent, packOf([b, a], [])).text,
        );
    });
});
