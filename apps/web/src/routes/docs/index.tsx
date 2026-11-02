import { createFileRoute } from "@tanstack/react-router"

import { docsHead } from "@/docs/head"
import { DocsArticle } from "@/docs/docs-article"
import { docsHome } from "@/docs/pages"

export const Route = createFileRoute("/docs/")({
  head: () => docsHead(docsHome),
  component: () => <DocsArticle page={docsHome} />,
})
