import { readFileSync } from "node:fs";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";
import { describe, expect, it } from "vitest";

/**
 * Structural lint over the two-tier token layer (ADR-0036).
 *
 * jsdom resolves neither custom-property substitution nor layout, so a
 * computed-value test is not reachable with this stack. The sheets are read as
 * text instead — the same shape `crates/kul-svg/tests/visual.rs` uses to lint
 * the baked token layer. What this catches is what a 120-token rewrite
 * actually breaks: dangling names and layer violations.
 */

const SRC = join(dirname(fileURLToPath(import.meta.url)), "..", "src");

function readSheet(name: string): string {
    // Comments carry the tier documentation and the reserved-hue rationale;
    // stripping them keeps prose out of every scan below.
    return readFileSync(join(SRC, name), "utf8").replace(/\/\*[\s\S]*?\*\//g, "");
}

const themeSheet = readSheet("preview-themes.css");
const appSheet = readSheet("preview.css");

/**
 * Tier-1 tokens with no consumer yet. The query paints and the filter alpha
 * are reserved by ADR-0036 for the chrome later slices of epic #296 build;
 * they are defined here so the paint vocabulary is decided once, in one place,
 * rather than invented per slice. Every other name must earn its keep — the
 * rule that forced `--kul-tooltip-*` out with `tooltip.ts` (#300).
 */
const RESERVED_PENDING_CONSUMERS = [
    "--kul-hue-query-selection",
    "--kul-hue-query-result",
    "--kul-hue-query-uncertain",
    "--kul-hue-query-path",
    "--kul-dim-alpha",
];

/**
 * Tier-1 tokens whose values are literal hues rather than palette bridges,
 * each documented in place in the theme sheet. This is the *whole* list of
 * places a raw colour may appear in the preview.
 */
const RESERVED_HUES = [
    "--kul-hue-sync-selection",
    ...RESERVED_PENDING_CONSUMERS.filter((name) => name !== "--kul-dim-alpha"),
];

/**
 * Ceiling on the theme contract. ADR-0036's load-bearing promise is that a
 * theme supplies a small register that does **not** grow per feature — chrome
 * growth lands in tier 2, which costs a theme nothing. Raising this number is
 * therefore a deliberate act: it means a genuinely new *kind* of value (a role,
 * a scale step, a reserved hue) entered the contract.
 */
const TIER_1_BUDGET = 38;

const HEX_PATTERN = "#[0-9a-fA-F]{3,8}(?![0-9a-zA-Z_-])";

interface Block {
    selector: string;
    declarations: Map<string, string>;
}

/** Token blocks carry no nested braces, so a flat scan is sufficient. */
function parseBlocks(css: string): Block[] {
    const blocks: Block[] = [];
    for (const match of css.matchAll(/([^{}]+)\{([^{}]*)\}/g)) {
        const declarations = new Map<string, string>();
        for (const decl of match[2].split(";")) {
            const colon = decl.indexOf(":");
            if (colon < 0) {
                continue;
            }
            const property = decl.slice(0, colon).trim();
            if (property.startsWith("--kul-")) {
                declarations.set(property, decl.slice(colon + 1).trim());
            }
        }
        blocks.push({ selector: match[1].trim(), declarations });
    }
    return blocks;
}

function kulReferences(value: string): string[] {
    return [...value.matchAll(/var\(\s*(--kul-[a-z0-9-]+)/g)].map((m) => m[1]);
}

const blocks = parseBlocks(themeSheet);
const themeBlocks = blocks.filter((b) => /^body\[data-theme=/.test(b.selector));
const tier2Block = blocks.find((b) => b.selector === "body");

if (!tier2Block) {
    throw new Error("preview-themes.css must declare the tier-2 layer on `body`");
}

// A name declared in both layers is a theme's per-site override (ADR-0016),
// which stays tier 2 — the override does not promote it into the contract.
const tier2 = tier2Block.declarations;
const tier1 = new Map<string, string>();
for (const block of themeBlocks) {
    for (const [property, value] of block.declarations) {
        if (!tier2.has(property)) {
            tier1.set(property, value);
        }
    }
}

const consumedByApp = new Set(kulReferences(appSheet));
const consumedByTier2 = new Set(
    [...tier2.values()].flatMap((value) => kulReferences(value)),
);

describe("two-tier token layer", () => {
    it("splits every --kul-* declaration between a theme block and the tier-2 layer", () => {
        const strays = blocks
            .filter((b) => b.declarations.size > 0)
            .filter((b) => b !== tier2Block && !themeBlocks.includes(b))
            .map((b) => b.selector);
        expect(strays).toEqual([]);
        expect(themeBlocks.map((b) => b.selector)).toEqual([
            'body[data-theme="vscode"]',
        ]);
    });

    it("defines every --kul-* token that either sheet consumes", () => {
        const defined = new Set([...tier1.keys(), ...tier2.keys()]);
        const dangling = [...consumedByApp, ...consumedByTier2]
            .filter((name) => !defined.has(name))
            .sort();
        expect(dangling).toEqual([]);
    });

    it("bridges to --vscode-* in tier 1 and nowhere else", () => {
        expect([...appSheet.matchAll(/var\(\s*--vscode-[a-z0-9-]+/gi)]).toEqual([]);
        const leaks = [...tier2.entries()]
            .filter(([, value]) => value.includes("--vscode-"))
            .map(([property]) => property);
        expect(leaks).toEqual([]);
    });

    it("resolves every tier-2 alias to a tier-1 primitive", () => {
        const violations = [...tier2.entries()]
            .flatMap(([property, value]) =>
                kulReferences(value).map((target) => ({ property, target })),
            )
            .filter(({ target }) => !tier1.has(target))
            .map(({ property, target }) => `${property} -> ${target}`);
        expect(violations).toEqual([]);
    });

    it("keeps raw hex out of both sheets except the documented reserved hues", () => {
        // Id selectors cannot collide: the pattern needs three hex digits and a
        // token boundary, and every id in the sheet starts with `root` / `kul-`.
        expect(appSheet.match(new RegExp(HEX_PATTERN, "g"))).toBeNull();
        const hexDeclarations = [...tier1.entries(), ...tier2.entries()]
            .filter(([, value]) => new RegExp(HEX_PATTERN).test(value))
            .map(([property]) => property)
            .sort();
        expect(hexDeclarations).toEqual([...RESERVED_HUES].sort());
    });

    it("consumes every token it defines", () => {
        const unusedTier2 = [...tier2.keys()]
            .filter((name) => !consumedByApp.has(name))
            .sort();
        expect(unusedTier2).toEqual([]);

        const reserved = new Set(RESERVED_PENDING_CONSUMERS);
        const unusedTier1 = [...tier1.keys()]
            .filter((name) => !consumedByTier2.has(name) && !consumedByApp.has(name))
            .filter((name) => !reserved.has(name))
            .sort();
        expect(unusedTier1).toEqual([]);

        // The carve-out itself cannot rot: a reserved token that gains a
        // consumer must leave the list.
        const consumedReserved = RESERVED_PENDING_CONSUMERS.filter(
            (name) => consumedByTier2.has(name) || consumedByApp.has(name),
        );
        expect(consumedReserved).toEqual([]);
    });

    it("holds the theme contract to a fixed budget", () => {
        expect(tier1.size).toBeLessThanOrEqual(TIER_1_BUDGET);
    });
});
