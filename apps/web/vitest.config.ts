import { playwright } from "@vitest/browser-playwright"
import { defineConfig } from "vitest/config"

import { siteVersion } from "./scripts/site-version.mjs"

// Two projects: pure logic in Node (`*.test.ts`, and the build scripts'
// `scripts/*.test.mjs`), and code that needs a real browser (`*.browser.test.ts`:
// workers, WASM, matchMedia, storage) in Chromium and WebKit.
export default defineConfig({
  resolve: { tsconfigPaths: true },
  define: {
    "import.meta.env.KUL_SITE_VERSION": JSON.stringify(siteVersion()),
  },
  test: {
    projects: [
      {
        extends: true,
        test: {
          name: "node",
          environment: "node",
          include: ["src/**/*.test.ts", "scripts/*.test.mjs"],
          exclude: ["src/**/*.browser.test.ts", "scripts/serve-dist.test.mjs"],
        },
      },
      {
        extends: true,
        test: {
          name: "browser",
          include: ["src/**/*.browser.test.ts"],
          browser: {
            enabled: true,
            headless: true,
            provider: playwright(),
            instances: [
              { browser: "chromium" },
              {
                browser: "webkit",
                // Playwright's WebKit has no origin private file system in
                // an ephemeral context (getDirectory() rejects); a
                // persistent context has the whole API.
                provider: playwright({
                  persistentContext: "node_modules/.cache/vitest-webkit",
                }),
                // A persistent context serves one test file at a time.
                fileParallelism: false,
              },
            ],
          },
        },
      },
    ],
  },
})
