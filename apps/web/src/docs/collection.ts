// The docs pages: MDX under content/docs, compiled by Fumadocs MDX at build
// time. Only each page's frontmatter ships up front; its body loads when
// the page is opened. The pages render from this collection directly;
// source.ts builds the Markdown variants, llms files and search index on it.

import { defineDocs } from "fumadocs-mdx/macro"

export const docs = defineDocs({
  dir: "content/docs",
  docs: {
    async: true,
    postprocess: { includeProcessedMarkdown: true },
  },
})
