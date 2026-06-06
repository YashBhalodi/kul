import { describe, expect, it } from "vitest";

import {
    buildTooltip,
    getNonce,
    LEGEND_ROWS,
    legendSwatchInnerSvg,
    presentLegendRows,
    previewHtml,
} from "./preview-html";

function attr(name: string, value: string): { name: string; value: string } {
    return { name, value };
}

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

    it("keeps 'unsafe-inline' on style-src", () => {
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

    it("adds a vertical divider between pan/zoom and the legend toggle", () => {
        const html = build();
        expect(html).toContain('class="kul-control-divider"');
        const zoomOutIdx = html.indexOf('data-action="zoom-out"');
        const dividerIdx = html.indexOf('class="kul-control-divider"');
        const toggleIdx = html.indexOf('data-action="toggle-legend"');
        expect(zoomOutIdx).toBeLessThan(dividerIdx);
        expect(dividerIdx).toBeLessThan(toggleIdx);
    });

    it("includes a legend-toggle (ⓘ) button that starts unpressed", () => {
        const html = build();
        expect(html).toContain('data-action="toggle-legend"');
        expect(html).toMatch(
            /data-action="toggle-legend"[^>]*aria-pressed="false"/,
        );
    });
});

describe("previewHtml click-to-source", () => {
    it("acquires the VSCode API for posting messages", () => {
        expect(build()).toContain("acquireVsCodeApi()");
    });

    it("attaches a click listener on #root", () => {
        // #root survives every innerHTML swap; wire the listener there once.
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
        // Birth/adoption edges carry data-marriage-id too; the predicate
        // must key on data-link-kind="marriage" so those stay inert.
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
        expect(html).toContain("panZoom.getSizes()");
        expect(html).toContain("getBBox()");
        expect(html).toContain("panZoom.pan(");
        expect(html).toContain("panToElement(el)");
    });

    it("eases the centring pan over rAF rather than snapping", () => {
        const html = build();
        expect(html).toContain("requestAnimationFrame(step)");
        expect(html).toContain("cancelPanAnim()");
        expect(html).toContain("performance.now()");
    });
});

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

    it("omits the Start row when a marriage has no data-start (#198)", () => {
        const rows = rowsOf([
            attr("data-marriage-id", "m1"),
            attr("data-link-kind", "marriage"),
            attr("data-host-id", "a"),
            attr("data-joining-id", "b"),
            attr("data-is-ended", "false"),
        ]);
        expect(rows).toEqual([]);
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
        expect(build()).toContain("function buildTooltip(");
    });

    it("binds the embedded builder to a const so minify-renaming can't break the call", () => {
        // esbuild --minify renames the inner function, so binding the
        // serialized source to a const fixes the callable name.
        expect(build()).toContain("const buildTooltip = function");
    });

    it("delegates hover on #root via mouseover/mouseout", () => {
        const html = build();
        expect(html).toContain("root.addEventListener('mouseover'");
        expect(html).toContain("root.addEventListener('mouseout'");
        expect(html).toContain("closest('.kul-card, .kul-edge')");
    });

    it("delays the reveal on a hover-intent timer, cancellable before it fires", () => {
        const html = build();
        expect(html).toContain("const HOVER_DELAY_MS =");
        expect(html).toContain("hoverTimer = setTimeout(");
        expect(html).toContain("clearTimeout(hoverTimer)");
    });

    it("resolves names off the card labels for the typed header", () => {
        const html = build();
        expect(html).toContain("function resolveName(id)");
        expect(html).toContain(".kul-label-name");
        expect(html).toContain("buildTooltip(attrs, resolveName)");
    });

    it("builds a floating .kul-tooltip div with a typed header and field grid", () => {
        const html = build();
        expect(html).toContain("'kul-tooltip'");
        expect(html).toContain("'kul-tooltip-header'");
        expect(html).toContain("'kul-tooltip-kind'");
        expect(html).toContain("'kul-tooltip-fields'");
        expect(html).toContain("getBoundingClientRect()");
        expect(html).toContain("document.body.appendChild(el)");
    });

    it("tears the tooltip down on re-render and on pan/zoom", () => {
        const html = build();
        expect(html).toContain("removeTooltip()");
        expect(html).toContain("onPan: removeTooltip");
        expect(html).toContain("onZoom: removeTooltip");
    });

    it("scales the tooltip with the diagram, capped at the high end", () => {
        const html = build();
        expect(html).toContain("panZoom.getSizes");
        expect(html).toContain("sizes.realZoom");
        expect(html).toContain("const MAX_TOOLTIP_SCALE =");
        expect(html).toContain("'scale(' + tooltipScale()");
    });
});

describe("LEGEND_ROWS normative table", () => {
    it("lists the canonical eight rows in normative order", () => {
        expect(LEGEND_ROWS.map((r) => r.key)).toEqual([
            "gender-male",
            "gender-female",
            "gender-other",
            "past-record",
            "birth",
            "adoption",
            "marriage",
            "ended-marriage",
        ]);
    });

    it("uses the normative English label strings", () => {
        expect(LEGEND_ROWS.map((r) => r.label)).toEqual([
            "Male",
            "Female",
            "Other",
            "Past record",
            "Birth",
            "Adoption",
            "Marriage",
            "Ended marriage",
        ]);
    });

    it("keys each row on the production data-* attribute (the seam, not a new vocabulary)", () => {
        // Selectors target the same `data-*` attributes the live SVG carries
        // (ADR-0021), so presence tests read the same contract as the diagram.
        const map = Object.fromEntries(
            LEGEND_ROWS.map((r) => [r.key, r.presenceSelector]),
        );
        expect(map["gender-male"]).toContain('data-gender="male"');
        expect(map["gender-female"]).toContain('data-gender="female"');
        expect(map["gender-other"]).toContain('data-gender="other"');
        expect(map["past-record"]).toContain('data-kind="ghost"');
        expect(map["birth"]).toContain('data-link-kind="birth"');
        expect(map["adoption"]).toContain('data-link-kind="adoption"');
        expect(map["marriage"]).toContain('data-link-kind="marriage"');
        expect(map["marriage"]).toContain(':not([data-is-ended="true"])');
        expect(map["ended-marriage"]).toContain('data-link-kind="marriage"');
        expect(map["ended-marriage"]).toContain('data-is-ended="true"');
    });
});

describe("presentLegendRows dynamic presence", () => {
    function fakeHas(present: ReadonlyArray<string>): (selector: string) => unknown {
        const set = new Set(present);
        return (selector) => (set.has(selector) ? {} : null);
    }

    it("returns every row when every category is present, in canonical order", () => {
        const allSelectors = LEGEND_ROWS.map((r) => r.presenceSelector);
        const rows = presentLegendRows(fakeHas(allSelectors));
        expect(rows.map((r) => r.key)).toEqual(LEGEND_ROWS.map((r) => r.key));
    });

    it("returns the empty list when no category is present", () => {
        expect(presentLegendRows(fakeHas([])).length).toBe(0);
    });

    it("filters to only the present categories (no adoption → no Adoption row)", () => {
        const present = [
            '.kul-card[data-gender="male"]',
            '.kul-card[data-gender="female"]',
            '.kul-edge[data-link-kind="birth"]',
            '.kul-edge[data-link-kind="marriage"]:not([data-is-ended="true"])',
        ];
        const rows = presentLegendRows(fakeHas(present));
        expect(rows.map((r) => r.key)).toEqual([
            "gender-male",
            "gender-female",
            "birth",
            "marriage",
        ]);
    });

    it("shows only Ended marriage when the only marriage in the diagram is ended", () => {
        const present = [
            '.kul-card[data-gender="male"]',
            '.kul-card[data-gender="female"]',
            '.kul-edge[data-link-kind="marriage"][data-is-ended="true"]',
        ];
        const rows = presentLegendRows(fakeHas(present));
        expect(rows.map((r) => r.key)).toEqual([
            "gender-male",
            "gender-female",
            "ended-marriage",
        ]);
    });
});

describe("legendSwatchInnerSvg", () => {
    it("emits a card swatch reusing the production class + data-* per gender", () => {
        expect(legendSwatchInnerSvg("gender-male")).toContain(
            'class="kul-card" data-kind="canonical" data-gender="male"',
        );
        expect(legendSwatchInnerSvg("gender-female")).toContain(
            'data-gender="female"',
        );
        expect(legendSwatchInnerSvg("gender-other")).toContain(
            'data-gender="other"',
        );
    });

    it("emits a ghost swatch with the inline structural dashed border (mirrors production)", () => {
        const svg = legendSwatchInnerSvg("past-record");
        expect(svg).toContain('class="kul-card" data-kind="ghost"');
        expect(svg).toContain('stroke-dasharray="3 2"');
    });

    it("emits edge swatches reusing the production class + data-link-kind", () => {
        expect(legendSwatchInnerSvg("birth")).toContain(
            'class="kul-edge" data-link-kind="birth"',
        );
        const adoption = legendSwatchInnerSvg("adoption");
        expect(adoption).toContain('data-link-kind="adoption"');
        // ADR-0016: structural dasharrays ship inline.
        expect(adoption).toContain('stroke-dasharray="6 4"');
        expect(legendSwatchInnerSvg("marriage")).toContain(
            'class="kul-edge" data-link-kind="marriage"',
        );
        const ended = legendSwatchInnerSvg("ended-marriage");
        expect(ended).toContain('data-link-kind="marriage"');
        expect(ended).toContain('data-is-ended="true"');
    });

    it("never bakes a colour into a swatch (no fill/stroke= attributes beyond fill=none)", () => {
        // ADR-0022: colour comes from the stylesheet via the data-* seam.
        for (const row of LEGEND_ROWS) {
            const svg = legendSwatchInnerSvg(row.key);
            const stripped = svg.replace(/fill="none"/g, "");
            expect(stripped).not.toContain(' fill="');
            expect(stripped).not.toContain(' stroke="');
        }
    });

    it("returns the empty string for an unknown key (defensive)", () => {
        expect(legendSwatchInnerSvg("not-a-real-row")).toBe("");
    });
});

describe("previewHtml renderError last-good persistence (#203)", () => {
    it("does NOT wipe #root in the renderError branch", () => {
        // The pre-#203 implementation called root.innerHTML = '' before
        // appending a full-pane banner; the new behavior keeps the last-good
        // SVG mounted. Guard against the regression by asserting that NO
        // root.innerHTML wipe and NO kul-error-banner survive anywhere — the
        // render-success branch's `root.innerHTML = msg.svg` is the only
        // legitimate place innerHTML appears, and that's not a wipe.
        const html = build();
        expect(html).not.toContain("root.innerHTML = ''");
        expect(html).not.toContain('root.innerHTML = ""');
        expect(html).not.toContain("kul-error-banner");
    });

    it("applies kul-render-stale to the last-good SVG on renderError", () => {
        const html = build();
        expect(html).toContain("function setStaleSvg(");
        expect(html).toContain("kul-render-stale");
        expect(html).toMatch(/setStaleSvg\(true\)/);
    });

    it("clears the error state on a successful render", () => {
        // The render handler calls setErrors([]) so a previously-stuck
        // error button drops away the moment the document parses again.
        const html = build();
        expect(html).toContain("setErrors([])");
    });
});

describe("previewHtml error button (#203)", () => {
    it("renders the error button inside kul-controls, hidden by default", () => {
        const html = build();
        expect(html).toContain('id="kul-error-button"');
        expect(html).toContain('data-action="toggle-errors"');
        expect(html).toMatch(/id="kul-error-button"[^>]*hidden/);
    });

    it("ships an error-count badge slot inside the button", () => {
        const html = build();
        expect(html).toContain('class="kul-error-count"');
    });

    it("wraps the pan/zoom + legend controls in a sub-group", () => {
        // Wrapping lets the pan/zoom group hide independently of the error
        // button — first-open with errors shows only the error icon.
        const html = build();
        expect(html).toContain('id="kul-controls-group"');
        const groupIdx = html.indexOf('id="kul-controls-group"');
        const zoomInIdx = html.indexOf('data-action="zoom-in"');
        const toggleIdx = html.indexOf('data-action="toggle-legend"');
        const errorBtnIdx = html.indexOf('id="kul-error-button"');
        expect(groupIdx).toBeLessThan(zoomInIdx);
        expect(zoomInIdx).toBeLessThan(toggleIdx);
        expect(toggleIdx).toBeLessThan(errorBtnIdx);
    });

    it("flips the popover via toggleErrors when the button is clicked", () => {
        const html = build();
        expect(html).toContain("action === 'toggle-errors'");
        expect(html).toContain("toggleErrors()");
        expect(html).toContain("errorsVisible = !errorsVisible");
    });

    it("reflects the toggle state into the button (aria-pressed + label)", () => {
        const html = build();
        expect(html).toContain("errorButton.setAttribute('aria-pressed'");
        expect(html).toContain("'Hide errors'");
        expect(html).toContain("'Show errors'");
    });

    it("reconciles panel visibility from render-state OR error presence", () => {
        // Once at least one render lands or one error fires, the panel is
        // visible — otherwise it stays hidden.
        const html = build();
        expect(html).toContain("function reconcileControlsVisibility()");
        expect(html).toContain("!hasRender && errors.length === 0");
    });
});

describe("previewHtml error popover (#203)", () => {
    it("renders the popover container as a sibling of #root, hidden initially", () => {
        const html = build();
        expect(html).toContain('id="kul-error-popover"');
        expect(html).toMatch(/id="kul-error-popover"[^>]*hidden/);
    });

    it("delegates row clicks on the popover (innerHTML-rebuildable rows)", () => {
        const html = build();
        expect(html).toContain("errorPopover.addEventListener('click'");
        expect(html).toContain(".kul-error-row[data-error-index]");
    });

    it("posts revealSource with uri + range for a clicked error row", () => {
        const html = build();
        // The webview ships the row's bound diagnostic back to the extension
        // verbatim — uri + LSP range, no entity id.
        expect(html).toContain("type: 'revealSource'");
        expect(html).toContain("uri: err.uri");
        expect(html).toContain("range: err.range");
    });

    it("escapes message/code/location HTML to keep the popover XSS-safe", () => {
        // Diagnostics arrive from the LSP; their message is author-controlled
        // by way of the source file. innerHTML assembly must escape <, >, &.
        const html = build();
        expect(html).toContain(".replace(/&/g, '&amp;')");
        expect(html).toContain(".replace(/</g, '&lt;')");
        expect(html).toContain(".replace(/>/g, '&gt;')");
    });

    it("renders rows with one-based line / column for display", () => {
        // LSP positions are zero-based; the popover surfaces them one-based
        // to match the Problems pane's convention.
        const html = build();
        expect(html).toContain("err.range.start.line + 1");
        expect(html).toContain("err.range.start.character + 1");
    });

    it("opens only when errors are present (toggleErrors is a no-op on empty)", () => {
        const html = build();
        expect(html).toContain("if (errors.length === 0) { return; }");
    });

    it("re-hides the popover whenever errors clears (next successful render)", () => {
        // setErrors([]) drops errorsVisible back to false so a subsequent
        // good render doesn't leave the popover stranded open.
        const html = build();
        expect(html).toMatch(/if \(errors\.length === 0\) \{ errorsVisible = false; \}/);
    });
});

describe("previewHtml chrome legend overlay", () => {
    it("renders the legend container as a sibling of #root (survives innerHTML swaps)", () => {
        const html = build();
        expect(html).toContain('id="kul-legend"');
        expect(html).toContain('class="kul-preview-legend"');
        expect(html).toMatch(/id="kul-legend"[^>]*hidden/);
    });

    it("embeds the normative LEGEND_ROWS table in the bootstrap", () => {
        const html = build();
        expect(html).toContain("const LEGEND_ROWS = ");
        expect(html).toContain('"Male"');
        expect(html).toContain('"Past record"');
        expect(html).toContain('"Ended marriage"');
    });

    it("embeds legendSwatchInnerSvg behind a stable const for minify safety", () => {
        expect(build()).toContain("const legendSwatchInnerSvg = function");
    });

    it("builds rows from the rendered SVG DOM via the same querySelectorAll seam", () => {
        const html = build();
        expect(html).toContain("renderLegend(svg)");
        expect(html).toContain("svgRoot.querySelector(row.presenceSelector)");
    });

    it("hides the legend on render error and on a missing <svg>", () => {
        const html = build();
        expect(html).toContain("hideLegend()");
        expect(html).toMatch(/if \(!svg\) \{ hideLegend\(\); reconcileControlsVisibility/);
    });

    it("tracks whether the current diagram has any legend content", () => {
        // Visibility gates on user toggle AND content presence, so clicking
        // ⓘ on an empty diagram is a no-op rather than a vacant reveal.
        expect(build()).toContain("legendHasContent");
    });

    it("starts hidden — the ⓘ toggle is the discovery affordance", () => {
        expect(build()).toContain("let legendVisible = false");
    });

    it("flips the toggle and re-applies visibility on a toggle-legend click", () => {
        const html = build();
        expect(html).toContain("action === 'toggle-legend'");
        expect(html).toContain("toggleLegend()");
        expect(html).toContain("legendVisible = !legendVisible");
        expect(html).toContain("applyLegendVisibility()");
    });

    it("reflects the open/closed state into the toggle button (aria-pressed + label)", () => {
        const html = build();
        expect(html).toContain("setAttribute('aria-pressed', String(shouldShow))");
        expect(html).toContain("'Hide legend'");
        expect(html).toContain("'Show legend'");
    });
});
