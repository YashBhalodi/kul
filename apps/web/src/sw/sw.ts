// The service worker, served as /sw.js with scope /. scripts/build-sw.mjs
// bundles this file after `vite build` and injects the precache manifest:
// every file the build wrote (pages, the SPA shell, the Markdown and text
// files, the docs search index, scripts, styles, the .wasm, fonts, icons).
//
// - Paths under /assets/ are content-hashed: the precache answers them first.
// - Everything else on this origin (pages, the shell, llms.txt, the .md
//   variants, the search index) goes to the network first, and falls back to
//   the precached copy when the network fails or takes longer than
//   NETWORK_TIMEOUT_MS. Every /app path falls back to /_shell.html.
// - A new build installs beside the running one and waits. The page asks
//   before it takes over (src/offline/updates.ts), with SKIP_WAITING.

import {
  cleanupOutdatedCaches,
  matchPrecache,
  precacheAndRoute,
} from "workbox-precaching"
import type { PrecacheEntry } from "workbox-precaching"
import { registerRoute } from "workbox-routing"

import { isHashedAsset, isSkipWaiting, precachedFileFor } from "./routing"

declare const self: ServiceWorkerGlobalScope & {
  __WB_MANIFEST: Array<PrecacheEntry | string>
}

/** How long a page waits for the network before the precached copy. */
const NETWORK_TIMEOUT_MS = 3000

// Routes match in registration order, so this one sees pages before the
// precache's own (cache-first) route does.
registerRoute(
  ({ url, sameOrigin }) => sameOrigin && !isHashedAsset(url.pathname),
  ({ request, url }) => networkFirst(request, url.pathname)
)
precacheAndRoute(self.__WB_MANIFEST)
cleanupOutdatedCaches()

self.addEventListener("message", (event) => {
  if (isSkipWaiting(event.data)) void self.skipWaiting()
})

async function networkFirst(
  request: Request,
  pathname: string
): Promise<Response> {
  const network = fetch(request)
  try {
    return await withTimeout(network, NETWORK_TIMEOUT_MS)
  } catch {
    const cached = await matchPrecache(precachedFileFor(pathname))
    // Nothing precached: the network's own answer (or failure) stands.
    return cached ?? network
  }
}

function withTimeout<T>(promise: Promise<T>, ms: number): Promise<T> {
  return new Promise((resolve, reject) => {
    const timer = setTimeout(() => {
      reject(new Error(`no response within ${ms} ms`))
    }, ms)
    promise.then(
      (value) => {
        clearTimeout(timer)
        resolve(value)
      },
      (error: unknown) => {
        clearTimeout(timer)
        reject(error instanceof Error ? error : new Error(String(error)))
      }
    )
  })
}
