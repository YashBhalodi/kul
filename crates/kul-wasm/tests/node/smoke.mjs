// End-to-end smoke test: imports the wasm-pack bundler-target output
// exactly as a downstream JS consumer would, calls `format` on a fixture
// from the example corpus, and asserts the result is a non-empty string.
//
// Run via `node --experimental-wasm-modules` (Node 22+). The bundler
// target uses ESM `import * as wasm from "./*.wasm"`, which Node accepts
// under that flag without a build step. Catches WASM-toolchain or
// JS-glue regressions invisible to Rust-only tests.

import { readFileSync } from 'node:fs';
import {
    EXPORT_SCHEMA_VERSION,
    KUL_CORE_VERSION,
    KUL_LANGUAGE_VERSION,
    check,
    exportGraph,
    format,
} from '../../pkg/kul_wasm.js';

const fixture = new URL('../../../../examples/03-three-generations.kul', import.meta.url);
const source = readFileSync(fixture, 'utf8');

// JS callers construct the manifest inline — discovery is the host's job.
const manifest = { kul: '0.1' };

const formatted = format(source);
if (typeof formatted !== 'string' || formatted.length === 0) {
    console.error(`format returned non-string or empty: ${typeof formatted}, length=${formatted?.length}`);
    process.exit(1);
}

const cleanResult = check(source, manifest);
if (!Array.isArray(cleanResult?.diagnostics)) {
    console.error(`check returned non-array diagnostics on clean fixture: ${JSON.stringify(cleanResult)}`);
    process.exit(1);
}
if (cleanResult.diagnostics.length !== 0) {
    console.error(`expected clean fixture to have zero diagnostics, got: ${JSON.stringify(cleanResult.diagnostics)}`);
    process.exit(1);
}

const brokenResult = check('person alice gender:female\n', manifest);
if (!Array.isArray(brokenResult?.diagnostics) || brokenResult.diagnostics.length < 1) {
    console.error(`expected broken source to produce at least one diagnostic, got: ${JSON.stringify(brokenResult)}`);
    process.exit(1);
}
const diag = brokenResult.diagnostics[0];
for (const field of ['code', 'severity', 'message']) {
    if (typeof diag[field] !== 'string' || diag[field].length === 0) {
        console.error(`diagnostic.${field} not a non-empty string: ${JSON.stringify(diag)}`);
        process.exit(1);
    }
}
if (!diag.primary || typeof diag.primary.byteStart !== 'number') {
    console.error(`diagnostic.primary.byteStart missing or non-numeric: ${JSON.stringify(diag)}`);
    process.exit(1);
}

const coreVersion = KUL_CORE_VERSION();
const langVersion = KUL_LANGUAGE_VERSION();
const schemaVersion = EXPORT_SCHEMA_VERSION();

if (typeof coreVersion !== 'string' || coreVersion.length === 0) {
    console.error(`KUL_CORE_VERSION returned non-string or empty: ${coreVersion}`);
    process.exit(1);
}
if (typeof langVersion !== 'string' || langVersion.length === 0) {
    console.error(`KUL_LANGUAGE_VERSION returned non-string or empty: ${langVersion}`);
    process.exit(1);
}
if (!Number.isInteger(schemaVersion) || schemaVersion < 1) {
    console.error(`EXPORT_SCHEMA_VERSION not a positive integer: ${schemaVersion}`);
    process.exit(1);
}

const exportEnvelope = exportGraph(source, manifest);
if (exportEnvelope?.ok !== true) {
    console.error(`exportGraph on clean fixture did not produce success envelope: ${JSON.stringify(exportEnvelope)}`);
    process.exit(1);
}
if (typeof exportEnvelope.schema !== 'number' || exportEnvelope.schema < 1) {
    console.error(`exportGraph schema not a positive integer: ${JSON.stringify(exportEnvelope)}`);
    process.exit(1);
}
const persons = exportEnvelope.graph?.persons;
if (!Array.isArray(persons) || persons.length === 0) {
    console.error(`exportGraph graph.persons not a non-empty array: ${JSON.stringify(exportEnvelope.graph)}`);
    process.exit(1);
}
if (typeof persons[0].name !== 'string' || persons[0].name.length === 0) {
    console.error(`exportGraph graph.persons[0].name not a non-empty string: ${JSON.stringify(persons[0])}`);
    process.exit(1);
}

console.log(`smoke OK — kul-core ${coreVersion}, language ${langVersion}, schema ${schemaVersion}`);
console.log(`format produced ${formatted.length} bytes for 03-three-generations.kul`);
console.log(`check clean → 0 diagnostics; check broken → ${brokenResult.diagnostics.length} diagnostic(s), first ${diag.code}`);
console.log(`exportGraph clean → ${persons.length} person(s), first "${persons[0].name}"`);
