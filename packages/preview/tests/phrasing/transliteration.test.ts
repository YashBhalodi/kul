// Per-term transliteration (#302, ADR-0044).
//
// The rule under test is the never-guess rule wearing a second hat: a Latin
// reading is a *record* in the pack, never something the layer computes from
// the script, so the interesting cases are the ones where a record is missing
// and the answer has to be "no gloss" rather than "a partial one".

import { describe, expect, it } from "vitest";

import { EN } from "../../src/phrasing/packs/en.js";
import { GU } from "../../src/phrasing/packs/gu.js";
import type { LanguagePack } from "../../src/phrasing/pack.js";
import { phrase } from "../../src/phrasing/phrase.js";
import { collateral, descriptorOf, down, lineal, up } from "./fixtures.js";

/** *kākā* — a crisp lexical term in a script a reader may not read. */
const KAKA = descriptorOf({
    classification: collateral(2, 1),
    side: "paternal",
    alterGender: "male",
    path: [up("male"), up("male"), down("male")],
});

/** *māmā-no putra* — a composed chain: head entry + hop noun + genitive. */
const MAMA_NO_PUTRA = descriptorOf({
    classification: collateral(2, 2),
    side: "maternal",
    alterGender: "male",
    path: [up("female"), up("male"), down("male"), down("male")],
});

/** *par-dādā* — an entry decorated by the pack's one affix rule. */
const PAR_DADA = descriptorOf({
    classification: lineal("ancestor", 3),
    side: "paternal",
    path: [up("male"), up("male"), up("male")],
});

/** A pack stripped of one kind of Latin record, to watch the gloss withdraw. */
function without(part: "entries" | "affixes" | "hops" | "genitive"): LanguagePack {
    return {
        ...GU,
        entries:
            part === "entries"
                ? GU.entries.map(({ when, term }) => ({ when, term }))
                : GU.entries,
        affixes:
            part === "affixes"
                ? GU.affixes.map(({ translit: _drop, ...rule }) => rule)
                : GU.affixes,
        hopsTranslit: part === "hops" ? undefined : GU.hopsTranslit,
        genitiveTranslit: part === "genitive" ? undefined : GU.genitiveTranslit,
    };
}

describe("a pack that romanizes glosses what it renders", () => {
    it("carries the Latin reading of a lexical term", () => {
        expect(phrase(KAKA, GU)).toMatchObject({
            text: "કાકા",
            kind: "lexical",
            translit: "kākā",
        });
    });

    it("builds the Latin reading of a composed chain from the same records", () => {
        const result = phrase(MAMA_NO_PUTRA, GU);
        expect(result.kind).toBe("composed");
        expect(result.text).toBe("મામાનો પુત્ર");
        expect(result.translit).toBe("māmā-no putra");
    });

    it("decorates the Latin reading with the affix that fired", () => {
        expect(phrase(PAR_DADA, GU)).toMatchObject({
            text: "પરદાદા",
            translit: "par-dādā",
        });
    });
});

describe("a missing Latin record withdraws the gloss rather than inventing one", () => {
    it("offers no transliteration for a language already written in Latin", () => {
        // Not an omission: an English term is its own reading, and a gloss
        // repeating it would be noise on every hover.
        expect(phrase(KAKA, EN).translit).toBeUndefined();
        expect(phrase(MAMA_NO_PUTRA, EN).translit).toBeUndefined();
    });

    it.each([
        ["entries", KAKA],
        ["entries", MAMA_NO_PUTRA],
        ["affixes", PAR_DADA],
        ["hops", MAMA_NO_PUTRA],
        ["genitive", MAMA_NO_PUTRA],
    ] as const)("drops it when the pack lacks a Latin %s", (part, descriptor) => {
        const stripped = phrase(descriptor, without(part));
        // The phrase itself is untouched — only its second reading is gone.
        expect(stripped.text).toBe(phrase(descriptor, GU).text);
        expect(stripped.translit).toBeUndefined();
    });

    it("reads one phrase twice rather than looking one up twice", () => {
        // The two readings are folded by the same walk over the same winning
        // records, so they agree on which head the fallback picked: *māmā*
        // heads the script form iff *māmā* heads the Latin one. A second
        // lookup over a parallel table is exactly what could drift here.
        const result = phrase(MAMA_NO_PUTRA, GU);
        expect(result.text.startsWith("મામા")).toBe(true);
        expect(result.translit?.startsWith("māmā")).toBe(true);
        expect(result.hopCount).toBe(1);
    });
});
