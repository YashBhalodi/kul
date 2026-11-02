import { createFileRoute } from "@tanstack/react-router"

import { pageForMarkdownName, pageMarkdown } from "@/docs/pages"

// /docs/<slug>.md (and /docs/index.md for the docs home): each docs page as
// Markdown, prerendered to a static file.
export const Route = createFileRoute("/docs/{$name}.md")({
  server: {
    handlers: {
      GET: ({ params }) => {
        const page = pageForMarkdownName(params.name)
        if (!page) return new Response("Not found", { status: 404 })
        return new Response(pageMarkdown(page), {
          headers: { "Content-Type": "text/markdown; charset=utf-8" },
        })
      },
    },
  },
})
