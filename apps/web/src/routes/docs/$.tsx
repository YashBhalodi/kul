import { createFileRoute, getRouteApi, notFound } from "@tanstack/react-router"
import { ArrowLeftIcon, ArrowRightIcon } from "lucide-react"
import { Suspense } from "react"

import { DocsLink } from "@/docs/docs-link"
import { DocsMenu } from "@/docs/docs-nav"
import { docsHead } from "@/docs/head"
import { docsMdxComponents } from "@/docs/mdx-components"
import { neighbours } from "@/docs/page-tree"
import { loadPage, usePageBody } from "@/docs/pages"
import type { LoadedPage } from "@/docs/pages"
import type { NavPage } from "@/docs/page-tree"
import { docsUrl, markdownUrl } from "@/docs/urls"

export const Route = createFileRoute("/docs/$")({
  // The loader reads the docs source, so it loads with the docs, not with
  // every page of the site.
  codeSplitGroupings: [["loader"], ["component"]],
  loader: async ({ params }) => {
    const slugs = (params._splat ?? "").split("/").filter(Boolean)
    // The page's compiled body loads before render, so the prerendered
    // HTML and the hydrated page agree.
    const page = await loadPage(slugs)
    if (!page) throw notFound()
    return page
  },
  head: ({ loaderData }) => (loaderData ? docsHead(loaderData) : {}),
  component: DocsPageRoute,
})

const docsLayout = getRouteApi("/docs")

function Neighbour({
  page,
  direction,
}: {
  page: NavPage
  direction: "previous" | "next"
}) {
  const next = direction === "next"
  return (
    <DocsLink
      href={page.url}
      className="group flex flex-col gap-1 rounded-xl border p-4 transition-colors hover:bg-accent focus-visible:ring-3 focus-visible:ring-ring/50 focus-visible:outline-none data-[next]:items-end data-[next]:text-right"
      data-next={next ? "" : undefined}
    >
      <span className="flex items-center gap-1 text-sm text-muted-foreground">
        {!next && <ArrowLeftIcon aria-hidden="true" className="size-3.5" />}
        {next ? "Next" : "Previous"}
        {next && <ArrowRightIcon aria-hidden="true" className="size-3.5" />}
      </span>
      <span className="font-medium text-foreground">{page.title}</span>
    </DocsLink>
  )
}

function DocsPageRoute() {
  const page = Route.useLoaderData()
  // While the page hydrates, its body may still be loading; the
  // prerendered HTML stays on screen meanwhile.
  return (
    <Suspense>
      <DocsPageContent page={page} />
    </Suspense>
  )
}

function DocsPageContent({ page }: { page: LoadedPage }) {
  const { path, slugs, title, description } = page
  const { Body, toc } = usePageBody(path)
  const sections = docsLayout.useLoaderData()
  const url = docsUrl(slugs)
  const { previous, next } = neighbours(sections, url)
  const outline = toc.filter((item) => item.depth <= 3)

  return (
    <div className="grid min-w-0 grid-cols-[minmax(0,1fr)] gap-10 xl:grid-cols-[minmax(0,1fr)_13rem]">
      <main className="min-w-0 pb-16 lg:pt-10">
        <DocsMenu sections={sections} current={{ title, url }} />
        <article className="mx-auto max-w-3xl pt-8 lg:pt-0">
          <header className="mb-8 flex flex-col gap-3">
            <h1 className="text-3xl font-semibold tracking-tight sm:text-4xl">
              {title}
            </h1>
            {description && (
              <p className="text-lg leading-relaxed text-muted-foreground">
                {description}
              </p>
            )}
          </header>
          <div className="docs-prose">
            <Body components={docsMdxComponents(title)} />
          </div>
          <nav
            aria-label="More pages"
            className="mt-16 grid gap-3 border-t pt-8 sm:grid-cols-2"
          >
            {previous ? (
              <Neighbour page={previous} direction="previous" />
            ) : (
              <span />
            )}
            {next && <Neighbour page={next} direction="next" />}
          </nav>
          <p className="mt-6 text-sm text-muted-foreground">
            This page is also available{" "}
            <a
              href={markdownUrl(slugs)}
              className="font-medium text-link underline underline-offset-4 hover:no-underline"
            >
              as Markdown
            </a>
            .
          </p>
        </article>
      </main>
      {outline.length > 0 && (
        <nav
          aria-label="On this page"
          className="sticky top-14 hidden max-h-[calc(100svh-3.5rem)] self-start overflow-y-auto py-10 xl:block"
        >
          <p className="mb-3 text-sm font-semibold">On this page</p>
          <ul className="flex flex-col gap-1.5 border-l text-sm">
            {outline.map((item) => (
              <li key={item.url}>
                <a
                  href={item.url}
                  className="-ml-px block border-l border-transparent py-0.5 pl-3 text-muted-foreground transition-colors hover:border-foreground hover:text-foreground focus-visible:ring-3 focus-visible:ring-ring/50 focus-visible:outline-none data-[depth='3']:pl-6"
                  data-depth={item.depth}
                >
                  {item.title}
                </a>
              </li>
            ))}
          </ul>
        </nav>
      )}
    </div>
  )
}
