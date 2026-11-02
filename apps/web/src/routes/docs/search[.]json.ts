import { createFileRoute } from "@tanstack/react-router"

import { htmlPath, docsPages, pagePlainText } from "@/docs/pages"

// The docs search index, prerendered to a static JSON file: one entry per
// page with its URL, title, summary and text.
export const Route = createFileRoute("/docs/search.json")({
  server: {
    handlers: {
      GET: () =>
        Response.json(
          docsPages.map((page) => ({
            url: htmlPath(page),
            title: page.title,
            description: page.description,
            content: pagePlainText(page),
          }))
        ),
    },
  },
})
