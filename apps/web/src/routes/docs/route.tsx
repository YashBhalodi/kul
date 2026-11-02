import { Link, Outlet, createFileRoute } from "@tanstack/react-router"

import { SiteFooter, SiteHeader } from "@/components/site-chrome"
import { docsPages } from "@/docs/pages"
import type { DocsPage } from "@/docs/pages"

export const Route = createFileRoute("/docs")({
  component: DocsLayout,
})

const NAV_LINK =
  "-mx-2 block rounded-md px-2 py-1.5 text-sm text-muted-foreground transition-colors hover:bg-accent hover:text-foreground focus-visible:ring-3 focus-visible:ring-ring/50 focus-visible:outline-none"
const NAV_LINK_ACTIVE = "bg-accent font-medium text-foreground"

function DocsNavLink({ page }: { page: DocsPage }) {
  const props = {
    className: NAV_LINK,
    activeProps: { className: NAV_LINK_ACTIVE },
    activeOptions: { exact: true },
    children: page.title,
  }
  return page.slug === "" ? (
    <Link to="/docs/" {...props} />
  ) : (
    <Link to="/docs/$slug/" params={{ slug: page.slug }} {...props} />
  )
}

function DocsLayout() {
  return (
    <div className="flex min-h-svh flex-col">
      <SiteHeader />
      <div className="mx-auto grid w-full max-w-6xl flex-1 grid-cols-[minmax(0,1fr)] gap-8 px-4 py-10 sm:px-6 md:grid-cols-[12rem_minmax(0,1fr)] md:gap-12 md:py-14">
        <nav aria-label="Docs" className="md:sticky md:top-24 md:self-start">
          <p className="mb-3 text-sm font-semibold">Learn Kul</p>
          <ul className="flex flex-wrap gap-x-4 gap-y-1 md:flex-col md:gap-1">
            {docsPages.map((page) => (
              <li key={page.slug}>
                <DocsNavLink page={page} />
              </li>
            ))}
          </ul>
        </nav>
        <main className="max-w-3xl min-w-0">
          <Outlet />
        </main>
      </div>
      <SiteFooter />
    </div>
  )
}
