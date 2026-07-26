// Conformance lint for the hand-mirrored WASM wire types.
//
// `packages/preview/src/engine-wire.ts` restates declarations that are
// generated into `crates/kul-wasm/types/kul_wasm.d.ts`. Nothing in the build
// links the two — the preview never imports `@kullang/wasm` (ADR-0040) — so
// this test is the link: it re-reads the generated snapshot and fails when a
// mirrored declaration has drifted from it.

import { readFileSync, readdirSync } from "node:fs";
import { dirname, resolve } from "node:path";
import { fileURLToPath } from "node:url";

import { describe, expect, it } from "vitest";

const HERE = dirname(fileURLToPath(import.meta.url));
const GENERATED = resolve(
    HERE,
    "..",
    "..",
    "..",
    "crates",
    "kul-wasm",
    "types",
    "kul_wasm.d.ts",
);
const MIRROR = resolve(HERE, "..", "src", "engine-wire.ts");

/** Every declaration `engine-wire.ts` claims to mirror. */
const MIRRORED = [
    "WasmInputFile",
    "Manifest",
    "ExportedSpan",
    "ExportedRelated",
    "ExportedDiagnostic",
    "QueryOk",
    "QueryError",
    "QueryEnvelope",
    "ExportedDate",
    "ExportedPerson",
    "ExportedMarriage",
    "ParenthoodLinkKind",
    "ExportedParenthoodLink",
    "LinkedPerson",
    "MarriageTie",
    "DetailTarget",
    "EntityDetail",
    "DetailLookupResult",
];

/** Drop block comments, then collapse all runs of whitespace to one space. */
function normalize(declaration: string): string {
    return declaration
        .replace(/\/\*[\s\S]*?\*\//g, " ")
        .replace(/\/\/[^\n]*/g, " ")
        .replace(/\s+/g, " ")
        .trim();
}

/**
 * Extract the declaration of `name` from a TypeScript source: an
 * `export interface X … { … }` up to its balanced closing brace, or an
 * `export type X = …;` up to its terminating semicolon.
 */
function declarationOf(source: string, name: string): string {
    const interfaceStart = source.search(
        new RegExp(`^export interface ${name}(?![A-Za-z0-9_])`, "m"),
    );
    if (interfaceStart >= 0) {
        const open = source.indexOf("{", interfaceStart);
        let depth = 0;
        for (let i = open; i < source.length; i++) {
            if (source[i] === "{") depth++;
            else if (source[i] === "}") {
                depth--;
                if (depth === 0) {
                    return source.slice(interfaceStart, i + 1);
                }
            }
        }
        throw new Error(`unbalanced braces in interface ${name}`);
    }
    const typeStart = source.search(
        new RegExp(`^export type ${name}(?![A-Za-z0-9_])`, "m"),
    );
    if (typeStart >= 0) {
        const end = source.indexOf(";", typeStart);
        return source.slice(typeStart, end + 1);
    }
    throw new Error(`no declaration of ${name}`);
}

const generated = readFileSync(GENERATED, "utf8");
const mirror = readFileSync(MIRROR, "utf8");

describe("engine-wire mirrors crates/kul-wasm/types/kul_wasm.d.ts", () => {
    it.each(MIRRORED)("%s matches the generated declaration", (name) => {
        expect(normalize(declarationOf(mirror, name))).toBe(
            normalize(declarationOf(generated, name)),
        );
    });

    it("mirrors every declaration engine-wire.ts exports as a wire type", () => {
        // Guards the list above: a type added to engine-wire.ts without being
        // added to MIRRORED would otherwise go unchecked. `isQueryOk` is a
        // consumer-side narrow, not a wire shape, so it is excluded by being
        // a function rather than a type declaration.
        const declared = Array.from(
            mirror.matchAll(/^export (?:interface|type) ([A-Za-z0-9_]+)/gm),
        ).map((m) => m[1]);
        expect(declared.sort()).toEqual([...MIRRORED].sort());
    });

    it("never imports the engine as a runtime module specifier", () => {
        // ADR-0040: the engine reaches the webview as a host-supplied build
        // asset, so no source file may carry a dependency edge to the npm
        // package — the whole reason these types are mirrored by hand.
        const src = resolve(HERE, "..", "src");
        const offenders = readdirSync(src)
            .filter((f) => f.endsWith(".ts"))
            .filter((f) =>
                /(?:from|import|require)\s*\(?\s*["']@kullang\/wasm["']/.test(
                    readFileSync(resolve(src, f), "utf8"),
                ),
            );
        expect(offenders).toEqual([]);
    });
});
