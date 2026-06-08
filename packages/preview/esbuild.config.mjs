// Two outputs:
//   dist/*.js              — per-file ESM library (no bundling, no inlining).
//                            Re-exports stay graph-shaped so consumers can
//                            tree-shake unused entry points — critical for
//                            the VSCode extension, which imports only
//                            `previewHtml` + `getNonce` and must not pull in
//                            the browser-only `svg-pan-zoom` transitively.
//   dist/preview-webview.js — single bundled IIFE the VSCode webview's
//                            <script src> loads. Inlines svg-pan-zoom.

import { readdirSync } from "node:fs";
import { resolve } from "node:path";
import * as esbuild from "esbuild";

const SRC = resolve("src");
const libraryEntries = readdirSync(SRC)
    .filter((f) => f.endsWith(".ts") && !f.endsWith(".test.ts"))
    .map((f) => `src/${f}`);

await Promise.all([
    esbuild.build({
        entryPoints: libraryEntries,
        outdir: "dist",
        format: "esm",
        target: "es2022",
        platform: "neutral",
        sourcemap: true,
        bundle: false,
        logLevel: "info",
    }),
    esbuild.build({
        entryPoints: ["src/entry-vscode.ts"],
        outfile: "dist/preview-webview.js",
        format: "iife",
        target: "es2022",
        platform: "browser",
        sourcemap: true,
        bundle: true,
        minify: true,
        logLevel: "info",
    }),
]);
