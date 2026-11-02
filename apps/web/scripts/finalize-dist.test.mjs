import { describe, expect, it } from "vitest"

import {
  buildHeaders,
  contentSecurityPolicy,
  inlineScripts,
  requestPaths,
  scriptHash,
} from "./finalize-dist.mjs"
import { headersFor, parseHeaders } from "./serve-dist.mjs"

const PAGE = `<!doctype html><html><head>
<script src="/assets/theme.js"></script>
<script type="module" src="/assets/index.js"></script>
</head><body><script>boot()</script><script data-x="">done()</script></body></html>`

describe("inlineScripts", () => {
  it("returns the bodies of src-less scripts only", () => {
    expect(inlineScripts(PAGE)).toEqual(["boot()", "done()"])
  })
})

describe("contentSecurityPolicy", () => {
  it("adds each hash to script-src once", () => {
    const hash = scriptHash("boot()")
    const policy = contentSecurityPolicy([hash, hash])
    expect(policy).toContain(`script-src 'self' 'wasm-unsafe-eval' ${hash};`)
    expect(policy.split(hash)).toHaveLength(2)
    expect(policy).toContain("connect-src 'self'")
  })
})

describe("requestPaths", () => {
  it("maps directory indexes to their directory path", () => {
    expect(requestPaths("index.html")).toEqual(["/", "/index.html"])
    expect(requestPaths("docs/x/index.html")).toEqual([
      "/docs/x/",
      "/docs/x/index.html",
    ])
    expect(requestPaths("llms.txt")).toEqual(["/llms.txt"])
  })
})

describe("buildHeaders", () => {
  const files = [
    "index.html",
    "_shell.html",
    "docs/index.html",
    "docs/index.md",
    "llms.txt",
    "sw.js",
    "assets/index-abc.js",
    "assets/kul_wasm_bg-abc.wasm",
    "_redirects",
  ]
  const blocks = parseHeaders(buildHeaders(files, () => PAGE))
  const get = (path, name) => headersFor(blocks, path).get(name)?.value

  it("gives every path exactly one Cache-Control", () => {
    for (const path of [
      "/",
      "/docs/",
      "/docs/index.md",
      "/llms.txt",
      "/sw.js",
      "/app",
      "/app/abc",
      "/assets/index-abc.js",
      "/assets/kul_wasm_bg-abc.wasm",
    ]) {
      const value = get(path, "cache-control")
      expect(value, path).toBeDefined()
      expect(value, path).not.toContain(", no-cache")
    }
    expect(get("/sw.js", "cache-control")).toBe("no-cache")
    expect(get("/assets/index-abc.js", "cache-control")).toBe(
      "public, max-age=31536000, immutable"
    )
  })

  it("hashes each page's inline scripts into its one CSP", () => {
    for (const path of ["/", "/docs/", "/app", "/app/some/project"]) {
      const csp = get(path, "content-security-policy")
      expect(csp, path).toContain(scriptHash("boot()"))
      expect(csp, path).toContain(scriptHash("done()"))
    }
  })

  it("gives the service worker the policy without page hashes", () => {
    expect(get("/sw.js", "content-security-policy")).toBe(
      contentSecurityPolicy()
    )
    expect(get("/llms.txt", "content-security-policy")).toBeUndefined()
  })

  it("serves the WASM as application/wasm and Markdown as text/markdown", () => {
    expect(get("/assets/kul_wasm_bg-abc.wasm", "content-type")).toBe(
      "application/wasm"
    )
    expect(get("/docs/index.md", "content-type")).toMatch(/^text\/markdown/)
  })

  it("sends the security headers with every response", () => {
    for (const path of ["/", "/llms.txt", "/assets/index-abc.js"]) {
      expect(get(path, "referrer-policy")).toBe("no-referrer")
      expect(get(path, "x-content-type-options")).toBe("nosniff")
      expect(get(path, "permissions-policy")).toContain("camera=()")
    }
  })
})
