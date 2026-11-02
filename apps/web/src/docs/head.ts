import { SITE_ORIGIN } from "@/site"
import { htmlPath, markdownPath } from "./pages"
import type { DocsPage } from "./pages"

/** A docs page's <head>: title, summary, canonical URL, Markdown variant. */
export function docsHead(page: DocsPage) {
  return {
    meta: [
      { title: `${page.title} · Kul docs` },
      { name: "description", content: page.description },
    ],
    links: [
      { rel: "canonical", href: `${SITE_ORIGIN}${htmlPath(page)}` },
      {
        rel: "alternate",
        type: "text/markdown",
        href: markdownPath(page),
      },
    ],
  }
}
