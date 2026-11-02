#!/usr/bin/env node
// Checks every internal link in the production build: the links of every
// HTML page (anchors and <link> tags), of llms.txt, llms-full.txt and the
// docs' Markdown variants. A link to this site (a root-relative path, a
// relative path or https://kullang.dev/…) must name a built file, and a
// #fragment on an HTML page must name an element id there. A link into the
// repository on GitHub (…/tree/main/<path>, …/blob/main/<path>) must name
// a path that exists in this checkout.
//
//   node scripts/check-links.mjs [--dir <dir>]
//
// Prints one line per broken link and exits 1 when there is any.

import { existsSync, readFileSync } from "node:fs"
import { join } from "node:path"
import { fileURLToPath, pathToFileURL } from "node:url"
import { parseArgs } from "node:util"

import { listFiles } from "./finalize-dist.mjs"

export const SITE_ORIGIN = "https://kullang.dev"
export const REPO_URL = "https://github.com/YashBhalodi/kul"

const here = fileURLToPath(new URL(".", import.meta.url))
const REPO_ROOT = join(here, "..", "..", "..")

function decodeEntities(text) {
  return text
    .replaceAll("&amp;", "&")
    .replaceAll("&quot;", '"')
    .replaceAll("&#x27;", "'")
    .replaceAll("&#39;", "'")
    .replaceAll("&lt;", "<")
    .replaceAll("&gt;", ">")
}

/** The href of every <a> and <link> in an HTML page. */
export function htmlLinks(html) {
  const links = []
  for (const match of html.matchAll(/<(?:a|link)\b[^>]*?\shref="([^"]*)"/gi)) {
    links.push(decodeEntities(match[1]))
  }
  return links
}

/** The target of every Markdown link `[text](target)` outside code. */
export function markdownLinks(markdown) {
  const prose = markdown.replace(/^```[\s\S]*?^```/gm, "")
  const links = []
  for (const match of prose.matchAll(/\]\(([^)\s]+)\)/g)) links.push(match[1])
  return links
}

/** Every element id in an HTML page. */
export function htmlIds(html) {
  return new Set(
    [...html.matchAll(/\sid="([^"]+)"/g)].map((match) =>
      decodeEntities(match[1])
    )
  )
}

/** The built file a site path is served from, if any. */
export function fileFor(path, files) {
  const clean = decodeURIComponent(path).replace(/^\/+/, "")
  if (clean === "") return files.has("index.html") ? "index.html" : undefined
  if (clean.endsWith("/")) {
    const index = `${clean}index.html`
    return files.has(index) ? index : undefined
  }
  if (files.has(clean)) return clean
  if (files.has(`${clean}/index.html`)) return `${clean}/index.html`
  return undefined
}

/** /app and everything under it is the client-only app (the SPA shell). */
function isAppPath(path) {
  return path === "/app" || path.startsWith("/app/")
}

/**
 * Every broken link in the build at `root`, as human-readable lines.
 * `repoRoot` is the checkout GitHub links are checked against.
 */
export function checkLinks(root, { repoRoot = REPO_ROOT } = {}) {
  const files = new Set(listFiles(root))
  const read = (file) => readFileSync(join(root, file), "utf8")
  const idsCache = new Map()
  const idsOf = (file) => {
    if (!idsCache.has(file)) idsCache.set(file, htmlIds(read(file)))
    return idsCache.get(file)
  }
  const problems = []

  const sources = [...files].filter(
    (file) =>
      file.endsWith(".html") ||
      file.endsWith(".md") ||
      file === "llms.txt" ||
      file === "llms-full.txt"
  )
  for (const source of sources.sort()) {
    const text = read(source)
    const links = source.endsWith(".html")
      ? htmlLinks(text)
      : markdownLinks(text)
    const base = new URL(`${SITE_ORIGIN}/${source.replace(/index\.html$/, "")}`)
    for (const link of links) {
      if (link === "" || link.startsWith("mailto:")) continue
      if (link.startsWith(`${REPO_URL}/`)) {
        const match = /^\/(?:tree|blob)\/main\/([^#?]*)/.exec(
          link.slice(REPO_URL.length)
        )
        if (match && !existsSync(join(repoRoot, decodeURIComponent(match[1]))))
          problems.push(`${source}: ${link} names no file in the repository`)
        continue
      }
      let url
      try {
        url = new URL(link, base)
      } catch {
        problems.push(`${source}: ${link} is not a valid URL`)
        continue
      }
      if (url.origin !== SITE_ORIGIN) continue
      if (isAppPath(url.pathname)) continue
      const target = fileFor(url.pathname, files)
      if (target === undefined) {
        problems.push(`${source}: ${link} is not in the build`)
        continue
      }
      const fragment = decodeURIComponent(url.hash.slice(1))
      if (fragment !== "" && target.endsWith(".html")) {
        if (!idsOf(target).has(fragment))
          problems.push(`${source}: ${link} names no #${fragment} on ${target}`)
      }
    }
  }
  return problems
}

function main() {
  const { values } = parseArgs({
    options: {
      dir: { type: "string", default: join(here, "..", "dist", "client") },
    },
  })
  if (!existsSync(values.dir)) {
    console.error(
      `check-links: ${values.dir} does not exist; run the build first`
    )
    process.exit(1)
  }
  const problems = checkLinks(values.dir)
  for (const problem of problems) console.error(`check-links: ${problem}`)
  if (problems.length > 0) process.exit(1)
  console.log(`check-links: every internal link in ${values.dir} resolves`)
}

if (import.meta.url === pathToFileURL(process.argv[1] ?? "").href) main()
