/* tslint:disable */
/* eslint-disable */
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
 * One `.kul` input file as the JS host hands it to the bridge. Mirrors
 * [`kul_core::ast::InputFile`]; exists separately so `tsify` can derive
 * a TS type without leaking the feature dependency onto `kul-core`.
 */
export interface WasmInputFile {
    name: string;
    source: string;
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
    start: ExportedDate;
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

export function renderSvg(files: WasmInputFile[], manifest: Manifest): RenderEnvelope;
