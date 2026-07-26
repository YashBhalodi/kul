import type { Gender } from "./descriptor.js";
import type { FacetMatch, NumericFacet } from "./key.js";

/**
 * One lexical entry: a partial match over the {@link FacetMatch phrasing key}
 * and the term it yields. An omitted facet is a wildcard, so a coarse entry
 * and a specific one legitimately coexist — **precedence is by specificity**
 * (the entry keying the most facets wins) and declaration order carries no
 * meaning whatsoever.
 */
export interface PackEntry {
    when: FacetMatch;
    term: string;
    /**
     * The term in Latin script, for a pack whose `term` is not already Latin.
     * The reading surface for it is the hover gloss on the lens pill (#302).
     *
     * **Never derived.** There is no romanization algorithm anywhere in this
     * layer: an entry that omits this yields a phrase with no transliteration
     * at all, exactly as `seniority: unknown` yields the unmarked term rather
     * than a guessed one (ADR-0033's never-guess rule, ADR-0044). `en` omits it
     * throughout, because a term already in Latin script has nothing to gloss.
     */
    translit?: string;
}

/** A numeric floor on one facet, carried inside an affix rule's `when`. */
export interface NumericBound {
    facet: NumericFacet;
    value: number;
}

/**
 * Productive morphology, as data. A flat table cannot cover English's
 * unbounded `great-great-…` spine or Gujarati's single productive `par-`, so a
 * pack declares affix rules alongside its entries (ADR-0033).
 *
 * The `affix` is concatenated **literally**, with no separator the layer
 * invents: `"grand"` yields *grandmother*, `"great-"` yields *great-grandmother*,
 * `"former "` yields *former mother-in-law*. Hyphenation is the pack's
 * business, which is what keeps the shared layer free of per-language code.
 *
 * Rules apply innermost-first in a fully determined order — by threshold value
 * ascending (thresholdless rules last, so they end up outermost), then by
 * governed facet, then by specificity descending, then by the affix itself.
 * Declaration order is meaningless here too.
 */
export interface AffixRule {
    /** Facet match, plus an optional numeric floor on one numeric facet. */
    when: FacetMatch & { atLeast?: NumericBound };
    affix: string;
    /** The affix in Latin script. Same never-derived rule as {@link PackEntry.translit}. */
    translit?: string;
    position: "prefix" | "suffix";
    /**
     * Repeat the affix once per unit of this facet at and above the floor:
     * `great-` with `atLeast {generations, 3}` yields one at generations 3,
     * two at 4. Requires `when.atLeast` on the same facet.
     */
    repeatPer?: NumericFacet;
    /**
     * Ceiling on the governed facet (`repeatPer ?? when.atLeast.facet`). Above
     * it the language has no term for this region at all, so lexicalization is
     * **refused** and the compositional fallback fires — which is what
     * speakers actually do past Gujarati's *par-dādā*. A `notApplicable` facet
     * value is not a number and is never capped.
     */
    cap?: number;
}

/**
 * Gendered nouns for a single hop, used by the fallback's genitive chain.
 * Keyed by the gender of the person the hop lands on.
 */
export interface HopLexicon {
    up: Record<Gender, string>;
    down: Record<Gender, string>;
    across: Record<Gender, string>;
}

/**
 * How the language joins a possessor to a possessed noun. `{possessor}` and
 * `{possessed}` are substituted. A plain string is gender-invariant (English
 * `'s`); a record selects by the *possessed* noun's gender, which is what
 * Gujarati's agreeing `-no` / `-nī` / `-nũ` needs.
 */
export type GenitivePattern = string | Record<Gender, string>;

/**
 * A language pack is **data**: entries, affix rules, a hop lexicon and a
 * genitive pattern. Adding a language is one additive module of records and
 * zero lines of logic — there is deliberately no per-language code hook
 * (ADR-0033).
 */
export interface LanguagePack {
    /** BCP-47 language subtag; also the `lang=` value chrome should emit. */
    code: string;
    /** Endonym-ish display label for a locale control. */
    label: string;
    entries: ReadonlyArray<PackEntry>;
    affixes: ReadonlyArray<AffixRule>;
    hops: HopLexicon;
    genitive: GenitivePattern;
    /**
     * The Latin-script twins of `hops` and `genitive` — the two token sources a
     * *composed* phrase draws on that are not entries. A pack supplies them iff
     * it supplies `translit` on its records, which
     * `tests/phrasing/pack-invariants.test.ts` checks: a pack romanizes
     * completely or not at all, because a gloss that appears on some terms and
     * not others reads as breakage rather than as an absence (ADR-0044).
     *
     * A phrase is transliterated only when **every** token it is built from has
     * a Latin form; one missing token means no transliteration, never a mixed-
     * script string.
     */
    hopsTranslit?: HopLexicon;
    genitiveTranslit?: GenitivePattern;
}
