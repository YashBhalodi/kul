import { describe, expect, it } from "vitest";

import { buildTooltip, getNonce, previewHtml } from "./preview-html";

/** Shorthand for the `{ name, value }` attribute shape buildTooltip takes. */
function attr(name: string, value: string): { name: string; value: string } {
    return { name, value };
}

/** An `id → name` resolver backed by a map, falling back to the id. */
function namer(map: Record<string, string> = {}): (id: string) => string {
    return (id) => map[id] ?? id;
}

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

/** Convenience: the rows of a built model (asserting it isn't null). */
function rowsOf(
    attrs: Array<{ name: string; value: string }>,
    resolve = namer(),
): Array<{ label: string; value: string }> {
    const model = buildTooltip(attrs, resolve);
    expect(model).not.toBeNull();
    return model!.rows;
}

describe("buildTooltip typed header", () => {
    it("titles a person card and resolves its display name", () => {
        const model = buildTooltip(
            [attr("data-person-id", "alice"), attr("data-gender", "female")],
            namer({ alice: "Alice Adeyemi" }),
        );
        expect(model?.title).toBe("Person");
        expect(model?.identity).toBe("Alice Adeyemi");
    });

    it("titles a marriage edge and joins both spouse names", () => {
        const model = buildTooltip(
            [
                attr("data-link-kind", "marriage"),
                attr("data-host-id", "a"),
                attr("data-joining-id", "b"),
                attr("data-start", "1962"),
            ],
            namer({ a: "Babatunde Adeyemi", b: "Amaka Adeyemi" }),
        );
        expect(model?.title).toBe("Marriage");
        expect(model?.identity).toBe("Babatunde Adeyemi & Amaka Adeyemi");
    });

    it("titles an adoption edge and resolves the child name", () => {
        const model = buildTooltip(
            [
                attr("data-link-kind", "adoption"),
                attr("data-child-id", "c"),
                attr("data-adoption-start", "1990"),
            ],
            namer({ c: "Bisi Adeyemi" }),
        );
        expect(model?.title).toBe("Adoption");
        expect(model?.identity).toBe("Bisi Adeyemi");
    });

    it("falls back to the id when a name can't be resolved", () => {
        const model = buildTooltip(
            [attr("data-person-id", "ghost42")],
            namer(),
        );
        expect(model?.identity).toBe("ghost42");
    });

    it("returns null for a birth edge (purely structural)", () => {
        expect(
            buildTooltip(
                [
                    attr("data-link-kind", "birth"),
                    attr("data-child-id", "c"),
                    attr("data-is-past", "true"),
                ],
                namer({ c: "Bisi Adeyemi" }),
            ),
        ).toBeNull();
    });

    it("returns null for an element that is neither a card nor a known edge", () => {
        expect(buildTooltip([attr("class", "kul-canvas")], namer())).toBeNull();
    });
});

describe("buildTooltip rows — denylist", () => {
    it("omits the structural attributes, keeping only display fields", () => {
        // A canonical person card carries identity/layout/styling attributes
        // alongside its display fields; only the latter become rows.
        const rows = rowsOf([
            attr("data-person-id", "alice"),
            attr("data-kind", "canonical"),
            attr("data-gender", "female"),
            attr("data-is-alive", "true"),
            attr("data-born", "1850"),
            attr("data-generation", "0"),
        ]);
        expect(rows).toEqual([
            { label: "Gender", value: "Female" },
            { label: "Born", value: "1850" },
        ]);
    });

    it("ignores non-data-* attributes entirely", () => {
        const rows = rowsOf([
            attr("data-person-id", "p"),
            attr("class", "kul-card"),
            attr("transform", "translate(10,20)"),
            attr("data-given", "Alice"),
        ]);
        expect(rows).toEqual([{ label: "Given name", value: "Alice" }]);
    });

    it("omits empty values (no placeholder rows)", () => {
        const rows = rowsOf([
            attr("data-person-id", "p"),
            attr("data-born", "1850"),
            attr("data-died", ""),
            attr("data-family", ""),
        ]);
        expect(rows).toEqual([{ label: "Born", value: "1850" }]);
    });
});

describe("buildTooltip rows — scope and order", () => {
    it("surfaces a person's non-empty fields in DOM (emit) order", () => {
        const rows = rowsOf([
            attr("data-person-id", "p"),
            attr("data-kind", "canonical"),
            attr("data-gender", "male"),
            attr("data-is-alive", "false"),
            attr("data-born", "1820"),
            attr("data-died", "1890"),
            attr("data-family", "Curie"),
            attr("data-given", "Pierre"),
            attr("data-generation", "0"),
        ]);
        expect(rows).toEqual([
            { label: "Gender", value: "Male" },
            { label: "Born", value: "1820" },
            { label: "Died", value: "1890" },
            { label: "Family name", value: "Curie" },
            { label: "Given name", value: "Pierre" },
        ]);
    });

    it("surfaces a marriage edge's start, end, and end-reason", () => {
        const rows = rowsOf([
            attr("data-marriage-id", "m1"),
            attr("data-link-kind", "marriage"),
            attr("data-host-id", "a"),
            attr("data-joining-id", "b"),
            attr("data-start", "1870"),
            attr("data-is-ended", "true"),
            attr("data-end", "1885"),
            attr("data-end-reason", "divorce"),
        ]);
        expect(rows).toEqual([
            { label: "Start", value: "1870" },
            { label: "End", value: "1885" },
            { label: "End reason", value: "Divorce" },
        ]);
    });

    it("surfaces an adoption edge's adoption start/end", () => {
        const rows = rowsOf([
            attr("data-marriage-id", "m1"),
            attr("data-link-kind", "adoption"),
            attr("data-child-id", "c"),
            attr("data-is-past", "false"),
            attr("data-adoption-start", "1900"),
            attr("data-adoption-end", "1905"),
        ]);
        expect(rows).toEqual([
            { label: "Adoption start", value: "1900" },
            { label: "Adoption end", value: "1905" },
        ]);
    });
});

describe("buildTooltip rows — label humanization", () => {
    it("strips data-, turns - into space, and capitalizes", () => {
        expect(
            rowsOf([attr("data-link-kind", "marriage"), attr("data-end-reason", "x")])[0]
                .label,
        ).toBe("End reason");
        expect(
            rowsOf([
                attr("data-link-kind", "adoption"),
                attr("data-adoption-start", "x"),
            ])[0].label,
        ).toBe("Adoption start");
    });

    it("applies the family/given override map", () => {
        expect(
            rowsOf([attr("data-person-id", "p"), attr("data-family", "x")])[0].label,
        ).toBe("Family name");
        expect(
            rowsOf([attr("data-person-id", "p"), attr("data-given", "x")])[0].label,
        ).toBe("Given name");
    });
});

describe("buildTooltip rows — value capitalization", () => {
    it("capitalizes the first letter of a worded value", () => {
        expect(
            rowsOf([attr("data-person-id", "p"), attr("data-gender", "male")])[0].value,
        ).toBe("Male");
        expect(
            rowsOf([
                attr("data-link-kind", "marriage"),
                attr("data-end-reason", "divorce"),
            ])[0].value,
        ).toBe("Divorce");
    });

    it("passes dates through verbatim, preserving the ~ approximate marker", () => {
        expect(
            rowsOf([attr("data-person-id", "p"), attr("data-born", "1850")])[0].value,
        ).toBe("1850");
        expect(
            rowsOf([attr("data-person-id", "p"), attr("data-died", "~1890")])[0].value,
        ).toBe("~1890");
    });
});

describe("previewHtml hover tooltip", () => {
    it("embeds the shared buildTooltip logic in the bootstrap", () => {
        // The webview runs the exact same content-builder the tests cover, so
        // its source is serialized into the bootstrap rather than reimplemented.
        expect(build()).toContain("function buildTooltip(");
    });

    it("binds the embedded builder to a const so minify-renaming can't break the call", () => {
        // The production esbuild --minify pass renames the internal function,
        // so a bare `function NAME(){}` declaration would no longer match the
        // `buildTooltip(...)` call site (ReferenceError). Binding the
        // serialized source to `const buildTooltip = …` fixes the callable
        // name regardless of how the body was minified.
        expect(build()).toContain("const buildTooltip = function");
    });

    it("delegates hover on #root via mouseover/mouseout", () => {
        const html = build();
        expect(html).toContain("root.addEventListener('mouseover'");
        expect(html).toContain("root.addEventListener('mouseout'");
        expect(html).toContain("closest('.kul-card, .kul-edge')");
    });

    it("resolves names off the card labels for the typed header", () => {
        const html = build();
        // The person/spouse/child names come from the rendered .kul-label-name
        // text (not a data-* attribute), resolved by id.
        expect(html).toContain("function resolveName(id)");
        expect(html).toContain(".kul-label-name");
        expect(html).toContain("buildTooltip(attrs, resolveName)");
    });

    it("builds a floating .kul-tooltip div with a typed header", () => {
        const html = build();
        expect(html).toContain("'kul-tooltip'");
        expect(html).toContain("'kul-tooltip-header'");
        expect(html).toContain("'kul-tooltip-kind'");
        expect(html).toContain("getBoundingClientRect()");
        expect(html).toContain("document.body.appendChild(el)");
    });

    it("tears the tooltip down on re-render and on pan/zoom", () => {
        const html = build();
        expect(html).toContain("removeTooltip()");
        expect(html).toContain("onPan: removeTooltip");
        expect(html).toContain("onZoom: removeTooltip");
    });

    it("scales the tooltip with the diagram so it reads as inline", () => {
        const html = build();
        // The tooltip is sized in diagram units: it reads svg-pan-zoom's live
        // realZoom and applies it as a CSS scale, so it grows/shrinks with the
        // surrounding cards rather than staying a fixed screen overlay.
        expect(html).toContain("panZoom.getSizes");
        expect(html).toContain("sizes.realZoom");
        expect(html).toContain("'scale('");
    });
});
