#!/usr/bin/env node
// Checks the production build's static layout after `npm run build`: every
// prerendered page, the SPA shell, the llms files, the docs' Markdown
// variants, the search index, the service worker and the Netlify config sit
// at their exact paths, nothing that must stay client-only was prerendered,
// the service worker precaches every file the site serves, the landing page
// shows what the engine makes of its examples, and the Open Graph image,
// favicons, robots.txt and sitemap.xml are in place.
//
//   node scripts/check-dist.mjs [--dir <dir>]
//
// Prints one line per problem and exits 1 when there is any.

import { existsSync, readFileSync } from "node:fs"
import { join } from "node:path"
import { fileURLToPath, pathToFileURL } from "node:url"
import { parseArgs } from "node:util"

import { SW_FILE, precachedUrls } from "./build-sw.mjs"
import { landingProblems, siteFileProblems } from "./check-landing.mjs"
import { inlineScripts, listFiles, scriptHash } from "./finalize-dist.mjs"
import { headersFor, parseHeaders, parseRedirects } from "./serve-dist.mjs"

const SITE_ORIGIN = "https://kullang.dev"

/**
 * Every problem with the build at `root`, as human-readable lines. The
 * landing page is checked against `landing`, the content the engine makes
 * (src/landing/landing-content.ts).
 */
export function checkDist(root, landing) {
  const problems = []
  const files = new Set(listFiles(root))
  const read = (file) => readFileSync(join(root, file), "utf8")
  const requireFile = (file, why) => {
    if (!files.has(file)) problems.push(`missing ${file} (${why})`)
    return files.has(file)
  }

  if (requireFile("index.html", "the landing page")) {
    problems.push(...landingProblems(read("index.html"), landing))
  }
  problems.push(
    ...siteFileProblems(
      [...files],
      (file) => readFileSync(join(root, file)),
      SITE_ORIGIN
    )
  )
  requireFile("_shell.html", "the SPA shell for /app/*")
  requireFile("docs/index.html", "the docs home")
  requireFile("docs/index.md", "the docs home as Markdown")
  requireFile("_redirects", "Netlify redirects")
  requireFile("_headers", "Netlify headers")
  for (const file of ["llms.txt", "llms-full.txt"]) {
    if (requireFile(file, "llms.txt convention") && read(file).trim() === "") {
      problems.push(`${file} is empty`)
    }
    if (files.has(`${file}/index.html`)) {
      problems.push(`${file} was emitted as a page (${file}/index.html)`)
    }
  }
  if (requireFile("docs/search.json", "the docs search index")) {
    try {
      JSON.parse(read("docs/search.json"))
    } catch (error) {
      problems.push(`docs/search.json is not JSON: ${error.message}`)
    }
  }

  // Each docs page has its Markdown variant, and each variant its page.
  const docsPages = [...files].filter((file) =>
    /^docs\/.+\/index\.html$/.test(file)
  )
  const markdown = [...files].filter(
    (file) => /^docs\/.+\.md$/.test(file) && file !== "docs/index.md"
  )
  if (docsPages.length === 0)
    problems.push("no docs page under docs/<slug>/index.html")
  for (const page of docsPages) {
    const slug = page.slice("docs/".length, -"/index.html".length)
    requireFile(`docs/${slug}.md`, `the Markdown variant of /docs/${slug}/`)
  }
  for (const variant of markdown) {
    const slug = variant.slice("docs/".length, -".md".length)
    if (slug !== "index")
      requireFile(`docs/${slug}/index.html`, `the page of ${variant}`)
  }

  // The service worker precaches every file the site serves: by name, the
  // shell, the .wasm, the language worker's chunk and every docs page.
  if (requireFile(SW_FILE, "the service worker")) {
    const precached = new Set(precachedUrls(read(SW_FILE)))
    const unserved = new Set([SW_FILE, "_headers", "_redirects"])
    for (const file of files) {
      if (unserved.has(file) || file.endsWith(".map")) continue
      if (!precached.has(`/${file}`))
        problems.push(`${SW_FILE} does not precache /${file}`)
    }
    for (const url of precached) {
      if (!files.has(url.slice(1)))
        problems.push(`${SW_FILE} precaches ${url}, which is not in the build`)
    }
    const named = [
      ["the SPA shell", /^\/_shell\.html$/],
      ["the .wasm", /^\/assets\/kul_wasm_bg-[\w-]+\.wasm$/],
      ["the language worker", /^\/assets\/worker-[\w-]+\.js$/],
      ["the docs search index", /^\/docs\/search\.json$/],
    ]
    for (const [what, pattern] of named) {
      if (![...precached].some((url) => pattern.test(url)))
        problems.push(`${SW_FILE} does not precache ${what}`)
    }
    for (const page of ["docs/index.html", ...docsPages]) {
      if (!precached.has(`/${page}`))
        problems.push(`${SW_FILE} does not precache the docs page /${page}`)
    }
  }

  // The app is client-only: only the shell renders it.
  for (const file of files) {
    if (file === "app/index.html" || file.startsWith("app/")) {
      problems.push(`${file} was prerendered; /app must be client-only`)
    }
  }

  // Every llms.txt link to this site resolves to a built file.
  if (files.has("llms.txt")) {
    for (const [, url] of read("llms.txt").matchAll(/\]\(([^)]+)\)/g)) {
      if (!url.startsWith(SITE_ORIGIN)) continue
      const file = url.slice(SITE_ORIGIN.length + 1)
      if (!files.has(file))
        problems.push(`llms.txt links ${url}, which is not in the build`)
    }
  }

  // /app/* rewrites to the shell.
  if (files.has("_redirects")) {
    const rules = parseRedirects(read("_redirects"))
    const app = rules.find((rule) => rule.segments.join("/") === "app/*")
    if (!app || app.to !== "/_shell.html" || app.status !== 200) {
      problems.push("_redirects lacks `/app/* /_shell.html 200`")
    }
  }

  // Every HTML file carries one CSP, which hashes each of its inline scripts.
  if (files.has("_headers")) {
    const blocks = parseHeaders(read("_headers"))
    for (const file of files) {
      if (!file.endsWith(".html")) continue
      const path =
        file === "index.html" ? "/" : `/${file.replace(/index\.html$/, "")}`
      const csp = headersFor(blocks, path).get("content-security-policy")?.value
      if (!csp) {
        problems.push(`${path} has no Content-Security-Policy`)
        continue
      }
      if (csp.includes(", "))
        problems.push(`${path} gets more than one Content-Security-Policy`)
      for (const body of inlineScripts(read(file))) {
        if (!csp.includes(scriptHash(body))) {
          problems.push(`${path}: an inline script is not hashed in its CSP`)
        }
      }
      if (/<script[^>]+src=["']data:/i.test(read(file))) {
        problems.push(
          `${path} loads a script from a data: URL, which the CSP blocks`
        )
      }
    }
  }
  // The text variants are served as text: llms files as plain text, the
  // docs' Markdown variants as Markdown.
  if (files.has("_headers")) {
    const blocks = parseHeaders(read("_headers"))
    for (const file of files) {
      const expected = file.endsWith(".md")
        ? "text/markdown; charset=utf-8"
        : file.endsWith(".txt")
          ? "text/plain; charset=utf-8"
          : undefined
      if (!expected) continue
      const type = headersFor(blocks, `/${file}`).get("content-type")?.value
      if (type !== expected)
        problems.push(
          `/${file} is served as ${type ?? "nothing"}, not ${expected}`
        )
    }
  }
  // Browsers revalidate the service worker on every check, under the CSP.
  if (files.has("_headers") && files.has(SW_FILE)) {
    const headers = headersFor(parseHeaders(read("_headers")), `/${SW_FILE}`)
    if (headers.get("cache-control")?.value !== "no-cache")
      problems.push(`/${SW_FILE} is not served with Cache-Control: no-cache`)
    if (!headers.get("content-security-policy"))
      problems.push(`/${SW_FILE} has no Content-Security-Policy`)
  }
  return problems
}

async function main() {
  const here = fileURLToPath(new URL(".", import.meta.url))
  const { values } = parseArgs({
    options: {
      dir: { type: "string", default: join(here, "..", "dist", "client") },
    },
  })
  if (!existsSync(values.dir)) {
    console.error(
      `check-dist: ${values.dir} does not exist; run the build first`
    )
    process.exit(1)
  }
  const { buildLandingContent } =
    await import("../src/landing/landing-content.ts")
  const problems = checkDist(values.dir, buildLandingContent())
  for (const problem of problems) console.error(`check-dist: ${problem}`)
  if (problems.length > 0) process.exit(1)
  console.log(`check-dist: ${values.dir} is complete`)
}

if (import.meta.url === pathToFileURL(process.argv[1] ?? "").href) await main()
