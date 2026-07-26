import type { RelationshipDescriptor } from "../../src/phrasing/descriptor.js";
import { phrasingKeyOf } from "../../src/phrasing/key.js";
import { affixOrderKey, affixVerdict, facetsMatch, specificityOf } from "../../src/phrasing/lexicalize.js";
import type { LanguagePack } from "../../src/phrasing/pack.js";

/** How a descriptor's backbone reads in a failure message. */
export function describePath(descriptor: RelationshipDescriptor): string {
    return descriptor.path.map((hop) => `${hop.step}:${hop.gender[0]}`).join("·") || "(self)";
}

/**
 * Entries that tie for maximum specificity on some enumerated key while
 * yielding **different** terms — the pack defect of ADR-0039. Tied entries
 * that agree are legitimate (English writes *brother-in-law* twice).
 *
 * Lives here rather than inline in the suite so the detector can itself be
 * tested against a pack with a known defect: a conflict scan that silently
 * reports every pack clean is worse than no scan at all.
 */
export function findEntryConflicts(
    pack: LanguagePack,
    descriptors: ReadonlyArray<RelationshipDescriptor>,
): string[] {
    const conflicts = new Set<string>();
    for (const descriptor of descriptors) {
        const key = phrasingKeyOf(descriptor);
        let best = -1;
        let winners: string[] = [];
        for (const entry of pack.entries) {
            if (!facetsMatch(entry.when, key)) continue;
            const specificity = specificityOf(entry.when);
            if (specificity > best) {
                best = specificity;
                winners = [entry.term];
            } else if (specificity === best) {
                winners.push(entry.term);
            }
        }
        const distinct = [...new Set(winners)].sort();
        if (distinct.length > 1) {
            conflicts.add(`${distinct.join(" | ")} on ${describePath(descriptor)}`);
        }
    }
    return [...conflicts];
}

/**
 * Affix rules that fire on one key in the same order slot with different
 * affixes — the composition would depend on declaration order.
 */
export function findAffixConflicts(
    pack: LanguagePack,
    descriptors: ReadonlyArray<RelationshipDescriptor>,
): string[] {
    const conflicts = new Set<string>();
    for (const descriptor of descriptors) {
        const key = phrasingKeyOf(descriptor);
        const slots = new Map<string, string>();
        for (const rule of pack.affixes) {
            if (affixVerdict(rule, key).kind !== "fires") continue;
            const slot = JSON.stringify(affixOrderKey(rule));
            const held = slots.get(slot);
            if (held !== undefined && held !== rule.affix) {
                conflicts.add(`${held} | ${rule.affix} on ${describePath(descriptor)}`);
            }
            slots.set(slot, rule.affix);
        }
    }
    return [...conflicts];
}
