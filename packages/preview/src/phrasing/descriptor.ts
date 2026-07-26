/**
 * The slice of the query wire form the phrasing layer reads: the
 * **relationship descriptor** (ADR-0026) and its lossless path backbone.
 *
 * These declarations are a *verbatim mirror* of the committed tsify output at
 * `crates/kul-wasm/types/kul_wasm.d.ts` (ADR-0012). They are copied rather
 * than imported because `@kullang/preview` does not depend on `@kullang/wasm`
 * — the phrasing module must stay a pure, dependency-free module that can be
 * extracted mechanically (ADR-0033). `tests/phrasing/wire-conformance.test.ts`
 * reads the committed `.d.ts` as text and fails the moment the two drift, so
 * the mirror can never silently rot.
 *
 * Do not "improve" these types. They are the wire, character for character.
 */

export type Gender = "male" | "female" | "other";
export type Seniority = "elder" | "younger" | "unknown" | "notApplicable";
export type EdgeNature = "blood" | "adoptive";
export type Affinity = "blood" | "step" | "inLaw";
export type Sharing = "full" | "half" | "notApplicable";
export type Side = "maternal" | "paternal" | "other" | "both" | "notApplicable";
export type HopEdge = "bio" | "adoptive";
export type MarriageStatus = "ongoing" | "ended";
export type LinealRole = "ancestor" | "descendant";
export type Classification = { kind: "self" } | { kind: "lineal"; role: LinealRole; generations: number } | { kind: "collateral"; up: number; down: number; cousinDegree: number; removed: number };
export type PathHop = { step: "up"; to: string; gender: Gender; edge: HopEdge } | { step: "down"; to: string; gender: Gender; edge: HopEdge } | { step: "across"; to: string; gender: Gender; marriage: string; status: MarriageStatus; endReason?: string };
export interface RelationshipDescriptor {
    egoId: string;
    alterId: string;
    egoGender: Gender;
    alterGender: Gender;
    classification: Classification;
    edgeNature: EdgeNature;
    affinity: Affinity;
    sharing: Sharing;
    side: Side;
    seniority: Seniority;
    apexSeniority: Seniority;
    path: PathHop[];
}
