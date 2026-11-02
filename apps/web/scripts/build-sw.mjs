#!/usr/bin/env node
// Writes dist/client/sw.js after `vite build`. Vite bundles src/sw/sw.ts
// into one classic script; workbox-build's injectManifest then puts the
// precache manifest into it. The manifest lists every file the build wrote,
// found by glob when this runs: so a page, Markdown file or asset that a
// later build adds is precached with no change here.
//
//   node scripts/build-sw.mjs [--dir <dir>]
//
// Paths under /assets/ are content-hashed and carry no revision; every other
// entry is revisioned by its content's hash, so any change to a page (its
// footer's version included) gives a new sw.js, which browsers install as an
// update.

import { mkdtempSync, rmSync } from "node:fs"
import { tmpdir } from "node:os"
import { join } from "node:path"
import { fileURLToPath, pathToFileURL } from "node:url"
import { parseArgs } from "node:util"

import { build } from "vite"
import { getManifest, injectManifest } from "workbox-build"

const here = fileURLToPath(new URL(".", import.meta.url))

export const SW_FILE = "sw.js"

/**
 * What the precache leaves out: the worker itself and Netlify's config
 * files, which are never served.
 */
const IGNORED = [SW_FILE, "_headers", "_redirects", "**/*.map"]

/** No precached file may be larger (the .wasm is about 0.5 MB). */
const MAX_FILE_BYTES = 4 * 1024 * 1024

/** The manifest options for a build at `root`. */
function manifestOptions(root) {
  return {
    globDirectory: root,
    globPatterns: ["**/*"],
    globIgnores: IGNORED,
    modifyURLPrefix: { "": "/" },
    dontCacheBustURLsMatching: /^\/assets\//,
    maximumFileSizeToCacheInBytes: MAX_FILE_BYTES,
  }
}

function failOnWarnings(warnings) {
  if (warnings.length > 0) {
    throw new Error(`build-sw: ${warnings.join("; ")}`)
  }
}

/** The precache entries for the build at `root`, sorted by URL. */
export async function precacheEntries(root) {
  const { manifestEntries, warnings } = await getManifest(manifestOptions(root))
  failOnWarnings(warnings)
  return [...manifestEntries].sort((a, b) => a.url.localeCompare(b.url))
}

/** The precached URLs written into a built sw.js. */
export function precachedUrls(swText) {
  return [...swText.matchAll(/"url":"([^"]+)"/g)].map(([, url]) => url)
}

/** Bundle the service worker and write it, with its manifest, into `root`. */
export async function buildServiceWorker(root) {
  const scratch = mkdtempSync(join(tmpdir(), "kul-sw-"))
  try {
    await build({
      configFile: false,
      root: join(here, ".."),
      logLevel: "warn",
      publicDir: false,
      // Workbox drops its development logging in production.
      define: { "process.env.NODE_ENV": JSON.stringify("production") },
      build: {
        outDir: scratch,
        emptyOutDir: true,
        copyPublicDir: false,
        minify: true,
        lib: {
          entry: join(here, "../src/sw/sw.ts"),
          formats: ["iife"],
          name: "kulServiceWorker",
          fileName: () => SW_FILE,
        },
      },
    })
    const { count, size, warnings } = await injectManifest({
      ...manifestOptions(root),
      swSrc: join(scratch, SW_FILE),
      swDest: join(root, SW_FILE),
    })
    failOnWarnings(warnings)
    return { count, size }
  } finally {
    rmSync(scratch, { recursive: true, force: true })
  }
}

async function main() {
  const { values } = parseArgs({
    options: {
      dir: { type: "string", default: join(here, "..", "dist", "client") },
    },
  })
  const { count, size } = await buildServiceWorker(values.dir)
  console.log(
    `build-sw: wrote ${join(values.dir, SW_FILE)} precaching ${count} files (${(size / 1024).toFixed(0)} KiB)`
  )
}

if (import.meta.url === pathToFileURL(process.argv[1] ?? "").href) {
  main().catch((error) => {
    console.error(error instanceof Error ? error.message : error)
    process.exit(1)
  })
}
