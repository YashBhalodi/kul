// Unit tests for serve-dist.mjs: the `_redirects` and `_headers` rules the
// site relies on, served from a fixture directory. Run with
// `node --test apps/web/scripts/serve-dist.test.mjs` (part of `just check`).

import assert from "node:assert/strict"
import { mkdirSync, mkdtempSync, rmSync, writeFileSync } from "node:fs"
import { request } from "node:http"
import { tmpdir } from "node:os"
import { dirname, join } from "node:path"
import { after, before, describe, test } from "node:test"
import { gunzipSync } from "node:zlib"

import {
  createDistServer,
  headersFor,
  matchRedirect,
  parseHeaders,
  parseRedirects,
} from "./serve-dist.mjs"

const REDIRECTS = `
# canonical host
https://www.kullang.dev/*  https://kullang.dev/:splat  301!
/old-docs/:slug  /docs/:slug/  301
/app/*  /_shell.html  200
/forced  /target.txt  200!
/shadowed  /target.txt  200
`

const HEADERS = `
/*
  X-Content-Type-Options: nosniff
  Content-Security-Policy: default-src 'self'; connect-src 'self'
/assets/*
  Cache-Control: public, max-age=31536000, immutable
/_shell.html
  Cache-Control: no-cache
/app/*
  Cache-Control: no-cache
/*.wasm
  Content-Type: application/wasm
/multi
  X-Multi: one
  X-Multi: two
`

const FILES = {
  "index.html": "<!doctype html><title>home</title>",
  "_shell.html": "<!doctype html><title>shell</title>",
  "404.html": "<!doctype html><title>missing</title>",
  "docs/index.html": "<!doctype html><title>docs</title>",
  "docs/guide/index.html": "<!doctype html><title>guide</title>",
  "llms.txt": "# Kul\n",
  "page.html": "<!doctype html><title>page</title>",
  "assets/app-1234.js": `console.log(${JSON.stringify("x".repeat(2000))});`,
  "assets/kul_wasm_bg-abcd.wasm": Buffer.from([0, 97, 115, 109, 1, 0, 0, 0]),
  shadowed: "the static file",
  forced: "the static file",
  "target.txt": "the rewrite target",
  _redirects: REDIRECTS,
  _headers: HEADERS,
}

let root
let server
let port

before(async () => {
  root = mkdtempSync(join(tmpdir(), "serve-dist-"))
  for (const [name, body] of Object.entries(FILES)) {
    mkdirSync(dirname(join(root, name)), { recursive: true })
    writeFileSync(join(root, name), body)
  }
  server = createDistServer({ root })
  await new Promise((done) => server.listen(0, "127.0.0.1", done))
  port = server.address().port
})

after(() => {
  server.close()
  rmSync(root, { recursive: true, force: true })
})

function get(path, headers = {}) {
  return new Promise((done, fail) => {
    const req = request({ host: "127.0.0.1", port, path, headers }, (res) => {
      const chunks = []
      res.on("data", (c) => chunks.push(c))
      res.on("end", () =>
        done({
          status: res.statusCode,
          headers: res.headers,
          body: Buffer.concat(chunks),
        })
      )
    })
    req.on("error", fail)
    req.end()
  })
}

describe("static files", () => {
  test("serves the root index", async () => {
    const res = await get("/")
    assert.equal(res.status, 200)
    assert.match(res.body.toString(), /home/)
    assert.equal(res.headers["content-type"], "text/html; charset=utf-8")
  })

  test("serves a directory index with a trailing slash", async () => {
    const res = await get("/docs/guide/")
    assert.equal(res.status, 200)
    assert.match(res.body.toString(), /guide/)
  })

  test("adds the trailing slash to a directory with a 301", async () => {
    const res = await get("/docs/guide?q=1")
    assert.equal(res.status, 301)
    assert.equal(res.headers.location, "/docs/guide/?q=1")
  })

  test("redirects an `index` path to its directory", async () => {
    for (const [path, location] of [
      ["/index", "/"],
      ["/docs/index", "/docs/"],
      ["/docs/guide/index/?q=1", "/docs/guide/?q=1"],
    ]) {
      const res = await get(path)
      assert.equal(res.status, 301, path)
      assert.equal(res.headers.location, location, path)
    }
  })

  test("a directory redirect carries no _headers headers", async () => {
    const res = await get("/docs/guide")
    assert.equal(res.headers["x-content-type-options"], undefined)
    assert.equal(res.headers["content-security-policy"], undefined)
  })

  test("serves `page.html` at `/page`", async () => {
    const res = await get("/page")
    assert.equal(res.status, 200)
    assert.match(res.body.toString(), /page/)
  })

  test("serves llms.txt as text, not as a directory", async () => {
    const res = await get("/llms.txt")
    assert.equal(res.status, 200)
    assert.equal(res.headers["content-type"], "text/plain; charset=utf-8")
  })

  test("a miss serves 404.html with status 404", async () => {
    const res = await get("/nope")
    assert.equal(res.status, 404)
    assert.match(res.body.toString(), /missing/)
  })

  test("never serves the rule files", async () => {
    assert.equal((await get("/_redirects")).status, 404)
    assert.equal((await get("/_headers")).status, 404)
  })

  test("rejects path traversal", async () => {
    assert.equal((await get("/../../etc/passwd")).status, 404)
    assert.equal((await get("/%2e%2e/%2e%2e/etc/passwd")).status, 404)
  })

  test("answers If-None-Match with 304", async () => {
    const first = await get("/llms.txt")
    const second = await get("/llms.txt", {
      "if-none-match": first.headers.etag,
    })
    assert.equal(second.status, 304)
  })

  test("compresses text when the client accepts it", async () => {
    const res = await get("/assets/app-1234.js", { "accept-encoding": "gzip" })
    assert.equal(res.headers["content-encoding"], "gzip")
    assert.match(gunzipSync(res.body).toString(), /^console\.log/)
    const br = await get("/assets/app-1234.js", {
      "accept-encoding": "gzip, br",
    })
    assert.equal(br.headers["content-encoding"], "br")
  })
})

describe("_redirects", () => {
  test("rewrites a deep app link to the shell with status 200", async () => {
    for (const path of ["/app/abc", "/app/abc/def", "/app/", "/app"]) {
      const res = await get(path)
      assert.equal(res.status, 200, path)
      assert.match(res.body.toString(), /shell/, path)
    }
  })

  test("a forced rule wins over an existing file", async () => {
    const res = await get("/forced")
    assert.equal(res.status, 200)
    assert.equal(res.body.toString(), "the rewrite target")
  })

  test("an existing file shadows an unforced rule", async () => {
    const res = await get("/shadowed")
    assert.equal(res.body.toString(), "the static file")
  })

  test("a host rule redirects 301 with the splat", async () => {
    for (const [path, location] of [
      ["/", "https://kullang.dev/"],
      ["/docs/guide/?x=1", "https://kullang.dev/docs/guide/?x=1"],
      ["/docs/guide", "https://kullang.dev/docs/guide"],
      ["/llms.txt", "https://kullang.dev/llms.txt"],
    ]) {
      const res = await get(path, { host: "www.kullang.dev" })
      assert.equal(res.status, 301, path)
      assert.equal(res.headers.location, location, path)
    }
  })

  test("a rule's redirect carries the _headers headers", async () => {
    const res = await get("/old-docs/adoption")
    assert.equal(res.headers["x-content-type-options"], "nosniff")
  })

  test("a host rule leaves other hosts alone", async () => {
    for (const host of ["kullang.dev", "pr-1--www.kullang.dev"]) {
      const res = await get("/docs/guide/", { host })
      assert.equal(res.status, 200, host)
    }
  })

  test("matchRedirect returns the first rule that applies", () => {
    const rules = parseRedirects(REDIRECTS)
    const at = (pathname, shadowed, hostname = "127.0.0.1") =>
      matchRedirect(rules, { hostname, pathname, search: "", shadowed })
    assert.equal(at("/app/x", false)?.target, "/_shell.html")
    assert.equal(at("/shadowed", true), null)
    assert.equal(at("/forced", true)?.target, "/target.txt")
    assert.equal(
      at("/llms.txt", true, "www.kullang.dev")?.target,
      "https://kullang.dev/llms.txt"
    )
  })

  test("placeholders fill the target", async () => {
    const res = await get("/old-docs/adoption")
    assert.equal(res.status, 301)
    assert.equal(res.headers.location, "/docs/adoption/")
  })

  test("parses status, force and host", () => {
    const [host, placeholder, rewrite] = parseRedirects(REDIRECTS)
    assert.deepEqual(
      { host: host.host, status: host.status, force: host.force },
      { host: "www.kullang.dev", status: 301, force: true }
    )
    assert.equal(placeholder.status, 301)
    assert.deepEqual(
      { status: rewrite.status, force: rewrite.force },
      { status: 200, force: false }
    )
  })

  test("rejects rules it cannot emulate", () => {
    assert.throws(
      () => parseRedirects("/store id=:id /blog/:id 301"),
      /not supported/
    )
    assert.throws(
      () => parseRedirects("/api/* https://api.example.com/:splat 200"),
      /proxying/
    )
    assert.throws(() => parseRedirects("/a/*/b /c 301"), /must end/)
  })
})

describe("_headers", () => {
  test("applies every matching block", async () => {
    const res = await get("/assets/app-1234.js")
    assert.equal(res.headers["x-content-type-options"], "nosniff")
    assert.equal(
      res.headers["cache-control"],
      "public, max-age=31536000, immutable"
    )
    assert.match(res.headers["content-security-policy"], /connect-src 'self'/)
  })

  test("matches the requested path for a rewritten response", async () => {
    const res = await get("/app/abc")
    assert.equal(res.headers["cache-control"], "no-cache")
  })

  test("a custom Content-Type replaces the default", async () => {
    const res = await get("/assets/kul_wasm_bg-abcd.wasm")
    assert.equal(res.headers["content-type"], "application/wasm")
  })

  test("joins a header named twice", () => {
    const merged = headersFor(parseHeaders(HEADERS), "/multi")
    assert.equal(merged.get("x-multi").value, "one, two")
  })

  test("placeholders match one segment", () => {
    const blocks = parseHeaders("/docs/:page\n  X-Page: yes\n")
    assert.equal(
      headersFor(blocks, "/docs/adoption").get("x-page")?.value,
      "yes"
    )
    assert.equal(headersFor(blocks, "/docs/a/b").get("x-page"), undefined)
  })

  test("rejects a header line outside a block", () => {
    assert.throws(() => parseHeaders("  X-Orphan: 1\n"), /expected/)
  })
})
