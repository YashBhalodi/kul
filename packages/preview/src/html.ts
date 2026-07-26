// Pure HTML shell — no DOM access, runs under Node for the extension's
// webview.html assignment. The bundled `preview-webview.js` then mounts the
// chrome inside `#kul-preview-mount` via `mountPreview`.

import type { EngineSource } from "./engine.js";

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

/**
 * The **chrome's** language, stamped on `<html>`.
 *
 * It stays `en` whatever the reader picks in the locale toggle: UI chrome
 * labels are English and chrome i18n is a separate question (ADR-0022's
 * stance). What changes language is the kinship phrasing, and each phrase
 * carries its **own** `lang` — per-element, not per-document (ADR-0033).
 *
 * That split is a correctness item rather than a styling preference: a
 * Gujarati phrase needs `lang="gu"` for the browser to select a font that can
 * render the script, while the English chrome around it must not be tagged
 * `gu`. Retagging the document instead would get both wrong at once.
 */
export const CHROME_LANG = "en";

/**
 * Data attributes the shell stamps on the mount point to tell the webview
 * entry where the host put the engine (ADR-0040). The entry reads them rather
 * than importing a module, which is what keeps this package host-agnostic.
 */
export const ENGINE_MODULE_ATTR = "data-kul-engine-module";
export const ENGINE_WASM_ATTR = "data-kul-engine-wasm";

function escapeAttribute(value: string): string {
    return value
        .replace(/&/g, "&amp;")
        .replace(/</g, "&lt;")
        .replace(/>/g, "&gt;")
        .replace(/"/g, "&quot;");
}

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
    /**
     * Where the host put the query engine (ADR-0040). Supplying it stamps the
     * URIs onto the mount point *and* widens the CSP so the module can be
     * imported, fetched and instantiated. Omit it and the shell keeps the
     * tighter posture — a host that ships no engine pays nothing for one.
     */
    engineSource?: EngineSource;
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
        engineSource,
    } = opts;
    // script-src is nonce-gated (browsers ignore 'unsafe-inline' once a nonce
    // is present). style-src keeps 'unsafe-inline' for the injected SVG's
    // structural inline styles (ADR-0016).
    //
    // Running the engine in the webview costs two additions (ADR-0034), and
    // both are the narrowest grant that does the job:
    //
    //   script-src  'wasm-unsafe-eval'  — compile + instantiate WebAssembly.
    //                                     NOT 'unsafe-eval', which would
    //                                     re-enable eval() on a surface that
    //                                     renders untrusted document content.
    //   connect-src ${cspSource}        — the glue module fetch()es the .wasm
    //                                     from the host's resource origin.
    //                                     default-src 'none' blocks it
    //                                     otherwise.
    //
    // The glue module itself is imported from ${cspSource}, which script-src
    // already allows (a nonce does not extend to dynamically imported modules;
    // the host-source expression is what covers them).
    const scriptSrc = engineSource
        ? `'nonce-${nonce}' ${cspSource} 'wasm-unsafe-eval'`
        : `'nonce-${nonce}' ${cspSource}`;
    const connectSrc = engineSource ? ` connect-src ${cspSource};` : "";
    const csp = `default-src 'none'; style-src ${cspSource} 'unsafe-inline'; script-src ${scriptSrc};${connectSrc}`;
    const engineAttrs = engineSource
        ? ` ${ENGINE_MODULE_ATTR}="${escapeAttribute(engineSource.moduleUri)}" ${ENGINE_WASM_ATTR}="${escapeAttribute(engineSource.wasmUri)}"`
        : "";
    return `<!DOCTYPE html>
<html lang="${CHROME_LANG}">
<head>
<meta charset="UTF-8">
<meta http-equiv="Content-Security-Policy" content="${csp}">
<link rel="stylesheet" href="${themeStylesheetUri}">
<link rel="stylesheet" href="${applicationStylesheetUri}">
<title>Kul Preview</title>
</head>
<body data-theme="${themeName}">
<div id="${MOUNT_POINT_ID}"${engineAttrs}></div>
<script nonce="${nonce}" src="${scriptUri}"></script>
</body>
</html>`;
}
