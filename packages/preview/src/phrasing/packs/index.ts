import type { LanguagePack } from "../pack.js";
import { EN } from "./en.js";

/**
 * Every language pack the preview ships, in offer order.
 *
 * This array is the whole registration surface: a new language is one module
 * of records plus one entry here, and it inherits the pack suites (coverage,
 * conflict-freedom, well-formedness) without adding a line of logic or a line
 * of test. That is the additivity promise ADR-0033 designed for, expressed as
 * a diff shape rather than an assertion.
 */
export const PACKS: ReadonlyArray<LanguagePack> = [EN];

/** The pack for a BCP-47 subtag, or `undefined` when none is registered. */
export function packFor(code: string): LanguagePack | undefined {
    return PACKS.find((pack) => pack.code === code);
}

export { EN };
