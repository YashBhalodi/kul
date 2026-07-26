import type { Gender, PathHop, RelationshipDescriptor } from "./descriptor.js";
import { phrasingKeyOf, subPathKeyOf } from "./key.js";
import { lexicalize } from "./lexicalize.js";
import type { GenitivePattern, LanguagePack } from "./pack.js";

/** Whether a phrase is a term the language has, or one it had to build. */
export type PhraseKind = "lexical" | "composed";

/**
 * The phrasing layer's result. Never a bare string: chrome sizes a slot for a
 * crisp term differently than for a five-hop chain, and must not have to
 * string-sniff for apostrophes to tell them apart (ADR-0033).
 */
export interface Phrase {
    /** The bare phrase — "mother's brother", never "your mother's brother". */
    text: string;
    kind: PhraseKind;
    /** Hops spelled out as a genitive chain rather than named. `0` when lexical. */
    hopCount: number;
}

/**
 * Phrase one relationship descriptor in one language.
 *
 * Whole-path lookup first. When the language has no term, the fallback is
 * **prefix lexicalization over the same table**: walk the backbone's prefixes
 * longest to shortest, derive a sub-path key for each, look it up, and render
 * the remaining hops as a genitive chain off the first hit. So the composed
 * form reuses a term the language has — "first cousin's wife", not a five-link
 * walk from ego. There is no second lexicon.
 *
 * **Exactly one prefix is lexicalized.** The tail is spelled hop by hop and is
 * never re-read as a relationship of its own, so an `up·down` tail comes out
 * "X's mother's son" rather than "X's brother" (a spouse's uncle phrases
 * "father-in-law's mother's son"). That is a deliberate bound on the fallback,
 * not an oversight: re-lexicalizing tails would need a *second* ego to be
 * relative to, and every hop of the backbone is relative to the one ego the
 * descriptor names. Where the flattening reads as the *wrong* relationship
 * rather than a clumsy one, the fix is an entry in the pack — see the
 * `other`-gender parent's sibling in `packs/en.ts` (ADR-0039).
 */
export function phrase(descriptor: RelationshipDescriptor, pack: LanguagePack): Phrase {
    const lexical = lexicalize(phrasingKeyOf(descriptor), pack);
    if (lexical !== null) return { text: lexical, kind: "lexical", hopCount: 0 };

    const hops = descriptor.path;
    for (let head = hops.length - 1; head >= 1; head -= 1) {
        const term = lexicalize(subPathKeyOf(descriptor, head), pack);
        if (term === null) continue;
        return {
            text: chain(term, hops.slice(head), pack),
            kind: "composed",
            hopCount: hops.length - head,
        };
    }
    // Nothing lexicalized, not even a single hop: spell the whole backbone.
    // A pack whose hop lexicon covers `up` / `down` / `across` always reaches
    // here with something to say; only the empty path (`self`) can yield "",
    // and the coverage test requires every pack to name that.
    const first = hops[0];
    if (first === undefined) return { text: "", kind: "composed", hopCount: 0 };
    return {
        text: chain(hopNoun(first, pack), hops.slice(1), pack),
        kind: "composed",
        hopCount: hops.length,
    };
}

/** Fold a genitive chain left-to-right off a lexicalized head. */
function chain(head: string, tail: ReadonlyArray<PathHop>, pack: LanguagePack): string {
    return tail.reduce(
        (possessor, hop) => join(pack.genitive, possessor, hopNoun(hop, pack), hop.gender),
        head,
    );
}

function join(
    pattern: GenitivePattern,
    possessor: string,
    possessed: string,
    possessedGender: Gender,
): string {
    const template = typeof pattern === "string" ? pattern : pattern[possessedGender];
    return template.replace("{possessor}", possessor).replace("{possessed}", possessed);
}

/** The pack's gendered noun for one hop. */
function hopNoun(hop: PathHop, pack: LanguagePack): string {
    return pack.hops[hop.step][hop.gender];
}
