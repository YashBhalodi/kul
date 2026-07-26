import { readdirSync, readFileSync } from "node:fs";
import { dirname, join, relative, resolve } from "node:path";
import { fileURLToPath } from "node:url";

import { describe, expect, it } from "vitest";

/**
 * The standing rule of ADR-0033: `src/phrasing/` imports no DOM, so extracting
 * it to its own package stays mechanical. Asserted as a structural lint over
 * the sources, the same way `crates/kul-svg/tests/visual.rs` lints the baked
 * token layer as text.
 */
const PHRASING_DIR = resolve(dirname(fileURLToPath(import.meta.url)), "../../src/phrasing");

function sourceFiles(dir: string): string[] {
    return readdirSync(dir, { withFileTypes: true }).flatMap((entry) => {
        const path = join(dir, entry.name);
        if (entry.isDirectory()) return sourceFiles(path);
        return entry.name.endsWith(".ts") ? [path] : [];
    });
}

const FILES = sourceFiles(PHRASING_DIR);

/** Host globals and browser-only types the module must never reach for. */
const DOM_IDENTIFIERS = [
    "document",
    "window",
    "navigator",
    "localStorage",
    "HTMLElement",
    "SVGElement",
    "Element",
    "Node",
    "Event",
    "addEventListener",
    "querySelector",
    "acquireVsCodeApi",
];

describe("the phrasing module is pure", () => {
    it("has sources to lint", () => {
        expect(FILES.length).toBeGreaterThan(4);
    });

    it("names no DOM global or DOM type", () => {
        const offenders: string[] = [];
        for (const file of FILES) {
            const source = readFileSync(file, "utf8").replace(/\/\*[\s\S]*?\*\//g, "");
            for (const identifier of DOM_IDENTIFIERS) {
                if (new RegExp(`\\b${identifier}\\b`).test(source)) {
                    offenders.push(`${relative(PHRASING_DIR, file)}: ${identifier}`);
                }
            }
        }
        expect(offenders).toEqual([]);
    });

    it("imports nothing from outside the module", () => {
        const escapes: string[] = [];
        for (const file of FILES) {
            const source = readFileSync(file, "utf8");
            for (const match of source.matchAll(/(?:from|import)\s+"([^"]+)"/g)) {
                const specifier = match[1]!;
                if (!specifier.startsWith(".")) {
                    escapes.push(`${relative(PHRASING_DIR, file)}: ${specifier}`);
                    continue;
                }
                const target = resolve(dirname(file), specifier);
                if (!target.startsWith(PHRASING_DIR)) {
                    escapes.push(`${relative(PHRASING_DIR, file)}: ${specifier}`);
                }
            }
        }
        expect(escapes).toEqual([]);
    });
});
