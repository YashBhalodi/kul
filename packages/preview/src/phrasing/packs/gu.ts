import type { LanguagePack } from "../pack.js";

/**
 * The **Gujarati language pack** — pure data over the phrasing key (ADR-0033).
 * Source: `docs/kinship-term-inventory.md` §1–§8, script column.
 *
 * Gujarati is the forcing case the whole layer was designed against, and this
 * module is the additivity promise's test: it adds a language with **zero
 * lines of logic**, using only the vocabulary ADR-0033 decided and ADR-0039
 * pinned.
 *
 * Four things about it are worth reading before editing:
 *
 * - **The affinal collision class.** Seven terms — *sāḷo* / *jeṭh* / *diyar* /
 *   *naṇand* / *bhābhī* / *banevī* / *vevāī* — share one normalized cell
 *   (`collateral {up:1, down:1}` + `affinity: inLaw`). What separates them is
 *   where the `across` hop sits (`acrossAtStart` / `acrossAtEnd` /
 *   `acrossCount`), the gender of the person a specific hop lands on
 *   (`spouseGender`, `linkGender`), and `apexSeniority`. Every one of those is
 *   a derived facet; none is a normalized field. English merges the lot into
 *   *brother-in-law* and never notices.
 * - **`par-` is the pack's one affix rule**, at `generations = 3`, with no
 *   repeat and `cap: 3`. Generation 4 and beyond is **refused**, so the
 *   compositional fallback fires — *par-dādā*'s father — which is exactly what
 *   speakers do. Seniority modifiers (*moṭā* / *nānā bhāī*) are **not** rules:
 *   they are ordinary entries keying `seniority`, so `seniority: unknown`
 *   falls through to the unmarked *bhāī* by the never-guess rule alone.
 * - **The genitive join is morphological.** Gujarati's possessive particle
 *   agrees with the noun that *follows* it, so `genitive` is a record keyed by
 *   the possessed noun's gender: નો / ની / નું. *મામાનો દીકરો*.
 * - **Many-to-one collapses are this pack's own business.** Colloquial
 *   Gujarati conflates `affinity: step` and `sharing: half` under *savkā*, so
 *   two entries point at one term. The shared layer never collapses a
 *   dimension on a pack's behalf (ADR-0033), and tied entries that *agree* are
 *   legitimate (ADR-0039).
 *
 * Three regions are deliberately left to compose, because that is what the
 * language does: the *māmā* and *foī* cousin lines (a genitive chain is the
 * actual usage there, so no coarse `cousinDegree: 1` entry is written — one
 * would swallow all four lines into *bhāī*), a husband's brother whose
 * `apexSeniority` is `unknown` (no unmarked lexeme exists between *jeṭh* and
 * *diyar*), and every `other`-gender or `side: other` region past the
 * immediate family, where no lexicon anywhere has a word to offer.
 *
 * Every record also carries its **Latin-script twin** — `translit` on each
 * entry and on the `par-` rule, plus `hopsTranslit` and `genitiveTranslit` —
 * so a reader who does not read the script yet can hover a term and read it
 * (#302). It is data like everything else here: no romanization is computed
 * from the script anywhere, and a term whose Latin form were omitted would
 * simply carry no gloss rather than an invented one (ADR-0044). Source for the
 * romanizations: `docs/kinship-term-inventory.md`, the same document the terms
 * came from.
 *
 * Nothing keys `edgeNature`, so an adoptive mother is *mā* — *dattak* is
 * formal/legal register. Nothing keys `endedMarriage` either: Gujarati does
 * not lexicalize an ended marriage, so a former mother-in-law is *sāsu* and
 * disclosure is the chrome's job (the backbone carries `status` and
 * `endReason`). Both are ADR-0033 policies, and in a data-only pack a policy
 * of "render unmarked" is expressed by writing nothing at all.
 */
export const GU: LanguagePack = {
    code: "gu",
    label: "ગુજરાતી",

    entries: [
        // --- self, spouse, co-spouse ------------------------------------------
        // A bare `across` path counts zero vertical hops, so a spouse is
        // `classification: self` + `affinity: inLaw`; two `across` hops is the
        // co-wife, which `acrossCount` alone separates.
        { when: { classification: "self", acrossCount: 0 }, term: "પોતે", translit: "pote" },
        { when: { classification: "self", acrossCount: 1, alterGender: "male" }, term: "પતિ", translit: "pati" },
        { when: { classification: "self", acrossCount: 1, alterGender: "female" }, term: "પત્ની", translit: "patnī" },
        { when: { classification: "self", acrossCount: 1, alterGender: "other" }, term: "જીવનસાથી", translit: "jīvansāthī" },
        { when: { classification: "self", acrossCount: 2, alterGender: "female" }, term: "સવત", translit: "savat" },

        // --- lineal ancestors --------------------------------------------------
        // Parents key `generations: 1` because `side` is `notApplicable` there
        // ("your mother is not your maternal side") — and Gujarati parent terms
        // indeed carry no side.
        {
            when: { classification: "lineal", role: "ancestor", generations: 1, affinity: "blood", alterGender: "female" },
            term: "મા", translit: "mā",
        },
        {
            when: { classification: "lineal", role: "ancestor", generations: 1, affinity: "blood", alterGender: "male" },
            term: "પપ્પા", translit: "pappā",
        },
        // Not an invented neutral: વાલી is the ordinary word for the parent /
        // guardian of a child, and it is also this pack's `up` hop noun for an
        // `other`-gender parent, so the lexical and composed forms agree.
        {
            when: { classification: "lineal", role: "ancestor", generations: 1, affinity: "blood", alterGender: "other" },
            term: "વાલી", translit: "vālī",
        },
        // The grandparent spine deliberately leaves `generations` a wildcard so
        // the `par-` rule below can carry generation 3 off these four terms.
        // Keying `side` is what stops them reaching a parent (whose `side` is
        // `notApplicable`), and `cap: 3` is what stops them reaching generation
        // 4 — where the rule refuses and the fallback takes over.
        {
            when: { classification: "lineal", role: "ancestor", affinity: "blood", side: "paternal", alterGender: "male" },
            term: "દાદા", translit: "dādā",
        },
        {
            when: { classification: "lineal", role: "ancestor", affinity: "blood", side: "paternal", alterGender: "female" },
            term: "દાદી", translit: "dādī",
        },
        {
            when: { classification: "lineal", role: "ancestor", affinity: "blood", side: "maternal", alterGender: "male" },
            term: "નાના", translit: "nānā",
        },
        {
            when: { classification: "lineal", role: "ancestor", affinity: "blood", side: "maternal", alterGender: "female" },
            term: "નાની", translit: "nānī",
        },

        // --- lineal descendants ------------------------------------------------
        {
            when: { classification: "lineal", role: "descendant", generations: 1, affinity: "blood", alterGender: "male" },
            term: "દીકરો", translit: "dīkro",
        },
        {
            when: { classification: "lineal", role: "descendant", generations: 1, affinity: "blood", alterGender: "female" },
            term: "દીકરી", translit: "dīkrī",
        },
        {
            when: { classification: "lineal", role: "descendant", generations: 1, affinity: "blood", alterGender: "other" },
            term: "સંતાન", translit: "santān",
        },
        // Grandchildren split on the **linking child's** gender — a backbone
        // fact, and the first place normalized fields cannot select a term:
        // `side` is `notApplicable` on every descendant path.
        {
            when: { classification: "lineal", role: "descendant", generations: 2, affinity: "blood", linkGender: "male", alterGender: "male" },
            term: "પૌત્ર", translit: "pautra",
        },
        {
            when: { classification: "lineal", role: "descendant", generations: 2, affinity: "blood", linkGender: "male", alterGender: "female" },
            term: "પૌત્રી", translit: "pautrī",
        },
        {
            when: { classification: "lineal", role: "descendant", generations: 2, affinity: "blood", linkGender: "female", alterGender: "male" },
            term: "દોહિત્ર", translit: "dohitra",
        },
        {
            when: { classification: "lineal", role: "descendant", generations: 2, affinity: "blood", linkGender: "female", alterGender: "female" },
            term: "દોહિત્રી", translit: "dohitrī",
        },
        // Generation 3 is lexicalized on the son's line only, and it is written
        // out rather than affixed: *pra-* is not productive past here, and the
        // daughter's line has no generation-3 term at all.
        {
            when: { classification: "lineal", role: "descendant", generations: 3, affinity: "blood", linkGender: "male", alterGender: "male" },
            term: "પ્રપૌત્ર", translit: "prapautra",
        },
        {
            when: { classification: "lineal", role: "descendant", generations: 3, affinity: "blood", linkGender: "male", alterGender: "female" },
            term: "પ્રપૌત્રી", translit: "prapautrī",
        },

        // --- parents-in-law and children-in-law --------------------------------
        // Gujarati stops at one generation, as English does: a spouse's
        // grandmother composes off *sāsu*.
        {
            when: { classification: "lineal", role: "ancestor", generations: 1, affinity: "inLaw", acrossCount: 1, acrossAtStart: true, alterGender: "male" },
            term: "સસરો", translit: "sasro",
        },
        {
            when: { classification: "lineal", role: "ancestor", generations: 1, affinity: "inLaw", acrossCount: 1, acrossAtStart: true, alterGender: "female" },
            term: "સાસુ", translit: "sāsu",
        },
        // *jamāī* and *vahu* key `linkGender`, **not** `alterGender` — the ADR-0033
        // policy, and the reason it exists: the terms mean "daughter's husband"
        // and "son's wife", so they key the linking child. Under a same-sex
        // marriage, alter-gender keying would render a son's husband *jamāī*,
        // which is wrong rather than merely coarse.
        {
            when: { classification: "lineal", role: "descendant", generations: 1, affinity: "inLaw", acrossCount: 1, acrossAtEnd: true, linkGender: "female" },
            term: "જમાઈ", translit: "jamāī",
        },
        {
            when: { classification: "lineal", role: "descendant", generations: 1, affinity: "inLaw", acrossCount: 1, acrossAtEnd: true, linkGender: "male" },
            term: "વહુ", translit: "vahu",
        },

        // --- step-parents and step-children -------------------------------------
        // A step-parent is `up·across` — the marriage hop sits in ancestor
        // position, so the position rule reads `affinity: step`. A step-child is
        // `across·down`, which the same rule reads as `inLaw`, hence
        // `acrossAtStart`.
        {
            when: { classification: "lineal", role: "ancestor", generations: 1, affinity: "step", alterGender: "female" },
            term: "સાવકી મા", translit: "savkī mā",
        },
        {
            when: { classification: "lineal", role: "ancestor", generations: 1, affinity: "step", alterGender: "male" },
            term: "સાવકો બાપ", translit: "savko bāp",
        },
        {
            when: { classification: "lineal", role: "descendant", generations: 1, affinity: "inLaw", acrossCount: 1, acrossAtStart: true, alterGender: "male" },
            term: "સાવકો દીકરો", translit: "savko dīkro",
        },
        {
            when: { classification: "lineal", role: "descendant", generations: 1, affinity: "inLaw", acrossCount: 1, acrossAtStart: true, alterGender: "female" },
            term: "સાવકી દીકરી", translit: "savkī dīkrī",
        },

        // --- siblings -------------------------------------------------------------
        // `cousinDegree: 0, removed: 0` is exactly `up: 1, down: 1`.
        {
            when: { classification: "collateral", cousinDegree: 0, removed: 0, affinity: "blood", alterGender: "male" },
            term: "ભાઈ", translit: "bhāī",
        },
        {
            when: { classification: "collateral", cousinDegree: 0, removed: 0, affinity: "blood", alterGender: "female" },
            term: "બહેન", translit: "bahen",
        },
        {
            when: { classification: "collateral", cousinDegree: 0, removed: 0, affinity: "blood", alterGender: "other" },
            term: "સહોદર", translit: "sahodar",
        },
        // Seniority is a compositional modifier in Gujarati, not a distinct
        // lexeme — so these are ordinary entries keying `seniority`, never affix
        // rules. `seniority: unknown` disqualifies them and the unmarked *bhāī*
        // wins, which is the honest rendering and needs no code.
        //
        // They key `sharing: full` as well, so an elder *half*-sibling reads
        // *savkā bhāī* rather than tying the *savkā* entry below at equal
        // specificity with a different term.
        {
            when: { classification: "collateral", cousinDegree: 0, removed: 0, affinity: "blood", sharing: "full", seniority: "elder", alterGender: "male" },
            term: "મોટા ભાઈ", translit: "moṭā bhāī",
        },
        {
            when: { classification: "collateral", cousinDegree: 0, removed: 0, affinity: "blood", sharing: "full", seniority: "elder", alterGender: "female" },
            term: "મોટી બહેન", translit: "moṭī bahen",
        },
        {
            when: { classification: "collateral", cousinDegree: 0, removed: 0, affinity: "blood", sharing: "full", seniority: "younger", alterGender: "male" },
            term: "નાના ભાઈ", translit: "nānā bhāī",
        },
        {
            when: { classification: "collateral", cousinDegree: 0, removed: 0, affinity: "blood", sharing: "full", seniority: "younger", alterGender: "female" },
            term: "નાની બહેન", translit: "nānī bahen",
        },
        // The many-to-one collapse: colloquial Gujarati puts `affinity: step`
        // and `sharing: half` on the same word. Two entries, one term — the
        // shared layer never merges dimensions on a pack's behalf, and English
        // composes the two prefixes freely, which is why ADR-0026 split them.
        {
            when: { classification: "collateral", cousinDegree: 0, removed: 0, affinity: "blood", sharing: "half", alterGender: "male" },
            term: "સાવકા ભાઈ", translit: "savkā bhāī",
        },
        {
            when: { classification: "collateral", cousinDegree: 0, removed: 0, affinity: "blood", sharing: "half", alterGender: "female" },
            term: "સાવકી બહેન", translit: "savkī bahen",
        },
        {
            when: { classification: "collateral", cousinDegree: 0, removed: 0, affinity: "step", alterGender: "male" },
            term: "સાવકા ભાઈ", translit: "savkā bhāī",
        },
        {
            when: { classification: "collateral", cousinDegree: 0, removed: 0, affinity: "step", alterGender: "female" },
            term: "સાવકી બહેન", translit: "savkī bahen",
        },

        // --- the parents'-siblings tier: ten terms, five splits ---------------------
        // The hardest tier in any Indic language, and it selects on normalized
        // fields alone — `{side, affinity, alterGender, apexSeniority}` — plus
        // `linkGender` on the affinal four, which is what makes *kākī* "the
        // brother's wife" rather than "the female in-law on the father's side".
        //
        // `up: 2, down: 1` is keyed exactly rather than as `cousinDegree: 0,
        // removed: 1`, which would also reach the nephew tier. Great-uncles
        // (`up: 3`) have no dedicated terms and compose.
        {
            when: { classification: "collateral", up: 2, down: 1, affinity: "blood", side: "paternal", alterGender: "male" },
            term: "કાકા", translit: "kākā",
        },
        // *moṭā bāpā* compares the uncle to ego's **father**, not to ego — that
        // is `apexSeniority`, the field ADR-0026 added for exactly this. When it
        // is `unknown`, the unmarked *kākā* wins, which is also what families who
        // do not make the distinction say.
        {
            when: { classification: "collateral", up: 2, down: 1, affinity: "blood", side: "paternal", apexSeniority: "elder", alterGender: "male" },
            term: "મોટા બાપા", translit: "moṭā bāpā",
        },
        {
            when: { classification: "collateral", up: 2, down: 1, affinity: "blood", side: "paternal", alterGender: "female" },
            term: "ફોઈ", translit: "foī",
        },
        {
            when: { classification: "collateral", up: 2, down: 1, affinity: "blood", side: "maternal", alterGender: "male" },
            term: "મામા", translit: "māmā",
        },
        {
            when: { classification: "collateral", up: 2, down: 1, affinity: "blood", side: "maternal", alterGender: "female" },
            term: "માસી", translit: "māsī",
        },
        // The affinal four are `inLaw` because their trailing `across` is not in
        // ancestor position. `acrossAtEnd` says the alter is someone's spouse;
        // `linkGender` says whose.
        {
            when: { classification: "collateral", up: 2, down: 1, affinity: "inLaw", side: "paternal", acrossAtEnd: true, linkGender: "male", alterGender: "female" },
            term: "કાકી", translit: "kākī",
        },
        {
            when: { classification: "collateral", up: 2, down: 1, affinity: "inLaw", side: "paternal", acrossAtEnd: true, linkGender: "female", alterGender: "male" },
            term: "ફુવા", translit: "fuvā",
        },
        {
            when: { classification: "collateral", up: 2, down: 1, affinity: "inLaw", side: "maternal", acrossAtEnd: true, linkGender: "male", alterGender: "female" },
            term: "મામી", translit: "māmī",
        },
        {
            when: { classification: "collateral", up: 2, down: 1, affinity: "inLaw", side: "maternal", acrossAtEnd: true, linkGender: "female", alterGender: "male" },
            term: "માસા", translit: "māsā",
        },

        // --- cousins ----------------------------------------------------------------
        // The four first-cousin lines split on `side` × the parent's-sibling's
        // gender — the second again a backbone fact. Only two lines carry a
        // dedicated adjective; the other two are genitive constructions the
        // fallback builds off *māmā* and *foī* for free, which is why no coarse
        // `cousinDegree: 1` entry is written. One would collapse all four into
        // *bhāī* and retire the distinction that matters culturally.
        {
            when: { classification: "collateral", cousinDegree: 1, removed: 0, affinity: "blood", side: "paternal", linkGender: "male", alterGender: "male" },
            term: "પિત્રાઈ ભાઈ", translit: "pitrāī bhāī",
        },
        {
            when: { classification: "collateral", cousinDegree: 1, removed: 0, affinity: "blood", side: "paternal", linkGender: "male", alterGender: "female" },
            term: "પિત્રાઈ બહેન", translit: "pitrāī bahen",
        },
        {
            when: { classification: "collateral", cousinDegree: 1, removed: 0, affinity: "blood", side: "maternal", linkGender: "female", alterGender: "male" },
            term: "મસિયાઈ ભાઈ", translit: "masiyāī bhāī",
        },
        {
            when: { classification: "collateral", cousinDegree: 1, removed: 0, affinity: "blood", side: "maternal", linkGender: "female", alterGender: "female" },
            term: "મસિયાઈ બહેન", translit: "masiyāī bahen",
        },

        // --- nephews and nieces --------------------------------------------------------
        // The linking **sibling's** gender selects, and `side` is `both` here
        // (couple apex) and does not discriminate. `egoGender` is not a selector
        // either: a woman's brother's son is her *bhatrījo* too.
        {
            when: { classification: "collateral", up: 1, down: 2, affinity: "blood", linkGender: "male", alterGender: "male" },
            term: "ભત્રીજો", translit: "bhatrījo",
        },
        {
            when: { classification: "collateral", up: 1, down: 2, affinity: "blood", linkGender: "male", alterGender: "female" },
            term: "ભત્રીજી", translit: "bhatrījī",
        },
        {
            when: { classification: "collateral", up: 1, down: 2, affinity: "blood", linkGender: "female", alterGender: "male" },
            term: "ભાણેજ", translit: "bhāṇej",
        },
        {
            when: { classification: "collateral", up: 1, down: 2, affinity: "blood", linkGender: "female", alterGender: "female" },
            term: "ભાણી", translit: "bhāṇī",
        },

        // --- the affinal collision class ------------------------------------------------
        // Seven terms, one normalized cell (`collateral {1,1,0,0}` + `inLaw`).
        // Read the block as a table over four derived facets:
        //
        //   acrossCount · acrossAtStart · acrossAtEnd  →  path shape
        //     1 · true  · false  →  across·up·down   the spouse's sibling
        //     1 · false · true   →  up·down·across   the sibling's spouse
        //     1 · false · false  →  down·across·up   the child's parent-in-law
        //     2 · true  · true   →  across·up·down·across  the spouse's sibling's spouse
        //
        // then `spouseGender` (whose spouse the path ran through),
        // `linkGender` (whose spouse the alter is) and `apexSeniority` (elder or
        // younger than the spouse) pick the word.
        //
        // All four coordinates are keyed on every entry, and each is
        // load-bearing somewhere — `tests/phrasing/gu-facets.test.ts` drops
        // them one at a time and pins the wrong term that appears:
        //
        // - `acrossCount` keeps the two-marriage chains out of the
        //   one-marriage terms. Without it a co-spouse's brother
        //   (`across·across·up·down`) renders *sāḷo*, and a child's spouse's
        //   spouse's parent renders *vevāī*.
        // - `acrossAtEnd: false` keeps *vevāī*'s shape out of *bhābhī* and
        //   *banevī* — the two differ only in where the single `across` sits.
        //   On the across-initial entries it guards nothing on its own, but
        //   with `acrossCount` it is what keeps *jeṭhāṇī*'s path off *naṇand*
        //   when `apexSeniority` is `unknown`: exactly the case a missing
        //   birth date produces, where a husband's brother's **wife** would
        //   otherwise be called a husband's **sister**.
        //
        // A shape none of the four names — a wife's brother's wife, a spouse's
        // child's spouse's parent — matches nothing and composes, rather than
        // being swept into a neighbouring term.
        {
            when: { classification: "collateral", cousinDegree: 0, removed: 0, affinity: "inLaw", acrossCount: 1, acrossAtStart: true, acrossAtEnd: false, spouseGender: "female", alterGender: "male" },
            term: "સાળો", translit: "sāḷo",
        },
        {
            when: { classification: "collateral", cousinDegree: 0, removed: 0, affinity: "inLaw", acrossCount: 1, acrossAtStart: true, acrossAtEnd: false, spouseGender: "female", alterGender: "female" },
            term: "સાળી", translit: "sāḷī",
        },
        // The husband's brother is seniority-split with no unmarked lexeme
        // between the two, so `apexSeniority: unknown` matches neither and the
        // fallback answers *sasro-no dīkro* — "father-in-law's son". That is the
        // never-guess rule choosing an honest description over a coin toss.
        {
            when: { classification: "collateral", cousinDegree: 0, removed: 0, affinity: "inLaw", acrossCount: 1, acrossAtStart: true, acrossAtEnd: false, spouseGender: "male", apexSeniority: "elder", alterGender: "male" },
            term: "જેઠ", translit: "jeṭh",
        },
        {
            when: { classification: "collateral", cousinDegree: 0, removed: 0, affinity: "inLaw", acrossCount: 1, acrossAtStart: true, acrossAtEnd: false, spouseGender: "male", apexSeniority: "younger", alterGender: "male" },
            term: "દિયર", translit: "diyar",
        },
        {
            when: { classification: "collateral", cousinDegree: 0, removed: 0, affinity: "inLaw", acrossCount: 1, acrossAtStart: true, acrossAtEnd: false, spouseGender: "male", alterGender: "female" },
            term: "નણંદ", translit: "naṇand",
        },
        {
            when: { classification: "collateral", cousinDegree: 0, removed: 0, affinity: "inLaw", acrossCount: 1, acrossAtStart: false, acrossAtEnd: true, linkGender: "male", alterGender: "female" },
            term: "ભાભી", translit: "bhābhī",
        },
        {
            when: { classification: "collateral", cousinDegree: 0, removed: 0, affinity: "inLaw", acrossCount: 1, acrossAtStart: false, acrossAtEnd: true, linkGender: "female", alterGender: "male" },
            term: "બનેવી", translit: "banevī",
        },
        // *vevāī* / *vevāṇ* is the shape with an `across` at neither end — a
        // child's spouse's parent. English has no lexeme for it at all.
        {
            when: { classification: "collateral", cousinDegree: 0, removed: 0, affinity: "inLaw", acrossCount: 1, acrossAtStart: false, acrossAtEnd: false, alterGender: "male" },
            term: "વેવાઈ", translit: "vevāī",
        },
        {
            when: { classification: "collateral", cousinDegree: 0, removed: 0, affinity: "inLaw", acrossCount: 1, acrossAtStart: false, acrossAtEnd: false, alterGender: "female" },
            term: "વેવાણ", translit: "vevāṇ",
        },
        // The two-`across` corner. Every lexicalized multi-marriage term in the
        // language uses exactly two, which is ADR-0027's ceiling — nothing in
        // Gujarati exceeds it.
        {
            when: { classification: "collateral", cousinDegree: 0, removed: 0, affinity: "inLaw", acrossCount: 2, acrossAtStart: true, acrossAtEnd: true, spouseGender: "female", alterGender: "male" },
            term: "સાઢુ", translit: "sāḍhu",
        },
        {
            when: { classification: "collateral", cousinDegree: 0, removed: 0, affinity: "inLaw", acrossCount: 2, acrossAtStart: true, acrossAtEnd: true, spouseGender: "male", alterGender: "male" },
            term: "નણદોઈ", translit: "naṇdoī",
        },
        {
            when: { classification: "collateral", cousinDegree: 0, removed: 0, affinity: "inLaw", acrossCount: 2, acrossAtStart: true, acrossAtEnd: true, spouseGender: "male", apexSeniority: "elder", alterGender: "female" },
            term: "જેઠાણી", translit: "jeṭhāṇī",
        },
        {
            when: { classification: "collateral", cousinDegree: 0, removed: 0, affinity: "inLaw", acrossCount: 2, acrossAtStart: true, acrossAtEnd: true, spouseGender: "male", apexSeniority: "younger", alterGender: "female" },
            term: "દેરાણી", translit: "derāṇī",
        },
    ],

    affixes: [
        // The pack's only rule, and the one `cap` was designed for. `par-` is
        // productive for exactly one generation past *dādā* / *nānā*; at
        // generation 4 the cap **refuses** lexicalization outright rather than
        // dropping the affix, so the fallback fires and answers *par-dādā*'s
        // father — a phrase built from the longest prefix the language still
        // has a word for. Yielding a bare *dādā* there would assert a term
        // Gujarati does not have for that generation.
        {
            when: {
                classification: "lineal",
                role: "ancestor",
                affinity: "blood",
                atLeast: { facet: "generations", value: 3 },
            },
            affix: "પર",
            // The Latin form hyphenates where the script does not, following
            // the convention `docs/kinship-term-inventory.md` §1 writes the
            // productive prefix in: *par-dādā*, one word in Gujarati.
            translit: "par-",
            position: "prefix",
            cap: 3,
        },
    ],

    // The fallback's per-hop nouns, in the **formal register throughout** —
    // પિતા / પુત્ર / પતિ, not પપ્પા / દીકરો / વર. Two reasons, and the second
    // is measurable (ADR-0041):
    //
    // 1. A hop noun is not what a speaker calls a relative; it is how a chain
    //    spells one out. The lexical entries above keep the colloquial words a
    //    reader wants for their own kin (દીકરો, મા, પપ્પા) — the formal
    //    register lives only inside a composed genitive, which is already a
    //    descriptive construction rather than a name.
    // 2. Every noun here is **oblique-invariant**. A masculine `-ો` noun takes
    //    an oblique `-ા` stem before a postposition (દીકરો → દીકરાનો), and the
    //    genitive pattern below agrees with the *following* noun, so it cannot
    //    produce that. Consonant- and `-ા`-final stems do not change, so
    //    choosing them removes the artefact rather than papering over it: it
    //    drops from 43.8% of composed phrases to 9.6%, the residual being the
    //    `-ો` **entry** terms (સસરો, ભત્રીજો, સાળો, દીકરો) that can head a
    //    chain, which no lexicon choice can reach.
    hops: {
        up: { male: "પિતા", female: "માતા", other: "વાલી" },
        down: { male: "પુત્ર", female: "પુત્રી", other: "સંતાન" },
        across: { male: "પતિ", female: "પત્ની", other: "જીવનસાથી" },
    },
    // The possessive particle agrees with the noun that **follows** it, not
    // with the possessor — so the pattern is a record keyed by the possessed
    // noun's gender, which is the case ADR-0033 gave `GenitivePattern` its
    // record form for. It attaches to the possessor without a space, as
    // Gujarati orthography writes it: મામાનો પુત્ર.
    genitive: {
        male: "{possessor}નો {possessed}",
        female: "{possessor}ની {possessed}",
        other: "{possessor}નું {possessed}",
    },

    // The Latin-script twins of the two token sources above, so a *composed*
    // phrase glosses end to end rather than only where an entry happened to
    // head it. Written out record by record for the same reason the terms are:
    // there is no romanization function anywhere in this layer, and a pack that
    // romanizes must romanize completely (ADR-0044). The postposition is
    // hyphenated in Latin — *māmā-no dīkro*, as the inventory §5 writes it —
    // where the script joins it to the possessor without a break.
    hopsTranslit: {
        up: { male: "pitā", female: "mātā", other: "vālī" },
        down: { male: "putra", female: "putrī", other: "santān" },
        across: { male: "pati", female: "patnī", other: "jīvansāthī" },
    },
    genitiveTranslit: {
        male: "{possessor}-no {possessed}",
        female: "{possessor}-nī {possessed}",
        other: "{possessor}-nũ {possessed}",
    },
};
