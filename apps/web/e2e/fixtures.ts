import { mkdtempSync, rmSync } from "node:fs"
import { tmpdir } from "node:os"
import { join } from "node:path"

import { test as base } from "@playwright/test"

export { expect } from "@playwright/test"
export type { Page } from "@playwright/test"

/**
 * The end-to-end `test`. Playwright's WebKit has no origin private file
 * system in an ordinary (ephemeral) browser context: `getDirectory()`
 * rejects, and the app falls back to memory. So each WebKit test runs in a
 * fresh persistent context, which has the whole API and starts empty, as
 * every other browser's context does.
 */
export const test = base.extend({
  context: async (
    {
      browserName,
      context,
      playwright,
      launchOptions,
      headless,
      baseURL,
      viewport,
      userAgent,
      deviceScaleFactor,
      isMobile,
      hasTouch,
      colorScheme,
      locale,
      javaScriptEnabled,
      acceptDownloads,
    },
    // Playwright's fixture callback, named so React's hook lint leaves it be.
    provide
  ) => {
    if (browserName !== "webkit") {
      await provide(context)
      return
    }
    const userDataDir = mkdtempSync(join(tmpdir(), "kul-webkit-"))
    const persistent = await playwright.webkit.launchPersistentContext(
      userDataDir,
      {
        ...launchOptions,
        headless,
        baseURL,
        viewport,
        userAgent,
        deviceScaleFactor,
        isMobile,
        hasTouch,
        colorScheme,
        locale,
        javaScriptEnabled,
        acceptDownloads,
      }
    )
    try {
      await provide(persistent)
    } finally {
      await persistent.close()
      rmSync(userDataDir, { recursive: true, force: true })
    }
  },
})
