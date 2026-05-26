import { describe, expect, it } from "vitest";

import { getNonce, previewHtml } from "./preview-html";

const THEME_HREF =
    "https://file%2B.vscode-resource.example/media/preview-themes.css";
const CSS_HREF = "https://file%2B.vscode-resource.example/media/preview.css";
const SCRIPT_HREF =
    "https://file%2B.vscode-resource.example/media/vendor/dist/svg-pan-zoom.min.js";
const CSP_SOURCE = "https://file%2B.vscode-resource.example";
const NONCE = "abc123ABC123abc123ABC123abc12345";

function build(): string {
    return previewHtml(THEME_HREF, CSS_HREF, SCRIPT_HREF, CSP_SOURCE, NONCE);
}

/** Pull a single CSP directive (e.g. `script-src`) out of the rendered HTML. */
function cspDirective(html: string, name: string): string {
    const match = html.match(
        /content="([^"]*)"\s*>/,
    );
    expect(match, "CSP <meta> content present").not.toBeNull();
    const directive = match![1]
        .split(";")
        .map((d) => d.trim())
        .find((d) => d.startsWith(`${name} `) || d === name);
    expect(directive, `CSP has a ${name} directive`).toBeDefined();
    return directive!;
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
});

describe("previewHtml CSP", () => {
    it("nonce-gates script-src against cspSource without 'unsafe-inline'", () => {
        const scriptSrc = cspDirective(build(), "script-src");
        expect(scriptSrc).toContain(`nonce-${NONCE}`);
        expect(scriptSrc).toContain(CSP_SOURCE);
        expect(scriptSrc).not.toContain("'unsafe-inline'");
    });

    it("keeps 'unsafe-inline' on style-src (unchanged)", () => {
        const styleSrc = cspDirective(build(), "style-src");
        expect(styleSrc).toContain(CSP_SOURCE);
        expect(styleSrc).toContain("'unsafe-inline'");
    });
});

describe("previewHtml scripts", () => {
    it("stamps the nonce on the inline bootstrap script", () => {
        expect(build()).toContain(`<script nonce="${NONCE}">`);
    });

    it("stamps the nonce on the vendored script and sets its src", () => {
        expect(build()).toContain(
            `<script nonce="${NONCE}" src="${SCRIPT_HREF}"></script>`,
        );
    });

    it("links both the theme and application stylesheets", () => {
        const html = build();
        expect(html).toContain(`href="${THEME_HREF}"`);
        expect(html).toContain(`href="${CSS_HREF}"`);
        // Theme tokens are linked before the rules that consume them.
        expect(html.indexOf(THEME_HREF)).toBeLessThan(html.indexOf(CSS_HREF));
    });
});

describe("previewHtml bootstrap", () => {
    it("inits svg-pan-zoom, tears it down, and guards a missing <svg>", () => {
        const html = build();
        expect(html).toContain("svgPanZoom(");
        expect(html).toContain("destroy()");
        expect(html).toContain("if (!svg)");
    });
});

describe("previewHtml overlay controls", () => {
    it("renders custom HTML buttons for zoom-in, reset, and zoom-out", () => {
        const html = build();
        expect(html).toContain('data-action="zoom-in"');
        expect(html).toContain('data-action="reset"');
        expect(html).toContain('data-action="zoom-out"');
    });

    it("uses custom controls, not the library's built-in icons", () => {
        expect(build()).toContain("controlIconsEnabled: false");
    });

    it("wires the buttons to the pan/zoom instance", () => {
        const html = build();
        expect(html).toContain("panZoom.zoomIn()");
        expect(html).toContain("panZoom.zoomOut()");
        expect(html).toContain("panZoom.reset()");
    });
});
