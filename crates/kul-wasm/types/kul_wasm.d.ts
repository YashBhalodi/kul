/* tslint:disable */
/* eslint-disable */
/**
 * A date as projected into the envelope. Splits the source `~YYYY[-MM[-DD]]`
 * form into `value` (no circa marker), `precision` (year / month / day),
 * and `circa` (the `~` flag) so consumers don\'t have to re-parse strings.
 */
export interface ExportedDate {
    value: string;
    precision: string;
    circa: boolean;
}

/**
 * A graph payload inside a [`SuccessEnvelope`].
 *
 * Untagged at the wire level: the JSON looks identical to whichever
 * inner shape was chosen (kinship-native objects with `persons` /
 * `marriages` / `parenthood_links`, or cytoscape objects with `nodes` /
 * `edges`). Consumers know which to expect based on the `--format` they
 * asked for; the envelope\'s `schema` is the same integer regardless of
 * shape because both shapes are projections of the same underlying data.
 */
export type GraphPayload = ExportedGraph | CytoscapeGraph;

/**
 * Caller-tunable knobs for [`export`]. Defaults are the most common path.
 *
 * `Deserialize` is camelCase and field-level `default` so a JS-side caller
 * can pass `{}`, `{ withPositions: true }`, or `{ format: \"cytoscape\" }`
 * and the omitted fields fall back to [`ExportOptions::default`]. The
 * `kul-wasm` `exportGraph` bridge uses this directly.
 */
export interface ExportOptions {
    format?: ExportFormat;
    /**
     * When `true`, every exported entity carries a `span: [byte_start,
     * byte_end]` field pointing back to its declaration in the source.
     * Default `false` keeps the envelope compact; opt in when the
     * consumer needs to map a click on a graph node back to a source
     * location (\"highlight Alice\'s declaration\").
     */
    withPositions?: boolean;
}

/**
 * JS-side return type of [`check`]. Carries the full diagnostic list —
 * errors, warnings, and notes alike. An empty `diagnostics` array means
 * a clean document; consumers discriminate on emptiness rather than an
 * `ok` field, per [ADR-0011](../../docs/adr/0011-wasm-surface-three-shapes-no-wrappers.md).
 *
 * Diagnostic entries reuse `kul_core::export::ExportedDiagnostic` — the
 * same shape that the failure-envelope path of `kul export` emits, so the
 * TS type lands as a single source of truth across CLI export and WASM
 * check.
 */
export interface CheckEnvelope {
    diagnostics: ExportedDiagnostic[];
}

/**
 * One node\'s `data` payload. Untagged: the variant is chosen at
 * serialization time by which fields are present, matching the Cytoscape
 * convention of \"the data object is whatever the consumer wants.\
 */
export type NodeData = PersonNodeData | MarriageNodeData;

/**
 * Output format for [`export`].
 *
 * `Deserialize` accepts the lowercase wire form (`\"json\"`, `\"cytoscape\"`)
 * so JS-side consumers and CLI flag parsing share one vocabulary. See
 * [`ExportOptions`] for the camelCase wrapper that `kul-wasm`\'s
 * `exportGraph` uses on its options input.
 */
export type ExportFormat = "json" | "cytoscape";

/**
 * The Cytoscape JSON graph shape.
 */
export interface CytoscapeGraph {
    nodes: CytoscapeNode[];
    edges: CytoscapeEdge[];
}

/**
 * The export envelope returned by [`export`]. Either a success payload
 * carrying the graph, or a failure payload carrying the diagnostic list.
 *
 * Serialized untagged: serde picks the variant by structure. Both variants
 * carry an `ok` boolean so consumers can discriminate without inspecting
 * other fields.
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

export interface CytoscapeEdge {
    data: EdgeData;
}

export interface CytoscapeNode {
    data: NodeData;
}

export interface EdgeData {
    /**
     * `m:<marriage-id>` (always; every edge originates at a marriage).
     */
    source: string;
    /**
     * `p:<person-id>` (always; every edge ends at a person).
     */
    target: string;
    /**
     * `\"spouse\"`, `\"biological_child\"`, or `\"adoptive_child\"`.
     */
    type: string;
    /**
     * `start:` of an adoption. Always absent on spouse and bio-child edges.
     */
    start?: ExportedDate;
    /**
     * `end:` of an adoption. Always absent on spouse and bio-child edges.
     */
    end?: ExportedDate;
}

export interface ExportedDiagnostic {
    code: string;
    severity: string;
    message: string;
    primary: ExportedSpan;
    related: ExportedRelated[];
}

export interface ExportedMarriage {
    id: string;
    /**
     * The two spouse ids, in declaration order. Both ids resolve to a
     * `person` in `persons` (the failure envelope would have fired
     * otherwise).
     */
    spouses: [string, string];
    start: ExportedDate;
    end?: ExportedDate;
    endReason?: string;
    /**
     * `[byte_start, byte_end]` covering the source-level statement.
     * Present only when `ExportOptions::with_positions` was `true`.
     */
    span?: [number, number];
}

export interface ExportedParenthoodLink {
    marriageId: string;
    childId: string;
    /**
     * `\"biological\"` or `\"adoptive\"`. New kinds (e.g. surrogacy) would
     * land additively per [ADR-0010](../../../docs/adr/0010-export-schema-versioning.md).
     */
    kind: string;
    /**
     * `start:` of an adoption. Always absent for biological links.
     */
    start?: ExportedDate;
    /**
     * `end:` of an adoption. Always absent for biological links.
     */
    end?: ExportedDate;
    /**
     * `[byte_start, byte_end]` covering the source-level `birth` or
     * `adoption` sub-statement. Present only when
     * `ExportOptions::with_positions` was `true`.
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
     * `[byte_start, byte_end]` covering the source-level statement.
     * Present only when `ExportOptions::with_positions` was `true`.
     */
    span?: [number, number];
}

export interface ExportedRelated extends ExportedSpan {
    label: string;
}

export interface ExportedSpan {
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
     * Every diagnostic the validator produced — errors, warnings, and
     * notes alike — so the consumer sees the full picture of why export
     * refused.
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
    start: ExportedDate;
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
     * Schema version this envelope conforms to. See [`SCHEMA_VERSION`].
     */
    schema: number;
    /**
     * Kul language version of the source document — either the version
     * declared by `kul <version>` at the top of the document, or
     * [`LANGUAGE_VERSION`] if absent.
     */
    kul: string;
    /**
     * The exported graph. Either the kinship-native shape (the canonical
     * foundation) or a derived shape such as Cytoscape, depending on
     * [`ExportOptions::format`]. Untagged in the JSON: the consumer
     * knows which shape to expect from the format they requested.
     */
    graph: GraphPayload;
}


export function EXPORT_SCHEMA_VERSION(): number;

export function KUL_CORE_VERSION(): string;

export function KUL_LANGUAGE_VERSION(): string;

export function check(source: string): CheckEnvelope;

export function exportGraph(source: string, options?: ExportOptions | null): ExportEnvelope;

export function format(source: string): string;
