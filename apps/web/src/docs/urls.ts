// The docs pages' addresses. Kept apart from source.ts, so the pages'
// <head> can name them without loading the docs themselves.

/** A page's HTML path: /docs/ or /docs/<slugs>/ (directory pages). */
export function docsUrl(slugs: ReadonlyArray<string>): string {
  return slugs.length === 0 ? "/docs/" : `/docs/${slugs.join("/")}/`
}

/** A page's Markdown variant: /docs/index.md or /docs/<slugs>.md. */
export function markdownUrl(slugs: ReadonlyArray<string>): string {
  return slugs.length === 0 ? "/docs/index.md" : `/docs/${slugs.join("/")}.md`
}

/** The page slugs a Markdown-variant path names (the part after /docs/). */
export function slugsOfMarkdown(path: string): Array<string> | undefined {
  if (!path.endsWith(".md")) return undefined
  const slugs = path.slice(0, -".md".length).split("/")
  return slugs.length === 1 && slugs[0] === "index" ? [] : slugs
}
