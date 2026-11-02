// Builds of the site for the service-worker suites, and a server they
// control. `dist/client` is the build under test (`npm run build`). The next
// build is the same source built again with another version string
// (KUL_SITE_VERSION): global-setup.ts writes it before the suites run. A
// suite serves one build, then serves the other on the same origin, as a
// deploy does.

import { readFileSync } from "node:fs"
import type { AddressInfo } from "node:net"
import { join } from "node:path"

import { createDistServer } from "../scripts/serve-dist.mjs"
import type { Page } from "./fixtures"

export const APP_DIR = join(import.meta.dirname, "..")
export const BUILD_DIR = join(APP_DIR, "dist/client")
/** Where global-setup.ts builds the next site (its client output). */
export const NEXT_OUT_DIR = join(APP_DIR, "test-results/next-site")
export const NEXT_BUILD_DIR = join(NEXT_OUT_DIR, "client")

const { version } = JSON.parse(
  readFileSync(join(APP_DIR, "package.json"), "utf8")
) as { version: string }

export const CURRENT_VERSION = version
export const NEXT_VERSION = `${version}+next`

export interface SiteServer {
  /** `http://127.0.0.1:<port>`, the same whichever build is served. */
  origin: string
  /** Serve `dir` from now on: the server restarts on the same port. */
  serve: (dir: string) => Promise<void>
  close: () => Promise<void>
}

/** Serve the build at `dir` on a free port. */
export async function startSite(dir: string): Promise<SiteServer> {
  let server = await listen(dir, 0)
  const { port } = server.address() as AddressInfo
  const stop = () =>
    new Promise<void>((resolve, reject) => {
      server.close((error) => {
        if (error) reject(error)
        else resolve()
      })
      server.closeAllConnections()
    })
  return {
    origin: `http://127.0.0.1:${port}`,
    serve: async (next) => {
      await stop()
      server = await listen(next, port)
    },
    close: stop,
  }
}

function listen(root: string, port: number) {
  const server = createDistServer({ root })
  return new Promise<typeof server>((resolve, reject) => {
    server.once("error", reject)
    server.listen(port, "127.0.0.1", () => {
      resolve(server)
    })
  })
}

/**
 * Serve the next build on `site`, and have the open page look for it: the
 * page checks for a new build whenever it becomes visible.
 */
export async function deployNextBuild(page: Page, site: SiteServer) {
  await site.serve(NEXT_BUILD_DIR)
  await page.evaluate(() => {
    document.dispatchEvent(new Event("visibilitychange"))
  })
}
