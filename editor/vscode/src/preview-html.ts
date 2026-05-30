// Pure webview-HTML generation for the Kul preview panel. No `vscode` import
// so the static HTML contract can be unit-tested under Vitest.

// 32-char nonce stamped on every `<script>` so the CSP can drop
// `'unsafe-inline'`. Standard VSCode webview pattern.
export function getNonce(): string {
    let text = "";
    const chars =
        "ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789";
    for (let i = 0; i < 32; i++) {
        text += chars.charAt(Math.floor(Math.random() * chars.length));
    }
    return text;
}

// `currentColor` lets icons track the button's themed `color` via a single
// `--kul-control-fg` token (ADR-0016).
const ICON_ZOOM_IN = `<svg viewBox="0 0 16 16" aria-hidden="true" focusable="false"><path d="M8 3.5v9M3.5 8h9" fill="none" stroke="currentColor" stroke-width="1.5" stroke-linecap="round"/></svg>`;
const ICON_ZOOM_OUT = `<svg viewBox="0 0 16 16" aria-hidden="true" focusable="false"><path d="M3.5 8h9" fill="none" stroke="currentColor" stroke-width="1.5" stroke-linecap="round"/></svg>`;
const ICON_RESET = `<svg viewBox="0 0 16 16" aria-hidden="true" focusable="false"><path d="M3 6V3.5A.5.5 0 0 1 3.5 3H6M10 3h2.5a.5.5 0 0 1 .5.5V6M13 10v2.5a.5.5 0 0 1-.5.5H10M6 13H3.5a.5.5 0 0 1-.5-.5V10" fill="none" stroke="currentColor" stroke-width="1.5" stroke-linecap="round" stroke-linejoin="round"/></svg>`;
const ICON_INFO = `<svg viewBox="0 0 16 16" aria-hidden="true" focusable="false"><circle cx="8" cy="8" r="6.25" fill="none" stroke="currentColor" stroke-width="1.5"/><circle cx="8" cy="4.75" r="0.85" fill="currentColor"/><path d="M8 7.25v4.5" fill="none" stroke="currentColor" stroke-width="1.5" stroke-linecap="round"/></svg>`;

// Sibling of #root (not a child) so the per-render `root.innerHTML = …` swap
// never wipes it. Hidden until the first successful render.
const CONTROLS = `<div id="kul-controls" class="kul-preview-controls" role="group" aria-label="Diagram view controls" hidden>
<button type="button" class="kul-control-btn" data-action="zoom-in" title="Zoom in" aria-label="Zoom in">${ICON_ZOOM_IN}</button>
<button type="button" class="kul-control-btn" data-action="reset" title="Reset view" aria-label="Reset view">${ICON_RESET}</button>
<button type="button" class="kul-control-btn" data-action="zoom-out" title="Zoom out" aria-label="Zoom out">${ICON_ZOOM_OUT}</button>
<span class="kul-control-divider" aria-hidden="true"></span>
<button type="button" class="kul-control-btn" data-action="toggle-legend" title="Show legend" aria-label="Show legend" aria-pressed="false">${ICON_INFO}</button>
</div>`;

// ADR-0022: sibling of #root, populated from the rendered SVG on each render.
const LEGEND = `<div id="kul-legend" class="kul-preview-legend" role="region" aria-label="Diagram legend" hidden></div>`;

export interface TooltipModel {
    /** Entity-kind kicker: "Person" / "Marriage" / "Adoption". */
    title: string;
    /** Display name (person), "A & B" (marriage), or child name (adoption);
     * empty when no name could be resolved. */
    identity: string;
    rows: Array<{ label: string; value: string }>;
}

/**
 * Single source of truth for tooltip content. Exported for Vitest AND embedded
 * verbatim into {@link BOOTSTRAP} via `.toString()`, so it must stay
 * self-contained: no module imports, no closure over module-level constants.
 * `resolveName` is caller-supplied (webview reads cards; tests pass a map).
 *
 * ADR-0021 / #156: every Person/Marriage/Adoption property rides the SVG as a
 * `data-*` attribute, so the tooltip reads them directly — no LSP round-trip.
 * Rows use a denylist of structural attrs so new display fields surface
 * automatically. Returns `null` for birth edges (purely structural) and
 * anything that is not a person card or marriage/adoption edge.
 */
export function buildTooltip(
    attrs: ReadonlyArray<{ name: string; value: string }>,
    resolveName: (id: string) => string,
): TooltipModel | null {
    // Structural attrs: identity / layout / styling, not user-facing fields.
    const DENYLIST = new Set([
        "data-person-id",
        "data-marriage-id",
        "data-host-id",
        "data-joining-id",
        "data-child-id",
        "data-kind",
        "data-link-kind",
        "data-generation",
        "data-ghost-reason",
        "data-is-alive",
        "data-is-ended",
        "data-is-past",
    ]);
    // "Family" / "Given" on their own read ambiguously.
    const LABEL_OVERRIDES: Record<string, string> = {
        family: "Family name",
        given: "Given name",
    };
    function get(name: string): string {
        for (const attr of attrs) {
            if (attr.name === name) {
                return attr.value;
            }
        }
        return "";
    }
    function has(name: string): boolean {
        for (const attr of attrs) {
            if (attr.name === name) {
                return true;
            }
        }
        return false;
    }
    // Capitalize the first cased character, leaving the rest (and any leading
    // non-letter like the `~` approximate marker) verbatim. Bare/approximate
    // dates pass through unchanged. `toLowerCase() !== toUpperCase()` is a
    // locale-robust "is cased letter" test.
    function displayValue(raw: string): string {
        for (let i = 0; i < raw.length; i++) {
            const ch = raw[i];
            if (ch.toLowerCase() !== ch.toUpperCase()) {
                return raw.slice(0, i) + ch.toUpperCase() + raw.slice(i + 1);
            }
        }
        return raw;
    }

    let title: string;
    let identity: string;
    if (has("data-person-id")) {
        title = "Person";
        // Display name lives on the card's rendered label, not a data-* attr.
        identity = resolveName(get("data-person-id"));
    } else {
        const linkKind = get("data-link-kind");
        if (linkKind === "marriage") {
            title = "Marriage";
            const host = resolveName(get("data-host-id"));
            const joining = resolveName(get("data-joining-id"));
            identity = host && joining ? host + " & " + joining : host || joining;
        } else if (linkKind === "adoption") {
            title = "Adoption";
            identity = resolveName(get("data-child-id"));
        } else {
            return null;
        }
    }

    const rows: Array<{ label: string; value: string }> = [];
    for (const attr of attrs) {
        const name = attr.name;
        const value = attr.value;
        if (name.indexOf("data-") !== 0) {
            continue;
        }
        if (DENYLIST.has(name) || value === "") {
            continue;
        }
        const key = name.slice("data-".length);
        let label = LABEL_OVERRIDES[key];
        if (!label) {
            const spaced = key.replace(/-/g, " ");
            label = spaced.charAt(0).toUpperCase() + spaced.slice(1);
        }
        rows.push({ label: label, value: displayValue(value) });
    }
    return { title: title, identity: identity, rows: rows };
}

export interface LegendRow {
    /** Stable key — surfaces on the row's `data-row` attribute. */
    key: string;
    /** Human label shown beside the swatch. English only (ADR-0022). */
    label: string;
    /** CSS selector tested against the rendered SVG; only present categories
     * yield a row. */
    presenceSelector: string;
}

/**
 * Normative legend table (ADR-0022). The chrome legend (here) and the CLI
 * baked legend (`crates/kul-svg/src/emit.rs`) share order + label strings;
 * `docs/canonical-ui-pattern.md` is the normative spec. Adding a category is
 * a same-PR change across this table, the kul-svg `LegendRow` enum, and the
 * canonical pattern doc.
 */
export const LEGEND_ROWS: ReadonlyArray<LegendRow> = [
    {
        key: "gender-male",
        label: "Male",
        presenceSelector: '.kul-card[data-gender="male"]',
    },
    {
        key: "gender-female",
        label: "Female",
        presenceSelector: '.kul-card[data-gender="female"]',
    },
    {
        key: "gender-other",
        label: "Other",
        presenceSelector: '.kul-card[data-gender="other"]',
    },
    {
        key: "past-record",
        label: "Past record",
        presenceSelector: '.kul-card[data-kind="ghost"]',
    },
    {
        key: "birth",
        label: "Birth",
        presenceSelector: '.kul-edge[data-link-kind="birth"]',
    },
    {
        key: "adoption",
        label: "Adoption",
        presenceSelector: '.kul-edge[data-link-kind="adoption"]',
    },
    {
        key: "marriage",
        label: "Marriage",
        // Un-ended only; ended marriages get their own row below.
        presenceSelector:
            '.kul-edge[data-link-kind="marriage"]:not([data-is-ended="true"])',
    },
    {
        key: "ended-marriage",
        label: "Ended marriage",
        presenceSelector:
            '.kul-edge[data-link-kind="marriage"][data-is-ended="true"]',
    },
];

/**
 * Filter {@link LEGEND_ROWS} to categories present in the diagram. Takes a
 * `querySelector`-like predicate so it stays DOM-free and unit testable.
 */
export function presentLegendRows(
    querySelector: (selector: string) => unknown,
): ReadonlyArray<LegendRow> {
    return LEGEND_ROWS.filter(
        (row) => querySelector(row.presenceSelector) != null,
    );
}

/**
 * Inline-SVG markup for one swatch: a miniature of the real glyph carrying the
 * production class + `data-*` attributes so the stylesheet themes it for free
 * (ADR-0022). Only structural inline attrs (ghost dashed border, adoption
 * dashed edge) ship here; colour/stroke-width come from CSS. Empty string for
 * an unknown key.
 */
export function legendSwatchInnerSvg(key: string): string {
    switch (key) {
        case "gender-male":
        case "gender-female":
        case "gender-other": {
            const gender = key.substring("gender-".length);
            return (
                '<g class="kul-card" data-kind="canonical" data-gender="' +
                gender +
                '"><rect x="0.75" y="2" width="28.5" height="14" rx="3" ry="3"/></g>'
            );
        }
        case "past-record":
            return (
                '<g class="kul-card" data-kind="ghost">' +
                '<rect x="0.75" y="2" width="28.5" height="14" rx="3" ry="3" stroke-dasharray="3 2"/></g>'
            );
        case "birth":
            return '<path class="kul-edge" data-link-kind="birth" fill="none" d="M 0 9 L 30 9"/>';
        case "adoption":
            return '<path class="kul-edge" data-link-kind="adoption" fill="none" d="M 0 9 L 30 9" stroke-dasharray="6 4"/>';
        case "marriage":
            return '<path class="kul-edge" data-link-kind="marriage" fill="none" d="M 0 9 L 30 9"/>';
        case "ended-marriage":
            return '<path class="kul-edge" data-link-kind="marriage" data-is-ended="true" fill="none" d="M 0 9 L 30 9"/>';
        default:
            return "";
    }
}

/**
 * Inline bootstrap that runs inside the webview. Owns a single module-level
 * `svg-pan-zoom` instance and reconciles it against `render` / `renderError`
 * messages: on `render`, it captures the live viewport, destroys the old
 * instance, swaps in the new SVG, and re-applies the saved viewport so live
 * edits don't yank the view back to fit. On `renderError`, it tears the
 * instance down so no stale pan/zoom surface survives behind the banner.
 */
const BOOTSTRAP = `
(function () {
    const root = document.getElementById('root');
    const controls = document.getElementById('kul-controls');
    const legend = document.getElementById('kul-legend');
    const vscode = acquireVsCodeApi();
    let panZoom = null;

    // Embedded verbatim from the exported source so webview and Vitest run
    // identical code. Bound to a const because esbuild --minify renames the
    // inner function — the const fixes the callable name.
    const buildTooltip = ${buildTooltip.toString()};

    // ADR-0022 normative table + swatch builder embedded; same minify guard.
    const LEGEND_ROWS = ${JSON.stringify(LEGEND_ROWS)};
    const legendSwatchInnerSvg = ${legendSwatchInnerSvg.toString()};

    // Click-to-source. Birth/adoption edges also carry data-marriage-id, so
    // keying on data-link-kind="marriage" (not the bare attr) keeps them
    // inert. Registered on #root, which survives every innerHTML swap.
    if (root) {
        root.addEventListener('click', function (event) {
            // Clicking a non-focusable SVG doesn't move focus off the text
            // editor; focus #root explicitly so the window keydown handler
            // receives arrows/+/-/0.
            root.focus();
            const person = event.target.closest('[data-person-id]');
            if (person) {
                vscode.postMessage({
                    type: 'revealSource',
                    id: person.getAttribute('data-person-id'),
                });
                return;
            }
            const marriage = event.target.closest('[data-link-kind="marriage"]');
            if (marriage) {
                const id = marriage.getAttribute('data-marriage-id');
                if (id) {
                    vscode.postMessage({ type: 'revealSource', id: id });
                }
            }
        });
    }

    // A single floating <div> is reused, anchored to the hovered card/edge
    // and torn down on mouseleave, re-render, and pan/zoom so it never
    // strands. Hover-intent delay (350ms tracks VSCode's editor-hover feel)
    // avoids flashing on pointer pass-through; any leave/switch/pan/render
    // routes through removeTooltip, which clears the pending timer.
    const HOVER_DELAY_MS = 350;
    let hoverTarget = null;
    let hoverTimer = null;
    let tooltipEl = null;
    function removeTooltip() {
        if (hoverTimer !== null) {
            clearTimeout(hoverTimer);
            hoverTimer = null;
        }
        if (tooltipEl) {
            tooltipEl.remove();
            tooltipEl = null;
        }
        hoverTarget = null;
    }
    // Live diagram user-unit→pixel zoom, capped so very high zoom doesn't
    // balloon the tooltip; shrinks freely below 1× when zoomed out.
    const MAX_TOOLTIP_SCALE = 1.5;
    function tooltipScale() {
        const sizes = panZoom && panZoom.getSizes ? panZoom.getSizes() : null;
        const z = sizes && sizes.realZoom > 0 ? sizes.realZoom : 1;
        return z < MAX_TOOLTIP_SCALE ? z : MAX_TOOLTIP_SCALE;
    }
    // Anchor below the element's left edge, clamp to viewport, flip above if
    // the panel would overflow the bottom.
    function positionTooltip(target) {
        if (!tooltipEl || typeof target.getBoundingClientRect !== 'function') {
            return;
        }
        const anchor = target.getBoundingClientRect();
        const tip = tooltipEl.getBoundingClientRect();
        // Gap scales with the diagram so it stays proportional at any zoom.
        const GAP = 8 * tooltipScale();
        const MARGIN = 4;
        let left = anchor.left;
        let top = anchor.bottom + GAP;
        if (left + tip.width > window.innerWidth - MARGIN) {
            left = window.innerWidth - MARGIN - tip.width;
        }
        if (left < MARGIN) { left = MARGIN; }
        if (top + tip.height > window.innerHeight - MARGIN) {
            top = anchor.top - GAP - tip.height;
        }
        if (top < MARGIN) { top = MARGIN; }
        tooltipEl.style.left = left + 'px';
        tooltipEl.style.top = top + 'px';
    }
    // The display name lives on the card's rendered <text class="kul-label-name">,
    // not a data-* attr. Falls back to raw id if the card is absent.
    function resolveName(id) {
        if (!id || !root) { return id || ''; }
        const card = root.querySelector('[data-person-id="' + id + '"]');
        if (!card) { return id; }
        const label = card.querySelector('.kul-label-name');
        return label && label.textContent ? label.textContent : id;
    }
    function showTooltip(target) {
        const attrs = [];
        for (let i = 0; i < target.attributes.length; i++) {
            const a = target.attributes[i];
            attrs.push({ name: a.name, value: a.value });
        }
        const model = buildTooltip(attrs, resolveName);
        if (!model) { return; }
        const el = document.createElement('div');
        el.className = 'kul-tooltip';
        el.setAttribute('role', 'tooltip');
        // Treat px metrics as diagram user-units so the tooltip reads as part
        // of the drawing rather than a fixed-size screen overlay.
        el.style.transformOrigin = 'top left';
        el.style.transform = 'scale(' + tooltipScale() + ')';
        const header = document.createElement('div');
        header.className = 'kul-tooltip-header';
        const kind = document.createElement('span');
        kind.className = 'kul-tooltip-kind';
        kind.textContent = model.title;
        header.appendChild(kind);
        if (model.identity) {
            const name = document.createElement('span');
            name.className = 'kul-tooltip-title';
            name.textContent = model.identity;
            header.appendChild(name);
        }
        el.appendChild(header);
        // Cells append directly to a single two-column grid (no per-row
        // wrapper) so labels and values share one track sizing.
        if (model.rows.length) {
            const fields = document.createElement('div');
            fields.className = 'kul-tooltip-fields';
            model.rows.forEach(function (row) {
                const label = document.createElement('span');
                label.className = 'kul-tooltip-label';
                label.textContent = row.label;
                const value = document.createElement('span');
                value.className = 'kul-tooltip-value';
                value.textContent = row.value;
                fields.appendChild(label);
                fields.appendChild(value);
            });
            el.appendChild(fields);
        }
        document.body.appendChild(el);
        tooltipEl = el;
        positionTooltip(target);
    }
    if (root) {
        // Delegated on #root via bubbling. hoverTarget skips rebuilds while
        // moving within the same entity; mouseout only clears when the pointer
        // leaves it (relatedTarget outside), not when crossing children.
        root.addEventListener('mouseover', function (event) {
            const entity = event.target.closest('.kul-card, .kul-edge');
            if (entity === hoverTarget) { return; }
            removeTooltip();
            if (entity) {
                hoverTarget = entity;
                hoverTimer = setTimeout(function () {
                    hoverTimer = null;
                    showTooltip(entity);
                }, HOVER_DELAY_MS);
            }
        });
        root.addEventListener('mouseout', function (event) {
            const to = event.relatedTarget;
            if (hoverTarget && (!to || !hoverTarget.contains(to))) {
                removeTooltip();
            }
        });
    }

    // Handle for the in-flight centring tween, so a new highlight or teardown
    // can cancel it before it fights the new target / destroyed instance.
    let panAnimRaf = null;
    function cancelPanAnim() {
        if (panAnimRaf !== null) {
            cancelAnimationFrame(panAnimRaf);
            panAnimRaf = null;
        }
    }

    function teardown() {
        cancelPanAnim();
        if (panZoom) {
            panZoom.destroy();
            panZoom = null;
        }
    }

    // Selection-sync highlighting is stateless: every message strips
    // .kul-selected from prior matches, then re-applies it.
    function clearHighlight() {
        root.querySelectorAll('.kul-selected').forEach(function (el) {
            el.classList.remove('kul-selected');
        });
    }

    // Translate only — never zoom. svg-pan-zoom maps a user-coord point to
    // viewport pixels as pan + realZoom*point, so the pan that centres a
    // bbox is width/2 - centre*realZoom. Eased over rAF so rapid cursor
    // moves chase the latest target without snapping or stacking.
    const PAN_ANIM_MS = 500;
    function panToElement(el) {
        if (!panZoom || typeof el.getBBox !== 'function') { return; }
        const bbox = el.getBBox();
        if (!bbox || (bbox.width === 0 && bbox.height === 0)) { return; }
        const sizes = panZoom.getSizes();
        const realZoom = sizes.realZoom;
        const cx = bbox.x + bbox.width / 2;
        const cy = bbox.y + bbox.height / 2;
        const targetX = sizes.width / 2 - cx * realZoom;
        const targetY = sizes.height / 2 - cy * realZoom;
        cancelPanAnim();
        const start = panZoom.getPan();
        const dx = targetX - start.x;
        const dy = targetY - start.y;
        if (Math.abs(dx) < 0.5 && Math.abs(dy) < 0.5) {
            panZoom.pan({ x: targetX, y: targetY });
            return;
        }
        const startTime = performance.now();
        function step(now) {
            if (!panZoom) { panAnimRaf = null; return; }
            const t = Math.min(1, (now - startTime) / PAN_ANIM_MS);
            // ease-out cubic.
            const eased = 1 - Math.pow(1 - t, 3);
            panZoom.pan({ x: start.x + dx * eased, y: start.y + dy * eased });
            panAnimRaf = t < 1 ? requestAnimationFrame(step) : null;
        }
        panAnimRaf = requestAnimationFrame(step);
    }

    function highlightEntity(id, kind) {
        clearHighlight();
        if (!id) { return; }
        const selector = kind === 'marriage'
            ? '[data-link-kind="marriage"][data-marriage-id="' + id + '"]'
            : '[data-person-id="' + id + '"]';
        const el = root.querySelector(selector);
        if (!el) { return; }
        el.classList.add('kul-selected');
        panToElement(el);
    }

    function showControls(visible) {
        if (controls) { controls.hidden = !visible; }
    }

    // ADR-0016: CSS cannot generate an SVG element, so the surface draws the
    // ghost ↺ badge. createElementNS is required — document.createElement
    // yields an inert HTML element that won't render inside <svg>.
    function injectGhostBadges(svgRoot) {
        const SVG_NS = 'http://www.w3.org/2000/svg';
        const ghosts = svgRoot.querySelectorAll('.kul-card[data-kind="ghost"]');
        ghosts.forEach(function (card) {
            const rect = card.querySelector('rect');
            if (!rect) { return; }
            const x = parseFloat(rect.getAttribute('x'));
            const y = parseFloat(rect.getAttribute('y'));
            const w = parseFloat(rect.getAttribute('width'));
            if (!isFinite(x) || !isFinite(y) || !isFinite(w)) { return; }
            const badge = document.createElementNS(SVG_NS, 'text');
            badge.setAttribute('class', 'kul-ghost-badge');
            badge.setAttribute('x', String(x + w - 12));
            badge.setAttribute('y', String(y + 14));
            badge.setAttribute('text-anchor', 'middle');
            badge.textContent = '↺';
            card.appendChild(badge);
        });
    }

    // Two-step lifecycle (ADR-0022): renderLegend(svg) rebuilds the row list
    // from the SVG DOM on each render; applyLegendVisibility() reconciles
    // legend.hidden from the user's toggle state AND content presence.
    // hideLegend() clears the panel on error but preserves legendVisible.
    let legendVisible = false;
    let legendHasContent = false;
    function renderLegend(svgRoot) {
        if (!legend) { return; }
        const present = LEGEND_ROWS.filter(function (row) {
            return svgRoot.querySelector(row.presenceSelector) !== null;
        });
        legendHasContent = present.length > 0;
        if (!legendHasContent) {
            legend.innerHTML = '';
        } else {
            legend.innerHTML = present.map(function (row) {
                return '<div class="kul-legend-row" data-row="' + row.key + '">' +
                    '<svg class="kul-legend-swatch" viewBox="0 0 30 18" aria-hidden="true">' +
                    legendSwatchInnerSvg(row.key) +
                    '</svg>' +
                    '<span class="kul-legend-label">' + row.label + '</span>' +
                    '</div>';
            }).join('');
        }
        applyLegendVisibility();
    }
    function applyLegendVisibility() {
        if (!legend) { return; }
        const shouldShow = legendVisible && legendHasContent;
        legend.hidden = !shouldShow;
        const toggle = document.querySelector('button[data-action="toggle-legend"]');
        if (toggle) {
            toggle.setAttribute('aria-pressed', String(shouldShow));
            const labelText = shouldShow ? 'Hide legend' : 'Show legend';
            toggle.setAttribute('aria-label', labelText);
            toggle.setAttribute('title', labelText);
        }
    }
    function toggleLegend() {
        legendVisible = !legendVisible;
        applyLegendVisibility();
    }
    function hideLegend() {
        if (legend) {
            legendHasContent = false;
            legend.innerHTML = '';
            legend.hidden = true;
        }
        applyLegendVisibility();
    }

    if (controls) {
        controls.addEventListener('click', function (event) {
            const btn = event.target.closest('button[data-action]');
            if (!btn) { return; }
            const action = btn.getAttribute('data-action');
            // Toggle works before the first render; pan/zoom actions require it.
            if (action === 'toggle-legend') { toggleLegend(); return; }
            if (!panZoom) { return; }
            if (action === 'zoom-in') { panZoom.zoomIn(); }
            else if (action === 'zoom-out') { panZoom.zoomOut(); }
            else if (action === 'reset') { panZoom.reset(); }
        });
    }

    // Modifier (ctrl/meta/alt) bails without preventDefault so VSCode
    // shortcuts like Cmd+0 still pass through. Held arrows drive a rAF loop
    // (12 px/frame) instead of one-shot per keydown, avoiding OS key-repeat
    // stutter. blur clears the held set so a key can't stick mid-hold.
    const PAN_SPEED = 12;
    const heldPan = new Set();
    let panRaf = null;
    function panFrame() {
        if (!panZoom || heldPan.size === 0) { panRaf = null; return; }
        let dx = 0;
        let dy = 0;
        if (heldPan.has('ArrowDown')) { dy -= PAN_SPEED; }
        if (heldPan.has('ArrowUp')) { dy += PAN_SPEED; }
        if (heldPan.has('ArrowRight')) { dx -= PAN_SPEED; }
        if (heldPan.has('ArrowLeft')) { dx += PAN_SPEED; }
        if (dx !== 0 || dy !== 0) { panZoom.panBy({ x: dx, y: dy }); }
        panRaf = requestAnimationFrame(panFrame);
    }
    window.addEventListener('keydown', function (event) {
        if (event.ctrlKey || event.metaKey || event.altKey) { return; }
        if (!panZoom) { return; }
        switch (event.key) {
            case 'ArrowDown':
            case 'ArrowUp':
            case 'ArrowRight':
            case 'ArrowLeft':
                heldPan.add(event.key);
                if (panRaf === null) { panRaf = requestAnimationFrame(panFrame); }
                break;
            case '+': case '=': panZoom.zoomIn(); break;
            case '-': panZoom.zoomOut(); break;
            case '0': panZoom.reset(); break;
            default: return;
        }
        event.preventDefault();
    });
    window.addEventListener('keyup', function (event) {
        heldPan.delete(event.key);
    });
    window.addEventListener('blur', function () {
        heldPan.clear();
    });

    window.addEventListener('message', function (event) {
        const msg = event.data;
        if (!msg || typeof msg !== 'object') { return; }
        if (msg.type === 'render') {
            // Drop the tooltip before its anchor SVG is swapped out.
            removeTooltip();
            let savedPan = null;
            let savedZoom = null;
            if (panZoom) {
                // Stop the in-flight tween before its instance is destroyed.
                cancelPanAnim();
                savedPan = panZoom.getPan();
                savedZoom = panZoom.getZoom();
                panZoom.destroy();
                panZoom = null;
            }
            root.innerHTML = msg.svg;
            const svg = root.querySelector('svg');
            if (!svg) { showControls(false); hideLegend(); return; }
            injectGhostBadges(svg);
            renderLegend(svg);
            panZoom = svgPanZoom(svg, {
                zoomEnabled: true,
                panEnabled: true,
                controlIconsEnabled: false,
                fit: true,
                center: true,
                minZoom: 0.25,
                maxZoom: 20,
                zoomScaleSensitivity: 0.3,
                dblClickZoomEnabled: true,
                mouseWheelZoomEnabled: true,
                // Any pan/zoom drops the tooltip so it never strands stale.
                onPan: removeTooltip,
                onZoom: removeTooltip,
            });
            if (savedZoom !== null) {
                panZoom.zoom(savedZoom);
                panZoom.pan(savedPan);
            }
            showControls(true);
        } else if (msg.type === 'highlightEntity') {
            highlightEntity(msg.id, msg.kind);
        } else if (msg.type === 'renderError') {
            removeTooltip();
            teardown();
            showControls(false);
            hideLegend();
            const banner = document.createElement('div');
            banner.className = 'kul-error-banner';
            banner.textContent = msg.message;
            root.innerHTML = '';
            root.appendChild(banner);
        }
    });
}());
`;

/**
 * Build the full webview HTML. The two stylesheets are the ADR-0016 token
 * split: `themeHref` carries the per-theme `--kul-*` tokens, `styleHref` the
 * application rules that consume them.
 */
export function previewHtml(
    themeHref: string,
    styleHref: string,
    scriptHref: string,
    cspSource: string,
    nonce: string,
): string {
    // script-src is nonce-gated (browsers ignore 'unsafe-inline' once a nonce
    // is present). style-src keeps 'unsafe-inline' for the injected SVG's
    // structural inline styles (ADR-0016).
    const csp = `default-src 'none'; style-src ${cspSource} 'unsafe-inline'; script-src 'nonce-${nonce}' ${cspSource};`;
    return `<!DOCTYPE html>
<html lang="en">
<head>
<meta charset="UTF-8">
<meta http-equiv="Content-Security-Policy" content="${csp}">
<link rel="stylesheet" href="${themeHref}">
<link rel="stylesheet" href="${styleHref}">
<title>Kul Preview</title>
</head>
<body data-theme="vscode">
<div id="root" tabindex="-1" style="outline: none;"></div>
${CONTROLS}
${LEGEND}
<script nonce="${nonce}" src="${scriptHref}"></script>
<script nonce="${nonce}">${BOOTSTRAP}</script>
</body>
</html>`;
}
