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
 *   swap in the new SVG, then re-create the instance — re-applying the
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
    let panZoom = null;

    function teardown() {
        if (panZoom) {
            panZoom.destroy();
            panZoom = null;
        }
    }

    function showControls(visible) {
        if (controls) { controls.hidden = !visible; }
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
 * @param cssHref    webview URI of `media/preview.css`
 * @param scriptHref webview URI of the vendored `svg-pan-zoom.min.js`
 * @param cspSource  the webview's `cspSource` (the `vscode-resource:` origin)
 * @param nonce      a fresh per-build nonce (see {@link getNonce})
 */
export function previewHtml(
    cssHref: string,
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
<link rel="stylesheet" href="${cssHref}">
<title>Kul Preview</title>
</head>
<body data-theme="vscode">
<div id="root"></div>
${CONTROLS}
<script nonce="${nonce}" src="${scriptHref}"></script>
<script nonce="${nonce}">${BOOTSTRAP}</script>
</body>
</html>`;
}
