// Builds the next site (see sites.ts) before the suites run: the source
// built again with KUL_SITE_VERSION set, into its own directory
// (next-site.vite.config.ts), then the same post-build steps as
// `npm run build`. It takes a few seconds; the update suite and the tour
// serve it.

import { execFileSync } from "node:child_process"
import type { ExecFileSyncOptions } from "node:child_process"
import { readFileSync } from "node:fs"
import { join } from "node:path"

import { APP_DIR, NEXT_BUILD_DIR, NEXT_VERSION } from "./sites"

export default function globalSetup() {
  const options: ExecFileSyncOptions = {
    cwd: APP_DIR,
    env: { ...process.env, KUL_SITE_VERSION: NEXT_VERSION },
    stdio: ["ignore", "ignore", "inherit"],
  }
  execFileSync(
    "npx",
    [
      "--no-install",
      "vite",
      "build",
      "--config",
      "e2e/next-site.vite.config.ts",
      "--logLevel",
      "error",
    ],
    options
  )
  for (const script of ["build-sw", "finalize-dist", "check-dist"]) {
    execFileSync(
      process.execPath,
      [join("scripts", `${script}.mjs`), "--dir", NEXT_BUILD_DIR],
      options
    )
  }
  // The prerendered pages, not only the scripts, carry the next version.
  const landing = readFileSync(join(NEXT_BUILD_DIR, "index.html"), "utf8")
  if (!landing.includes(NEXT_VERSION)) {
    throw new Error(`the next site's landing page lacks ${NEXT_VERSION}`)
  }
}
