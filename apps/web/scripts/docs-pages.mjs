// The docs pages' paths, from the files under content/docs: each page's
// directory path and its Markdown variant. The prerender lists them all, so
// no page depends on being linked to be built.
//
//   docsPagePaths() → ["/docs/", "/docs/index.md", "/docs/guide/dates/", …]

import { globSync } from "node:fs"
import { fileURLToPath } from "node:url"

const CONTENT = fileURLToPath(new URL("../content/docs", import.meta.url))

/** A content file's slugs: `guide/dates.mdx` → ["guide", "dates"]. */
export function slugsOf(file) {
  const parts = file.replace(/\.mdx$/, "").split("/")
  if (parts.at(-1) === "index") parts.pop()
  return parts
}

/** Every docs page's HTML path and Markdown-variant path. */
export function docsPagePaths(root = CONTENT) {
  return globSync("**/*.mdx", { cwd: root })
    .map((file) => slugsOf(file.split("\\").join("/")))
    .sort((a, b) => a.join("/").localeCompare(b.join("/")))
    .flatMap((slugs) =>
      slugs.length === 0
        ? ["/docs/", "/docs/index.md"]
        : [`/docs/${slugs.join("/")}/`, `/docs/${slugs.join("/")}.md`]
    )
}
