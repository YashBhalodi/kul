import { mkdirSync, mkdtempSync, writeFileSync } from "node:fs"
import { tmpdir } from "node:os"
import { dirname, join } from "node:path"

import { describe, expect, it } from "vitest"

import {
  checkLinks,
  fileFor,
  htmlLinks,
  markdownLinks,
} from "./check-links.mjs"

/** A build directory holding `files` (path → text). */
function build(files) {
  const root = mkdtempSync(join(tmpdir(), "check-links-"))
  for (const [path, text] of Object.entries(files)) {
    mkdirSync(dirname(join(root, path)), { recursive: true })
    writeFileSync(join(root, path), text)
  }
  return root
}

describe("htmlLinks and markdownLinks", () => {
  it("read anchors and link tags, unescaping entities", () => {
    expect(
      htmlLinks(
        '<link rel="alternate" href="/docs/a.md"><a class="x" href="/docs/?a=1&amp;b=2">x</a>'
      )
    ).toEqual(["/docs/a.md", "/docs/?a=1&b=2"])
  })

  it("read Markdown links outside fenced code", () => {
    expect(
      markdownLinks(
        "See [a](/docs/a/).\n\n```kul\n[x](/nope)\n```\n[b](https://kullang.dev/llms.txt)"
      )
    ).toEqual(["/docs/a/", "https://kullang.dev/llms.txt"])
  })
})

describe("fileFor", () => {
  const files = new Set(["index.html", "docs/index.html", "docs/a.md"])
  it("serves directories from their index and files as they are", () => {
    expect(fileFor("/", files)).toBe("index.html")
    expect(fileFor("/docs/", files)).toBe("docs/index.html")
    expect(fileFor("/docs", files)).toBe("docs/index.html")
    expect(fileFor("/docs/a.md", files)).toBe("docs/a.md")
    expect(fileFor("/docs/b/", files)).toBeUndefined()
  })
})

describe("checkLinks", () => {
  const repoRoot = build({ "skills/kul-authoring/SKILL.md": "x" })

  it("passes a build whose links all resolve", () => {
    const root = build({
      "index.html":
        '<a href="/docs/">Docs</a><a href="/app/">App</a><a href="https://example.com">x</a>',
      "docs/index.html":
        '<h2 id="next">Next</h2><a href="#next">x</a><a href="a/">A</a><link rel="alternate" href="/docs/index.md">',
      "docs/a/index.html": '<a href="../#next">back</a>',
      "docs/index.md": "[A](https://kullang.dev/docs/a.md)",
      "docs/a.md": "[home](/docs/)",
      "llms.txt":
        "- [skill](https://github.com/YashBhalodi/kul/tree/main/skills/kul-authoring)\n- [all](https://kullang.dev/llms-full.txt)",
      "llms-full.txt": "[a](https://kullang.dev/docs/a.md)",
    })
    expect(checkLinks(root, { repoRoot })).toEqual([])
  })

  it("reports missing files, missing fragments and missing repository paths", () => {
    const root = build({
      "index.html": '<a href="/docs/missing/">x</a><a href="/docs/#nope">y</a>',
      "docs/index.html": "<h2>No ids</h2>",
      "llms.txt":
        "- [gone](https://kullang.dev/docs/gone.md)\n- [skill](https://github.com/YashBhalodi/kul/blob/main/skills/nothing.md)",
    })
    expect(checkLinks(root, { repoRoot })).toEqual([
      "index.html: /docs/missing/ is not in the build",
      "index.html: /docs/#nope names no #nope on docs/index.html",
      "llms.txt: https://kullang.dev/docs/gone.md is not in the build",
      "llms.txt: https://github.com/YashBhalodi/kul/blob/main/skills/nothing.md names no file in the repository",
    ])
  })
})
