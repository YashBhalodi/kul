// The Netlify rules a build must carry, checked over its parsed `_redirects`
// and `_headers` (check-dist runs these on every build):
//
//   - one canonical host: https://www.kullang.dev and the site's production
//     netlify.app host redirect 301 to https://kullang.dev, files included,
//     while the canonical host, deploy previews, deploy permalinks and local
//     servers are never sent off-site;
//   - every /app path is served the SPA shell with status 200;
//   - every HTML response, /app paths included, carries one
//     Content-Security-Policy that hashes each of its inline scripts and
//     keeps every request same-origin;
//   - pages and other files outside /assets/ are revalidated (no-cache),
//     content-hashed /assets/* are immutable, the WASM is application/wasm,
//     the text variants are text/markdown and text/plain;
//   - every response carries the security headers.
//
// The paths are matched the way Netlify matches them (serve-dist.mjs).

import { SW_FILE } from "./build-sw.mjs"
import { inlineScripts, scriptHash } from "./finalize-dist.mjs"
import { headersFor, matchRedirect } from "./serve-dist.mjs"

export const CANONICAL_HOST = "kullang.dev"
/** The Netlify site; its production host is `<site>.netlify.app`. */
export const NETLIFY_SITE = "kullang-app"

/** Hosts that redirect to the canonical host. */
export const ALIAS_HOSTS = [
  `www.${CANONICAL_HOST}`,
  `${NETLIFY_SITE}.netlify.app`,
]

/** Hosts that serve the site as they are. */
export const SERVING_HOSTS = [
  CANONICAL_HOST,
  `pr-357--${NETLIFY_SITE}.netlify.app`,
  `0123456789abcdef01234567--${NETLIFY_SITE}.netlify.app`,
  "127.0.0.1",
  "localhost",
]

const SECURITY_HEADERS = {
  "referrer-policy": "no-referrer",
  "x-content-type-options": "nosniff",
}

const TEXT_TYPES = {
  ".md": "text/markdown; charset=utf-8",
  ".txt": "text/plain; charset=utf-8",
}

/** The request path that serves a built file: `docs/index.html` → `/docs/`. */
function servedPath(file) {
  if (file === "index.html") return "/"
  return file.endsWith("/index.html")
    ? `/${file.slice(0, -"index.html".length)}`
    : `/${file}`
}

/** True when a built file answers `pathname` (so unforced rules are shadowed). */
function hasFile(files, pathname) {
  if (pathname.endsWith("/")) return files.has(`${pathname.slice(1)}index.html`)
  return files.has(pathname.slice(1))
}

/**
 * Problems with the parsed `_redirects` rules for a build whose served
 * files are `files` (paths relative to the root).
 */
export function redirectProblems(rules, files) {
  const problems = []
  const route = (hostname, pathname, search = "") =>
    matchRedirect(rules, {
      hostname,
      pathname,
      search,
      shadowed: hasFile(files, pathname),
    })

  // A sample of every kind of path: pages, a text file, an asset, the app.
  const asset = [...files].find((file) => file.startsWith("assets/"))
  const samples = ["/", "/docs/", "/docs", "/llms.txt", "/app", "/app/x/y"]
  if (asset) samples.push(`/${asset}`)

  for (const host of ALIAS_HOSTS) {
    for (const path of samples) {
      const match = route(host, path)
      const expected = `https://${CANONICAL_HOST}${path}`
      if (match?.rule.status !== 301 || match.target !== expected) {
        problems.push(
          `https://${host}${path} does not redirect 301 to ${expected}`
        )
      }
    }
    const query = route(host, "/docs/", "?q=1")?.target
    if (query !== `https://${CANONICAL_HOST}/docs/?q=1`) {
      problems.push(`https://${host}/docs/?q=1 loses its query string`)
    }
  }

  for (const host of SERVING_HOSTS) {
    for (const path of samples) {
      const match = route(host, path)
      if (match && /^https?:\/\//.test(match.target)) {
        problems.push(`${host}${path} is sent off-site to ${match.target}`)
      }
    }
    for (const path of ["/app", "/app/", "/app/x", "/app/x/y/"]) {
      const match = route(host, path)
      if (match?.rule.status !== 200 || match.target !== "/_shell.html") {
        problems.push(`${host}${path} is not served /_shell.html with 200`)
      }
    }
  }
  return problems
}

/**
 * Problems with the parsed `_headers` blocks for a build whose served files
 * are `files`; `read` returns a file's text.
 */
export function headerProblems(blocks, files, read) {
  const problems = []
  const headers = (path) => headersFor(blocks, path)
  const value = (path, name) => headers(path).get(name)?.value

  const shellCsp = value("/_shell.html", "content-security-policy")
  for (const path of ["/app", "/app/", "/app/x/y"]) {
    if (!shellCsp || value(path, "content-security-policy") !== shellCsp)
      problems.push(`${path} does not get the shell's Content-Security-Policy`)
  }

  const served = [...files].filter(
    (file) => file !== "_headers" && file !== "_redirects"
  )
  for (const file of served) {
    const path = servedPath(file)
    const all = headers(path)
    for (const [name, expected] of Object.entries(SECURITY_HEADERS)) {
      if (all.get(name)?.value !== expected)
        problems.push(`${path} lacks ${name}: ${expected}`)
    }
    if (!all.get("permissions-policy"))
      problems.push(`${path} lacks a Permissions-Policy`)

    const cache = all.get("cache-control")?.value
    const expectedCache = file.startsWith("assets/")
      ? "public, max-age=31536000, immutable"
      : "no-cache"
    if (cache !== expectedCache)
      problems.push(
        `${path} is cached as ${cache ?? "nothing"}, not ${expectedCache}`
      )

    const type = all.get("content-type")?.value
    const extension = file.slice(file.lastIndexOf("."))
    if (extension === ".wasm" && type !== "application/wasm")
      problems.push(
        `${path} is served as ${type ?? "nothing"}, not application/wasm`
      )
    if (TEXT_TYPES[extension] && type !== TEXT_TYPES[extension])
      problems.push(
        `${path} is served as ${type ?? "nothing"}, not ${TEXT_TYPES[extension]}`
      )

    const csp = all.get("content-security-policy")?.value
    if (file.endsWith(".html") || file === SW_FILE) {
      if (!csp) {
        problems.push(`${path} has no Content-Security-Policy`)
        continue
      }
      if (csp.includes(", "))
        problems.push(`${path} gets more than one Content-Security-Policy`)
      if (!/(^|; )connect-src 'self'(;|$)/.test(csp))
        problems.push(`${path}: the CSP does not keep requests same-origin`)
      if (/script-src[^;]*'unsafe-inline'/.test(csp))
        problems.push(`${path}: the CSP allows inline scripts`)
    }
    if (file.endsWith(".html")) {
      const html = read(file)
      for (const body of inlineScripts(html)) {
        if (!csp.includes(scriptHash(body)))
          problems.push(`${path}: an inline script is not hashed in its CSP`)
      }
      if (/<script[^>]+src=["']data:/i.test(html))
        problems.push(
          `${path} loads a script from a data: URL, which the CSP blocks`
        )
    }
  }
  return problems
}
