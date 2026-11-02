#!/usr/bin/env node
// Checks the production build's static layout after `npm run build`: every
// prerendered page, the SPA shell, the llms files, the docs' Markdown
// variants, the search index and the Netlify config sit at their exact
// paths, and nothing that must stay client-only was prerendered.
//
//   node scripts/check-dist.mjs [--dir <dir>]
//
// Prints one line per problem and exits 1 when there is any.

import { existsSync, readFileSync } from "node:fs"
import { join } from "node:path"
import { fileURLToPath, pathToFileURL } from "node:url"
import { parseArgs } from "node:util"

import { inlineScripts, listFiles, scriptHash } from "./finalize-dist.mjs"
import { headersFor, parseHeaders, parseRedirects } from "./serve-dist.mjs"

const SITE_ORIGIN = "https://kullang.dev"

/** Every problem with the build at `root`, as human-readable lines. */
export function checkDist(root) {
  const problems = []
  const files = new Set(listFiles(root))
  const read = (file) => readFileSync(join(root, file), "utf8")
  const requireFile = (file, why) => {
    if (!files.has(file)) problems.push(`missing ${file} (${why})`)
    return files.has(file)
  }

  requireFile("index.html", "the landing page")
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
  const markdown = [...files].filter((file) => /^docs\/[^/]+\.md$/.test(file))
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
  return problems
}

function main() {
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
  const problems = checkDist(values.dir)
  for (const problem of problems) console.error(`check-dist: ${problem}`)
  if (problems.length > 0) process.exit(1)
  console.log(`check-dist: ${values.dir} is complete`)
}

if (import.meta.url === pathToFileURL(process.argv[1] ?? "").href) main()
