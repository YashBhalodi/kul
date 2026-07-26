import type {
    Affinity,
    EdgeNature,
    Gender,
    LinealRole,
    PathHop,
    RelationshipDescriptor,
    Seniority,
    Sharing,
    Side,
} from "./descriptor.js";

/**
 * The **phrasing key** (ADR-0033): one flat record of facets a language pack
 * matches on. It is the relationship descriptor's normalized dimensions —
 * taken from the descriptor, never re-derived — plus six facets derived
 * mechanically from the path backbone.
 *
 * Two facets carry a value the wire form does not have: `sharing` and `side`
 * gain `"unknown"`, which only a {@link subPathKeyOf sub-path key} produces.
 * `"unknown"` is never a guess and never matches: an entry keying a facet that
 * is `unknown` on the key is disqualified (see `lexicalize`). `notApplicable`
 * is a *value* — a fact about the path's shape — and matches normally.
 *
 * Flat by design: a pack entry is a `Partial<PhrasingKey>` and its specificity
 * is its number of own keys, so precedence is countable rather than implicit.
 */
export interface PhrasingKey {
    /** `classification.kind` of the descriptor. */
    classification: "self" | "lineal" | "collateral";
    /** Lineal direction; `notApplicable` on self / collateral keys. */
    role: LinealRole | "notApplicable";
    /** Lineal hop count; `notApplicable` on self / collateral keys. */
    generations: number | "notApplicable";
    /** Collateral ascent; `notApplicable` on self / lineal keys. */
    up: number | "notApplicable";
    /** Collateral descent; `notApplicable` on self / lineal keys. */
    down: number | "notApplicable";
    /** Materialized `min(up,down) − 1`; `notApplicable` off collateral keys. */
    cousinDegree: number | "notApplicable";
    /** Materialized `|up − down|`; `notApplicable` off collateral keys. */
    removed: number | "notApplicable";
    edgeNature: EdgeNature;
    affinity: Affinity;
    sharing: Sharing | "unknown";
    side: Side | "unknown";
    seniority: Seniority;
    apexSeniority: Seniority;
    egoGender: Gender;
    alterGender: Gender;
    /** Derived: number of `across` hops (0–2 under ADR-0027's ceiling). */
    acrossCount: number;
    /** Derived: the path's *first* hop is `across` — it runs through ego's own spouse. */
    acrossAtStart: boolean;
    /** Derived: the path's *last* hop is `across` — the alter is someone's spouse. */
    acrossAtEnd: boolean;
    /** Derived: gender of the person a path-initial `across` lands on. */
    spouseGender: Gender | "notApplicable";
    /** Derived: gender of the person the **first `down` hop** lands on. */
    linkGender: Gender | "notApplicable";
    /** Derived: any `across` hop on the path carries `status: "ended"`. */
    endedMarriage: boolean;
}

/** Every facet name, in declaration order. The key space is closed. */
export const PHRASING_FACETS = [
    "classification",
    "role",
    "generations",
    "up",
    "down",
    "cousinDegree",
    "removed",
    "edgeNature",
    "affinity",
    "sharing",
    "side",
    "seniority",
    "apexSeniority",
    "egoGender",
    "alterGender",
    "acrossCount",
    "acrossAtStart",
    "acrossAtEnd",
    "spouseGender",
    "linkGender",
    "endedMarriage",
] as const satisfies ReadonlyArray<keyof PhrasingKey>;

/** The facets an affix rule can threshold or repeat over. */
export const NUMERIC_FACETS = [
    "generations",
    "up",
    "down",
    "cousinDegree",
    "removed",
    "acrossCount",
] as const satisfies ReadonlyArray<keyof PhrasingKey>;

export type NumericFacet = (typeof NUMERIC_FACETS)[number];

/**
 * A partial match over the phrasing key. An omitted facet is a wildcard; a
 * present one must be equal. Specificity is `Object.keys(match).length`.
 */
export type FacetMatch = Partial<PhrasingKey>;

/** The phrasing key of a whole descriptor. */
export function phrasingKeyOf(descriptor: RelationshipDescriptor): PhrasingKey {
    const c = descriptor.classification;
    return {
        classification: c.kind,
        role: c.kind === "lineal" ? c.role : "notApplicable",
        generations: c.kind === "lineal" ? c.generations : "notApplicable",
        up: c.kind === "collateral" ? c.up : "notApplicable",
        down: c.kind === "collateral" ? c.down : "notApplicable",
        cousinDegree: c.kind === "collateral" ? c.cousinDegree : "notApplicable",
        removed: c.kind === "collateral" ? c.removed : "notApplicable",
        edgeNature: descriptor.edgeNature,
        affinity: descriptor.affinity,
        sharing: descriptor.sharing,
        side: descriptor.side,
        seniority: descriptor.seniority,
        apexSeniority: descriptor.apexSeniority,
        egoGender: descriptor.egoGender,
        alterGender: descriptor.alterGender,
        ...derivedFacetsOf(descriptor.path),
    };
}

/**
 * The phrasing key of the first `hopCount` hops of a descriptor's backbone —
 * the input to the fallback's prefix lexicalization.
 *
 * A hop sequence is not a graph, so three dimensions it cannot honestly
 * compute are marked `"unknown"` rather than guessed (ADR-0033):
 *
 * - `sharing` and `apexSeniority` compare the two branch siblings' parent sets
 *   and birth order at the apex junction — graph facts the hops do not carry;
 * - `side` keeps its routing derivation, except where the answer could be
 *   `both`: that is exactly an initial `up` hop immediately followed by a
 *   `down` hop (a couple apex), so that shape yields `unknown`;
 * - `seniority` compares two birth dates, which no hop carries either.
 *
 * The never-guess rule then makes all four automatically safe: an entry that
 * keys one of them is disqualified instead of matching on an invented value.
 */
export function subPathKeyOf(
    descriptor: RelationshipDescriptor,
    hopCount: number,
): PhrasingKey {
    const hops = descriptor.path.slice(0, hopCount);
    const last = hops[hops.length - 1];
    return {
        ...classificationFacetsOf(hops),
        edgeNature: edgeNatureOf(hops),
        affinity: affinityOf(hops),
        sharing: "unknown",
        side: sideOf(hops),
        seniority: "unknown",
        apexSeniority: "unknown",
        egoGender: descriptor.egoGender,
        alterGender: last === undefined ? descriptor.egoGender : last.gender,
        ...derivedFacetsOf(hops),
    };
}

type ClassificationFacets = Pick<
    PhrasingKey,
    "classification" | "role" | "generations" | "up" | "down" | "cousinDegree" | "removed"
>;

/** `classification` and its materialized numbers, from vertical hop counts. */
function classificationFacetsOf(hops: ReadonlyArray<PathHop>): ClassificationFacets {
    const up = hops.filter((h) => h.step === "up").length;
    const down = hops.filter((h) => h.step === "down").length;
    const none = {
        role: "notApplicable",
        generations: "notApplicable",
        up: "notApplicable",
        down: "notApplicable",
        cousinDegree: "notApplicable",
        removed: "notApplicable",
    } as const;
    if (up === 0 && down === 0) return { classification: "self", ...none };
    if (down === 0) return { classification: "lineal", ...none, role: "ancestor", generations: up };
    if (up === 0) {
        return { classification: "lineal", ...none, role: "descendant", generations: down };
    }
    return {
        classification: "collateral",
        ...none,
        up,
        down,
        cousinDegree: Math.min(up, down) - 1,
        removed: Math.abs(up - down),
    };
}

/** `adoptive` iff any vertical hop is an adoption edge. */
function edgeNatureOf(hops: ReadonlyArray<PathHop>): EdgeNature {
    return hops.some((h) => h.step !== "across" && h.edge === "adoptive") ? "adoptive" : "blood";
}

/**
 * `blood` with no `across` hop; `step` when every `across` hop is in ancestor
 * position (preceded by at least one hop, all of them `up`); `inLaw` otherwise.
 */
function affinityOf(hops: ReadonlyArray<PathHop>): Affinity {
    let any = false;
    let allAncestorPosition = true;
    hops.forEach((hop, i) => {
        if (hop.step !== "across") return;
        any = true;
        const ancestorPosition = i >= 1 && hops.slice(0, i).every((h) => h.step === "up");
        if (!ancestorPosition) allAncestorPosition = false;
    });
    if (!any) return "blood";
    return allAncestorPosition ? "step" : "inLaw";
}

/**
 * `side` from a hop sequence alone: the routing rule of ADR-0026, with the
 * couple-apex `both` case reported as `unknown` because it needs the graph.
 */
function sideOf(hops: ReadonlyArray<PathHop>): Side | "unknown" {
    const first = hops[0];
    if (first === undefined || first.step !== "up") return "notApplicable";
    if (hops.length === 1) return "notApplicable";
    // A first `up` immediately followed by a `down` is the one shape whose
    // apex may be a couple apex, which would make `side` = `both`.
    if (hops[1]?.step === "down") return "unknown";
    if (first.gender === "female") return "maternal";
    if (first.gender === "male") return "paternal";
    return "other";
}

type DerivedFacets = Pick<
    PhrasingKey,
    "acrossCount" | "acrossAtStart" | "acrossAtEnd" | "spouseGender" | "linkGender" | "endedMarriage"
>;

/**
 * The six backbone-derived facets (ADR-0033). Pure functions of the hop
 * sequence — no engine change, no new wire field.
 */
export function derivedFacetsOf(hops: ReadonlyArray<PathHop>): DerivedFacets {
    const first = hops[0];
    const last = hops[hops.length - 1];
    const firstDown = hops.find((h) => h.step === "down");
    return {
        acrossCount: hops.filter((h) => h.step === "across").length,
        acrossAtStart: first?.step === "across",
        acrossAtEnd: last?.step === "across",
        spouseGender: first?.step === "across" ? first.gender : "notApplicable",
        linkGender: firstDown === undefined ? "notApplicable" : firstDown.gender,
        endedMarriage: hops.some((h) => h.step === "across" && h.status === "ended"),
    };
}
