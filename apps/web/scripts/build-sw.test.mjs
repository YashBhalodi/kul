import {
  mkdirSync,
  mkdtempSync,
  readFileSync,
  rmSync,
  writeFileSync,
} from "node:fs"
import { tmpdir } from "node:os"
import { dirname, join } from "node:path"

import { afterEach, beforeEach, describe, expect, it } from "vitest"

import {
  SW_FILE,
  buildServiceWorker,
  precachedUrls,
  precacheEntries,
} from "./build-sw.mjs"

/** A build's output, including a docs page the code names nowhere. */
const BUILD = {
  "index.html": "<h1>Kul</h1>",
  "_shell.html": "<div>app</div>",
  _headers: "/*\n  X: y\n",
  _redirects: "/app/* /_shell.html 200\n",
  "llms.txt": "# Kul",
  "llms-full.txt": "# Kul, in full",
  "favicon.svg": "<svg/>",
  "docs/index.html": "<h1>Docs</h1>",
  "docs/index.md": "# Docs",
  "docs/search.json": "[]",
  "docs/reference/cheat-sheet/index.html": "<h1>Cheat sheet</h1>",
  "docs/reference/cheat-sheet.md": "# Cheat sheet",
  "docs/guide/families/index.html": "<h1>Families</h1>",
  "docs/guide/families.md": "# Families",
  "assets/index-AbC123.js": "console.log(1)",
  "assets/worker-DeF456.js": "self.onmessage = null",
  "assets/kul_wasm_bg-GhI789.wasm": "\0asm",
  "assets/inter-latin-JkL012.woff2": "font",
  "assets/styles-MnO345.css": "body{}",
}

let root

beforeEach(() => {
  root = mkdtempSync(join(tmpdir(), "kul-build-sw-"))
  for (const [file, text] of Object.entries(BUILD)) {
    mkdirSync(dirname(join(root, file)), { recursive: true })
    writeFileSync(join(root, file), text)
  }
})

afterEach(() => {
  rmSync(root, { recursive: true, force: true })
})

const servedFiles = Object.keys(BUILD)
  .filter((file) => file !== "_headers" && file !== "_redirects")
  .map((file) => `/${file}`)
  .sort()

describe("precacheEntries", () => {
  it("lists every served file the build wrote, found by glob", async () => {
    const entries = await precacheEntries(root)
    expect(entries.map((entry) => entry.url).sort()).toEqual(servedFiles)
  })

  it("revisions everything outside /assets/ by content", async () => {
    const entries = await precacheEntries(root)
    for (const { url, revision } of entries) {
      if (url.startsWith("/assets/")) expect(revision, url).toBeNull()
      else expect(revision, url).toMatch(/^[0-9a-f]{32}$/)
    }
  })

  it("gives a page a new revision when its content changes", async () => {
    const before = await precacheEntries(root)
    writeFileSync(join(root, "index.html"), "<h1>Kul</h1><p>0.4.4</p>")
    const after = await precacheEntries(root)
    const revision = (entries) =>
      entries.find((entry) => entry.url === "/index.html").revision
    expect(revision(after)).not.toBe(revision(before))
  })
})

describe("buildServiceWorker", () => {
  it("writes sw.js with the manifest injected and nothing else precached", async () => {
    await buildServiceWorker(root)
    const sw = readFileSync(join(root, SW_FILE), "utf8")
    expect(precachedUrls(sw).sort()).toEqual(servedFiles)
    expect(sw).not.toContain("__WB_MANIFEST")
    expect(sw).not.toContain("process.env")
    expect(sw).not.toMatch(/^\s*(import|export)\b/m)
  })

  it("leaves sw.js itself out of a rebuild's manifest", async () => {
    await buildServiceWorker(root)
    await buildServiceWorker(root)
    const sw = readFileSync(join(root, SW_FILE), "utf8")
    expect(precachedUrls(sw)).not.toContain(`/${SW_FILE}`)
  })
})
