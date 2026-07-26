// Copy `@kullang/preview` build artefacts into media/preview/ so the webview
// can load them as bundled resources. Invoked before `vsce package` and as a
// step of `npm run build` so the .vsix always carries the freshest dist/.
//
// The query engine is staged the same way (ADR-0040): the `wasm-pack --target
// web` output in crates/kul-wasm/pkg-web/ is copied into media/preview/wasm/,
// and the extension hands the webview its URIs at preview-open. That is why
// wasm-pack is a prerequisite for *packaging* the extension — but not for
// `npm ci`, which must keep working on a fresh clone.

import { copyFileSync, existsSync, mkdirSync, rmSync } from "node:fs";
import { dirname, resolve } from "node:path";
import { fileURLToPath } from "node:url";

const here = dirname(fileURLToPath(import.meta.url));
const ext = resolve(here, "..");
const repo = resolve(ext, "..", "..");
const previewDist = resolve(repo, "packages", "preview", "dist");
const wasmPkg = resolve(repo, "crates", "kul-wasm", "pkg-web");
const target = resolve(ext, "media", "preview");

rmSync(target, { recursive: true, force: true });
mkdirSync(target, { recursive: true });
for (const file of [
    "preview-webview.js",
    "preview.css",
    "preview-themes.css",
]) {
    copyFileSync(resolve(previewDist, file), resolve(target, file));
    console.log(`copied ${file} → media/preview/`);
}

// Strict by default, because the failure mode of a lenient default is a
// published .vsix whose preview cannot answer a single query. The one caller
// that legitimately has no engine is `just check`, which exists to prove the
// TypeScript graph is sound and which ADR-0040 promises will never need a
// wasm toolchain — it opts out explicitly.
const engineOptional = process.env.KUL_ALLOW_MISSING_ENGINE === "1";

if (!existsSync(wasmPkg)) {
    if (engineOptional) {
        console.log(
            `skipped the query engine — no ${wasmPkg} (KUL_ALLOW_MISSING_ENGINE=1).`,
        );
        process.exit(0);
    }
    console.error(
        `\nmissing ${wasmPkg}\n` +
            "The preview's query engine is a build asset, not an npm dependency.\n" +
            "Build it with `just wasm` (needs wasm-pack) and re-run.\n",
    );
    process.exit(1);
}

const wasmTarget = resolve(target, "wasm");
mkdirSync(wasmTarget, { recursive: true });
for (const file of ["kul_wasm.js", "kul_wasm_bg.wasm"]) {
    copyFileSync(resolve(wasmPkg, file), resolve(wasmTarget, file));
    console.log(`copied ${file} → media/preview/wasm/`);
}
