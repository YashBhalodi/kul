// End-to-end smoke test: imports the wasm-pack bundler-target output
// exactly as a downstream JS consumer would, drives `format`, `check`,
// and `exportGraph` against the example corpus, and asserts the results
// look sensible. Includes a multi-file invocation to confirm the
// array-based `check` / `exportGraph` signatures wire up correctly.
//
// Run via `node --experimental-wasm-modules` (Node 22+). The bundler
// target uses ESM `import * as wasm from "./*.wasm"`, which Node accepts
// under that flag without a build step. Catches WASM-toolchain or
// JS-glue regressions invisible to Rust-only tests.

import { readdirSync, readFileSync } from 'node:fs';
import { fileURLToPath } from 'node:url';
import {
    EXPORT_SCHEMA_VERSION,
    KUL_CORE_VERSION,
    KUL_LANGUAGE_VERSION,
    check,
    exportGraph,
    format,
} from '../../pkg/kul_wasm.js';

const fixture = new URL('../../../../examples/03-three-generations/three-generations.kul', import.meta.url);
const source = readFileSync(fixture, 'utf8');

// JS callers construct the manifest inline — discovery is the host's job.
const manifest = { kul: '0.1' };

// `format` stays single-source — the per-file asymmetry is intentional.
const formatted = format(source);
if (typeof formatted !== 'string' || formatted.length === 0) {
    console.error(`format returned non-string or empty: ${typeof formatted}, length=${formatted?.length}`);
    process.exit(1);
}

// `check` and `exportGraph` take an array of `{name, source}`. Single-
// file invocation: a one-element array.
const singleFile = [{ name: 'three-generations.kul', source }];
const cleanResult = check(singleFile, manifest);
if (!Array.isArray(cleanResult?.diagnostics)) {
    console.error(`check returned non-array diagnostics on clean fixture: ${JSON.stringify(cleanResult)}`);
    process.exit(1);
}
if (cleanResult.diagnostics.length !== 0) {
    console.error(`expected clean fixture to have zero diagnostics, got: ${JSON.stringify(cleanResult.diagnostics)}`);
    process.exit(1);
}

const brokenResult = check([{ name: 'input.kul', source: 'person alice gender:female\n' }], manifest);
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
if (diag.primary.file !== 'input.kul') {
    console.error(`diagnostic.primary.file should echo the input file name; got ${JSON.stringify(diag.primary)}`);
    process.exit(1);
}

// Multi-file invocation: read every `.kul` file in the multi-file
// example and feed them in as an array. Confirms the array-based
// signature carries cross-file resolution through the WASM ABI.
const multiDir = fileURLToPath(new URL('../../../../examples/07-multi-file-extended-family/', import.meta.url));
const multiFiles = readdirSync(multiDir)
    .filter((name) => name.endsWith('.kul'))
    .sort()
    .map((name) => ({ name, source: readFileSync(`${multiDir}${name}`, 'utf8') }));
if (multiFiles.length < 2) {
    console.error(`expected multi-file example to have >= 2 .kul files, got: ${multiFiles.length}`);
    process.exit(1);
}
const multiCheck = check(multiFiles, manifest);
if (!Array.isArray(multiCheck?.diagnostics) || multiCheck.diagnostics.length !== 0) {
    console.error(`expected multi-file example to be clean, got: ${JSON.stringify(multiCheck)}`);
    process.exit(1);
}
const multiExport = exportGraph(multiFiles, manifest);
if (multiExport?.ok !== true) {
    console.error(`exportGraph on multi-file example did not produce success envelope: ${JSON.stringify(multiExport)}`);
    process.exit(1);
}
const multiPersons = multiExport.graph?.persons;
if (!Array.isArray(multiPersons) || multiPersons.length === 0) {
    console.error(`exportGraph multi-file graph.persons not a non-empty array: ${JSON.stringify(multiExport.graph)}`);
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

const exportEnvelope = exportGraph(singleFile, manifest);
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
console.log(`format produced ${formatted.length} bytes for 03-three-generations/three-generations.kul`);
console.log(`check clean → 0 diagnostics; check broken → ${brokenResult.diagnostics.length} diagnostic(s), first ${diag.code}`);
console.log(`exportGraph clean → ${persons.length} person(s), first "${persons[0].name}"`);
console.log(`multi-file check → 0 diagnostics across ${multiFiles.length} files; exportGraph → ${multiPersons.length} person(s)`);
