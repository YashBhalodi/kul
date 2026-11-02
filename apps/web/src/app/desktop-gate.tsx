import { Link } from "@tanstack/react-router"
import { MonitorIcon } from "lucide-react"
import { useSyncExternalStore } from "react"
import type { ReactNode } from "react"

import { KulLogo } from "@/components/kul-mark"
import { buttonVariants } from "@/components/ui/button"

/**
 * The app needs a window at least 1024 px wide and a precise pointer (a
 * mouse or trackpad). Phones, small tablets and touch-only devices get the
 * gate instead.
 */
export const DESKTOP_QUERY = "(min-width: 1024px) and (any-pointer: fine)"

function subscribe(onChange: () => void): () => void {
  const media = window.matchMedia(DESKTOP_QUERY)
  media.addEventListener("change", onChange)
  return () => media.removeEventListener("change", onChange)
}

export function useIsDesktop(): boolean {
  return useSyncExternalStore(
    subscribe,
    () => window.matchMedia(DESKTOP_QUERY).matches,
    () => true
  )
}

/** The app on a desktop; a friendly notice anywhere smaller. */
export function DesktopGate({ children }: { children: ReactNode }) {
  return useIsDesktop() ? children : <SmallScreenNotice />
}

export function SmallScreenNotice() {
  return (
    <main
      data-testid="desktop-gate"
      className="flex min-h-svh flex-col bg-background px-6 py-10"
    >
      <KulLogo />
      <div className="mx-auto flex w-full max-w-md flex-1 flex-col justify-center gap-6 py-12">
        <div className="flex size-12 items-center justify-center rounded-xl border bg-card text-primary shadow-sm">
          <MonitorIcon className="size-6" aria-hidden="true" />
        </div>
        <div className="flex flex-col gap-3">
          <h1 className="text-2xl font-semibold tracking-tight">
            Kul’s editor needs a larger screen
          </h1>
          <p className="text-base leading-relaxed text-muted-foreground">
            The editor and the family tree sit side by side, which needs a
            window at least 1024 pixels wide and a mouse or trackpad. Open Kul
            on a computer to start writing.
          </p>
          <p className="text-base leading-relaxed text-muted-foreground">
            The guide to the language reads well on any screen.
          </p>
        </div>
        <div className="flex flex-col gap-3 sm:flex-row">
          <Link to="/docs/" className={buttonVariants({ size: "lg" })}>
            Learn the language
          </Link>
          <Link
            to="/"
            className={buttonVariants({ size: "lg", variant: "outline" })}
          >
            Back to the home page
          </Link>
        </div>
      </div>
    </main>
  )
}
