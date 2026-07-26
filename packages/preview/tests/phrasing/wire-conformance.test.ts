import { readFileSync } from "node:fs";
import { dirname, resolve } from "node:path";
import { fileURLToPath } from "node:url";

import { describe, expect, it } from "vitest";

/**
 * `src/phrasing/descriptor.ts` mirrors the committed tsify output rather than
 * importing it (the phrasing module has no dependencies, by ADR-0033). This
 * lint reads both files as text and fails the moment they drift — so a change
 * to the Rust descriptor that lands in `crates/kul-wasm/types/kul_wasm.d.ts`
 * (ADR-0012) cannot silently leave the phrasing key keying a stale wire.
 */
const HERE = dirname(fileURLToPath(import.meta.url));
const COMMITTED = resolve(HERE, "../../../../crates/kul-wasm/types/kul_wasm.d.ts");
const MIRROR = resolve(HERE, "../../src/phrasing/descriptor.ts");

/** Every wire type the phrasing key reads. */
const MIRRORED_TYPES = [
    "Gender",
    "Seniority",
    "EdgeNature",
    "Affinity",
    "Sharing",
    "Side",
    "HopEdge",
    "MarriageStatus",
    "LinealRole",
    "Classification",
    "PathHop",
    "RelationshipDescriptor",
];

/** Pull one `export type X = …;` or `export interface X { … }` declaration. */
function declarationOf(source: string, name: string): string | null {
    const alias = new RegExp(`^export type ${name} = .*$`, "m").exec(source);
    if (alias !== null) return alias[0];
    const start = source.indexOf(`export interface ${name} {`);
    if (start === -1) return null;
    const end = source.indexOf("\n}", start);
    if (end === -1) return null;
    return source.slice(start, end + 2);
}

describe("the mirrored descriptor types", () => {
    const committed = readFileSync(COMMITTED, "utf8");
    const mirror = readFileSync(MIRROR, "utf8");

    it.each(MIRRORED_TYPES)("match the committed wire form for %s", (name) => {
        const wire = declarationOf(committed, name);
        expect(wire, `${name} is missing from the committed .d.ts`).not.toBeNull();
        expect(declarationOf(mirror, name)).toBe(wire);
    });

    it("mirrors no type the wire does not declare", () => {
        const declared = [...mirror.matchAll(/^export (?:type|interface) (\w+)/gm)].map(
            (match) => match[1]!,
        );
        expect(declared.sort()).toEqual([...MIRRORED_TYPES].sort());
    });
});
