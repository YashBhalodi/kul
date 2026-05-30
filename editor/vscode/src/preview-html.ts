// Pure webview-HTML generation for the Kul preview panel.
//
// This module is deliberately free of any `vscode` import so the static
// HTML contract (CSP shape, nonce stamping, vendored-script wiring, and the
// pan/zoom bootstrap) can be unit-tested under Vitest without a browser or
// the VSCode host. `extension.ts` supplies the webview-resolved hrefs and a
// fresh nonce; everything here is string-in, string-out.

/**
 * Build a cryptographically-unguessable nonce for the webview CSP. Mirrors
 * the standard VSCode webview pattern: 32 chars from a fixed alphabet. A
 * fresh nonce is generated per webview-HTML build and stamped on every
 * `<script>` so the CSP can drop `'unsafe-inline'`.
 */
export function getNonce(): string {
    let text = "";
    const chars =
        "ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789";
    for (let i = 0; i < 32; i++) {
        text += chars.charAt(Math.floor(Math.random() * chars.length));
    }
    return text;
}

// Inline SVG glyphs for the overlay controls. `currentColor` lets the icon
// track the button's themed `color`, so a single `--kul-control-fg` token
// drives every glyph (ADR-0016). `aria-hidden` keeps them out of the
// accessibility tree — the buttons carry their own `aria-label`.
const ICON_ZOOM_IN = `<svg viewBox="0 0 16 16" aria-hidden="true" focusable="false"><path d="M8 3.5v9M3.5 8h9" fill="none" stroke="currentColor" stroke-width="1.5" stroke-linecap="round"/></svg>`;
const ICON_ZOOM_OUT = `<svg viewBox="0 0 16 16" aria-hidden="true" focusable="false"><path d="M3.5 8h9" fill="none" stroke="currentColor" stroke-width="1.5" stroke-linecap="round"/></svg>`;
const ICON_RESET = `<svg viewBox="0 0 16 16" aria-hidden="true" focusable="false"><path d="M3 6V3.5A.5.5 0 0 1 3.5 3H6M10 3h2.5a.5.5 0 0 1 .5.5V6M13 10v2.5a.5.5 0 0 1-.5.5H10M6 13H3.5a.5.5 0 0 1-.5-.5V10" fill="none" stroke="currentColor" stroke-width="1.5" stroke-linecap="round" stroke-linejoin="round"/></svg>`;

// The overlay control cluster. Lives as a sibling of #root (not inside it),
// so the per-render `root.innerHTML = …` swap never wipes it; it is wired
// once and acts on the current pan/zoom instance. Hidden until the first
// successful render and re-hidden on error.
const CONTROLS = `<div id="kul-controls" class="kul-preview-controls" role="group" aria-label="Diagram view controls" hidden>
<button type="button" class="kul-control-btn" data-action="zoom-in" title="Zoom in" aria-label="Zoom in">${ICON_ZOOM_IN}</button>
<button type="button" class="kul-control-btn" data-action="reset" title="Reset view" aria-label="Reset view">${ICON_RESET}</button>
<button type="button" class="kul-control-btn" data-action="zoom-out" title="Zoom out" aria-label="Zoom out">${ICON_ZOOM_OUT}</button>
</div>`;

/**
 * The inline bootstrap that runs inside the webview. It owns a single
 * module-level `svg-pan-zoom` instance (the global `svgPanZoom` comes from
 * the vendored script) and reconciles it against `render` / `renderError`
 * messages:
 *
 * - `render`: capture the live pan/zoom (if any), destroy the old instance,
 *   swap in the new SVG, inject the ghost `↺` badges (surface chrome —
 *   CSS cannot generate an SVG element, so the surface draws them;
 *   ADR-0016), then re-create the instance — re-applying the
 *   captured viewport so a debounced live-edit re-render does not yank the
 *   view back to fit. The first render (no captured viewport) falls through
 *   to `fit`+`center`. A missing `<svg>` is guarded.
 * - `renderError`: tear the instance down so no stale pan/zoom surface
 *   survives behind the error banner.
 *
 * The on-screen controls are custom HTML (`controlIconsEnabled: false`)
 * wired here: zoom-in / zoom-out step the zoom, reset returns to the
 * instance's fit-and-centered original state.
 */
const BOOTSTRAP = `
(function () {
    const root = document.getElementById('root');
    const controls = document.getElementById('kul-controls');
    const vscode = acquireVsCodeApi();
    let panZoom = null;

    // Click-to-source (issue #135): a click on a person card or a
    // marriage bar posts { type: 'revealSource', id } so the extension
    // can open the declaration via kul/locate. Persons resolve through
    // data-person-id; marriage bars are data-link-kind="marriage" and
    // carry the id in data-marriage-id. Birth/adoption edges also carry
    // data-marriage-id but are NOT clickable — keying on
    // data-link-kind="marriage" (not "has data-marriage-id") keeps them
    // inert. closest(...) handles clicks landing on a child node of the
    // card/path. Registered on #root, which survives every innerHTML swap.
    if (root) {
        root.addEventListener('click', function (event) {
            // Pull keyboard focus into the iframe so the window-level
            // keydown handler (issue #180) receives arrows/+/-/0. Clicking a
            // non-focusable SVG does NOT move focus off the text editor on
            // its own, so the diagram surface (#root, tabindex="-1") is
            // focused explicitly here. tabindex="-1" keeps it out of the Tab
            // order; the inline outline:none on #root suppresses the focus
            // ring the host would otherwise paint, honoring the
            // no-focus-ring decision.
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

    function teardown() {
        if (panZoom) {
            panZoom.destroy();
            panZoom = null;
        }
    }

    function showControls(visible) {
        if (controls) { controls.hidden = !visible; }
    }

    // The ghost ↺ badge is surface chrome (ADR-0016): CSS cannot
    // generate an SVG element, so the surface draws it. For each ghost
    // card, append a <text> badge near the card's top-right corner,
    // placed from the card <rect>'s geometry. The node must be created
    // in the SVG namespace — document.createElement('text') yields an
    // inert HTML element that will not render inside <svg>. The
    // .kul-ghost-badge rule + --kul-ghost-badge-* tokens style it. Runs
    // on every render: each innerHTML swap wipes the prior badges.
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

    if (controls) {
        controls.addEventListener('click', function (event) {
            const btn = event.target.closest('button[data-action]');
            if (!btn || !panZoom) { return; }
            const action = btn.getAttribute('data-action');
            if (action === 'zoom-in') { panZoom.zoomIn(); }
            else if (action === 'zoom-out') { panZoom.zoomOut(); }
            else if (action === 'reset') { panZoom.reset(); }
        });
    }

    // Keyboard pan/zoom for sighted keyboard users (issue #180), mirroring
    // the mouse + on-screen-button controls. Fires whenever the preview
    // iframe holds focus (clicking the diagram or tabbing to a control
    // button). A modifier (ctrl/meta/alt) bails without preventDefault so
    // VSCode shortcuts like Cmd+0 still pass through; the null guard mirrors
    // the controls-click handler.
    //
    // Arrows scroll the viewport (panBy with scroll semantics — ArrowDown
    // reveals content below). Rather than panning once per keydown — which
    // rides the OS key-repeat and stutters (initial-repeat delay, then
    // discrete jumps) — held arrows are tracked in a set and a
    // requestAnimationFrame loop pans PAN_SPEED px/frame while any are down,
    // giving smooth ~60fps motion. keyup clears the key; window blur clears
    // all so a key can't stick when focus leaves mid-hold. +/=/-/0 stay
    // discrete one-shots sharing the exact zoom/reset methods the buttons
    // call. Repeat keydowns are harmless: re-adding a held key is a no-op and
    // the rAF loop is already running.
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
            let savedPan = null;
            let savedZoom = null;
            if (panZoom) {
                savedPan = panZoom.getPan();
                savedZoom = panZoom.getZoom();
                panZoom.destroy();
                panZoom = null;
            }
            root.innerHTML = msg.svg;
            const svg = root.querySelector('svg');
            if (!svg) { showControls(false); return; }
            injectGhostBadges(svg);
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
            });
            if (savedZoom !== null) {
                panZoom.zoom(savedZoom);
                panZoom.pan(savedPan);
            }
            showControls(true);
        } else if (msg.type === 'renderError') {
            teardown();
            showControls(false);
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
 * Build the full webview HTML.
 *
 * The two stylesheets are the ADR-0016 token split: `themeHref` carries the
 * per-theme `--kul-*` token definitions, `styleHref` the application rules
 * that consume them. The theme sheet is linked first.
 *
 * @param themeHref  webview URI of `media/preview-themes.css` (token layer)
 * @param styleHref  webview URI of `media/preview.css` (application rules)
 * @param scriptHref webview URI of the vendored `svg-pan-zoom.min.js`
 * @param cspSource  the webview's `cspSource` (the `vscode-resource:` origin)
 * @param nonce      a fresh per-build nonce (see {@link getNonce})
 */
export function previewHtml(
    themeHref: string,
    styleHref: string,
    scriptHref: string,
    cspSource: string,
    nonce: string,
): string {
    // script-src is nonce-gated (no 'unsafe-inline' — browsers ignore it
    // once a nonce is present). style-src keeps 'unsafe-inline' for the
    // injected SVG's structural inline styles (ADR-0016).
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
<script nonce="${nonce}" src="${scriptHref}"></script>
<script nonce="${nonce}">${BOOTSTRAP}</script>
</body>
</html>`;
}
