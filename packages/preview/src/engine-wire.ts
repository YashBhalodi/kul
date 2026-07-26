// Wire types of the `@kullang/wasm` query surface, mirrored by hand.
//
// PROVENANCE. The generating source is `crates/kul-wasm/types/kul_wasm.d.ts`,
// the tsify-derived snapshot committed under ADR-0012. These declarations are
// copied from it verbatim (doc comments trimmed), and
// `tests/engine-wire.test.ts` re-reads that file and fails if any of them has
// drifted. There is no `import` of `@kullang/wasm` anywhere in this package:
// per ADR-0040 the engine reaches the webview as a host-supplied build asset,
// never as an npm module specifier, so its *types* have to arrive some other
// way. A text-checked mirror is that way.
//
// Two closures are mirrored: the one `queryDetail` needs (ADR-0037) and the
// one `queryKin` needs — the `Query` value and the `QueryResult` it produces
// (ADR-0043). The descriptor family is **not** re-declared here: the phrasing
// layer already mirrors it (ADR-0033) and owns those words, so `Member` imports
// `RelationshipDescriptor` from there rather than restating it. Restating it
// would give the descriptor two provenance paths, which ADR-0024 and ADR-0034
// both refuse.

import type {
    Affinity,
    EdgeNature,
    LinealRole,
    RelationshipDescriptor,
    Sharing,
    Side,
} from "./phrasing/descriptor.js";

/** One `.kul` input file as the JS host hands it to the bridge. */
export interface WasmInputFile {
    name: string;
    source: string;
}

/** Typed `kul.yml` manifest. Serialized with the `kul:` field name. */
export interface Manifest {
    kul: string;
}

export interface ExportedSpan {
    file: string;
    byteStart: number;
    byteEnd: number;
    line: number;
    column: number;
}

export interface ExportedRelated extends ExportedSpan {
    label: string;
}

export interface ExportedDiagnostic {
    code: string;
    severity: string;
    message: string;
    primary?: ExportedSpan;
    related: ExportedRelated[];
}

/** Ok arm of a {@link QueryEnvelope}. `ok` is always `true`. */
export interface QueryOk<T> {
    ok: boolean;
    result: T;
}

/** Error arm of a {@link QueryEnvelope}. `ok` is always `false`. */
export interface QueryError {
    ok: boolean;
    diagnostics: ExportedDiagnostic[];
}

/**
 * Adapter-facing result of a query operation: an untagged union discriminated
 * by an `ok` boolean (ADR-0024). A project that fails its checks yields the
 * error arm whole, never a partial answer (ADR-0009).
 */
export type QueryEnvelope<T> = QueryOk<T> | QueryError;

export interface ExportedDate {
    value: string;
    precision: string;
    circa: boolean;
}

export interface ExportedPerson {
    id: string;
    name: string;
    family?: string;
    given?: string;
    gender: string;
    born?: ExportedDate;
    died?: ExportedDate;
    span?: [number, number];
}

export interface ExportedMarriage {
    id: string;
    spouses: [string, string];
    start?: ExportedDate;
    end?: ExportedDate;
    endReason?: string;
    span?: [number, number];
}

export type ParenthoodLinkKind = "biological" | "adoptive";

export interface ExportedParenthoodLink {
    marriageId: string;
    childId: string;
    kind: ParenthoodLinkKind;
    start?: ExportedDate;
    end?: ExportedDate;
    span?: [number, number];
}

/** A neighbouring person plus the parenthood link that reaches them. */
export interface LinkedPerson {
    person: ExportedPerson;
    link: ExportedParenthoodLink;
}

/** One marriage a person is a spouse in, with the person on the other side. */
export interface MarriageTie {
    marriage: ExportedMarriage;
    spouse?: ExportedPerson;
}

/** What one entry of a batched detail lookup addresses (ADR-0037). */
export type DetailTarget = { kind: "person"; id: string } | { kind: "marriage"; id: string } | { kind: "adoption"; childId: string; marriageId: string };

/** One entity's own fields plus its relational neighbourhood. */
export type EntityDetail = { kind: "person"; person: ExportedPerson; parents: LinkedPerson[]; marriages: MarriageTie[]; children: LinkedPerson[] } | { kind: "marriage"; marriage: ExportedMarriage; spouses: ExportedPerson[]; children: LinkedPerson[] } | { kind: "adoption"; adoption: ExportedParenthoodLink; child: ExportedPerson; parents: ExportedPerson[] };

export type DetailLookupResult = (EntityDetail | null)[];

// --- The kin-set query value (ADR-0025's one contract artifact) -------------
//
// A kin set is spelled as a `Query` and handed to `queryKin`; the preview is a
// thin constructor of that value and never a second evaluator (ADR-0043).

export interface IntRange {
    min: number;
    max?: number;
}

export type PatternClassification = { kind: "lineal"; role: LinealRole; generations: IntRange } | { kind: "collateral"; up: IntRange; down: IntRange } | { kind: "collateralByDegree"; degree: IntRange; removed: IntRange } | { kind: "any"; maxUp: number; maxDown: number };

export interface KinPattern {
    classification: PatternClassification;
    edgeNature?: EdgeNature;
    sharing?: Sharing;
    side?: Side;
    affinity?: Affinity;
    affinalHops?: IntRange;
}

export type QuerySource = { kind: "allPersons" } | { kind: "kinOf"; anchor: string; pattern: KinPattern };

export type PersonField = "id" | "name" | "family" | "given" | "gender" | "born" | "died";

export type SortDirection = "asc" | "desc";

export interface SortSpec {
    field: PersonField;
    direction?: SortDirection;
}

export type Predicate = { op: "eq"; field: PersonField; value: string } | { op: "neq"; field: PersonField; value: string } | { op: "lt"; field: PersonField; value: string } | { op: "lte"; field: PersonField; value: string } | { op: "gt"; field: PersonField; value: string } | { op: "gte"; field: PersonField; value: string } | { op: "in"; field: PersonField; values: string[] } | { op: "present"; field: PersonField } | { op: "absent"; field: PersonField };

export type FilterMode = "certain" | "includeUncertain";

export type Projection = "members" | "count";

export interface Query {
    source: QuerySource;
    where?: Predicate[];
    sort?: SortSpec;
    mode?: FilterMode;
    projection: Projection;
}

/** One member of a `members` result: the person id plus how it was reached. */
export interface Member {
    personId: string;
    descriptor: RelationshipDescriptor;
}

export type QueryResult = { kind: "members"; members: Member[] } | { kind: "personIds"; personIds: string[] } | { kind: "count"; count: number };

/**
 * True iff `envelope` is the ok arm. The shipped surface discriminates on an
 * `ok` boolean rather than a `status` tag (ADR-0024), and `ok` is typed
 * `boolean` on both arms, so the structural narrow keys on the payload field.
 */
export function isQueryOk<T>(
    envelope: QueryEnvelope<T>,
): envelope is QueryOk<T> {
    return "result" in envelope;
}
