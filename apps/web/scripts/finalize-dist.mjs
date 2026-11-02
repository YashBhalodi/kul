#!/usr/bin/env node
// Writes dist/client/_headers after `vite build`. The Content-Security-Policy
// allows no inline script, but every prerendered page carries the router's
// own inline bootstrap scripts, so each HTML file gets a policy that lists
// the SHA-256 hashes of exactly its inline scripts. Netlify (and serve-dist)
// apply every `_headers` block whose pattern matches, so the blocks here
// never overlap: one per file outside /assets/, and one for /assets/*.
//
//   node scripts/finalize-dist.mjs [--dir <dir>]
//
// Headers:
//   - every response: Referrer-Policy, X-Content-Type-Options and a
//     Permissions-Policy denying camera, microphone and geolocation;
//   - HTML (and /app/*, which _redirects serves /_shell.html for): the CSP
//     with that file's script hashes, and Cache-Control: no-cache;
//   - other files outside /assets/ (llms.txt, the .md variants, the search
//     index, the favicon, sw.js): Cache-Control: no-cache, and for a script
//     (sw.js) the CSP, which governs everything the service worker fetches;
//   - /assets/* (content-hashed by Vite): immutable for a year, and the CSP,
//     which governs the language worker's script; *.wasm is application/wasm.

import { createHash } from "node:crypto"
import { readdirSync, readFileSync, writeFileSync } from "node:fs"
import { join, relative, sep } from "node:path"
import { fileURLToPath, pathToFileURL } from "node:url"
import { parseArgs } from "node:util"

export const BASE_CSP = [
  ["default-src", "'self'"],
  ["script-src", "'self'", "'wasm-unsafe-eval'"],
  ["style-src", "'self'", "'unsafe-inline'"],
  ["img-src", "'self'", "data:", "blob:"],
  ["font-src", "'self'"],
  ["connect-src", "'self'"],
  ["worker-src", "'self'", "blob:"],
  ["manifest-src", "'self'"],
  ["frame-ancestors", "'none'"],
  ["base-uri", "'self'"],
  ["form-action", "'none'"],
  ["object-src", "'none'"],
]

const SECURITY_HEADERS = [
  ["Referrer-Policy", "no-referrer"],
  ["X-Content-Type-Options", "nosniff"],
  ["Permissions-Policy", "camera=(), microphone=(), geolocation=()"],
]

const NO_CACHE = ["Cache-Control", "no-cache"]
const IMMUTABLE = ["Cache-Control", "public, max-age=31536000, immutable"]

const CONTENT_TYPES = {
  ".md": "text/markdown; charset=utf-8",
  ".txt": "text/plain; charset=utf-8",
}

/** The text of every inline (src-less) <script> element, in order. */
export function inlineScripts(html) {
  const scripts = []
  for (const match of html.matchAll(
    /<script\b([^>]*)>([\s\S]*?)<\/script>/gi
  )) {
    const [, attributes, body] = match
    if (/\bsrc\s*=/i.test(attributes)) continue
    if (/\btype\s*=\s*["']?application\/(ld\+)?json/i.test(attributes)) continue
    scripts.push(body)
  }
  return scripts
}

/** A CSP source expression for a script body: 'sha256-<base64>'. */
export function scriptHash(body) {
  return `'sha256-${createHash("sha256").update(body, "utf8").digest("base64")}'`
}

/** The policy, with these script hashes added to script-src. */
export function contentSecurityPolicy(hashes = []) {
  return BASE_CSP.map(([directive, ...sources]) => {
    const all =
      directive === "script-src" ? [...sources, ...new Set(hashes)] : sources
    return [directive, ...all].join(" ")
  }).join("; ")
}

/** The request paths a built file answers: `docs/x/index.html` → `/docs/x/`. */
export function requestPaths(file) {
  const path = `/${file}`
  if (path === "/index.html") return ["/", path]
  if (path.endsWith("/index.html"))
    return [path.slice(0, -"index.html".length), path]
  return [path]
}

function block(patterns, headers) {
  const lines = []
  for (const pattern of patterns) {
    lines.push(pattern)
    for (const [name, value] of headers) lines.push(`  ${name}: ${value}`)
  }
  return lines.join("\n")
}

/**
 * The `_headers` text for a build, given its files (paths relative to the
 * root, with `/` separators) and a reader for their contents.
 */
export function buildHeaders(files, read) {
  const blocks = [
    block(
      ["/assets/*"],
      [
        ...SECURITY_HEADERS,
        IMMUTABLE,
        ["Content-Security-Policy", contentSecurityPolicy()],
      ]
    ),
    block(["/assets/*.wasm"], [["Content-Type", "application/wasm"]]),
  ]
  for (const file of [...files].sort()) {
    if (
      file.startsWith("assets/") ||
      file === "_headers" ||
      file === "_redirects"
    )
      continue
    const headers = [...SECURITY_HEADERS, NO_CACHE]
    const patterns = requestPaths(file)
    if (file.endsWith(".html")) {
      const hashes = inlineScripts(read(file)).map(scriptHash)
      headers.push(["Content-Security-Policy", contentSecurityPolicy(hashes)])
      if (file === "_shell.html") patterns.push("/app", "/app/*")
    } else {
      const extension = file.slice(file.lastIndexOf("."))
      const contentType = CONTENT_TYPES[extension]
      if (contentType) headers.push(["Content-Type", contentType])
      if (extension === ".js")
        headers.push(["Content-Security-Policy", contentSecurityPolicy()])
    }
    blocks.push(block(patterns, headers))
  }
  return `# Generated by scripts/finalize-dist.mjs. Do not edit.\n\n${blocks.join("\n\n")}\n`
}

/** Every file under `root`, relative, with `/` separators. */
export function listFiles(root) {
  return readdirSync(root, { recursive: true, withFileTypes: true })
    .filter((entry) => entry.isFile())
    .map((entry) =>
      relative(root, join(entry.parentPath, entry.name)).split(sep).join("/")
    )
}

function main() {
  const here = fileURLToPath(new URL(".", import.meta.url))
  const { values } = parseArgs({
    options: {
      dir: { type: "string", default: join(here, "..", "dist", "client") },
    },
  })
  const root = values.dir
  const text = buildHeaders(listFiles(root), (file) =>
    readFileSync(join(root, file), "utf8")
  )
  writeFileSync(join(root, "_headers"), text)
  console.log(`finalize-dist: wrote ${join(root, "_headers")}`)
}

if (import.meta.url === pathToFileURL(process.argv[1] ?? "").href) main()
