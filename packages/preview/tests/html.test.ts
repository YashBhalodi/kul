import { describe, expect, it } from "vitest";

import { getNonce, MOUNT_POINT_ID, previewHtml } from "../src/html.js";

const THEME_HREF =
    "https://file%2B.vscode-resource.example/media/preview-themes.css";
const CSS_HREF = "https://file%2B.vscode-resource.example/media/preview.css";
const SCRIPT_HREF =
    "https://file%2B.vscode-resource.example/media/preview/preview-webview.js";
const CSP_SOURCE = "https://file%2B.vscode-resource.example";
const NONCE = "abc123ABC123abc123ABC123abc12345";

function build(): string {
    return previewHtml({
        themeStylesheetUri: THEME_HREF,
        applicationStylesheetUri: CSS_HREF,
        scriptUri: SCRIPT_HREF,
        cspSource: CSP_SOURCE,
        nonce: NONCE,
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

    it("keeps 'unsafe-inline' on style-src", () => {
        const styleSrc = cspDirective(build(), "style-src");
        expect(styleSrc).toContain(CSP_SOURCE);
        expect(styleSrc).toContain("'unsafe-inline'");
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
