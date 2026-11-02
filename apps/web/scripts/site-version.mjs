// The version the site shows: apps/web's package.json version (in lockstep
// with the CLI's), or KUL_SITE_VERSION when the build sets it. The build
// defines it as `import.meta.env.KUL_SITE_VERSION`. The service-worker
// update test builds a second site with another version this way.

import { readFileSync } from "node:fs"

const packageJson = JSON.parse(
  readFileSync(new URL("../package.json", import.meta.url), "utf8")
)

/** @returns {string} */
export function siteVersion() {
  return process.env.KUL_SITE_VERSION || packageJson.version
}
