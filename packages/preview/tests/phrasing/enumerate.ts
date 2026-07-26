import type {
    Gender,
    PathHop,
    RelationshipDescriptor,
    Seniority,
    Sharing,
    Side,
} from "../../src/phrasing/descriptor.js";
import type { PhrasingKey } from "../../src/phrasing/key.js";
import type { LanguagePack } from "../../src/phrasing/pack.js";
import { collateral, lineal, self } from "./fixtures.js";

/**
 * A bounded enumeration of *realizable* relationship descriptors — the key
 * space the pack suites walk.
 *
 * Built structurally from the path grammar (1–3 blood segments, each `up*
 * down*`, joined by at most two `across` hops), so every descriptor it yields
 * is one the engine could actually emit. Classification, affinity and side are
 * derived here from the shape parameters rather than by calling the phrasing
 * layer, so the suites are not checking the module against itself.
 */
export interface EnumerationBounds {
    /** Ceiling on total `up` hops (bounds `generations` / `cousinDegree`). */
    maxUp: number;
    /** Ceiling on total `down` hops. */
    maxDown: number;
    /** Ceiling on `across` hops — 2 is ADR-0027's semantics, never a knob. */
    maxAcross: number;
    /** Ceiling on total hops, to keep the product tractable. */
    maxHops: number;
    /**
     * `product` walks every gender assignment (use with small `maxHops`);
     * `canonical` uses a handful of representative assignments.
     */
    genderMode: "product" | "canonical";
}

const GENDERS: ReadonlyArray<Gender> = ["male", "female", "other"];

/** All arrays of `slots` non-negative integers summing to at most `max`. */
function compositions(slots: number, max: number): number[][] {
    if (slots === 0) return [[]];
    const out: number[][] = [];
    for (let head = 0; head <= max; head += 1) {
        for (const tail of compositions(slots - 1, max - head)) out.push([head, ...tail]);
    }
    return out;
}

function genderAssignments(length: number, mode: "product" | "canonical"): Gender[][] {
    if (length === 0) return [[]];
    if (mode === "canonical") {
        return [
            Array.from({ length }, () => "male" as Gender),
            Array.from({ length }, () => "female" as Gender),
            Array.from({ length }, (_, i) => (i % 2 === 0 ? "female" : "male") as Gender),
            Array.from({ length }, () => "other" as Gender),
        ];
    }
    let out: Gender[][] = [[]];
    for (let i = 0; i < length; i += 1) {
        const grown: Gender[][] = [];
        for (const prefix of out) {
            for (const gender of GENDERS) grown.push([...prefix, gender]);
        }
        out = grown;
    }
    return out;
}

/** The values any pack discriminates on for one facet. */
function keyedValues(packs: ReadonlyArray<LanguagePack>, facet: keyof PhrasingKey): Set<unknown> {
    const values = new Set<unknown>();
    for (const pack of packs) {
        for (const entry of pack.entries) {
            if (entry.when[facet] !== undefined) values.add(entry.when[facet]);
        }
        for (const rule of pack.affixes) {
            if (rule.when[facet] !== undefined) values.add(rule.when[facet]);
        }
    }
    return values;
}

/**
 * Keep the first candidate (always) plus every candidate some pack can
 * discriminate on. The enumeration therefore *widens itself* as packs grow: a
 * pack that starts keying `apexSeniority` gets those keys enumerated for free,
 * with no test change.
 */
function narrow<T>(
    candidates: ReadonlyArray<T>,
    facet: keyof PhrasingKey,
    packs: ReadonlyArray<LanguagePack>,
): T[] {
    const keyed = keyedValues(packs, facet);
    return candidates.filter((value, index) => index === 0 || keyed.has(value));
}

function sideOfGender(gender: Gender): Side {
    if (gender === "female") return "maternal";
    if (gender === "male") return "paternal";
    return "other";
}

export function enumerateDescriptors(
    bounds: EnumerationBounds,
    packs: ReadonlyArray<LanguagePack>,
): RelationshipDescriptor[] {
    const out: RelationshipDescriptor[] = [];

    for (let acrossCount = 0; acrossCount <= bounds.maxAcross; acrossCount += 1) {
        const segments = acrossCount + 1;
        for (const ups of compositions(segments, bounds.maxUp)) {
            for (const downs of compositions(segments, bounds.maxDown)) {
                const totalUp = ups.reduce((a, b) => a + b, 0);
                const totalDown = downs.reduce((a, b) => a + b, 0);
                const length = totalUp + totalDown + acrossCount;
                if (length > bounds.maxHops) continue;

                // Hop kinds: `up^a down^b` per segment, joined by `across`.
                const kinds: Array<PathHop["step"]> = [];
                for (let s = 0; s < segments; s += 1) {
                    for (let i = 0; i < ups[s]!; i += 1) kinds.push("up");
                    for (let i = 0; i < downs[s]!; i += 1) kinds.push("down");
                    if (s < segments - 1) kinds.push("across");
                }

                // `affinity` from the ancestor-position rule, read off the
                // shape: only a first `across` preceded solely by `up` hops
                // qualifies, so any second `across` forces `inLaw`.
                const affinity =
                    acrossCount === 0
                        ? "blood"
                        : acrossCount === 1 && downs[0] === 0 && ups[0]! >= 1
                          ? "step"
                          : "inLaw";

                const classification =
                    totalUp === 0 && totalDown === 0
                        ? self
                        : totalDown === 0
                          ? lineal("ancestor", totalUp)
                          : totalUp === 0
                            ? lineal("descendant", totalDown)
                            : collateral(totalUp, totalDown);

                const hasJunction = ups.some((u, s) => u > 0 && downs[s]! > 0);
                const couplePossible = ups[0] === 1 && downs[0]! >= 1;

                for (const genders of genderAssignments(length, bounds.genderMode)) {
                    for (const edgeNature of narrow(
                        ["blood", "adoptive"] as const,
                        "edgeNature",
                        packs,
                    )) {
                        const path: PathHop[] = kinds.map((step, i) => {
                            const gender = genders[i]!;
                            if (step === "across") {
                                return { step, to: `p${i}`, gender, marriage: "m", status: "ongoing" };
                            }
                            return {
                                step,
                                to: `p${i}`,
                                gender,
                                edge: edgeNature === "adoptive" && i === 0 ? "adoptive" : "bio",
                            };
                        });

                        const sideBase: Side =
                            ups[0] === 0 || length <= 1
                                ? "notApplicable"
                                : sideOfGender(genders[0] ?? "male");
                        const sides = couplePossible ? [sideBase, "both" as Side] : [sideBase];
                        const sharings: Sharing[] = hasJunction
                            ? ["full", "half"]
                            : ["notApplicable"];
                        const apexSeniorities: Seniority[] = hasJunction
                            ? ["elder", "younger", "unknown"]
                            : ["notApplicable"];
                        const seniorities: Seniority[] =
                            length === 0 ? ["notApplicable"] : ["elder", "younger", "unknown"];

                        for (const side of narrow(sides, "side", packs)) {
                            for (const sharing of narrow(sharings, "sharing", packs)) {
                                for (const apexSeniority of narrow(
                                    apexSeniorities,
                                    "apexSeniority",
                                    packs,
                                )) {
                                    for (const seniority of narrow(
                                        seniorities,
                                        "seniority",
                                        packs,
                                    )) {
                                        for (const egoGender of narrow(
                                            GENDERS,
                                            "egoGender",
                                            packs,
                                        )) {
                                            out.push({
                                                egoId: "ego",
                                                alterId: "alter",
                                                egoGender,
                                                alterGender: genders[length - 1] ?? egoGender,
                                                classification,
                                                edgeNature,
                                                affinity,
                                                sharing,
                                                side,
                                                seniority,
                                                apexSeniority,
                                                path,
                                            });
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
    }
    return out;
}
