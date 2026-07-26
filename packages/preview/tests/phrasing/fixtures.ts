import type {
    Classification,
    Gender,
    HopEdge,
    PathHop,
    RelationshipDescriptor,
} from "../../src/phrasing/descriptor.js";

/** An `up` hop onto a person of `gender`. */
export function up(gender: Gender, edge: HopEdge = "bio"): PathHop {
    return { step: "up", to: `p_up_${gender}`, gender, edge };
}

/** A `down` hop onto a person of `gender`. */
export function down(gender: Gender, edge: HopEdge = "bio"): PathHop {
    return { step: "down", to: `p_down_${gender}`, gender, edge };
}

/** An `across` (marriage) hop onto a spouse of `gender`. */
export function across(gender: Gender, status: "ongoing" | "ended" = "ongoing"): PathHop {
    return { step: "across", to: `p_across_${gender}`, gender, marriage: "m", status };
}

export const self: Classification = { kind: "self" };

export function lineal(role: "ancestor" | "descendant", generations: number): Classification {
    return { kind: "lineal", role, generations };
}

/** Collateral, with `cousinDegree` / `removed` materialized as the engine does. */
export function collateral(upHops: number, downHops: number): Classification {
    return {
        kind: "collateral",
        up: upHops,
        down: downHops,
        cousinDegree: Math.min(upHops, downHops) - 1,
        removed: Math.abs(upHops - downHops),
    };
}

/**
 * A relationship descriptor fixture. Normalized dimensions are stated
 * explicitly (that is what the engine hands the phrasing layer); the
 * boilerplate endpoints default.
 */
export function descriptorOf(
    init: Partial<RelationshipDescriptor> & {
        classification: Classification;
        path: PathHop[];
    },
): RelationshipDescriptor {
    const last = init.path[init.path.length - 1];
    return {
        egoId: "ego",
        alterId: "alter",
        egoGender: "female",
        alterGender: last?.gender ?? "female",
        edgeNature: "blood",
        affinity: "blood",
        sharing: "notApplicable",
        side: "notApplicable",
        seniority: "unknown",
        apexSeniority: "notApplicable",
        ...init,
    };
}
