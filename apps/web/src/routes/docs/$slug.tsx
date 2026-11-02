import { createFileRoute, notFound } from "@tanstack/react-router"

import { docsHead } from "@/docs/head"
import { DocsArticle } from "@/docs/docs-article"
import { pageForSlug } from "@/docs/pages"

export const Route = createFileRoute("/docs/$slug")({
  loader: ({ params }) => {
    const page = pageForSlug(params.slug)
    if (!page) throw notFound()
    return page
  },
  head: ({ loaderData }) => (loaderData ? docsHead(loaderData) : {}),
  component: DocsSlugPage,
})

function DocsSlugPage() {
  const page = Route.useLoaderData()
  return <DocsArticle page={page} />
}
