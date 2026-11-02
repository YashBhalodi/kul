import { createFileRoute } from "@tanstack/react-router"
import { createFromSource } from "fumadocs-core/search/server"

import { source } from "@/docs/source"

// The docs search index, prerendered to a static file. The search dialog
// downloads it once and searches in the browser, so search needs no server
// and works offline.
export const Route = createFileRoute("/docs/search.json")({
  server: {
    handlers: {
      GET: () => createFromSource(source, { language: "english" }).staticGET(),
    },
  },
})
