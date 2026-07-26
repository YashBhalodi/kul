import { describe, expect, it } from "vitest";

import { phrasingKeyOf } from "../../src/phrasing/key.js";
import { specificityOf, winningEntries } from "../../src/phrasing/lexicalize.js";
import { EN } from "../../src/phrasing/packs/en.js";
import { phrase } from "../../src/phrasing/phrase.js";
import type { Phrase } from "../../src/phrasing/phrase.js";
import type { RelationshipDescriptor } from "../../src/phrasing/descriptor.js";
import { across, collateral, descriptorOf, down, lineal, self, up } from "./fixtures.js";

/**
 * Discrimination fixtures for the `en` pack: one row per region of
 * `docs/kinship-term-inventory.md` §9, including the regions where English
 * *under*-specifies (aunt covers an uncle's wife) and the regions where it has
 * no lexeme at all (a co-wife, a child's parent-in-law, a spouse's uncle).
 */
const CASES: ReadonlyArray<[string, RelationshipDescriptor, Partial<Phrase>]> = [
    // --- the lineal spine, carried by the affix rules --------------------
    [
        "mother",
        descriptorOf({ classification: lineal("ancestor", 1), path: [up("female")] }),
        { text: "mother", kind: "lexical", hopCount: 0 },
    ],
    [
        "grandfather",
        descriptorOf({
            classification: lineal("ancestor", 2),
            side: "paternal",
            path: [up("male"), up("male")],
        }),
        { text: "grandfather", kind: "lexical" },
    ],
    [
        "great-grandmother at generation 3",
        descriptorOf({
            classification: lineal("ancestor", 3),
            side: "maternal",
            path: [up("female"), up("female"), up("female")],
        }),
        { text: "great-grandmother", kind: "lexical" },
    ],
    [
        "great-great-grandmother at generation 4 — `great-` repeats, `grand` does not",
        descriptorOf({
            classification: lineal("ancestor", 4),
            side: "maternal",
            path: [up("female"), up("female"), up("female"), up("female")],
        }),
        { text: "great-great-grandmother", kind: "lexical" },
    ],
    [
        "grandson",
        descriptorOf({
            classification: lineal("descendant", 2),
            path: [down("female"), down("male")],
        }),
        { text: "grandson", kind: "lexical" },
    ],

    // --- siblings ---------------------------------------------------------
    [
        "sister",
        descriptorOf({
            classification: collateral(1, 1),
            sharing: "full",
            side: "both",
            path: [up("male"), down("female")],
        }),
        { text: "sister", kind: "lexical" },
    ],
    [
        "half-sister — `sharing` is the only difference",
        descriptorOf({
            classification: collateral(1, 1),
            sharing: "half",
            side: "paternal",
            path: [up("male"), down("female")],
        }),
        { text: "half-sister", kind: "lexical" },
    ],
    [
        "stepbrother — a step-parent's child",
        descriptorOf({
            classification: collateral(1, 1),
            affinity: "step",
            side: "maternal",
            path: [up("female"), across("male"), down("male")],
        }),
        { text: "stepbrother", kind: "lexical" },
    ],

    // --- the aunt / uncle tier: English merges blood and in-law -----------
    [
        "uncle",
        descriptorOf({
            classification: collateral(2, 1),
            side: "paternal",
            path: [up("male"), up("female"), down("male")],
        }),
        { text: "uncle", kind: "lexical" },
    ],
    [
        "aunt — the uncle's wife is 'aunt' too, so no entry keys `affinity`",
        descriptorOf({
            classification: collateral(2, 1),
            affinity: "inLaw",
            side: "paternal",
            path: [up("male"), up("female"), down("male"), across("female")],
        }),
        { text: "aunt", kind: "lexical" },
    ],
    [
        "great-aunt — one more ascent, via the collateral `great-` rule",
        descriptorOf({
            classification: collateral(3, 1),
            side: "paternal",
            path: [up("male"), up("male"), up("male"), down("female")],
        }),
        { text: "great-aunt", kind: "lexical" },
    ],
    [
        "a parent's sibling of `other` gender — English has no lexeme, but the composed form would read as a *parent*",
        descriptorOf({
            classification: collateral(2, 1),
            side: "paternal",
            path: [up("male"), up("female"), down("other")],
        }),
        { text: "parent's sibling", kind: "lexical" },
    ],
    [
        "an `other`-gender parent's sibling by marriage is named the same way",
        descriptorOf({
            classification: collateral(2, 1),
            affinity: "inLaw",
            side: "paternal",
            path: [up("male"), up("female"), down("male"), across("other")],
        }),
        { text: "parent's sibling", kind: "lexical" },
    ],
    [
        "a spouse's uncle has no English term and composes",
        descriptorOf({
            classification: collateral(2, 1),
            affinity: "inLaw",
            path: [across("female"), up("male"), up("female"), down("male")],
        }),
        { text: "father-in-law's mother's son", kind: "composed", hopCount: 2 },
    ],

    // --- nephews and nieces ------------------------------------------------
    [
        "niece",
        descriptorOf({
            classification: collateral(1, 2),
            side: "both",
            path: [up("male"), down("male"), down("female")],
        }),
        { text: "niece", kind: "lexical" },
    ],
    [
        "great-nephew",
        descriptorOf({
            classification: collateral(1, 3),
            side: "both",
            path: [up("male"), down("male"), down("female"), down("male")],
        }),
        { text: "great-nephew", kind: "lexical" },
    ],
    [
        "a nephew's wife is not a niece — English composes here",
        descriptorOf({
            classification: collateral(1, 2),
            affinity: "inLaw",
            path: [up("male"), down("male"), down("male"), across("female")],
        }),
        { text: "nephew's wife", kind: "composed", hopCount: 1 },
    ],

    // --- cousins, keyed on the materialized numbers -----------------------
    [
        "first cousin",
        descriptorOf({
            classification: collateral(2, 2),
            side: "maternal",
            path: [up("female"), up("male"), down("female"), down("male")],
        }),
        { text: "first cousin", kind: "lexical" },
    ],
    [
        "first cousin once removed",
        descriptorOf({
            classification: collateral(2, 3),
            side: "maternal",
            path: [up("female"), up("male"), down("female"), down("male"), down("male")],
        }),
        { text: "first cousin once removed", kind: "lexical" },
    ],
    [
        "second cousin",
        descriptorOf({
            classification: collateral(3, 3),
            side: "maternal",
            path: [
                up("female"),
                up("male"),
                up("male"),
                down("female"),
                down("male"),
                down("male"),
            ],
        }),
        { text: "second cousin", kind: "lexical" },
    ],
    [
        "a first cousin's wife composes off the cousin — the longest lexicalized prefix",
        descriptorOf({
            classification: collateral(2, 2),
            affinity: "inLaw",
            path: [
                up("female"),
                up("male"),
                down("female"),
                down("male"),
                across("female"),
            ],
        }),
        { text: "first cousin's wife", kind: "composed", hopCount: 1 },
    ],

    // --- spouse and the affinal heartland ---------------------------------
    [
        "wife",
        descriptorOf({
            classification: self,
            affinity: "inLaw",
            seniority: "younger",
            path: [across("female")],
        }),
        { text: "wife", kind: "lexical" },
    ],
    [
        "a co-wife composes: English has no term for two `across` hops",
        descriptorOf({
            classification: self,
            affinity: "inLaw",
            path: [across("male"), across("female")],
        }),
        { text: "husband's wife", kind: "composed", hopCount: 1 },
    ],
    [
        "father-in-law",
        descriptorOf({
            classification: lineal("ancestor", 1),
            affinity: "inLaw",
            path: [across("female"), up("male")],
        }),
        { text: "father-in-law", kind: "lexical" },
    ],
    [
        "a spouse's grandmother composes — English stops in-law marking at one generation",
        descriptorOf({
            classification: lineal("ancestor", 2),
            affinity: "inLaw",
            path: [across("female"), up("male"), up("female")],
        }),
        { text: "father-in-law's mother", kind: "composed", hopCount: 1 },
    ],
    [
        "stepmother — `up·across` puts the marriage hop in ancestor position",
        descriptorOf({
            classification: lineal("ancestor", 1),
            affinity: "step",
            side: "paternal",
            path: [up("male"), across("female")],
        }),
        { text: "stepmother", kind: "lexical" },
    ],
    [
        "a step-grandmother composes — the term is generation-1 only",
        descriptorOf({
            classification: lineal("ancestor", 2),
            affinity: "step",
            side: "paternal",
            path: [up("male"), up("male"), across("female")],
        }),
        { text: "grandfather's wife", kind: "composed", hopCount: 1 },
    ],
    [
        "stepson (`across·down`) and son-in-law (`down·across`) share a normalized cell",
        descriptorOf({
            classification: lineal("descendant", 1),
            affinity: "inLaw",
            path: [across("female"), down("male")],
        }),
        { text: "stepson", kind: "lexical" },
    ],
    [
        "son-in-law — same cell, the other backbone",
        descriptorOf({
            classification: lineal("descendant", 1),
            affinity: "inLaw",
            path: [down("female"), across("male")],
        }),
        { text: "son-in-law", kind: "lexical" },
    ],
    [
        "brother-in-law from the spouse's side",
        descriptorOf({
            classification: collateral(1, 1),
            affinity: "inLaw",
            path: [across("female"), up("male"), down("male")],
        }),
        { text: "brother-in-law", kind: "lexical" },
    ],
    [
        "brother-in-law from the sibling's side — the same term, the mirrored backbone",
        descriptorOf({
            classification: collateral(1, 1),
            affinity: "inLaw",
            side: "both",
            path: [up("male"), down("female"), across("male")],
        }),
        { text: "brother-in-law", kind: "lexical" },
    ],
    [
        "a spouse's sibling's spouse matches both in-law entries, which agree",
        descriptorOf({
            classification: collateral(1, 1),
            affinity: "inLaw",
            path: [across("female"), up("male"), down("female"), across("male")],
        }),
        { text: "brother-in-law", kind: "lexical" },
    ],
    [
        "a child's parent-in-law shares that cell and has no English term",
        descriptorOf({
            classification: collateral(1, 1),
            affinity: "inLaw",
            path: [down("female"), across("male"), up("male")],
        }),
        { text: "son-in-law's father", kind: "composed", hopCount: 1 },
    ],

    // --- ended marriages ride the affix vocabulary ------------------------
    [
        "former mother-in-law",
        descriptorOf({
            classification: lineal("ancestor", 1),
            affinity: "inLaw",
            path: [across("male", "ended"), up("female")],
        }),
        { text: "former mother-in-law", kind: "lexical" },
    ],
    [
        "former wife",
        descriptorOf({
            classification: self,
            affinity: "inLaw",
            path: [across("female", "ended")],
        }),
        { text: "former wife", kind: "lexical" },
    ],

    // --- adoption renders unmarked ----------------------------------------
    [
        "an adoptive mother is 'mother'",
        descriptorOf({
            classification: lineal("ancestor", 1),
            edgeNature: "adoptive",
            path: [up("female", "adoptive")],
        }),
        { text: "mother", kind: "lexical" },
    ],
    [
        "an adopted grandson is 'grandson'",
        descriptorOf({
            classification: lineal("descendant", 2),
            edgeNature: "adoptive",
            path: [down("male", "adoptive"), down("male")],
        }),
        { text: "grandson", kind: "lexical" },
    ],
];

describe("the `en` pack", () => {
    it.each(CASES.map((c) => [c[0], c[1], c[2]] as const))("phrases %s", (_name, descriptor, expected) => {
        expect(phrase(descriptor, EN)).toMatchObject(expected);
    });

    it("ties two entries on the spouse's sibling's spouse, and they agree", () => {
        // The relaxation ADR-0039 makes to ADR-0033's tie rule, exercised
        // rather than asserted: this key matches the `acrossAtStart` entry and
        // the `acrossAtEnd` entry at equal specificity, and they say the same
        // word. Breaking the tie (by keying `acrossCount` on one side) would
        // retire the case silently, so it is pinned here.
        const spousesSiblingsSpouse = descriptorOf({
            classification: collateral(1, 1),
            affinity: "inLaw",
            path: [across("female"), up("male"), down("female"), across("male")],
        });
        const winners = winningEntries(phrasingKeyOf(spousesSiblingsSpouse), EN.entries);
        expect(winners).toHaveLength(2);
        expect(new Set(winners.map((entry) => entry.term))).toEqual(new Set(["brother-in-law"]));
        expect(new Set(winners.map((entry) => specificityOf(entry.when)))).toHaveLength(1);
    });

    it("phrases bare — no ego-relative framing", () => {
        for (const [, descriptor] of CASES) {
            expect(phrase(descriptor, EN).text).not.toMatch(/\byour\b/);
        }
    });
});
