import { describe, expect, it } from "vitest";

import {
    ENGINE_MODULE_ATTR,
    ENGINE_WASM_ATTR,
    getNonce,
    MOUNT_POINT_ID,
    previewHtml,
} from "../src/html.js";

const THEME_HREF =
    "https://file%2B.vscode-resource.example/media/preview-themes.css";
const CSS_HREF = "https://file%2B.vscode-resource.example/media/preview.css";
const SCRIPT_HREF =
    "https://file%2B.vscode-resource.example/media/preview/preview-webview.js";
const CSP_SOURCE = "https://file%2B.vscode-resource.example";
const NONCE = "abc123ABC123abc123ABC123abc12345";

const ENGINE_SOURCE = {
    moduleUri: "https://file%2B.vscode-resource.example/media/preview/wasm/kul_wasm.js",
    wasmUri:
        "https://file%2B.vscode-resource.example/media/preview/wasm/kul_wasm_bg.wasm",
};

function build(): string {
    return previewHtml({
        themeStylesheetUri: THEME_HREF,
        applicationStylesheetUri: CSS_HREF,
        scriptUri: SCRIPT_HREF,
        cspSource: CSP_SOURCE,
        nonce: NONCE,
    });
}

function buildWithEngine(): string {
    return previewHtml({
        themeStylesheetUri: THEME_HREF,
        applicationStylesheetUri: CSS_HREF,
        scriptUri: SCRIPT_HREF,
        cspSource: CSP_SOURCE,
        nonce: NONCE,
        engineSource: ENGINE_SOURCE,
    });
}

function cspDirective(html: string, name: string): string {
    const match = html.match(/content="([^"]*)"\s*>/);
    expect(match, "CSP <meta> content present").not.toBeNull();
    const directive = match![1]
        .split(";")
        .map((d) => d.trim())
        .find((d) => d.startsWith(`${name} `) || d === name);
    expect(directive, `CSP has a ${name} directive`).toBeDefined();
    return directive!;
}

function cspDirectiveOrNull(html: string, name: string): string | null {
    const match = html.match(/content="([^"]*)"\s*>/);
    return (
        match![1]
            .split(";")
            .map((d) => d.trim())
            .find((d) => d.startsWith(`${name} `) || d === name) ?? null
    );
}

describe("getNonce", () => {
    it("returns a 32-char alphanumeric token", () => {
        const nonce = getNonce();
        expect(nonce).toHaveLength(32);
        expect(nonce).toMatch(/^[A-Za-z0-9]+$/);
    });

    it("returns a fresh value each call", () => {
        expect(getNonce()).not.toBe(getNonce());
    });

    it("draws from the CSPRNG so a batch is collision-free", () => {
        // A weak per-character PRNG (e.g. Math.random) collides far sooner
        // than a 190-bit CSPRNG token; 1000 draws must all be distinct.
        const seen = new Set<string>();
        for (let i = 0; i < 1000; i++) {
            seen.add(getNonce());
        }
        expect(seen.size).toBe(1000);
    });

    it("spends the full alphabet across many draws (not a stuck byte source)", () => {
        const alphabet =
            "ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789";
        const used = new Set<string>();
        for (let i = 0; i < 200; i++) {
            for (const ch of getNonce()) {
                used.add(ch);
            }
        }
        expect(used.size).toBe(alphabet.length);
    });
});

describe("previewHtml CSP", () => {
    it("nonce-gates script-src against cspSource without 'unsafe-inline'", () => {
        const scriptSrc = cspDirective(build(), "script-src");
        expect(scriptSrc).toContain(`nonce-${NONCE}`);
        expect(scriptSrc).toContain(CSP_SOURCE);
        expect(scriptSrc).not.toContain("'unsafe-inline'");
    });

    it("keeps 'unsafe-inline' on style-src", () => {
        const styleSrc = cspDirective(build(), "style-src");
        expect(styleSrc).toContain(CSP_SOURCE);
        expect(styleSrc).toContain("'unsafe-inline'");
    });

    it("grants no wasm allowance to a shell with no engine", () => {
        const html = build();
        expect(cspDirective(html, "script-src")).not.toContain("wasm-unsafe-eval");
        expect(cspDirectiveOrNull(html, "connect-src")).toBeNull();
    });
});

describe("previewHtml CSP with an engine (ADR-0034)", () => {
    it("adds 'wasm-unsafe-eval' to script-src", () => {
        expect(cspDirective(buildWithEngine(), "script-src")).toContain(
            "'wasm-unsafe-eval'",
        );
    });

    it("never substitutes the broad 'unsafe-eval'", () => {
        // The narrow grant exists for exactly this; the broad one would
        // re-enable eval() on a surface that renders document content.
        const scriptSrc = cspDirective(buildWithEngine(), "script-src");
        expect(scriptSrc).not.toMatch(/(^|[^-])'unsafe-eval'/);
    });

    it("keeps the nonce gate and adds no 'unsafe-inline'", () => {
        const scriptSrc = cspDirective(buildWithEngine(), "script-src");
        expect(scriptSrc).toContain(`nonce-${NONCE}`);
        expect(scriptSrc).not.toContain("'unsafe-inline'");
    });

    it("opens connect-src to the host origin so the .wasm can be fetched", () => {
        const connectSrc = cspDirective(buildWithEngine(), "connect-src");
        expect(connectSrc).toBe(`connect-src ${CSP_SOURCE}`);
    });

    it("leaves default-src at 'none' and style-src untouched", () => {
        const html = buildWithEngine();
        expect(cspDirective(html, "default-src")).toBe("default-src 'none'");
        expect(cspDirective(html, "style-src")).toBe(
            `style-src ${CSP_SOURCE} 'unsafe-inline'`,
        );
    });
});

describe("previewHtml engine source", () => {
    it("stamps both engine URIs on the mount point", () => {
        const html = buildWithEngine();
        expect(html).toContain(
            `${ENGINE_MODULE_ATTR}="${ENGINE_SOURCE.moduleUri}"`,
        );
        expect(html).toContain(`${ENGINE_WASM_ATTR}="${ENGINE_SOURCE.wasmUri}"`);
    });

    it("omits the attributes entirely when no engine is supplied", () => {
        const html = build();
        expect(html).not.toContain(ENGINE_MODULE_ATTR);
        expect(html).not.toContain(ENGINE_WASM_ATTR);
    });

    it("escapes a quote in a URI so it cannot break out of the attribute", () => {
        const html = previewHtml({
            themeStylesheetUri: THEME_HREF,
            applicationStylesheetUri: CSS_HREF,
            scriptUri: SCRIPT_HREF,
            cspSource: CSP_SOURCE,
            nonce: NONCE,
            engineSource: {
                moduleUri: 'x" onload="alert(1)',
                wasmUri: "y",
            },
        });
        expect(html).toContain('x&quot; onload=&quot;alert(1)');
        expect(html).not.toContain('onload="alert(1)"');
    });
});

describe("previewHtml scripts and stylesheets", () => {
    it("stamps the nonce on the webview entry script and sets its src", () => {
        expect(build()).toContain(
            `<script nonce="${NONCE}" src="${SCRIPT_HREF}"></script>`,
        );
    });

    it("links both the theme and application stylesheets in order", () => {
        const html = build();
        expect(html).toContain(`href="${THEME_HREF}"`);
        expect(html).toContain(`href="${CSS_HREF}"`);
        expect(html.indexOf(THEME_HREF)).toBeLessThan(html.indexOf(CSS_HREF));
    });

    it("includes the mount-point div the webview entry hooks", () => {
        expect(build()).toContain(`<div id="${MOUNT_POINT_ID}"></div>`);
    });

    it("emits the body[data-theme=\"vscode\"] default theme block", () => {
        expect(build()).toContain('<body data-theme="vscode">');
    });

    it("respects a custom themeName override", () => {
        const html = previewHtml({
            themeStylesheetUri: THEME_HREF,
            applicationStylesheetUri: CSS_HREF,
            scriptUri: SCRIPT_HREF,
            cspSource: CSP_SOURCE,
            nonce: NONCE,
            themeName: "custom",
        });
        expect(html).toContain('<body data-theme="custom">');
    });
});
