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
    renderSvg,
    queryPerson,
    queryMarriage,
    queryKin,
    queryResolve,
    type ExportedGraph,
    type CytoscapeGraph,
    type ExportedPerson,
    type ExportedMarriage,
    type Manifest,
    type WasmInputFile,
    type Query,
    type RelationshipDescriptor,
    type ResolveResult,
    type EmptyReason,
} from '../../pkg/kul_wasm.js';

// `format` accepts a string and returns a string unconditionally
// (best-effort even on partial-parse input — mirrors
// `kul_core::format::format_source`'s contract). `format` stays
// per-file: the asymmetry with `check` / `exportGraph` is intentional
// (formatting has no cross-file interaction).
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

// `check` and `exportGraph` take an array of `{name, source}` pairs —
// the JS host enumerates the project's `.kul` files itself, the bridge
// no longer hard-codes a single opaque file name.
const singleFile: WasmInputFile[] = [{ name: 'input.kul', source }];

// `check` returns `{ diagnostics }`. Empty array means clean — emptiness
// is the discriminator (no `ok` field per ADR-0011).
const cleanResult = check(singleFile, manifest);
if (cleanResult.diagnostics.length === 0) {
    // Clean-document short-circuit: downstream consumers proceed without
    // touching the diagnostic list.
}

// Multi-file invocation: the JS host hands the whole project across the
// ABI, mirroring how a project's `.kul` files live alongside `kul.yml`
// on disk. Cross-file id references resolve.
const multiFile: WasmInputFile[] = [
    { name: 'people.kul', source: 'person alice name:"Alice" gender:female\n' },
    { name: 'marriages.kul', source: 'person bob name:"Bob" gender:male\nmarriage m alice bob start:1970\n' },
];
const multiResult = check(multiFile, manifest);
void multiResult.diagnostics.length;

// Narrow into a real diagnostic against a known-broken source so the TS
// types for `code`, `severity`, `message`, and `primary?.byteStart` land.
// `primary` is optional: unanchored diagnostics like `KUL-M01` carry the
// would-be location in the message rather than a byte span.
const broken = check([{ name: 'input.kul', source: 'person alice gender:female\n' }], manifest);
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

// Type system must reject a bare string where an array is expected.
// @ts-expect-error check requires an array of input files
check('person alice gender:female\n', manifest);

// Type system must reject malformed file entries.
// @ts-expect-error check entries require both `name` and `source`
check([{ name: 'input.kul' }], manifest);

// `exportGraph` accepts options as a typed object; omitting it is valid.
const defaultExport = exportGraph(singleFile, manifest);
const positionedExport = exportGraph(singleFile, manifest, { withPositions: true });
const cytoscapeExport = exportGraph(singleFile, manifest, { format: 'cytoscape' });
const multiFileExport = exportGraph(multiFile, manifest);

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
exportGraph(singleFile, manifest, { format: 'graphviz' });

// `renderSvg` runs the full canonical-visual pipeline (kul-render →
// kul-layout → kul-svg) and returns a tagged envelope. Same input
// shape as `check` / `exportGraph` (files + manifest); no options in
// v1. The success arm carries an SVG string; the failure arm carries
// the same `ExportedDiagnostic` shape `exportGraph`'s failure
// envelope produces. The wire-level `ok` is a `boolean` rather than a
// literal, so structural narrowing (`'svg' in env`) is the right tool
// — mirrors the discrimination pattern used for `exportGraph` above.
const renderedClean = renderSvg(singleFile, manifest);
let renderedSvg = '';
let renderFailureCode = '';
if ('svg' in renderedClean) {
    renderedSvg = renderedClean.svg;
} else {
    renderFailureCode = renderedClean.diagnostics[0]?.code ?? '';
}

// Failure-arm exercise: the same broken source `check` rejected
// produces an `ok: false` envelope here.
const renderedBroken = renderSvg(
    [{ name: 'input.kul', source: 'person alice gender:female\n' }],
    manifest,
);
if ('diagnostics' in renderedBroken) {
    renderFailureCode = renderedBroken.diagnostics[0]?.code ?? renderFailureCode;
}

// Type system must reject a bare string where an array is expected,
// matching `check` / `exportGraph`'s signature.
// @ts-expect-error renderSvg requires an array of input files
renderSvg('person alice name:"A" gender:female\n', manifest);

// `queryPerson` / `queryMarriage` are the fourth WASM shape (ADR-0011):
// the kinship query surface. Same input shape as `check` / `exportGraph`
// (files + manifest) plus the id being looked up. The result is a
// `QueryEnvelope<T>` — an `ok`-discriminated union mirroring the other
// envelopes. The ok arm's `result` is the entity in the export shape, or
// `null` when no entity has that id; the error arm carries the same
// `ExportedDiagnostic` list a failing project produces.
// Discriminate the envelope arms by structural narrowing — the wire-level
// `ok` is a `boolean` rather than a literal type, so `'result' in env` is
// the right tool (mirrors the `exportGraph` / `renderSvg` pattern above).
let lookedUpPersonName = '';
let personLookupErrorCode = '';
const personEnvelope = queryPerson(singleFile, manifest, 'alice');
if ('result' in personEnvelope) {
    // `result` is `ExportedPerson | null` — null means no such person.
    const person: ExportedPerson | null = personEnvelope.result;
    if (person !== null) {
        lookedUpPersonName = person.name;
    }
} else {
    personLookupErrorCode = personEnvelope.diagnostics[0]?.code ?? '';
}

let lookedUpMarriageId = '';
const marriageEnvelope = queryMarriage(multiFile, manifest, 'm');
if ('result' in marriageEnvelope) {
    const marriage: ExportedMarriage | null = marriageEnvelope.result;
    if (marriage !== null) {
        lookedUpMarriageId = marriage.id;
    }
}

// Failure-arm exercise: a project that fails its checks yields the error
// arm, never a partial answer.
const brokenLookup = queryPerson(
    [{ name: 'input.kul', source: 'person alice gender:female\n' }],
    manifest,
    'alice',
);
if ('diagnostics' in brokenLookup) {
    personLookupErrorCode = brokenLookup.diagnostics[0]?.code ?? personLookupErrorCode;
}

// Type system must reject a bare string where an array is expected, and
// must require the id argument.
// @ts-expect-error queryPerson requires an array of input files
queryPerson('person alice name:"A" gender:female\n', manifest, 'alice');
// @ts-expect-error queryMarriage requires an id argument
queryMarriage(singleFile, manifest);

// `queryKin` is the kin-set variant of the fourth shape: it takes a
// declarative `Query` value (not an id string) and returns a
// `QueryEnvelope<QueryResult>`. Members carry a person id plus the full
// terminology-neutral `RelationshipDescriptor` — no person payload; the
// consumer hydrates via `queryPerson`. Construct the Query inline; the
// classification is an internally-tagged discriminated union.
const kinQuery: Query = {
    source: {
        kind: 'kinOf',
        anchor: 'alice',
        pattern: {
            classification: { kind: 'lineal', role: 'ancestor', generations: { min: 1 } },
        },
    },
    projection: 'members',
};
let firstKinId = '';
let kinErrorCode = '';
const kinEnvelope = queryKin(multiFile, manifest, kinQuery);
if ('result' in kinEnvelope) {
    // `QueryResult` is tagged on `kind`; this slice produces `members`.
    if (kinEnvelope.result.kind === 'members') {
        for (const member of kinEnvelope.result.members) {
            const id: string = member.personId;
            const descriptor: RelationshipDescriptor = member.descriptor;
            // The classification is a discriminated union to `switch` on.
            if (descriptor.classification.kind === 'lineal') {
                const generations: number = descriptor.classification.generations;
                void generations;
            }
            // `side` / `seniority` are explicit enums, never null/absent.
            const side: string = descriptor.side;
            void side;
            firstKinId = id;
        }
    }
} else {
    kinErrorCode = kinEnvelope.diagnostics[0]?.code ?? '';
}

// Collateral patterns are additive variants on the same Query value — a
// `collateralByDegree` cousins query with the new sharing/side filters must
// type-check without any new entry point.
const cousinsQuery: Query = {
    source: {
        kind: 'kinOf',
        anchor: 'alice',
        pattern: {
            classification: { kind: 'collateralByDegree', degree: { min: 1, max: 1 }, removed: { min: 0, max: 0 } },
            sharing: 'full',
            side: 'both',
        },
    },
    projection: 'members',
};
const cousinsEnvelope = queryKin(multiFile, manifest, cousinsQuery);
if ('result' in cousinsEnvelope && cousinsEnvelope.result.kind === 'members') {
    for (const member of cousinsEnvelope.result.members) {
        // The collateral arm carries the materialized derived numbers.
        if (member.descriptor.classification.kind === 'collateral') {
            const degree: number = member.descriptor.classification.cousinDegree;
            const removed: number = member.descriptor.classification.removed;
            void degree;
            void removed;
        }
        // `sharing` / `apexSeniority` are explicit enums, never null/absent.
        const sharing: string = member.descriptor.sharing;
        const apex: string = member.descriptor.apexSeniority;
        void sharing;
        void apex;
    }
}

// Type system must reject an id string where a Query value is expected.
// @ts-expect-error queryKin requires a Query value, not an id string
queryKin(multiFile, manifest, 'alice');

// `queryResolve` is the two-anchor variant of the fourth shape: two ids plus
// an optional config, returning `QueryEnvelope<ResolveResult>`. An omitted
// config uses the default generation budget. The result is a descriptor list
// plus — only when it is empty — an `emptyReason`.
let firstRelationshipKind = '';
let resolveEmptyReason: EmptyReason | undefined;
let resolveErrorCode = '';
const resolveEnvelope = queryResolve(multiFile, manifest, 'alice', 'bob');
if ('result' in resolveEnvelope) {
    const result: ResolveResult = resolveEnvelope.result;
    if (result.relationships.length === 0) {
        // `emptyReason` is present iff the list is empty.
        resolveEmptyReason = result.emptyReason;
    } else {
        firstRelationshipKind = result.relationships[0]!.classification.kind;
    }
} else {
    resolveErrorCode = resolveEnvelope.diagnostics[0]?.code ?? '';
}

// The config is optional; when present it carries only the generation budget.
queryResolve(multiFile, manifest, 'alice', 'bob', { maxApexGenerations: 3 });

// Type system must reject a missing second anchor id.
// @ts-expect-error queryResolve requires two anchor ids
queryResolve(multiFile, manifest, 'alice');

// Suppress "unused binding" diagnostics in --noUnusedLocals mode.
export const _exports = {
    formatted,
    coreVersion,
    langVersion,
    schemaVersion,
    cleanResult,
    multiResult,
    broken,
    defaultExport,
    positionedExport,
    cytoscapeExport,
    multiFileExport,
    firstPersonName,
    parenthoodLinkCount,
    cytoscapeNodeCount,
    renderedSvg,
    renderFailureCode,
    lookedUpPersonName,
    personLookupErrorCode,
    lookedUpMarriageId,
    firstKinId,
    kinErrorCode,
    firstRelationshipKind,
    resolveEmptyReason,
    resolveErrorCode,
};
