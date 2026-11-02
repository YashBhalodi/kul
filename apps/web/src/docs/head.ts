import { SITE_ORIGIN } from "@/site"
import { docsUrl, markdownUrl } from "./urls"

export interface DocsHeadInput {
  title: string
  description: string | undefined
  slugs: ReadonlyArray<string>
}

/** A docs page's <head>: title, summary, canonical URL, Markdown variant. */
export function docsHead({ title, description, slugs }: DocsHeadInput) {
  return {
    meta: [
      { title: `${title} · Learn Kul` },
      ...(description ? [{ name: "description", content: description }] : []),
    ],
    links: [
      { rel: "canonical", href: `${SITE_ORIGIN}${docsUrl(slugs)}` },
      { rel: "alternate", type: "text/markdown", href: markdownUrl(slugs) },
    ],
  }
}
