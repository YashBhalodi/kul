import { describe, expect, it } from "vitest";

import { PACKS } from "../../src/phrasing/packs/index.js";
import { PHRASING_FACETS, phrasingKeyOf, specificityOf } from "../../src/phrasing/index.js";
import { affixOrderKey, affixVerdict, facetsMatch } from "../../src/phrasing/lexicalize.js";
import { phrase } from "../../src/phrasing/phrase.js";
import type { LanguagePack } from "../../src/phrasing/pack.js";
import { enumerateDescriptors, type EnumerationBounds } from "./enumerate.js";

/**
 * The pack suites. They walk `PACKS`, so a new language pack inherits every
 * invariant below the moment it is registered — no test edit, which is the
 * additivity promise stated as a diff shape.
 */

/** Conflict-freedom needs breadth of *shape*, so genders are enumerated fully. */
const CONFLICT_BOUNDS: EnumerationBounds = {
    maxUp: 3,
    maxDown: 3,
    maxAcross: 2,
    maxHops: 5,
    genderMode: "product",
};

/**
 * Coverage needs depth of *classification*: `generations`, `cousinDegree` and
 * `removed` bounded at 4 (ADR-0033 — past that the fallback fires anyway), so
 * ascents and descents run to 5 and genders stay canonical.
 */
const COVERAGE_BOUNDS: EnumerationBounds = {
    maxUp: 5,
    maxDown: 5,
    maxAcross: 2,
    maxHops: 11,
    genderMode: "canonical",
};

const conflictDescriptors = enumerateDescriptors(CONFLICT_BOUNDS, PACKS);
const coverageDescriptors = enumerateDescriptors(COVERAGE_BOUNDS, PACKS);

describe.each(PACKS.map((pack) => [pack.code, pack] as const))("pack %s", (_code, pack) => {
    it("declares only facets the key space has, and never keys `unknown`", () => {
        const facets = new Set<string>(PHRASING_FACETS);
        const offenders: string[] = [];
        for (const entry of pack.entries) {
            for (const [facet, value] of Object.entries(entry.when)) {
                if (!facets.has(facet)) offenders.push(`entry "${entry.term}" keys ${facet}`);
                if (value === "unknown") {
                    offenders.push(`entry "${entry.term}" keys ${facet}: unknown`);
                }
            }
        }
        for (const rule of pack.affixes) {
            for (const [facet, value] of Object.entries(rule.when)) {
                if (facet === "atLeast") continue;
                if (!facets.has(facet)) offenders.push(`affix "${rule.affix}" keys ${facet}`);
                if (value === "unknown") {
                    offenders.push(`affix "${rule.affix}" keys ${facet}: unknown`);
                }
            }
        }
        expect(offenders).toEqual([]);
    });

    it("gives every affix rule that caps or repeats a governed numeric facet", () => {
        const offenders = pack.affixes.filter(
            (rule) =>
                ((rule.cap !== undefined || rule.repeatPer !== undefined) &&
                    rule.when.atLeast === undefined &&
                    rule.repeatPer === undefined) ||
                (rule.repeatPer !== undefined &&
                    rule.when.atLeast !== undefined &&
                    rule.when.atLeast.facet !== rule.repeatPer),
        );
        expect(offenders.map((rule) => rule.affix)).toEqual([]);
    });

    it("resolves every key in the bounded enumeration to a term or a well-formed chain", () => {
        const broken: string[] = [];
        for (const descriptor of coverageDescriptors) {
            const result = phrase(descriptor, pack);
            const shape = `${JSON.stringify(descriptor.classification)} ${descriptor.affinity} ${describePath(descriptor)}`;
            if (result.text.trim() === "" || result.text !== result.text.trim()) {
                broken.push(`empty/untrimmed: ${shape}`);
            }
            if (result.text.includes("undefined") || result.text.includes("{")) {
                broken.push(`unsubstituted: ${shape} -> ${result.text}`);
            }
            if (result.kind === "lexical" && result.hopCount !== 0) {
                broken.push(`lexical with hops: ${shape}`);
            }
            if (result.kind === "composed" && result.hopCount < 1) {
                broken.push(`composed without hops: ${shape}`);
            }
            if (result.kind === "composed" && result.hopCount > descriptor.path.length) {
                broken.push(`composed past the backbone: ${shape}`);
            }
            // A well-formed chain spells its tail: the last hop's noun is in it.
            const lastHop = descriptor.path[descriptor.path.length - 1];
            if (
                result.kind === "composed" &&
                lastHop !== undefined &&
                !result.text.includes(pack.hops[lastHop.step][lastHop.gender])
            ) {
                broken.push(`chain drops its tail: ${shape} -> ${result.text}`);
            }
        }
        expect(broken.slice(0, 10)).toEqual([]);
    });

    it("has no two entries winning one key with equal specificity and different terms", () => {
        const conflicts = new Set<string>();
        for (const descriptor of conflictDescriptors) {
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
            if (new Set(winners).size > 1) {
                conflicts.add(`${[...new Set(winners)].sort().join(" | ")} on ${describePath(descriptor)}`);
            }
        }
        expect([...conflicts].slice(0, 10)).toEqual([]);
    });

    it("has no two affix rules firing on one key in the same order slot", () => {
        const conflicts = new Set<string>();
        for (const descriptor of conflictDescriptors) {
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
        expect([...conflicts]).toEqual([]);
    });

    it("names the empty path, so `self` never phrases as nothing", () => {
        const selfDescriptor = coverageDescriptors.find((d) => d.path.length === 0);
        expect(selfDescriptor).toBeDefined();
        expect(phrase(selfDescriptor!, pack)).toMatchObject({ kind: "lexical", hopCount: 0 });
    });

    it("covers every hop of the backbone in its hop lexicon", () => {
        for (const step of ["up", "down", "across"] as const) {
            for (const gender of ["male", "female", "other"] as const) {
                expect(nonEmpty(pack, step, gender)).toBe(true);
            }
        }
    });
});

function nonEmpty(
    pack: LanguagePack,
    step: "up" | "down" | "across",
    gender: "male" | "female" | "other",
): boolean {
    return pack.hops[step][gender].trim().length > 0;
}

function describePath(descriptor: { path: ReadonlyArray<{ step: string; gender: string }> }): string {
    return descriptor.path.map((hop) => `${hop.step}:${hop.gender[0]}`).join("·") || "(self)";
}
