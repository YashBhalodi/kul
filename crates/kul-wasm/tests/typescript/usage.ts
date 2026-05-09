// TypeScript consumer compile-test for `@kullang/wasm`.
//
// Runs under `tsc --noEmit` in CI. Catches the case where the generated
// `.d.ts` compiles against itself but isn't usable in real consumer code:
// argument types, return types, and the shape of the named exports must
// all match what a downstream TypeScript user would expect.

import {
    EXPORT_SCHEMA_VERSION,
    KUL_CORE_VERSION,
    KUL_LANGUAGE_VERSION,
    check,
    exportGraph,
    format,
    type ExportedGraph,
    type CytoscapeGraph,
    type Manifest,
} from '../../pkg/kul_wasm.js';

// `format` accepts a string and returns a string unconditionally
// (best-effort even on partial-parse input — mirrors
// `kul_core::format::format_source`'s contract).
const source: string = 'person alice name:"A" gender:female\n';
const formatted: string = format(source);

// Type system must reject non-string inputs.
// @ts-expect-error format requires a string source
format(42);

// Version metadata: two strings and a number.
const coreVersion: string = KUL_CORE_VERSION();
const langVersion: string = KUL_LANGUAGE_VERSION();
const schemaVersion: number = EXPORT_SCHEMA_VERSION();

// Realistic consumer assertion pattern: gate behavior on schema version
// before parsing an envelope.
if (schemaVersion < 1) {
    throw new Error('unsupported schema');
}

// The project manifest is now a required argument — JS callers construct
// it inline. The on-disk YAML schema is `kul: "<MAJOR.MINOR>"`; the JS
// object mirrors that field name.
const manifest: Manifest = { kul: '0.1' };

// `check` returns `{ diagnostics }`. Empty array means clean — emptiness
// is the discriminator (no `ok` field per ADR-0011).
const cleanResult = check(source, manifest);
if (cleanResult.diagnostics.length === 0) {
    // Clean-document short-circuit: downstream consumers proceed without
    // touching the diagnostic list.
}

// Narrow into a real diagnostic against a known-broken source so the TS
// types for `code`, `severity`, `message`, and `primary?.byteStart` land.
// `primary` is optional: unanchored diagnostics like `KUL-M01` carry the
// would-be location in the message rather than a byte span.
const broken = check('person alice gender:female\n', manifest);
const firstDiagnostic = broken.diagnostics[0];
if (firstDiagnostic !== undefined) {
    const code: string = firstDiagnostic.code;
    const severity: string = firstDiagnostic.severity;
    const message: string = firstDiagnostic.message;
    const byteStart: number | undefined = firstDiagnostic.primary?.byteStart;
    const file: string | undefined = firstDiagnostic.primary?.file;
    void code;
    void severity;
    void message;
    void byteStart;
    void file;
}

// Type system must reject non-string inputs to `check`.
// @ts-expect-error check requires a string source
check(42, manifest);

// `exportGraph` accepts options as a typed object; omitting it is valid.
const defaultExport = exportGraph(source, manifest);
const positionedExport = exportGraph(source, manifest, { withPositions: true });
const cytoscapeExport = exportGraph(source, manifest, { format: 'cytoscape' });

// Discriminate success vs failure by structural narrowing — `in` checks
// the discriminating field directly. The wire-level `ok` is a `boolean`
// rather than a literal type, so structural narrowing is the right tool.
let firstPersonName = '';
let parenthoodLinkCount = 0;
if ('graph' in defaultExport) {
    // Narrow `GraphPayload` between native and cytoscape. The default
    // format is `"json"` (kinship-native), so the native branch is the
    // expected one here.
    const graph = defaultExport.graph as ExportedGraph;
    if (graph.persons.length > 0) {
        firstPersonName = graph.persons[0].name;
    }
    for (const link of graph.parenthoodLinks) {
        // Narrow into the link payload — every link carries marriageId
        // and childId regardless of `kind`.
        const _id: string = link.marriageId;
        void _id;
        parenthoodLinkCount += 1;
    }
} else {
    // Failure branch: diagnostics carry the same shape as `check`'s.
    const _code: string = defaultExport.diagnostics[0]?.code ?? '';
    void _code;
}

// Cytoscape branch narrows into a different graph payload.
let cytoscapeNodeCount = 0;
if ('graph' in cytoscapeExport) {
    const graph = cytoscapeExport.graph as CytoscapeGraph;
    cytoscapeNodeCount = graph.nodes.length;
}

// Type system must reject unknown format strings.
// @ts-expect-error "graphviz" is not a valid ExportFormat
exportGraph(source, manifest, { format: 'graphviz' });

// Suppress "unused binding" diagnostics in --noUnusedLocals mode.
export const _exports = {
    formatted,
    coreVersion,
    langVersion,
    schemaVersion,
    cleanResult,
    broken,
    defaultExport,
    positionedExport,
    cytoscapeExport,
    firstPersonName,
    parenthoodLinkCount,
    cytoscapeNodeCount,
};
