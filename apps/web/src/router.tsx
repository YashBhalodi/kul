import { createRouter } from "@tanstack/react-router"

import { routeTree } from "./routeTree.gen"

export function getRouter() {
  return createRouter({
    routeTree,
    // Every page is a directory index (/docs/, /docs/guide/dates/), so links
    // match the prerendered files without a redirect.
    trailingSlash: "always",
    scrollRestoration: true,
    defaultPreload: "intent",
    defaultPreloadStaleTime: 0,
  })
}

declare module "@tanstack/react-router" {
  interface Register {
    router: ReturnType<typeof getRouter>
  }
}
