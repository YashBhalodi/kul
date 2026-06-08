// Copy `@kullang/preview` build artefacts into media/preview/ so the webview
// can load them as bundled resources. Invoked before `vsce package` and as a
// step of `npm run build` so the .vsix always carries the freshest dist/.

import { copyFileSync, mkdirSync, rmSync } from "node:fs";
import { dirname, resolve } from "node:path";
import { fileURLToPath } from "node:url";

const here = dirname(fileURLToPath(import.meta.url));
const ext = resolve(here, "..");
const previewDist = resolve(ext, "..", "..", "packages", "preview", "dist");
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
