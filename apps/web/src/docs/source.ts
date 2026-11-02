// The docs as a Fumadocs source: what the Markdown variants, llms.txt,
// llms-full.txt and the search index are built from at prerender.

import { loader } from "fumadocs-core/source"

import { docs } from "./collection"
import { docsUrl } from "./urls"

export const source = loader({
  baseUrl: "/docs",
  source: docs.toFumadocsSource(),
  url: (slugs) => docsUrl(slugs),
})

export { docsUrl, markdownUrl, slugsOfMarkdown } from "./urls"

export type DocsPage = NonNullable<ReturnType<typeof source.getPage>>
