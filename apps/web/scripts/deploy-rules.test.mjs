import { readFileSync } from "node:fs"

import { describe, expect, it } from "vitest"

import {
  ALIAS_HOSTS,
  CANONICAL_HOST,
  NETLIFY_SITE,
  SERVING_HOSTS,
  headerProblems,
  redirectProblems,
} from "./deploy-rules.mjs"
import { buildHeaders } from "./finalize-dist.mjs"
import { matchRedirect, parseHeaders, parseRedirects } from "./serve-dist.mjs"

const REDIRECTS = readFileSync(
  new URL("../public/_redirects", import.meta.url),
  "utf8"
)

const PAGE = `<!doctype html><html><head>
<script type="module" src="/assets/index-abc.js"></script>
</head><body><script>boot()</script></body></html>`

const FILES = new Set([
  "index.html",
  "_shell.html",
  "docs/index.html",
  "docs/index.md",
  "docs/guide/index.html",
  "docs/guide.md",
  "llms.txt",
  "sw.js",
  "favicon.ico",
  "assets/index-abc.js",
  "assets/kul_wasm_bg-abc.wasm",
  "_headers",
  "_redirects",
])

const read = () => PAGE

describe("public/_redirects", () => {
  const rules = parseRedirects(REDIRECTS)
  const route = (hostname, pathname, shadowed = false) =>
    matchRedirect(rules, { hostname, pathname, search: "", shadowed })

  it("holds every rule", () => {
    expect(redirectProblems(rules, FILES)).toEqual([])
  })

  it("redirects www and the production netlify.app host, files included", () => {
    expect(ALIAS_HOSTS).toEqual([
      "www.kullang.dev",
      `${NETLIFY_SITE}.netlify.app`,
    ])
    for (const host of ALIAS_HOSTS) {
      expect(route(host, "/docs/guide/", true)).toMatchObject({
        rule: { status: 301, force: true },
        target: "https://kullang.dev/docs/guide/",
      })
    }
  })

  it("serves deploy previews and permalinks without redirecting", () => {
    for (const host of [
      `pr-357--${NETLIFY_SITE}.netlify.app`,
      `pr-1--${NETLIFY_SITE}.netlify.app`,
      `0123456789abcdef01234567--${NETLIFY_SITE}.netlify.app`,
      CANONICAL_HOST,
    ]) {
      expect(route(host, "/", true), host).toBeNull()
      expect(route(host, "/docs/", true), host).toBeNull()
      expect(route(host, "/app/x", false)?.target, host).toBe("/_shell.html")
    }
    expect(SERVING_HOSTS).toContain(`pr-357--${NETLIFY_SITE}.netlify.app`)
  })

  it("reports a canonical rule that files would shadow", () => {
    const unforced = REDIRECTS.replace(/301!/g, "301")
    expect(redirectProblems(parseRedirects(unforced), FILES)).toContain(
      "https://www.kullang.dev/ does not redirect 301 to https://kullang.dev/"
    )
  })

  it("reports a rule that sends a deploy preview off-site", () => {
    const wide = `https://pr-357--${NETLIFY_SITE}.netlify.app/* https://kullang.dev/:splat 301!\n${REDIRECTS}`
    expect(redirectProblems(parseRedirects(wide), FILES)).toContain(
      `pr-357--${NETLIFY_SITE}.netlify.app/ is sent off-site to https://kullang.dev/`
    )
  })

  it("reports a missing /app rewrite", () => {
    const withoutApp = REDIRECTS.replace(/^\/app\/\*.*$/m, "")
    expect(redirectProblems(parseRedirects(withoutApp), FILES)).toContain(
      "kullang.dev/app/x is not served /_shell.html with 200"
    )
  })
})

describe("generated _headers", () => {
  const text = buildHeaders([...FILES], read)

  it("holds every rule", () => {
    expect(headerProblems(parseHeaders(text), FILES, read)).toEqual([])
  })

  it("reports a page whose inline script is not hashed", () => {
    const other = () => `${PAGE}<script>later()</script>`
    expect(headerProblems(parseHeaders(text), FILES, other)).toContain(
      "/: an inline script is not hashed in its CSP"
    )
  })

  it("reports a cacheable service worker and a mistyped WASM", () => {
    const broken = text
      .replace(
        /(\/sw\.js\n(?: {2}.*\n)*? {2}Cache-Control: )no-cache/,
        "$1max-age=600"
      )
      .replace(
        "Content-Type: application/wasm",
        "Content-Type: application/octet-stream"
      )
    const problems = headerProblems(parseHeaders(broken), FILES, read)
    expect(problems).toContain("/sw.js is cached as max-age=600, not no-cache")
    expect(problems).toContain(
      "/assets/kul_wasm_bg-abc.wasm is served as application/octet-stream, not application/wasm"
    )
  })

  it("reports a policy that allows other origins", () => {
    const open = text.replaceAll("connect-src 'self'", "connect-src *")
    expect(headerProblems(parseHeaders(open), FILES, read)).toContain(
      "/: the CSP does not keep requests same-origin"
    )
  })

  it("reports an /app path without the shell's policy", () => {
    const blocks = parseHeaders(text).filter(
      (block) => !block.pattern.startsWith("/app")
    )
    expect(headerProblems(blocks, FILES, read)).toContain(
      "/app/x/y does not get the shell's Content-Security-Policy"
    )
  })
})
