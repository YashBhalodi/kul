import { copyFileSync, mkdirSync } from "node:fs";
import { dirname, resolve } from "node:path";
import { fileURLToPath } from "node:url";

const here = dirname(fileURLToPath(import.meta.url));
const src = resolve(here, "..", "src");
const dist = resolve(here, "..", "dist");

mkdirSync(dist, { recursive: true });
for (const file of ["preview.css", "preview-themes.css"]) {
    copyFileSync(resolve(src, file), resolve(dist, file));
    console.log(`copied ${file} → dist/`);
}
