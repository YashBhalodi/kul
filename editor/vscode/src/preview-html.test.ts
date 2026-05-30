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

describe("previewHtml click-to-source", () => {
    it("acquires the VSCode API for posting messages", () => {
        expect(build()).toContain("acquireVsCodeApi()");
    });

    it("attaches a click listener on #root", () => {
        // #root survives every innerHTML swap, so the listener is wired
        // there once rather than per-render on the SVG.
        expect(build()).toContain("root.addEventListener('click'");
    });

    it("posts revealSource with the person id for a clicked card", () => {
        const html = build();
        expect(html).toContain("event.target.closest('[data-person-id]')");
        expect(html).toContain("getAttribute('data-person-id')");
        expect(html).toContain("type: 'revealSource'");
    });

    it("posts revealSource with the marriage id for a clicked marriage bar", () => {
        const html = build();
        expect(html).toContain(
            "event.target.closest('[data-link-kind=\"marriage\"]')",
        );
        expect(html).toContain("getAttribute('data-marriage-id')");
    });

    it("ignores birth/adoption edges (keys on marriage, not bare data-marriage-id)", () => {
        // Birth/adoption edges carry data-marriage-id too. The predicate
        // must select on data-link-kind="marriage" so those stay inert —
        // there must be no closest("[data-marriage-id]") selector.
        expect(build()).not.toContain("closest('[data-marriage-id]')");
        expect(build()).not.toContain('closest("[data-marriage-id]")');
    });
});

describe("previewHtml selection sync", () => {
    it("handles the highlightEntity message", () => {
        const html = build();
        expect(html).toContain("msg.type === 'highlightEntity'");
        expect(html).toContain("highlightEntity(msg.id, msg.kind)");
    });

    it("clears every prior .kul-selected before applying (stateless)", () => {
        const html = build();
        expect(html).toContain("querySelectorAll('.kul-selected')");
        expect(html).toContain("classList.remove('kul-selected')");
    });

    it("treats a null id as clear-only", () => {
        // highlightEntity returns after clearing when id is falsy, so a
        // { id: null } message strips the highlight without re-applying.
        expect(build()).toContain("if (!id) { return; }");
    });

    it("selects persons by data-person-id and marriages by link-kind + id", () => {
        const html = build();
        expect(html).toContain(
            '[data-link-kind="marriage"][data-marriage-id="\' + id + \'"]',
        );
        expect(html).toContain('[data-person-id="\' + id + \'"]');
        expect(html).toContain("classList.add('kul-selected')");
    });

    it("pans (translate only) to centre the matched element", () => {
        const html = build();
        // Centering reads the live viewport via getSizes()+getBBox() and
        // calls panZoom.pan(...) — never zoom.
        expect(html).toContain("panZoom.getSizes()");
        expect(html).toContain("getBBox()");
        expect(html).toContain("panZoom.pan(");
        expect(html).toContain("panToElement(el)");
    });

    it("eases the centring pan over rAF rather than snapping", () => {
        const html = build();
        // The centring tween is requestAnimationFrame-driven (like the
        // keyboard pan) and cancels the prior tween so rapid cursor moves
        // chase the latest target without stacking.
        expect(html).toContain("requestAnimationFrame(step)");
        expect(html).toContain("cancelPanAnim()");
        expect(html).toContain("performance.now()");
    });
});
