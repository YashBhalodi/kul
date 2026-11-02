import { Link } from "@tanstack/react-router"
import { cn } from "cn"
import type { ReactNode } from "react"

import { KulLogo } from "@/components/kul-mark"
import { ThemeToggle } from "@/components/theme-toggle"
import { buttonVariants } from "@/components/ui/button"
import { APP_VERSION, REPO_URL } from "@/site"

const NAV_LINK =
  "rounded-md px-2 py-1 text-sm font-medium text-muted-foreground transition-colors hover:text-foreground focus-visible:ring-3 focus-visible:ring-ring/50 focus-visible:outline-none"

/**
 * The header of the landing page and the docs. `actions` (the docs'
 * search) sit before the theme switch; `wide` matches the docs' wider
 * layout.
 */
export function SiteHeader({
  actions,
  wide = false,
}: {
  actions?: ReactNode
  wide?: boolean
}) {
  return (
    <header className="sticky top-0 z-30 border-b bg-background/90 backdrop-blur supports-[backdrop-filter]:bg-background/75">
      <div
        className={cn(
          "mx-auto flex h-14 items-center gap-4 px-4 sm:px-6",
          wide ? "max-w-7xl" : "max-w-6xl"
        )}
      >
        <Link
          to="/"
          className="rounded-md focus-visible:ring-3 focus-visible:ring-ring/50 focus-visible:outline-none"
          aria-label="Kul home"
        >
          <KulLogo />
        </Link>
        <nav aria-label="Site" className="flex items-center gap-1 sm:gap-2">
          <Link
            to="/docs/"
            className={NAV_LINK}
            activeProps={{ className: "text-foreground" }}
          >
            Docs
          </Link>
          <a href={REPO_URL} className={`${NAV_LINK} hidden sm:inline-flex`}>
            GitHub
          </a>
        </nav>
        <div className="ml-auto flex items-center gap-2">
          {actions}
          <ThemeToggle />
          <a href="/app" className={buttonVariants({ size: "sm" })}>
            Open Kul
          </a>
        </div>
      </div>
    </header>
  )
}

/** The footer of the landing page and the docs (`wide` for the docs). */
export function SiteFooter({ wide = false }: { wide?: boolean }) {
  return (
    <footer className="border-t">
      <div
        className={cn(
          "mx-auto flex flex-col gap-4 px-4 py-8 text-sm text-muted-foreground sm:flex-row sm:items-center sm:justify-between sm:px-6",
          wide ? "max-w-7xl" : "max-w-6xl"
        )}
      >
        <p>Kul {APP_VERSION} · MIT licence</p>
        <nav aria-label="Footer" className="flex flex-wrap gap-x-5 gap-y-2">
          <Link to="/docs/" className="hover:text-foreground">
            Docs
          </Link>
          <a href={REPO_URL} className="hover:text-foreground">
            GitHub
          </a>
          <a href="/llms.txt" className="hover:text-foreground">
            llms.txt
          </a>
        </nav>
      </div>
    </footer>
  )
}
