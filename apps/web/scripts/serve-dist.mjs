#!/usr/bin/env node
// Offline static server for the built site, with Netlify's `_redirects` and
// `_headers` semantics for the rules this site uses. End-to-end tests,
// Lighthouse and the deploy smoke check run against it, so what passes here
// is what Netlify serves.
//
//   node apps/web/scripts/serve-dist.mjs [--dir <dir>] [--port <n>] [--host <h>] [--log]
//
// --dir defaults to apps/web/dist/client, --port to 4173 (0 picks a free
// port), --host to 127.0.0.1. Files, `_redirects` and `_headers` are read on
// every request, so a rebuild is served without a restart.
//
// Static files
//   `/a/b.js` serves `a/b.js`. `/docs/` serves `docs/index.html`, and `/docs`
//   redirects 301 to `/docs/` when that file exists. `/page` serves
//   `page.html`. `_redirects` and `_headers` themselves are never served. A
//   miss serves `404.html` with status 404 when it exists.
//
// _redirects
//   `from [to] [status][!]`, first matching rule wins, `#` starts a comment.
//   - `from` is a path, or an absolute URL whose host must equal the request
//     host (the scheme is not compared). Paths match with and without a
//     trailing slash. `:name` matches one segment; a trailing `*` matches the
//     rest of the path, including nothing, and fills `:splat` in `to`.
//   - status 200 rewrites (the URL stays, the target's content is served),
//     3xx redirects with `Location`, 404 and 410 serve the target with that
//     status. The default is 301. The query string carries over to the target
//     unless `to` has its own.
//   - Shadowing: when a static file answers the path, only forced rules (`!`)
//     apply.
//   - Not supported, and rejected with an error naming the line: query or
//     condition matching (`key=value`), and proxying (200 to an absolute URL).
//
// _headers
//   A path pattern line followed by indented `Name: value` lines. `*` matches
//   any run of characters, `:name` at the start of a segment matches one
//   segment. Every block whose pattern matches the request path applies, in
//   file order; a header named more than once is joined with ", ". Patterns
//   match the path the browser requested, also for rewritten responses.
//   Custom headers replace the defaults (Content-Type, ETag) of the same name.
//
// Responses carry an ETag and answer If-None-Match with 304. Text, JSON,
// SVG and WASM bodies are compressed (br, else gzip) when the client accepts
// it.

import { createHash } from "node:crypto"
import { existsSync, readFileSync, statSync } from "node:fs"
import { createServer } from "node:http"
import { dirname, extname, join, normalize, resolve, sep } from "node:path"
import { fileURLToPath, pathToFileURL } from "node:url"
import { brotliCompressSync, constants as zlib, gzipSync } from "node:zlib"

const CONTENT_TYPES = {
  ".html": "text/html; charset=utf-8",
  ".js": "text/javascript; charset=utf-8",
  ".mjs": "text/javascript; charset=utf-8",
  ".css": "text/css; charset=utf-8",
  ".json": "application/json; charset=utf-8",
  ".map": "application/json; charset=utf-8",
  ".webmanifest": "application/manifest+json; charset=utf-8",
  ".txt": "text/plain; charset=utf-8",
  ".md": "text/markdown; charset=utf-8",
  ".xml": "application/xml; charset=utf-8",
  ".svg": "image/svg+xml",
  ".wasm": "application/wasm",
  ".png": "image/png",
  ".jpg": "image/jpeg",
  ".jpeg": "image/jpeg",
  ".gif": "image/gif",
  ".webp": "image/webp",
  ".avif": "image/avif",
  ".ico": "image/x-icon",
  ".woff": "font/woff",
  ".woff2": "font/woff2",
  ".zip": "application/zip",
}

const COMPRESSIBLE =
  /^(text\/|application\/(json|javascript|wasm|xml|manifest\+json)|image\/svg\+xml)/
const NEVER_SERVED = new Set(["/_redirects", "/_headers"])

// ---------------------------------------------------------------- _redirects

/** Parses a `_redirects` file into rules, in file order. */
export function parseRedirects(text) {
  const rules = []
  text.split(/\r?\n/).forEach((raw, index) => {
    const line = raw.trim()
    if (line === "" || line.startsWith("#")) return
    const fail = (why) => {
      throw new Error(`_redirects line ${index + 1}: ${why}: ${line}`)
    }
    const tokens = line.split(/\s+/)
    if (tokens.length < 2) fail("expected `from to [status]`")
    if (tokens.some((t) => t.includes("=") && !/^https?:\/\//.test(t))) {
      fail("query and condition matching is not supported")
    }
    if (tokens.length > 3) fail("expected `from to [status]`")
    const [fromRaw, to, statusRaw = "301"] = tokens
    const status = Number.parseInt(statusRaw, 10)
    const force = statusRaw.endsWith("!")
    if (!/^\d{3}!?$/.test(statusRaw)) fail(`bad status \`${statusRaw}\``)
    if (status === 200 && /^https?:\/\//.test(to))
      fail("proxying is not supported")
    let host = null
    let path = fromRaw
    if (/^https?:\/\//.test(fromRaw)) {
      const url = new URL(fromRaw.replace(/\*$/, "__splat__"))
      host = url.hostname
      path = url.pathname.replace(/__splat__$/, "*")
    }
    if (!path.startsWith("/")) fail("`from` must be a path or an absolute URL")
    const segments = trimSlash(path).split("/").slice(1)
    const splatAt = segments.indexOf("*")
    if (splatAt !== -1 && splatAt !== segments.length - 1)
      fail("`*` must end the path")
    rules.push({ host, segments, to, status, force, line: index + 1 })
  })
  return rules
}

function trimSlash(path) {
  return path.length > 1 && path.endsWith("/") ? path.slice(0, -1) : path
}

/** Placeholder values when `rule` matches, otherwise null. */
function matchRule(rule, hostname, pathname) {
  if (rule.host !== null && rule.host !== hostname) return null
  const parts = trimSlash(pathname).split("/").slice(1)
  const params = {}
  for (let i = 0; i < rule.segments.length; i++) {
    const seg = rule.segments[i]
    if (seg === "*") {
      params.splat = parts.slice(i).join("/")
      return params
    }
    if (i >= parts.length) return null
    if (seg.startsWith(":")) {
      if (parts[i] === "") return null
      params[seg.slice(1)] = parts[i]
    } else if (seg !== parts[i]) {
      return null
    }
  }
  return parts.length === rule.segments.length ? params : null
}

function fillTarget(to, params) {
  return to.replace(/:([A-Za-z_][A-Za-z0-9_]*)/g, (whole, name) =>
    name in params ? params[name] : whole
  )
}

// ---------------------------------------------------------------- _headers

/** Parses a `_headers` file into `{ pattern, regex, headers }` blocks. */
export function parseHeaders(text) {
  const blocks = []
  let current = null
  text.split(/\r?\n/).forEach((raw, index) => {
    if (raw.trim() === "" || raw.trim().startsWith("#")) return
    if (!/^\s/.test(raw)) {
      const pattern = raw.trim()
      current = { pattern, regex: headerPatternRegex(pattern), headers: [] }
      blocks.push(current)
      return
    }
    const colon = raw.indexOf(":")
    if (current === null || colon === -1) {
      throw new Error(
        `_headers line ${index + 1}: expected \`Name: value\` under a path: ${raw.trim()}`
      )
    }
    current.headers.push([
      raw.slice(0, colon).trim(),
      raw.slice(colon + 1).trim(),
    ])
  })
  return blocks
}

function headerPatternRegex(pattern) {
  const body = pattern
    .split("/")
    .map((seg) =>
      seg.startsWith(":")
        ? "[^/]+"
        : seg.replace(/[.+?^${}()|[\]\\]/g, "\\$&").replace(/\*/g, ".*")
    )
    .join("/")
  return new RegExp(`^${body}$`)
}

/** The custom headers for `pathname`, as a lower-cased name -> value map. */
export function headersFor(blocks, pathname) {
  const out = new Map()
  for (const block of blocks) {
    if (!block.regex.test(pathname)) continue
    for (const [name, value] of block.headers) {
      const key = name.toLowerCase()
      out.set(
        key,
        out.has(key)
          ? {
              name: out.get(key).name,
              value: `${out.get(key).value}, ${value}`,
            }
          : { name, value }
      )
    }
  }
  return out
}

// ---------------------------------------------------------------- static files

/** `{ file }`, `{ redirect }` (add the trailing slash) or null. */
function resolveStatic(root, pathname) {
  if (NEVER_SERVED.has(pathname)) return null
  let decoded
  try {
    decoded = decodeURIComponent(pathname)
  } catch {
    return null
  }
  const local = normalize(join(root, decoded))
  if (local !== root && !local.startsWith(root + sep)) return null
  const isFile = (p) => existsSync(p) && statSync(p).isFile()
  if (decoded.endsWith("/")) {
    if (isFile(join(local, "index.html")))
      return { file: join(local, "index.html") }
    if (decoded !== "/" && isFile(`${local}.html`))
      return { file: `${local}.html` }
    return null
  }
  if (isFile(local)) return { file: local }
  if (isFile(join(local, "index.html"))) return { redirect: `${pathname}/` }
  if (isFile(`${local}.html`)) return { file: `${local}.html` }
  return null
}

function readOptional(path) {
  return existsSync(path) ? readFileSync(path, "utf8") : ""
}

// ---------------------------------------------------------------- server

/** An http.Server that serves `root` the way Netlify serves the deploy. */
export function createDistServer({ root, log = false }) {
  const dir = resolve(root)
  return createServer((req, res) => {
    try {
      handle(dir, req, res)
    } catch (error) {
      res.writeHead(500, { "content-type": "text/plain; charset=utf-8" })
      res.end(`serve-dist: ${error.message}\n`)
    }
    if (log) console.log(`${req.method} ${req.url} -> ${res.statusCode}`)
  })
}

function handle(dir, req, res) {
  if (req.method !== "GET" && req.method !== "HEAD") {
    res.writeHead(405, { allow: "GET, HEAD" })
    res.end()
    return
  }
  const url = new URL(req.url, "http://localhost")
  const hostname = (req.headers.host ?? "localhost").replace(/:\d+$/, "")
  const rules = parseRedirects(readOptional(join(dir, "_redirects")))
  const custom = headersFor(
    parseHeaders(readOptional(join(dir, "_headers"))),
    url.pathname
  )
  const found = resolveStatic(dir, url.pathname)

  for (const rule of rules) {
    if (found && !rule.force) continue
    const params = matchRule(rule, hostname, url.pathname)
    if (!params) continue
    let target = fillTarget(rule.to, params)
    if (url.search && !target.includes("?")) target += url.search
    if (rule.status >= 300 && rule.status < 400) {
      send(req, res, rule.status, { location: target }, null, custom)
      return
    }
    const targetPath = new URL(target, "http://localhost").pathname
    const resolved = resolveStatic(dir, targetPath)
    if (resolved?.file) {
      sendFile(req, res, rule.status, resolved.file, custom)
      return
    }
    break
  }

  if (found?.file) {
    sendFile(req, res, 200, found.file, custom)
  } else if (found?.redirect) {
    send(req, res, 301, { location: found.redirect + url.search }, null, custom)
  } else if (existsSync(join(dir, "404.html"))) {
    sendFile(req, res, 404, join(dir, "404.html"), custom)
  } else {
    send(
      req,
      res,
      404,
      { "content-type": "text/plain; charset=utf-8" },
      Buffer.from("Not found\n"),
      custom
    )
  }
}

function sendFile(req, res, status, file, custom) {
  const body = readFileSync(file)
  const type =
    CONTENT_TYPES[extname(file).toLowerCase()] ?? "application/octet-stream"
  const etag = `"${createHash("sha1").update(body).digest("hex")}"`
  if (status === 200 && req.headers["if-none-match"] === etag) {
    send(req, res, 304, { etag }, null, custom)
    return
  }
  send(req, res, status, { "content-type": type, etag }, body, custom)
}

function send(req, res, status, defaults, body, custom) {
  const headers = { ...defaults }
  let payload = body
  const type = headers["content-type"] ?? ""
  const accept = String(req.headers["accept-encoding"] ?? "")
  if (payload && payload.length > 0 && COMPRESSIBLE.test(type)) {
    headers.vary = "Accept-Encoding"
    if (/\bbr\b/.test(accept)) {
      payload = brotliCompressSync(payload, {
        params: { [zlib.BROTLI_PARAM_QUALITY]: 5 },
      })
      headers["content-encoding"] = "br"
    } else if (/\bgzip\b/.test(accept)) {
      payload = gzipSync(payload)
      headers["content-encoding"] = "gzip"
    }
  }
  for (const [key, { value }] of custom) headers[key] = value
  if (payload) headers["content-length"] = String(payload.length)
  res.writeHead(status, headers)
  res.end(req.method === "HEAD" ? undefined : (payload ?? undefined))
}

// ---------------------------------------------------------------- CLI

function parseArgs(argv) {
  const here = dirname(fileURLToPath(import.meta.url))
  const opts = {
    dir: resolve(here, "../dist/client"),
    port: 4173,
    host: "127.0.0.1",
    log: false,
  }
  for (let i = 0; i < argv.length; i++) {
    const arg = argv[i]
    if (arg === "--dir") opts.dir = resolve(argv[++i])
    else if (arg === "--port") opts.port = Number.parseInt(argv[++i], 10)
    else if (arg === "--host") opts.host = argv[++i]
    else if (arg === "--log") opts.log = true
    else throw new Error(`unknown argument ${arg}`)
  }
  return opts
}

if (import.meta.url === pathToFileURL(process.argv[1] ?? "").href) {
  const opts = parseArgs(process.argv.slice(2))
  if (!existsSync(opts.dir)) {
    console.error(
      `serve-dist: ${opts.dir} does not exist; build the site first`
    )
    process.exit(1)
  }
  const server = createDistServer({ root: opts.dir, log: opts.log })
  server.listen(opts.port, opts.host, () => {
    const { port } = server.address()
    console.log(
      `serve-dist: serving ${opts.dir} at http://${opts.host}:${port}/`
    )
  })
}
