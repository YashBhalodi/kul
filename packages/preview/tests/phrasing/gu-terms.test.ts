import { describe, expect, it } from "vitest";

import { GU } from "../../src/phrasing/packs/gu.js";
import { phrase } from "../../src/phrasing/phrase.js";
import type { Phrase } from "../../src/phrasing/phrase.js";
import type { RelationshipDescriptor } from "../../src/phrasing/descriptor.js";
import { across, collateral, descriptorOf, down, lineal, self, up } from "./fixtures.js";

/**
 * Discrimination fixtures for the `gu` pack.
 *
 * Every ⚠-marked row of `docs/kinship-term-inventory.md` — every term the
 * descriptor's **normalized fields could not select** — has a case here, plus
 * the two rows that inherit a ⚠ from the row above them (*sāḷī* from *sāḷo*,
 * *diyar* from *jeṭh*). Twenty-two terms in all, and they are the reason the
 * six derived facets exist: `spouseGender`, `linkGender`, `acrossAtStart`,
 * `acrossAtEnd`, `acrossCount` and the two seniorities carry the whole load,
 * and not one of them required a line of phrasing logic to key.
 *
 * The rest of the file pins what the inventory validates rather than flags:
 * the parents'-siblings tier (normalized fields alone), the `par-` cap, the
 * never-guess rule where Gujarati is seniority-split, and the three ADR-0033
 * terminology policies.
 */
const CASES: ReadonlyArray<[string, RelationshipDescriptor, Partial<Phrase>]> = [
    // === §1 lineal ancestors, and the `par-` cap =========================
    [
        "mā",
        descriptorOf({ classification: lineal("ancestor", 1), path: [up("female")] }),
        { text: "મા", kind: "lexical", hopCount: 0 },
    ],
    [
        "pappā",
        descriptorOf({ classification: lineal("ancestor", 1), path: [up("male")] }),
        { text: "પપ્પા", kind: "lexical" },
    ],
    [
        "dādā — `side` alone splits the grandparent pair Gujarati keeps apart",
        descriptorOf({
            classification: lineal("ancestor", 2),
            side: "paternal",
            path: [up("male"), up("male")],
        }),
        { text: "દાદા", kind: "lexical" },
    ],
    [
        "nānī — the same cell on the mother's side",
        descriptorOf({
            classification: lineal("ancestor", 2),
            side: "maternal",
            path: [up("female"), up("female")],
        }),
        { text: "નાની", kind: "lexical" },
    ],
    [
        "par-dādā at generation 3 — the pack's one affix rule",
        descriptorOf({
            classification: lineal("ancestor", 3),
            side: "paternal",
            path: [up("male"), up("male"), up("male")],
        }),
        { text: "પરદાદા", kind: "lexical" },
    ],
    [
        "par-nānī at generation 3",
        descriptorOf({
            classification: lineal("ancestor", 3),
            side: "maternal",
            path: [up("female"), up("female"), up("female")],
        }),
        { text: "પરનાની", kind: "lexical" },
    ],
    [
        "generation 4 — `cap: 3` refuses, so the fallback answers off par-dādā",
        descriptorOf({
            classification: lineal("ancestor", 4),
            side: "paternal",
            path: [up("male"), up("male"), up("male"), up("male")],
        }),
        { text: "પરદાદાનો પિતા", kind: "composed", hopCount: 1 },
    ],

    // === §2 lineal descendants — ⚠ the linking child's gender ============
    [
        "dīkro",
        descriptorOf({ classification: lineal("descendant", 1), path: [down("male")] }),
        { text: "દીકરો", kind: "lexical" },
    ],
    [
        "⚠ pautra — the son's son; `side` is notApplicable, `linkGender` is male",
        descriptorOf({
            classification: lineal("descendant", 2),
            path: [down("male"), down("male")],
        }),
        { text: "પૌત્ર", kind: "lexical" },
    ],
    [
        "⚠ pautrī — the son's daughter",
        descriptorOf({
            classification: lineal("descendant", 2),
            path: [down("male"), down("female")],
        }),
        { text: "પૌત્રી", kind: "lexical" },
    ],
    [
        "⚠ dohitra — the daughter's son; one normalized cell, the other term",
        descriptorOf({
            classification: lineal("descendant", 2),
            path: [down("female"), down("male")],
        }),
        { text: "દોહિત્ર", kind: "lexical" },
    ],
    [
        "⚠ dohitrī — the daughter's daughter",
        descriptorOf({
            classification: lineal("descendant", 2),
            path: [down("female"), down("female")],
        }),
        { text: "દોહિત્રી", kind: "lexical" },
    ],
    [
        "⚠ prapautra — generation 3, lexicalized on the son's line only",
        descriptorOf({
            classification: lineal("descendant", 3),
            path: [down("male"), down("male"), down("male")],
        }),
        { text: "પ્રપૌત્ર", kind: "lexical" },
    ],
    [
        "the daughter's line has no generation-3 term, so it composes off dohitra",
        descriptorOf({
            classification: lineal("descendant", 3),
            path: [down("female"), down("male"), down("male")],
        }),
        { text: "દોહિત્રનો પુત્ર", kind: "composed", hopCount: 1 },
    ],

    // === §3 siblings — seniority as entries, savkā as a many-to-one ======
    [
        "bhāī — `seniority: unknown` yields the unmarked term, never a guess",
        descriptorOf({
            classification: collateral(1, 1),
            sharing: "full",
            side: "both",
            seniority: "unknown",
            path: [up("male"), down("male")],
        }),
        { text: "ભાઈ", kind: "lexical" },
    ],
    [
        "moṭā bhāī — an ordinary entry keying `seniority`, not an affix rule",
        descriptorOf({
            classification: collateral(1, 1),
            sharing: "full",
            side: "both",
            seniority: "elder",
            path: [up("male"), down("male")],
        }),
        { text: "મોટા ભાઈ", kind: "lexical" },
    ],
    [
        "nānī bahen",
        descriptorOf({
            classification: collateral(1, 1),
            sharing: "full",
            side: "both",
            seniority: "younger",
            path: [up("male"), down("female")],
        }),
        { text: "નાની બહેન", kind: "lexical" },
    ],
    [
        "savkā bhāī from `sharing: half` — half of the many-to-one collapse",
        descriptorOf({
            classification: collateral(1, 1),
            sharing: "half",
            side: "paternal",
            path: [up("male"), down("male")],
        }),
        { text: "સાવકા ભાઈ", kind: "lexical" },
    ],
    [
        "savkā bhāī from `affinity: step` — the other half, one term",
        descriptorOf({
            classification: collateral(1, 1),
            affinity: "step",
            side: "maternal",
            path: [up("female"), across("male"), down("male")],
        }),
        { text: "સાવકા ભાઈ", kind: "lexical" },
    ],

    // === §4 the parents'-siblings tier — normalized fields alone =========
    [
        "kākā — `apexSeniority: unknown` falls through to the unmarked term",
        descriptorOf({
            classification: collateral(2, 1),
            side: "paternal",
            apexSeniority: "unknown",
            path: [up("male"), up("female"), down("male")],
        }),
        { text: "કાકા", kind: "lexical" },
    ],
    [
        "moṭā bāpā — the father's elder brother, keyed on `apexSeniority`",
        descriptorOf({
            classification: collateral(2, 1),
            side: "paternal",
            apexSeniority: "elder",
            path: [up("male"), up("female"), down("male")],
        }),
        { text: "મોટા બાપા", kind: "lexical" },
    ],
    [
        "foī — the father's sister",
        descriptorOf({
            classification: collateral(2, 1),
            side: "paternal",
            path: [up("male"), up("female"), down("female")],
        }),
        { text: "ફોઈ", kind: "lexical" },
    ],
    [
        "māmā — `side` is what keeps the mother's brother off kākā",
        descriptorOf({
            classification: collateral(2, 1),
            side: "maternal",
            path: [up("female"), up("male"), down("male")],
        }),
        { text: "મામા", kind: "lexical" },
    ],
    [
        "māsī — the mother's sister",
        descriptorOf({
            classification: collateral(2, 1),
            side: "maternal",
            path: [up("female"), up("male"), down("female")],
        }),
        { text: "માસી", kind: "lexical" },
    ],
    [
        "kākī — kākā's wife; the trailing `across` is not in ancestor position",
        descriptorOf({
            classification: collateral(2, 1),
            affinity: "inLaw",
            side: "paternal",
            path: [up("male"), up("female"), down("male"), across("female")],
        }),
        { text: "કાકી", kind: "lexical" },
    ],
    [
        "fuvā — foī's husband, the same cell with the linking sibling female",
        descriptorOf({
            classification: collateral(2, 1),
            affinity: "inLaw",
            side: "paternal",
            path: [up("male"), up("female"), down("female"), across("male")],
        }),
        { text: "ફુવા", kind: "lexical" },
    ],
    [
        "māmī — māmā's wife",
        descriptorOf({
            classification: collateral(2, 1),
            affinity: "inLaw",
            side: "maternal",
            path: [up("female"), up("male"), down("male"), across("female")],
        }),
        { text: "મામી", kind: "lexical" },
    ],
    [
        "māsā — māsī's husband",
        descriptorOf({
            classification: collateral(2, 1),
            affinity: "inLaw",
            side: "maternal",
            path: [up("female"), up("male"), down("female"), across("male")],
        }),
        { text: "માસા", kind: "lexical" },
    ],

    // === §5 the four cousin lines — ⚠ the parent's-sibling's gender ======
    [
        "⚠ pitrāī bhāī — the father's brother's son",
        descriptorOf({
            classification: collateral(2, 2),
            sharing: "full",
            side: "paternal",
            path: [up("male"), up("male"), down("male"), down("male")],
        }),
        { text: "પિત્રાઈ ભાઈ", kind: "lexical" },
    ],
    [
        "⚠ masiyāī bahen — the mother's sister's daughter",
        descriptorOf({
            classification: collateral(2, 2),
            sharing: "full",
            side: "maternal",
            path: [up("female"), up("female"), down("female"), down("female")],
        }),
        { text: "મસિયાઈ બહેન", kind: "lexical" },
    ],
    [
        "⚠ the māmā line — no adjective, so the genitive chain is itself the term",
        descriptorOf({
            classification: collateral(2, 2),
            sharing: "full",
            side: "maternal",
            path: [up("female"), up("male"), down("male"), down("male")],
        }),
        { text: "મામાનો પુત્ર", kind: "composed", hopCount: 1 },
    ],
    [
        "⚠ the foī line — likewise on the father's side",
        descriptorOf({
            classification: collateral(2, 2),
            sharing: "full",
            side: "paternal",
            path: [up("male"), up("female"), down("female"), down("male")],
        }),
        { text: "ફોઈનો પુત્ર", kind: "composed", hopCount: 1 },
    ],

    // === §6 nephews and nieces — ⚠ the linking sibling's gender ==========
    [
        "⚠ bhatrījo — the brother's son; `side` is `both` and cannot discriminate",
        descriptorOf({
            classification: collateral(1, 2),
            sharing: "full",
            side: "both",
            path: [up("male"), down("male"), down("male")],
        }),
        { text: "ભત્રીજો", kind: "lexical" },
    ],
    [
        "⚠ bhatrījī — the brother's daughter",
        descriptorOf({
            classification: collateral(1, 2),
            sharing: "full",
            side: "both",
            path: [up("male"), down("male"), down("female")],
        }),
        { text: "ભત્રીજી", kind: "lexical" },
    ],
    [
        "⚠ bhāṇej — the sister's son, one normalized cell away",
        descriptorOf({
            classification: collateral(1, 2),
            sharing: "full",
            side: "both",
            path: [up("male"), down("female"), down("male")],
        }),
        { text: "ભાણેજ", kind: "lexical" },
    ],
    [
        "⚠ bhāṇī — the sister's daughter",
        descriptorOf({
            classification: collateral(1, 2),
            sharing: "full",
            side: "both",
            path: [up("male"), down("female"), down("female")],
        }),
        { text: "ભાણી", kind: "lexical" },
    ],

    // === §7 spouse and spouse-side kin ===================================
    [
        "patnī — a bare `across` path is `classification: self` + `inLaw`",
        descriptorOf({
            classification: self,
            affinity: "inLaw",
            egoGender: "male",
            path: [across("female")],
        }),
        { text: "પત્ની", kind: "lexical" },
    ],
    [
        "⚠ savat — the co-wife; only `acrossCount` separates her from the spouse",
        descriptorOf({
            classification: self,
            affinity: "inLaw",
            path: [across("male"), across("female")],
        }),
        { text: "સવત", kind: "lexical" },
    ],
    [
        "sāsu — the spouse's mother; `side` is notApplicable on an across-initial path",
        descriptorOf({
            classification: lineal("ancestor", 1),
            affinity: "inLaw",
            path: [across("male"), up("female")],
        }),
        { text: "સાસુ", kind: "lexical" },
    ],
    [
        "sasro — and her husband, the same word for either spouse's parent",
        descriptorOf({
            classification: lineal("ancestor", 1),
            affinity: "inLaw",
            path: [across("male"), up("male")],
        }),
        { text: "સસરો", kind: "lexical" },
    ],
    [
        "⚠ jamāī — the daughter's husband, keyed on the linking child",
        descriptorOf({
            classification: lineal("descendant", 1),
            affinity: "inLaw",
            path: [down("female"), across("male")],
        }),
        { text: "જમાઈ", kind: "lexical" },
    ],
    [
        "⚠ vahu — the son's wife",
        descriptorOf({
            classification: lineal("descendant", 1),
            affinity: "inLaw",
            path: [down("male"), across("female")],
        }),
        { text: "વહુ", kind: "lexical" },
    ],
    [
        "a son's husband is never jamāī — `linkGender` decides, `alterGender` does not",
        descriptorOf({
            classification: lineal("descendant", 1),
            affinity: "inLaw",
            path: [down("male"), across("male")],
        }),
        { text: "વહુ", kind: "lexical" },
    ],
    [
        "savko dīkro — the mirror shape: the spouse's child, not the child's spouse",
        descriptorOf({
            classification: lineal("descendant", 1),
            affinity: "inLaw",
            path: [across("female"), down("male")],
        }),
        { text: "સાવકો દીકરો", kind: "lexical" },
    ],

    // --- the affinal collision class: seven terms, one normalized cell ---
    [
        "⚠ sāḷo — the wife's brother; `acrossAtStart` + `spouseGender: female`",
        descriptorOf({
            classification: collateral(1, 1),
            affinity: "inLaw",
            egoGender: "male",
            path: [across("female"), up("male"), down("male")],
        }),
        { text: "સાળો", kind: "lexical" },
    ],
    [
        "⚠ sāḷī — the wife's sister",
        descriptorOf({
            classification: collateral(1, 1),
            affinity: "inLaw",
            egoGender: "male",
            path: [across("female"), up("male"), down("female")],
        }),
        { text: "સાળી", kind: "lexical" },
    ],
    [
        "⚠ jeṭh — the husband's elder brother; the spouse hop flips to male",
        descriptorOf({
            classification: collateral(1, 1),
            affinity: "inLaw",
            apexSeniority: "elder",
            path: [across("male"), up("male"), down("male")],
        }),
        { text: "જેઠ", kind: "lexical" },
    ],
    [
        "⚠ diyar — the husband's younger brother; `apexSeniority` is the only difference",
        descriptorOf({
            classification: collateral(1, 1),
            affinity: "inLaw",
            apexSeniority: "younger",
            path: [across("male"), up("male"), down("male")],
        }),
        { text: "દિયર", kind: "lexical" },
    ],
    [
        "the husband's brother with `apexSeniority: unknown` — no unmarked lexeme, so it composes",
        descriptorOf({
            classification: collateral(1, 1),
            affinity: "inLaw",
            apexSeniority: "unknown",
            path: [across("male"), up("male"), down("male")],
        }),
        { text: "સસરોનો પુત્ર", kind: "composed", hopCount: 1 },
    ],
    [
        "⚠ naṇand — the husband's sister",
        descriptorOf({
            classification: collateral(1, 1),
            affinity: "inLaw",
            path: [across("male"), up("male"), down("female")],
        }),
        { text: "નણંદ", kind: "lexical" },
    ],
    [
        "⚠ bhābhī — the brother's wife; `acrossAtEnd` + `linkGender: male`",
        descriptorOf({
            classification: collateral(1, 1),
            affinity: "inLaw",
            sharing: "full",
            side: "both",
            path: [up("male"), down("male"), across("female")],
        }),
        { text: "ભાભી", kind: "lexical" },
    ],
    [
        "⚠ banevī — the sister's husband, the same shape with the link female",
        descriptorOf({
            classification: collateral(1, 1),
            affinity: "inLaw",
            sharing: "full",
            side: "both",
            path: [up("male"), down("female"), across("male")],
        }),
        { text: "બનેવી", kind: "lexical" },
    ],
    [
        "vevāī — the child's spouse's father; `across` at neither end",
        descriptorOf({
            classification: collateral(1, 1),
            affinity: "inLaw",
            path: [down("female"), across("male"), up("male")],
        }),
        { text: "વેવાઈ", kind: "lexical" },
    ],
    [
        "vevāṇ — and his wife, a cell English has no lexeme for at all",
        descriptorOf({
            classification: collateral(1, 1),
            affinity: "inLaw",
            path: [down("female"), across("male"), up("female")],
        }),
        { text: "વેવાણ", kind: "lexical" },
    ],
    [
        "⚠ sāḍhu — the wife's sister's husband; two `across` hops, spouse hop female",
        descriptorOf({
            classification: collateral(1, 1),
            affinity: "inLaw",
            egoGender: "male",
            path: [across("female"), up("male"), down("female"), across("male")],
        }),
        { text: "સાઢુ", kind: "lexical" },
    ],
    [
        "⚠ naṇdoī — the husband's sister's husband; the spouse hop is what differs",
        descriptorOf({
            classification: collateral(1, 1),
            affinity: "inLaw",
            path: [across("male"), up("male"), down("female"), across("male")],
        }),
        { text: "નણદોઈ", kind: "lexical" },
    ],
    [
        "⚠ jeṭhāṇī — jeṭh's wife; `apexSeniority` again, now two marriages out",
        descriptorOf({
            classification: collateral(1, 1),
            affinity: "inLaw",
            apexSeniority: "elder",
            path: [across("male"), up("male"), down("male"), across("female")],
        }),
        { text: "જેઠાણી", kind: "lexical" },
    ],
    [
        "⚠ derāṇī — diyar's wife",
        descriptorOf({
            classification: collateral(1, 1),
            affinity: "inLaw",
            apexSeniority: "younger",
            path: [across("male"), up("male"), down("male"), across("female")],
        }),
        { text: "દેરાણી", kind: "lexical" },
    ],

    // === §8 the three terminology policies ===============================
    [
        "an adoptive mother is mā — nothing keys `edgeNature`",
        descriptorOf({
            classification: lineal("ancestor", 1),
            edgeNature: "adoptive",
            path: [up("female", "adoptive")],
        }),
        { text: "મા", kind: "lexical" },
    ],
    [
        "savkī mā — the step-mother, where the `across` sits in ancestor position",
        descriptorOf({
            classification: lineal("ancestor", 1),
            affinity: "step",
            path: [up("male"), across("female")],
        }),
        { text: "સાવકી મા", kind: "lexical" },
    ],
    [
        "an ended marriage renders unmarked — Gujarati has no affix, so the pack has no rule",
        descriptorOf({
            classification: lineal("ancestor", 1),
            affinity: "inLaw",
            path: [across("male", "ended"), up("female")],
        }),
        { text: "સાસુ", kind: "lexical" },
    ],
    [
        "…and so does an ended marriage itself",
        descriptorOf({
            classification: self,
            affinity: "inLaw",
            egoGender: "male",
            path: [across("female", "ended")],
        }),
        { text: "પત્ની", kind: "lexical" },
    ],
];

describe("the `gu` pack", () => {
    it.each(CASES.map((c) => [c[0], c[1], c[2]] as const))(
        "phrases %s",
        (_name, descriptor, expected) => {
            expect(phrase(descriptor, GU)).toMatchObject(expected);
        },
    );

    it("keys the child-in-law terms on `linkGender`, never `alterGender`", () => {
        // ADR-0033's third terminology policy, asserted on the data rather than
        // only through a fixture: *jamāī* and *vahu* mean "daughter's husband"
        // and "son's wife", so they key the linking **child**. Keying the alter
        // would render a son's husband *jamāī* — wrong, not merely coarse.
        const childInLaw = GU.entries.filter(
            (entry) => entry.term === "જમાઈ" || entry.term === "વહુ",
        );
        expect(childInLaw).toHaveLength(2);
        for (const entry of childInLaw) {
            expect(entry.when.linkGender).toBeDefined();
            expect(entry.when.alterGender).toBeUndefined();
        }
    });

    it("separates the whole affinal collision class into distinct terms", () => {
        // Seven shapes that share `collateral {up:1, down:1}` + `affinity: inLaw`
        // and differ only in derived facets. English says "brother-in-law" to
        // five of them.
        const cell = CASES.filter(([, descriptor]) => {
            const c = descriptor.classification;
            return (
                c.kind === "collateral" &&
                c.up === 1 &&
                c.down === 1 &&
                descriptor.affinity === "inLaw"
            );
        }).map(([, descriptor]) => phrase(descriptor, GU));
        const lexical = cell.filter((result) => result.kind === "lexical");
        expect(lexical.length).toBeGreaterThanOrEqual(12);
        expect(new Set(lexical.map((result) => result.text)).size).toBe(lexical.length);
    });

    it("phrases bare, in Gujarati script, with nothing left unsubstituted", () => {
        for (const [, descriptor] of CASES) {
            const { text } = phrase(descriptor, GU);
            expect(text).not.toMatch(/[A-Za-z{}]/);
            expect(text).toBe(text.trim());
        }
    });
});
