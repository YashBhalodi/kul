import { createFileRoute } from "@tanstack/react-router"

import { pageMarkdown } from "@/docs/llms"
import { slugsOfMarkdown, source } from "@/docs/source"

// /docs/<slug>.md (and /docs/index.md for the docs home): each docs page as
// Markdown, prerendered to a static file.
export const Route = createFileRoute("/docs/{$}.md")({
  server: {
    handlers: {
      GET: async ({ params }) => {
        const slugs = slugsOfMarkdown(`${params._splat ?? ""}.md`)
        const page = slugs && source.getPage(slugs)
        if (!page) return new Response("Not found", { status: 404 })
        return new Response(await pageMarkdown(page), {
          headers: { "Content-Type": "text/markdown; charset=utf-8" },
        })
      },
    },
  },
})
