/* tslint:disable */
/* eslint-disable */

export type PersonLookupResult = ExportedPerson | null;
export type MarriageLookupResult = ExportedMarriage | null;


/**
 * A birth-order comparison under the strict-interval rule. `elder` /
 * `younger` only when *every* interpretation of one date strictly precedes
 * the other; `unknown` when dates are missing or intervals overlap;
 * `notApplicable` is reserved for `self`.
 */
export type Seniority = "elder" | "younger" | "unknown" | "notApplicable";

/**
 * A declarative descriptor pattern: which relationships to a person count
 * as matches. The named sugar (`parents_of`, `ancestors_of`, …) each
 * desugar to one of these.
 */
export interface KinPattern {
    classification: PatternClassification;
    /**
     * Optional filter on the path\'s edge nature; omitted (`None`) matches
     * both blood and adoptive.
     */
    edgeNature?: EdgeNature;
    /**
     * Optional filter on the sibling-junction [`Sharing`]; omitted (`None`)
     * matches every sharing. Only ever narrows collateral results — a lineal
     * path is always `notApplicable`.
     */
    sharing?: Sharing;
    /**
     * Optional filter on the derived [`Side`]; omitted (`None`) matches every
     * side. `Some(Side::Both)` selects couple-apex-rooted relations,
     * `Some(Side::Maternal)` a single family branch, and so on.
     */
    side?: Side;
}

/**
 * A marriage hop\'s status. Wire form: `\"ongoing\" | \"ended\"`. Not produced
 * this slice (no `across` hops yet).
 */
export type MarriageStatus = "ongoing" | "ended";

/**
 * Adapter-facing result of a query operation. Mirrors the existing
 * check/export/render surface: an untagged union discriminated by an `ok`
 * boolean — the ok arm carries the query `result`, the error arm carries
 * the structured `diagnostics` of a project that failed its checks.
 *
 * The engine never throws / never panics: a failing project yields the
 * [`QueryEnvelope::Error`] arm, not a partial answer (strict-on-errors,
 * ADR-0009). Generic over the payload `T` so later slices (kin-set
 * queries, relationship resolution) reuse the same envelope.
 */
export type QueryEnvelope<T> = QueryOk<T> | QueryError;

/**
 * An inclusive integer range; an absent `max` means unbounded. Used for a
 * lineal pattern\'s generation bounds.
 */
export interface IntRange {
    min: number;
    max?: number;
}

/**
 * Direction of a [`Classification::Lineal`] relationship.
 */
export type LinealRole = "ancestor" | "descendant";

/**
 * Endpoint / linking-relative gender. Wire form: `\"male\" | \"female\" |
 * \"other\"`. Mirrors [`ast::Gender`] but lives here so the descriptor\'s
 * serialized surface is self-contained (the AST enum is not part of any
 * wire contract).
 */
export type Gender = "male" | "female" | "other";

/**
 * Error arm of a [`QueryEnvelope`]. `ok` is always `false`;
 * `diagnostics` carries every diagnostic the failing project produced —
 * the same [`ExportedDiagnostic`] shape the check/export surfaces use.
 */
export interface QueryError {
    /**
     * Always `false`. Consumer-facing discriminator.
     */
    ok: boolean;
    /**
     * Every diagnostic the validator produced (errors, warnings, notes).
     */
    diagnostics: ExportedDiagnostic[];
}

/**
 * Failure arm of [`RenderEnvelope`]. Same diagnostic shape as
 * [`export_graph`]\'s failure path.
 */
export interface RenderFailure {
    /**
     * Always `false`. Consumer-facing discriminator.
     */
    ok: boolean;
    diagnostics: ExportedDiagnostic[];
}

/**
 * Graph payload inside a [`SuccessEnvelope`]. Untagged on the wire — the
 * consumer knows which shape from the `--format` they requested.
 */
export type GraphPayload = ExportedGraph | CytoscapeGraph;

/**
 * How the alter is classified relative to the ego. Internally tagged on
 * `kind`. This slice emits only [`Classification::Lineal`]; `self` and
 * `collateral` derive mechanically from the same hop counts (see
 * [`RelationshipDescriptor::derive`]) so later slices need no rework.
 */
export type Classification = { kind: "self" } | { kind: "lineal"; role: LinealRole; generations: number } | { kind: "collateral"; up: number; down: number; cousinDegree: number; removed: number };

/**
 * JS-side return type of [`check`]. Empty `diagnostics` means clean;
 * consumers discriminate on emptiness, not an `ok` field (ADR-0011).
 */
export interface CheckEnvelope {
    diagnostics: ExportedDiagnostic[];
}

/**
 * JS-side return type of [`render_svg`]. Untagged success/failure
 * discriminated by `ok`, bit-identical to
 * `kul_lsp::features::render::RenderResponse`. Rule-of-three: a shared
 * crate emerges only when a third independent consumer materializes.
 */
export type RenderEnvelope = RenderSuccess | RenderFailure;

/**
 * Ok arm of a [`QueryEnvelope`]. `ok` is always `true` (consumer-facing
 * discriminator); `result` is the query payload.
 */
export interface QueryOk<T> {
    /**
     * Always `true`. Consumer-facing discriminator.
     */
    ok: boolean;
    /**
     * The query answer (for a lookup: the entity, or `null`).
     */
    result: T;
}

/**
 * One `.kul` input file as the JS host hands it to the bridge. Mirrors
 * [`kul_core::ast::InputFile`]; exists separately so `tsify` can derive
 * a TS type without leaking the feature dependency onto `kul-core`.
 */
export interface WasmInputFile {
    name: string;
    source: string;
}

/**
 * One hop of the lossless path backbone. Internally tagged on `step`.
 * Vertical hops (`up` / `down`) carry the person landed on, that person\'s
 * gender, and the edge kind. The `across` variant (a marriage hop) is part
 * of the pinned type but not produced this slice.
 */
export type PathHop = { step: "up"; to: string; gender: Gender; edge: HopEdge } | { step: "down"; to: string; gender: Gender; edge: HopEdge } | { step: "across"; to: string; gender: Gender; marriage: string; status: MarriageStatus; endReason?: string };

/**
 * One member of a `members` result on the wire: the person id plus the
 * [`RelationshipDescriptor`] recording how it was reached. Carries **no
 * person payload** — consumers hydrate via the `person(id)` lookup. The
 * Rust-native evaluator returns a borrowed [`KinMember`](super::KinMember)
 * instead; this is its serialized projection.
 */
export interface Member {
    personId: string;
    descriptor: RelationshipDescriptor;
}

/**
 * Output format for [`export`]. Lowercase wire form shared by CLI flags
 * and JS consumers.
 */
export type ExportFormat = "json" | "cytoscape";

/**
 * Parenthood kind. Wire form: `\"biological\"` / `\"adoptive\"`.
 */
export type ParenthoodLinkKind = "biological" | "adoptive";

/**
 * Per-node `data` payload. Untagged: serialized variant is chosen by
 * which fields are present.
 */
export type NodeData = PersonNodeData | MarriageNodeData;

/**
 * Projected date: `value` (no circa marker), `precision`
 * (year/month/day), and `circa` flag.
 */
export interface ExportedDate {
    value: string;
    precision: string;
    circa: boolean;
}

/**
 * Sibling-junction parent-set sharing. An apex-junction comparison, so
 * `notApplicable` for every lineal / self path (there is no sibling
 * junction). Also usable as a kin-pattern filter, so it is `Deserialize`.
 */
export type Sharing = "full" | "half" | "notApplicable";

/**
 * Success arm of [`RenderEnvelope`].
 */
export interface RenderSuccess {
    /**
     * Always `true`. Consumer-facing discriminator.
     */
    ok: boolean;
    /**
     * Theme-agnostic SVG (semantic CSS classes, no inline colours).
     */
    svg: string;
}

/**
 * The Cytoscape JSON graph shape.
 */
export interface CytoscapeGraph {
    nodes: CytoscapeNode[];
    edges: CytoscapeEdge[];
}

/**
 * The classification a [`KinPattern`] selects for, internally tagged on
 * `kind`. `any` (an unclassified match) arrives with a later slice as a
 * further additive variant.
 */
export type PatternClassification = { kind: "lineal"; role: LinealRole; generations: IntRange } | { kind: "collateral"; up: IntRange; down: IntRange } | { kind: "collateralByDegree"; degree: IntRange; removed: IntRange };

/**
 * The edge tag on a vertical [`PathHop`]. Wire form: `\"bio\" | \"adoptive\"`.
 */
export type HopEdge = "bio" | "adoptive";

/**
 * The export envelope: success (graph) or failure (diagnostics).
 * Untagged with a shared `ok` boolean for consumer discrimination.
 */
export type ExportEnvelope = SuccessEnvelope | FailureEnvelope;

/**
 * The kinship-native graph: three flat collections.
 */
export interface ExportedGraph {
    persons: ExportedPerson[];
    marriages: ExportedMarriage[];
    parenthoodLinks: ExportedParenthoodLink[];
}

/**
 * The result of evaluating a [`Query`]. A tagged union so later
 * projections (`count`, the `allPersons` `personIds` shape) slot in without
 * reshaping. This slice produces only the `members` variant.
 */
export type QueryResult = { kind: "members"; members: Member[] };

/**
 * The single contract artifact: a declarative, serializable query. Every
 * surface builds this and hands it to [`evaluate`](super::evaluate).
 */
export interface Query {
    source: QuerySource;
    projection: Projection;
}

/**
 * The terminology-neutral record of how the alter relates to the ego, plus
 * the lossless [`PathHop`] backbone. One descriptor per distinct
 * relationship path — descriptor identity *is* path identity, and the
 * engine never collapses same-classification descriptors (ADR-0026).
 */
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

/**
 * Tunable knobs for [`export`]. CamelCase Deserialize with per-field
 * defaults so JS callers can pass `{}` or partial objects.
 */
export interface ExportOptions {
    format?: ExportFormat;
    /**
     * Attach `span: [byte_start, byte_end]` to every entity. Opt in when
     * the consumer needs to map a graph node back to its source location.
     */
    withPositions?: boolean;
}

/**
 * Typed `kul.yml` manifest. Serialized with the `kul:` field name.
 */
export interface Manifest {
    /**
     * Language version (`MAJOR.MINOR`) the sibling `.kul` files target.
     * Surfaced in the export envelope\'s `kul:` field.
     */
    kul: string;
}

/**
 * What the query produces. This slice ships only `members`; `count` (and
 * the `personIds` shape of the `allPersons` source) arrive later.
 */
export type Projection = "members";

/**
 * Where a query draws its candidate persons from. This slice ships only
 * `kinOf`; `{ kind: \"allPersons\" }` arrives with the filtering slice.
 */
export type QuerySource = { kind: "kinOf"; anchor: string; pattern: KinPattern };

/**
 * Whether the parent-child edges on the path are all blood or include at
 * least one adoption. `adoptive` iff *any* hop is an adoption edge; the
 * per-hop truth stays lossless in the [`PathHop`] backbone.
 */
export type EdgeNature = "blood" | "adoptive";

/**
 * Whether the relationship runs through marriage hops. Strictly about
 * `across` hops: none ⇒ `blood`. This slice produces only blood segments
 * (no `across` hops exist yet), so `affinity` is always `blood`; `step`
 * and `inLaw` arrive with the affinal-hop slice.
 */
export type Affinity = "blood" | "step" | "inLaw";

/**
 * Which side of the family the relationship routes through. Derived from
 * the path\'s *initial ascent*, never guessed. `both` marks a couple-apex
 * collateral path. Also usable as a kin-pattern filter, so it is
 * `Deserialize`.
 */
export type Side = "maternal" | "paternal" | "other" | "both" | "notApplicable";

export interface CytoscapeEdge {
    data: EdgeData;
}

export interface CytoscapeNode {
    data: NodeData;
}

export interface EdgeData {
    /**
     * `m:<marriage-id>`. Every edge originates at a marriage.
     */
    source: string;
    /**
     * `p:<person-id>`. Every edge ends at a person.
     */
    target: string;
    /**
     * `\"spouse\"`, `\"biological_child\"`, or `\"adoptive_child\"`.
     */
    type: string;
    /**
     * `start:` of an adoption. Absent on spouse/bio-child edges.
     */
    start?: ExportedDate;
    /**
     * `end:` of an adoption. Absent on spouse/bio-child edges.
     */
    end?: ExportedDate;
}

export interface ExportedDiagnostic {
    code: string;
    severity: string;
    message: string;
    /**
     * `None` for unanchored diagnostics (e.g. `KUL-M01`).
     */
    primary?: ExportedSpan;
    related: ExportedRelated[];
}

export interface ExportedMarriage {
    id: string;
    /**
     * Two spouse ids, in declaration order. Both resolve to entries in
     * `persons` (export refuses otherwise).
     */
    spouses: [string, string];
    start?: ExportedDate;
    end?: ExportedDate;
    endReason?: string;
    /**
     * `[byte_start, byte_end]`. Present iff `with_positions`.
     */
    span?: [number, number];
}

export interface ExportedParenthoodLink {
    marriageId: string;
    childId: string;
    kind: ParenthoodLinkKind;
    /**
     * `start:` of an adoption. Absent for biological links.
     */
    start?: ExportedDate;
    /**
     * `end:` of an adoption. Absent for biological links.
     */
    end?: ExportedDate;
    /**
     * `[byte_start, byte_end]`. Present iff `with_positions`.
     */
    span?: [number, number];
}

export interface ExportedPerson {
    id: string;
    name: string;
    family?: string;
    given?: string;
    gender: string;
    born?: ExportedDate;
    died?: ExportedDate;
    /**
     * `[byte_start, byte_end]`. Present iff `with_positions`.
     */
    span?: [number, number];
}

export interface ExportedRelated extends ExportedSpan {
    label: string;
}

export interface ExportedSpan {
    /**
     * Canonical file name (`InputFile.name`, or `manifest_name` for
     * `KUL-Mxx`).
     */
    file: string;
    byteStart: number;
    byteEnd: number;
    line: number;
    column: number;
}

export interface FailureEnvelope {
    /**
     * Always `false`. Consumer-facing discriminator.
     */
    ok: boolean;
    /**
     * Every diagnostic the validator produced (errors, warnings, notes).
     */
    diagnostics: ExportedDiagnostic[];
}

export interface MarriageNodeData {
    /**
     * `m:<marriage-id>`.
     */
    id: string;
    /**
     * Always `\"marriage\"`.
     */
    type: string;
    start?: ExportedDate;
    end?: ExportedDate;
    endReason?: string;
}

export interface PersonNodeData {
    /**
     * `p:<person-id>`.
     */
    id: string;
    /**
     * Always `\"person\"`.
     */
    type: string;
    name: string;
    family?: string;
    given?: string;
    gender: string;
    born?: ExportedDate;
    died?: ExportedDate;
}

export interface SuccessEnvelope {
    /**
     * Always `true`. Consumer-facing discriminator.
     */
    ok: boolean;
    /**
     * Schema version (see [`SCHEMA_VERSION`]).
     */
    schema: number;
    /**
     * Kul language version from the manifest\'s `kul:` field.
     */
    kul: string;
    /**
     * The exported graph (shape determined by [`ExportOptions::format`]).
     */
    graph: GraphPayload;
}


export function EXPORT_SCHEMA_VERSION(): number;

export function KUL_CORE_VERSION(): string;

export function KUL_LANGUAGE_VERSION(): string;

export function check(files: WasmInputFile[], manifest: Manifest): CheckEnvelope;

export function exportGraph(files: WasmInputFile[], manifest: Manifest, options?: ExportOptions | null): ExportEnvelope;

export function format(source: string): string;

/**
 * Kin-set queries on the fourth WASM shape: evaluate a declarative
 * [`Query`] value and return the matching members (person id + descriptor,
 * **no person payload** — consumers hydrate via [`query_person`]) in the
 * pinned deterministic order. Same load-and-check gate as the lookups; a
 * failing project or an unknown anchor yields the envelope's error arm with
 * a diagnostic, never a throw.
 */
export function queryKin(files: WasmInputFile[], manifest: Manifest, query: Query): QueryEnvelope<QueryResult>;

/**
 * Marriage-lookup counterpart to [`query_person`]. Same load-and-check
 * gate and never-throwing envelope; the ok arm carries the marriage in the
 * export shape, or `null` when no marriage has that id.
 */
export function queryMarriage(files: WasmInputFile[], manifest: Manifest, id: string): QueryEnvelope<MarriageLookupResult>;

/**
 * The fourth WASM shape (ADR-0011): the kinship query surface. Looks up a
 * person by id, gated on the project passing its checks (strict-on-errors,
 * ADR-0009). Never throws — a failing project yields the envelope's error
 * arm; a clean project yields the ok arm carrying the person in the export
 * shape, or `null` when no person has that id.
 */
export function queryPerson(files: WasmInputFile[], manifest: Manifest, id: string): QueryEnvelope<PersonLookupResult>;

export function renderSvg(files: WasmInputFile[], manifest: Manifest): RenderEnvelope;
