import type { AffixRule, LanguagePack, PackEntry } from "./pack.js";
import type { FacetMatch, PhrasingKey } from "./key.js";

/**
 * Does a partial facet match apply to this key?
 *
 * Two rules, both load-bearing (ADR-0033):
 *
 * - an omitted facet is a **wildcard**;
 * - a facet that is `"unknown"` on the key **disqualifies** the entry that
 *   keys it, so lookup falls through to the less specific, unmarked entry.
 *   `seniority: unknown` yields "brother", never a guessed elder form.
 *   `notApplicable` is a value and matches normally.
 */
export function facetsMatch(when: FacetMatch, key: PhrasingKey): boolean {
    for (const facet of Object.keys(when) as Array<keyof PhrasingKey>) {
        const expected = when[facet];
        if (expected === undefined) continue;
        const actual = key[facet];
        if (actual === "unknown") return false;
        if (actual !== expected) return false;
    }
    return true;
}

/** How many facets a match keys. Precedence is by this number, nothing else. */
export function specificityOf(when: FacetMatch): number {
    return Object.keys(when).filter((facet) => when[facet as keyof PhrasingKey] !== undefined)
        .length;
}

/**
 * The entries that match `key` and tie for maximum specificity — one member
 * for a well-formed pack. Two or more members yielding **different** terms is
 * a defect the pack's conflict test catches; resolving that by declaration
 * order would make precedence invisible and order-fragile.
 */
export function winningEntries(
    key: PhrasingKey,
    entries: ReadonlyArray<PackEntry>,
): ReadonlyArray<PackEntry> {
    let best = -1;
    let winners: PackEntry[] = [];
    for (const entry of entries) {
        if (!facetsMatch(entry.when, key)) continue;
        const specificity = specificityOf(entry.when);
        if (specificity > best) {
            best = specificity;
            winners = [entry];
        } else if (specificity === best) {
            winners.push(entry);
        }
    }
    return winners;
}

/**
 * The total order affix rules apply in: threshold ascending (a thresholdless
 * rule sorts last and therefore ends up outermost — *former* wraps
 * *mother-in-law*), then governed facet, then specificity descending, then the
 * affix itself. Nothing here consults declaration order.
 */
export function affixOrderKey(rule: AffixRule): [number, string, number, string] {
    const { atLeast, ...facets } = rule.when;
    return [
        atLeast?.value ?? Number.POSITIVE_INFINITY,
        rule.repeatPer ?? atLeast?.facet ?? "",
        -specificityOf(facets),
        rule.affix,
    ];
}

function compareAffixRules(a: AffixRule, b: AffixRule): number {
    const ka = affixOrderKey(a);
    const kb = affixOrderKey(b);
    for (let i = 0; i < ka.length; i += 1) {
        if (ka[i]! < kb[i]!) return -1;
        if (ka[i]! > kb[i]!) return 1;
    }
    return 0;
}

/** What an affix rule does to one key. */
type AffixVerdict =
    /** Facets do not match, or the numeric floor is unmet: the rule is silent. */
    | { kind: "silent" }
    /** The governed facet is past `cap`: the language has no term here at all. */
    | { kind: "refuses" }
    | { kind: "fires"; count: number };

/** Evaluate one affix rule against one key. */
export function affixVerdict(rule: AffixRule, key: PhrasingKey): AffixVerdict {
    const { atLeast, ...facets } = rule.when;
    if (!facetsMatch(facets, key)) return { kind: "silent" };
    const governed = rule.repeatPer ?? atLeast?.facet;
    const value = governed === undefined ? undefined : key[governed];
    if (rule.cap !== undefined && typeof value === "number" && value > rule.cap) {
        return { kind: "refuses" };
    }
    if (atLeast !== undefined) {
        const floorValue = key[atLeast.facet];
        if (typeof floorValue !== "number" || floorValue < atLeast.value) return { kind: "silent" };
    }
    if (rule.repeatPer === undefined) return { kind: "fires", count: 1 };
    const repeatValue = key[rule.repeatPer];
    if (typeof repeatValue !== "number") return { kind: "silent" };
    const floor = atLeast?.value ?? repeatValue;
    return { kind: "fires", count: Math.max(1, repeatValue - floor + 1) };
}

/**
 * One rendered token: the phrase text and, when the pack supplies one all the
 * way through, the same token in Latin script.
 *
 * `translit` is **absent, never approximated**: a record with no Latin form
 * makes the whole form untransliterated rather than half-romanized, and no
 * romanization is ever computed from the script (ADR-0044).
 */
export interface LexicalForm {
    text: string;
    translit?: string;
}

/**
 * Look one key up in a pack: the winning entry's term, decorated by every
 * affix rule that fires. `null` when no entry matches, or when a rule's `cap`
 * refuses — both mean "this language has no term here", and the caller falls
 * back to composition.
 */
export function lexicalize(key: PhrasingKey, pack: LanguagePack): LexicalForm | null {
    const winners = winningEntries(key, pack.entries);
    if (winners.length === 0) return null;
    // A tie is a pack defect (caught by the conflict test); picking by term
    // keeps runtime behaviour deterministic rather than order-dependent. The
    // winner is carried as a record, not as a bare string, so its Latin form
    // travels with the term it belongs to.
    const winner = [...winners].sort((a, b) =>
        a.term < b.term ? -1 : a.term > b.term ? 1 : 0,
    )[0]!;
    let text = winner.term;
    let translit = winner.translit;

    const firing: Array<{ rule: AffixRule; count: number }> = [];
    for (const rule of pack.affixes) {
        const verdict = affixVerdict(rule, key);
        if (verdict.kind === "refuses") return null;
        if (verdict.kind === "fires") firing.push({ rule, count: verdict.count });
    }
    firing.sort((a, b) => compareAffixRules(a.rule, b.rule));
    for (const { rule, count } of firing) {
        const affix = rule.affix.repeat(count);
        text = rule.position === "prefix" ? affix + text : text + affix;
        if (translit === undefined || rule.translit === undefined) {
            // A rule with no Latin affix cannot decorate a Latin term, so the
            // whole form loses its transliteration rather than gaining a
            // half-romanized one.
            translit = undefined;
            continue;
        }
        const latin = rule.translit.repeat(count);
        translit = rule.position === "prefix" ? latin + translit : translit + latin;
    }
    return translit === undefined ? { text } : { text, translit };
}
