// Pure HTML shell — no DOM access, runs under Node for the extension's
// webview.html assignment. The bundled `preview-webview.js` then mounts the
// chrome inside `#kul-preview-mount` via `mountPreview`.

// 32-char nonce stamped on every `<script>` so the CSP can drop
// `'unsafe-inline'`. Standard VSCode webview pattern.
//
// A CSP nonce is only as strong as its unpredictability: a nonce a page author
// can guess is worth no more than `'unsafe-inline'`. So the bytes come from the
// Web Crypto CSPRNG (`crypto.getRandomValues`), never `Math.random()` (a
// non-cryptographic PRNG). `globalThis.crypto` is a global — available both in
// the Node extension host that assigns `webview.html` (Node 18+) and in any
// browser embedding — so this stays free of a `node:crypto` import that would
// couple the neutral library build to Node. The modulo maps each byte onto the
// 62-char alphabet with negligible bias (256 mod 62), which does not
// meaningfully reduce the ~190 bits of entropy across 32 characters.
export function getNonce(): string {
    const chars =
        "ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789";
    const bytes = new Uint8Array(32);
    globalThis.crypto.getRandomValues(bytes);
    let text = "";
    for (let i = 0; i < bytes.length; i++) {
        text += chars.charAt(bytes[i] % chars.length);
    }
    return text;
}

/** Element id `mountPreview` looks for when called by the VSCode entry. */
export const MOUNT_POINT_ID = "kul-preview-mount";

export interface PreviewHtmlOptions {
    /** Webview-resource URI for the theme tokens stylesheet. */
    themeStylesheetUri: string;
    /** Webview-resource URI for the application stylesheet. */
    applicationStylesheetUri: string;
    /** Webview-resource URI for the bundled webview entry script. */
    scriptUri: string;
    /** VSCode `webview.cspSource` (or equivalent host CSP origin). */
    cspSource: string;
    /** 32-char nonce — generate with {@link getNonce}. */
    nonce: string;
    /** `data-theme` value on `<body>`. Defaults to `vscode`. */
    themeName?: string;
}

/**
 * VSCode-style full-doc HTML shell. The two stylesheets are the ADR-0016 token
 * split: `themeStylesheetUri` carries the per-theme `--kul-*` tokens,
 * `applicationStylesheetUri` the application rules that consume them.
 */
export function previewHtml(opts: PreviewHtmlOptions): string {
    const {
        themeStylesheetUri,
        applicationStylesheetUri,
        scriptUri,
        cspSource,
        nonce,
        themeName = "vscode",
    } = opts;
    // script-src is nonce-gated (browsers ignore 'unsafe-inline' once a nonce
    // is present). style-src keeps 'unsafe-inline' for the injected SVG's
    // structural inline styles (ADR-0016).
    const csp = `default-src 'none'; style-src ${cspSource} 'unsafe-inline'; script-src 'nonce-${nonce}' ${cspSource};`;
    return `<!DOCTYPE html>
<html lang="en">
<head>
<meta charset="UTF-8">
<meta http-equiv="Content-Security-Policy" content="${csp}">
<link rel="stylesheet" href="${themeStylesheetUri}">
<link rel="stylesheet" href="${applicationStylesheetUri}">
<title>Kul Preview</title>
</head>
<body data-theme="${themeName}">
<div id="${MOUNT_POINT_ID}"></div>
<script nonce="${nonce}" src="${scriptUri}"></script>
</body>
</html>`;
}
