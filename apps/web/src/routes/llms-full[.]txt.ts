import { createFileRoute } from "@tanstack/react-router"

import { llmsFullTxt } from "@/docs/llms"

export const Route = createFileRoute("/llms-full.txt")({
  server: {
    handlers: {
      GET: async () =>
        new Response(await llmsFullTxt(), {
          headers: { "Content-Type": "text/plain; charset=utf-8" },
        }),
    },
  },
})
