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
    check,
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

// `check` returns `{ diagnostics }`. Empty array means clean — emptiness
// is the discriminator (no `ok` field per PRD-0004).
const cleanResult = check(source);
if (cleanResult.diagnostics.length === 0) {
    // Clean-document short-circuit: downstream consumers proceed without
    // touching the diagnostic list.
}

// Narrow into a real diagnostic against a known-broken source so the TS
// types for `code`, `severity`, `message`, and `primary.byteStart` land.
const broken = check('person alice gender:female\n');
const firstDiagnostic = broken.diagnostics[0];
if (firstDiagnostic !== undefined) {
    const code: string = firstDiagnostic.code;
    const severity: string = firstDiagnostic.severity;
    const message: string = firstDiagnostic.message;
    const byteStart: number = firstDiagnostic.primary.byteStart;
    void code;
    void severity;
    void message;
    void byteStart;
}

// Type system must reject non-string inputs to `check`.
// @ts-expect-error check requires a string source
check(42);

// Suppress "unused binding" diagnostics in --noUnusedLocals mode.
export const _exports = {
    formatted,
    coreVersion,
    langVersion,
    schemaVersion,
    cleanResult,
    broken,
};
