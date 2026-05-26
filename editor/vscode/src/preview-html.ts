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
 */
const BOOTSTRAP = `
(function () {
    const root = document.getElementById('root');
    let panZoom = null;

    function teardown() {
        if (panZoom) {
            panZoom.destroy();
            panZoom = null;
        }
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
            if (!svg) { return; }
            panZoom = svgPanZoom(svg, {
                zoomEnabled: true,
                panEnabled: true,
                controlIconsEnabled: true,
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
        } else if (msg.type === 'renderError') {
            teardown();
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
<script nonce="${nonce}" src="${scriptHref}"></script>
<script nonce="${nonce}">${BOOTSTRAP}</script>
</body>
</html>`;
}
