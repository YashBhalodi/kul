import { Outlet, createFileRoute } from "@tanstack/react-router"

import { SiteFooter, SiteHeader } from "@/components/site-chrome"
import { DocsSidebar } from "@/docs/docs-nav"
import { loadNav } from "@/docs/pages"
import { SearchButton } from "@/docs/search-button"

export const Route = createFileRoute("/docs")({
  loader: () => loadNav(),
  staleTime: Infinity,
  component: DocsShell,
})

/**
 * The docs: the site's header with search, the pages' navigation beside
 * the page on wide screens, and the site's footer.
 */
function DocsShell() {
  const sections = Route.useLoaderData()
  return (
    <div className="flex min-h-svh flex-col">
      <SiteHeader wide actions={<SearchButton />} />
      <div className="mx-auto grid w-full max-w-7xl flex-1 grid-cols-[minmax(0,1fr)] gap-10 px-4 sm:px-6 lg:grid-cols-[14rem_minmax(0,1fr)]">
        <DocsSidebar sections={sections} />
        <Outlet />
      </div>
      <SiteFooter wide />
    </div>
  )
}
