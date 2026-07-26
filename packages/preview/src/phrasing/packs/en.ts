import type { LanguagePack } from "../pack.js";

/**
 * The **English language pack** — pure data over the phrasing key (ADR-0033).
 * Source: the `en` baseline in `docs/kinship-term-inventory.md` §9.
 *
 * English is the deliberately coarse case. It merges whole descriptor regions
 * — no `side` ("grandmother" covers both), no `seniority` in any lexeme, blood
 * and in-law merged in *aunt* / *uncle* — and it needs the compositional
 * fallback in more places than a speaker expects (a co-wife, a co-father-in-law,
 * a second cousin's spouse). Nothing here keys `edgeNature`, so an adoptive
 * mother is "mother": the fact stays on the record, the phrasing does not mark
 * it.
 *
 * Two facets English *does* need are backbone-derived, which is worth noting
 * against the assumption that only Gujarati forces them:
 *
 * - `acrossAtStart` separates a step-son (`across·down`, the spouse's child)
 *   from a son-in-law (`down·across`, the child's spouse) — one normalized
 *   cell, two terms;
 * - the *brother-in-law* pile-up is written as two entries, one keying
 *   `acrossAtStart` and one `acrossAtEnd`, because English merges the two
 *   shapes but has no term for the third (`down·across·up`, a child's
 *   parent-in-law) which shares their normalized cell.
 */
export const EN: LanguagePack = {
    code: "en",
    label: "English",

    entries: [
        // --- self and spouse -------------------------------------------------
        // `self` + one `across` hop is the spouse; two hops is a co-spouse,
        // which English does not lexicalize (it composes: "husband's wife").
        { when: { classification: "self", acrossCount: 0 }, term: "self" },
        { when: { classification: "self", acrossCount: 1, alterGender: "male" }, term: "husband" },
        { when: { classification: "self", acrossCount: 1, alterGender: "female" }, term: "wife" },
        { when: { classification: "self", acrossCount: 1, alterGender: "other" }, term: "spouse" },

        // --- the blood lineal spine -----------------------------------------
        // `generations` is deliberately a wildcard: the `grand` / `great-`
        // affix rules below carry the whole unbounded spine off these six
        // terms. Keying `affinity: blood` is what stops them reaching a
        // step-parent or a parent-in-law.
        {
            when: { classification: "lineal", role: "ancestor", affinity: "blood", alterGender: "male" },
            term: "father",
        },
        {
            when: { classification: "lineal", role: "ancestor", affinity: "blood", alterGender: "female" },
            term: "mother",
        },
        {
            when: { classification: "lineal", role: "ancestor", affinity: "blood", alterGender: "other" },
            term: "parent",
        },
        {
            when: { classification: "lineal", role: "descendant", affinity: "blood", alterGender: "male" },
            term: "son",
        },
        {
            when: { classification: "lineal", role: "descendant", affinity: "blood", alterGender: "female" },
            term: "daughter",
        },
        {
            when: { classification: "lineal", role: "descendant", affinity: "blood", alterGender: "other" },
            term: "child",
        },

        // --- parents-in-law and children-in-law (generation 1 only) ----------
        // English stops at one generation: a spouse's grandmother is "your
        // wife's grandmother", so generation 2 has no entry and composes.
        {
            when: {
                classification: "lineal",
                role: "ancestor",
                generations: 1,
                affinity: "inLaw",
                acrossCount: 1,
                alterGender: "male",
            },
            term: "father-in-law",
        },
        {
            when: {
                classification: "lineal",
                role: "ancestor",
                generations: 1,
                affinity: "inLaw",
                acrossCount: 1,
                alterGender: "female",
            },
            term: "mother-in-law",
        },
        {
            when: {
                classification: "lineal",
                role: "ancestor",
                generations: 1,
                affinity: "inLaw",
                acrossCount: 1,
                alterGender: "other",
            },
            term: "parent-in-law",
        },
        // `acrossAtEnd` — the child's spouse. Its mirror (`acrossAtStart`, the
        // spouse's child) is the step-child block below; the normalized fields
        // cannot tell them apart.
        {
            when: {
                classification: "lineal",
                role: "descendant",
                generations: 1,
                affinity: "inLaw",
                acrossCount: 1,
                acrossAtEnd: true,
                alterGender: "male",
            },
            term: "son-in-law",
        },
        {
            when: {
                classification: "lineal",
                role: "descendant",
                generations: 1,
                affinity: "inLaw",
                acrossCount: 1,
                acrossAtEnd: true,
                alterGender: "female",
            },
            term: "daughter-in-law",
        },
        {
            when: {
                classification: "lineal",
                role: "descendant",
                generations: 1,
                affinity: "inLaw",
                acrossCount: 1,
                acrossAtEnd: true,
                alterGender: "other",
            },
            term: "child-in-law",
        },

        // --- step-parents and step-children ---------------------------------
        // A step-parent is `up·across` (the marriage hop is in ancestor
        // position, so `affinity: step`); a step-child is `across·down`, which
        // the position rule reads as `inLaw` — hence `acrossAtStart`.
        {
            when: {
                classification: "lineal",
                role: "ancestor",
                generations: 1,
                affinity: "step",
                alterGender: "male",
            },
            term: "stepfather",
        },
        {
            when: {
                classification: "lineal",
                role: "ancestor",
                generations: 1,
                affinity: "step",
                alterGender: "female",
            },
            term: "stepmother",
        },
        {
            when: {
                classification: "lineal",
                role: "ancestor",
                generations: 1,
                affinity: "step",
                alterGender: "other",
            },
            term: "stepparent",
        },
        {
            when: {
                classification: "lineal",
                role: "descendant",
                generations: 1,
                affinity: "inLaw",
                acrossCount: 1,
                acrossAtStart: true,
                alterGender: "male",
            },
            term: "stepson",
        },
        {
            when: {
                classification: "lineal",
                role: "descendant",
                generations: 1,
                affinity: "inLaw",
                acrossCount: 1,
                acrossAtStart: true,
                alterGender: "female",
            },
            term: "stepdaughter",
        },
        {
            when: {
                classification: "lineal",
                role: "descendant",
                generations: 1,
                affinity: "inLaw",
                acrossCount: 1,
                acrossAtStart: true,
                alterGender: "other",
            },
            term: "stepchild",
        },

        // --- siblings --------------------------------------------------------
        // `cousinDegree: 0, removed: 0` is exactly `up: 1, down: 1`.
        // No entry keys `seniority`: English has no elder/younger lexeme, so
        // the never-guess rule has nothing to fall through to here — it does
        // its work in packs that do (Gujarati's *moṭā bhāī*).
        {
            when: {
                classification: "collateral",
                cousinDegree: 0,
                removed: 0,
                affinity: "blood",
                alterGender: "male",
            },
            term: "brother",
        },
        {
            when: {
                classification: "collateral",
                cousinDegree: 0,
                removed: 0,
                affinity: "blood",
                alterGender: "female",
            },
            term: "sister",
        },
        {
            when: {
                classification: "collateral",
                cousinDegree: 0,
                removed: 0,
                affinity: "blood",
                alterGender: "other",
            },
            term: "sibling",
        },
        {
            when: {
                classification: "collateral",
                cousinDegree: 0,
                removed: 0,
                affinity: "blood",
                sharing: "half",
                alterGender: "male",
            },
            term: "half-brother",
        },
        {
            when: {
                classification: "collateral",
                cousinDegree: 0,
                removed: 0,
                affinity: "blood",
                sharing: "half",
                alterGender: "female",
            },
            term: "half-sister",
        },
        {
            when: {
                classification: "collateral",
                cousinDegree: 0,
                removed: 0,
                affinity: "blood",
                sharing: "half",
                alterGender: "other",
            },
            term: "half-sibling",
        },
        {
            when: {
                classification: "collateral",
                cousinDegree: 0,
                removed: 0,
                affinity: "step",
                alterGender: "male",
            },
            term: "stepbrother",
        },
        {
            when: {
                classification: "collateral",
                cousinDegree: 0,
                removed: 0,
                affinity: "step",
                alterGender: "female",
            },
            term: "stepsister",
        },
        {
            when: {
                classification: "collateral",
                cousinDegree: 0,
                removed: 0,
                affinity: "step",
                alterGender: "other",
            },
            term: "stepsibling",
        },

        // --- siblings-in-law -------------------------------------------------
        // Two entries per gender, one keying each end of the path, both
        // yielding the same term: English merges the spouse's sibling with the
        // sibling's spouse, and a path that is both (`across·up·down·across`,
        // the spouse's sibling's spouse) matches both entries — a legitimate
        // tie, because they agree. The shape that is neither
        // (`down·across·up`, a child's parent-in-law) has no English term and
        // composes, which is the whole reason this is not one wildcard entry.
        {
            when: {
                classification: "collateral",
                cousinDegree: 0,
                removed: 0,
                affinity: "inLaw",
                acrossAtStart: true,
                alterGender: "male",
            },
            term: "brother-in-law",
        },
        {
            when: {
                classification: "collateral",
                cousinDegree: 0,
                removed: 0,
                affinity: "inLaw",
                acrossCount: 1,
                acrossAtEnd: true,
                alterGender: "male",
            },
            term: "brother-in-law",
        },
        {
            when: {
                classification: "collateral",
                cousinDegree: 0,
                removed: 0,
                affinity: "inLaw",
                acrossAtStart: true,
                alterGender: "female",
            },
            term: "sister-in-law",
        },
        {
            when: {
                classification: "collateral",
                cousinDegree: 0,
                removed: 0,
                affinity: "inLaw",
                acrossCount: 1,
                acrossAtEnd: true,
                alterGender: "female",
            },
            term: "sister-in-law",
        },
        {
            when: {
                classification: "collateral",
                cousinDegree: 0,
                removed: 0,
                affinity: "inLaw",
                acrossAtStart: true,
                alterGender: "other",
            },
            term: "sibling-in-law",
        },
        {
            when: {
                classification: "collateral",
                cousinDegree: 0,
                removed: 0,
                affinity: "inLaw",
                acrossCount: 1,
                acrossAtEnd: true,
                alterGender: "other",
            },
            term: "sibling-in-law",
        },

        // --- aunts / uncles and nephews / nieces ------------------------------
        // `up` and `down` are both keyed exactly. A wildcard on either side
        // would be tempting — it would let one `great-` affix rule carry the
        // whole spine — but `up: 1, down: 1` is the *co-parent-in-law* cell
        // (`down·across·up`, a child's parent-in-law), which English does not
        // name at all. An entry that reached it would answer "uncle" to a
        // question English answers "my son-in-law's father". So the spine is
        // written out, and it stops where usage does; a fourth `great-` falls
        // through to the fallback.
        //
        // `affinity` is *not* keyed on the ascending side: English calls an
        // uncle's wife "aunt" and a step-parent's sibling "aunt" too. It *is*
        // keyed on the descending side, because English does **not** extend
        // "niece" to a nephew's wife — that one composes.
        //
        // `acrossAtStart: false` keeps a spouse's uncle out; that composes as
        // English does. No `other`-gender entry: English has no gender-neutral
        // term for a parent's sibling, and the fallback says so rather than
        // inventing one.
        {
            when: { classification: "collateral", up: 2, down: 1, acrossAtStart: false, alterGender: "male" },
            term: "uncle",
        },
        {
            when: { classification: "collateral", up: 2, down: 1, acrossAtStart: false, alterGender: "female" },
            term: "aunt",
        },
        {
            when: { classification: "collateral", up: 3, down: 1, acrossAtStart: false, alterGender: "male" },
            term: "great-uncle",
        },
        {
            when: { classification: "collateral", up: 3, down: 1, acrossAtStart: false, alterGender: "female" },
            term: "great-aunt",
        },
        {
            when: { classification: "collateral", up: 4, down: 1, acrossAtStart: false, alterGender: "male" },
            term: "great-great-uncle",
        },
        {
            when: { classification: "collateral", up: 4, down: 1, acrossAtStart: false, alterGender: "female" },
            term: "great-great-aunt",
        },
        {
            when: { classification: "collateral", up: 1, down: 2, affinity: "blood", alterGender: "male" },
            term: "nephew",
        },
        {
            when: { classification: "collateral", up: 1, down: 2, affinity: "blood", alterGender: "female" },
            term: "niece",
        },
        {
            when: { classification: "collateral", up: 1, down: 3, affinity: "blood", alterGender: "male" },
            term: "great-nephew",
        },
        {
            when: { classification: "collateral", up: 1, down: 3, affinity: "blood", alterGender: "female" },
            term: "great-niece",
        },
        {
            when: { classification: "collateral", up: 1, down: 4, affinity: "blood", alterGender: "male" },
            term: "great-great-nephew",
        },
        {
            when: { classification: "collateral", up: 1, down: 4, affinity: "blood", alterGender: "female" },
            term: "great-great-niece",
        },

        // --- cousins ----------------------------------------------------------
        // Gender-neutral, and keyed on the materialized `cousinDegree` /
        // `removed` — the two numbers ADR-0026 materialized precisely so a
        // consumer never re-derives them. English's ordinals are not
        // productive morphology a rule can build, so they are written out as
        // the data they are; past third cousins twice removed the fallback
        // fires, which is what speakers do too.
        { when: { classification: "collateral", cousinDegree: 1, removed: 0, affinity: "blood" }, term: "first cousin" },
        { when: { classification: "collateral", cousinDegree: 1, removed: 1, affinity: "blood" }, term: "first cousin once removed" },
        { when: { classification: "collateral", cousinDegree: 1, removed: 2, affinity: "blood" }, term: "first cousin twice removed" },
        { when: { classification: "collateral", cousinDegree: 2, removed: 0, affinity: "blood" }, term: "second cousin" },
        { when: { classification: "collateral", cousinDegree: 2, removed: 1, affinity: "blood" }, term: "second cousin once removed" },
        { when: { classification: "collateral", cousinDegree: 2, removed: 2, affinity: "blood" }, term: "second cousin twice removed" },
        { when: { classification: "collateral", cousinDegree: 3, removed: 0, affinity: "blood" }, term: "third cousin" },
        { when: { classification: "collateral", cousinDegree: 3, removed: 1, affinity: "blood" }, term: "third cousin once removed" },
        { when: { classification: "collateral", cousinDegree: 3, removed: 2, affinity: "blood" }, term: "third cousin twice removed" },
    ],

    affixes: [
        // The lineal spine: `grand` from generation 2, `great-` once more per
        // generation from 3. Applied innermost-first by ascending threshold,
        // so generation 4 is "great-great-grandmother" and never the reverse.
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
        // Ended marriages ride the same vocabulary with a non-numeric `when`:
        // "former mother-in-law", "former stepfather", "former wife". English
        // lexicalizes this; a pack whose language does not simply omits the
        // rule and leaves disclosure to the chrome.
        {
            when: { endedMarriage: true },
            affix: "former ",
            position: "prefix",
        },
    ],

    // The fallback's per-hop nouns and join. English joins left-to-right with
    // `'s`, gender-invariantly.
    hops: {
        up: { male: "father", female: "mother", other: "parent" },
        down: { male: "son", female: "daughter", other: "child" },
        across: { male: "husband", female: "wife", other: "spouse" },
    },
    genitive: "{possessor}'s {possessed}",
};
