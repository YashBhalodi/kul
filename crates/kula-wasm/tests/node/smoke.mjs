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
    KULA_CORE_VERSION,
    KULA_LANGUAGE_VERSION,
    format,
} from '../../pkg/kula_wasm.js';

const fixture = new URL('../../../../examples/03-three-generations.kula', import.meta.url);
const source = readFileSync(fixture, 'utf8');

const formatted = format(source);
if (typeof formatted !== 'string' || formatted.length === 0) {
    console.error(`format returned non-string or empty: ${typeof formatted}, length=${formatted?.length}`);
    process.exit(1);
}

const coreVersion = KULA_CORE_VERSION();
const langVersion = KULA_LANGUAGE_VERSION();
const schemaVersion = EXPORT_SCHEMA_VERSION();

if (typeof coreVersion !== 'string' || coreVersion.length === 0) {
    console.error(`KULA_CORE_VERSION returned non-string or empty: ${coreVersion}`);
    process.exit(1);
}
if (typeof langVersion !== 'string' || langVersion.length === 0) {
    console.error(`KULA_LANGUAGE_VERSION returned non-string or empty: ${langVersion}`);
    process.exit(1);
}
if (!Number.isInteger(schemaVersion) || schemaVersion < 1) {
    console.error(`EXPORT_SCHEMA_VERSION not a positive integer: ${schemaVersion}`);
    process.exit(1);
}

console.log(`smoke OK — kula-core ${coreVersion}, language ${langVersion}, schema ${schemaVersion}`);
console.log(`format produced ${formatted.length} bytes for 03-three-generations.kula`);
