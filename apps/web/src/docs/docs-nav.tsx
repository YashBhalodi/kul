import { useLocation } from "@tanstack/react-router"
import { cn } from "cn"
import { ChevronDownIcon } from "lucide-react"
import { useEffect, useRef } from "react"

import { DocsLink } from "./docs-link"
import type { NavPage, NavSection } from "./page-tree"

const LINK =
  "-mx-2 block rounded-md px-2 py-1.5 text-sm text-muted-foreground transition-colors hover:bg-accent hover:text-foreground focus-visible:ring-3 focus-visible:ring-ring/50 focus-visible:outline-none aria-[current=page]:bg-accent aria-[current=page]:font-medium aria-[current=page]:text-foreground"

/** The sections' links; the router marks the current page's. */
function Sections({ sections }: { sections: ReadonlyArray<NavSection> }) {
  return (
    <div className="flex flex-col gap-6">
      {sections.map((section, index) => (
        <div key={section.title ?? index}>
          {section.title !== undefined && (
            <p className="mb-2 text-sm font-semibold text-foreground">
              {section.title}
            </p>
          )}
          <ul className="flex flex-col gap-0.5">
            {section.pages.map((page) => (
              <li key={page.url}>
                <DocsLink href={page.url} className={LINK}>
                  {page.title}
                </DocsLink>
              </li>
            ))}
          </ul>
        </div>
      ))}
    </div>
  )
}

/** The docs' navigation in a column beside the page, on wide screens. */
export function DocsSidebar({
  sections,
}: {
  sections: ReadonlyArray<NavSection>
}) {
  return (
    <nav
      aria-label="Docs"
      className="sticky top-14 hidden max-h-[calc(100svh-3.5rem)] self-start overflow-y-auto py-10 pr-2 lg:block"
    >
      <Sections sections={sections} />
    </nav>
  )
}

/**
 * The docs' navigation on narrow screens: a disclosure above the page,
 * which works without JavaScript and closes when a page opens.
 */
export function DocsMenu({
  sections,
  current,
}: {
  sections: ReadonlyArray<NavSection>
  current: NavPage | undefined
}) {
  const { pathname } = useLocation()
  const menu = useRef<HTMLDetailsElement>(null)
  useEffect(() => {
    if (menu.current) menu.current.open = false
  }, [pathname])
  return (
    <details
      ref={menu}
      className="group -mx-4 border-b px-4 sm:-mx-6 sm:px-6 lg:hidden"
    >
      <summary className="flex cursor-pointer list-none items-center justify-between gap-2 py-3 text-sm font-medium focus-visible:ring-3 focus-visible:ring-ring/50 focus-visible:outline-none [&::-webkit-details-marker]:hidden">
        <span>
          <span className="text-muted-foreground">Learn Kul</span>
          {current && <span> / {current.title}</span>}
        </span>
        <ChevronDownIcon
          aria-hidden="true"
          className={cn(
            "size-4 text-muted-foreground transition-transform group-open:rotate-180"
          )}
        />
      </summary>
      <nav aria-label="Docs menu" className="pt-1 pb-5">
        <Sections sections={sections} />
      </nav>
    </details>
  )
}
