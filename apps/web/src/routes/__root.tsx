import {
  HeadContent,
  Link,
  Scripts,
  createRootRoute,
} from "@tanstack/react-router"
import { Suspense, lazy, useEffect } from "react"
import type { ReactNode } from "react"

import { buttonVariants } from "@/components/ui/button"
import { SiteFooter, SiteHeader } from "@/components/site-chrome"
import { UpdatePrompt } from "@/offline/update-prompt"
import { startServiceWorker } from "@/offline/updates"
import { connectTheme } from "@/prefs/theme"
import interLatin from "@fontsource-variable/inter/files/inter-latin-wght-normal.woff2?url"

import appCss from "@/styles.css?url"
import themeScript from "@/prefs/theme-init.js?url&no-inline"

const RouterDevtools = import.meta.env.DEV
  ? lazy(() =>
      import("@tanstack/react-router-devtools").then((module) => ({
        default: module.TanStackRouterDevtools,
      }))
    )
  : () => null

export const Route = createRootRoute({
  head: () => ({
    meta: [
      { charSet: "utf-8" },
      { name: "viewport", content: "width=device-width, initial-scale=1" },
      { name: "color-scheme", content: "light dark" },
      { title: "Kul" },
      {
        name: "description",
        content:
          "Kul is a small plain-text language for describing families. Write who people are and how they are related, and Kul draws the family tree.",
      },
    ],
    links: [
      { rel: "stylesheet", href: appCss },
      // The Latin subset of the sans face sets the first screen's text;
      // fetch it alongside the stylesheet rather than after it.
      {
        rel: "preload",
        href: interLatin,
        as: "font",
        type: "font/woff2",
        crossOrigin: "anonymous",
      },
      { rel: "icon", href: "/favicon.svg", type: "image/svg+xml" },
    ],
    scripts: [{ src: themeScript }],
  }),
  shellComponent: RootDocument,
  notFoundComponent: NotFound,
})

function RootDocument({ children }: { children: ReactNode }) {
  useEffect(() => connectTheme(), [])
  useEffect(startServiceWorker, [])
  return (
    // theme-init.js sets the theme class before React hydrates.
    <html lang="en" suppressHydrationWarning>
      <head>
        <HeadContent />
      </head>
      <body>
        {children}
        <UpdatePrompt />
        <Suspense>
          <RouterDevtools />
        </Suspense>
        <Scripts />
      </body>
    </html>
  )
}

function NotFound() {
  return (
    <div className="flex min-h-svh flex-col">
      <SiteHeader />
      <main className="mx-auto flex w-full max-w-2xl flex-1 flex-col justify-center gap-4 px-6 py-24">
        <p className="text-sm font-medium text-muted-foreground">404</p>
        <h1 className="text-3xl font-semibold tracking-tight">
          This page doesn’t exist
        </h1>
        <p className="text-lg text-muted-foreground">
          The link may be old, or the address mistyped.
        </p>
        <div className="flex gap-3 pt-2">
          <Link to="/" className={buttonVariants({ size: "lg" })}>
            Go to the home page
          </Link>
          <Link
            to="/docs/"
            className={buttonVariants({ size: "lg", variant: "outline" })}
          >
            Read the docs
          </Link>
        </div>
      </main>
      <SiteFooter />
    </div>
  )
}
