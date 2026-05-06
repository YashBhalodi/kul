// TypeScript consumer compile-test for `@kulalang/wasm`.
//
// Runs under `tsc --noEmit` in CI. Catches the case where the generated
// `.d.ts` compiles against itself but isn't usable in real consumer code:
// argument types, return types, and the shape of the named exports must
// all match what a downstream TypeScript user would expect.

import {
    EXPORT_SCHEMA_VERSION,
    KULA_CORE_VERSION,
    KULA_LANGUAGE_VERSION,
    format,
} from '../../pkg/kula_wasm.js';

// `format` accepts a string and returns a string unconditionally
// (best-effort even on partial-parse input — see PRD-0004 user story 12).
const source: string = 'person alice name:"A" gender:female\n';
const formatted: string = format(source);

// Type system must reject non-string inputs.
// @ts-expect-error format requires a string source
format(42);

// Version metadata: two strings and a number.
const coreVersion: string = KULA_CORE_VERSION();
const langVersion: string = KULA_LANGUAGE_VERSION();
const schemaVersion: number = EXPORT_SCHEMA_VERSION();

// Realistic consumer assertion pattern: gate behavior on schema version
// before parsing an envelope.
if (schemaVersion < 1) {
    throw new Error('unsupported schema');
}

// Suppress "unused binding" diagnostics in --noUnusedLocals mode.
export const _exports = { formatted, coreVersion, langVersion, schemaVersion };
