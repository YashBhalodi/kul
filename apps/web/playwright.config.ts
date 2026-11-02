import { defineConfig, devices } from "@playwright/test"

// End-to-end suites and the screenshot tour run against the production
// build (dist/client) served by scripts/serve-dist.mjs, which applies the
// same _redirects and _headers rules Netlify does.
const PORT = 4173

export default defineConfig({
  testDir: "./e2e",
  outputDir: "./test-results/playwright",
  fullyParallel: true,
  forbidOnly: !!process.env.CI,
  retries: process.env.CI ? 1 : 0,
  reporter: process.env.CI ? [["list"], ["github"]] : [["list"]],
  use: {
    baseURL: `http://127.0.0.1:${PORT}`,
    trace: "retain-on-failure",
  },
  webServer: {
    command: `node scripts/serve-dist.mjs --port ${PORT}`,
    url: `http://127.0.0.1:${PORT}/`,
    reuseExistingServer: !process.env.CI,
  },
  projects: [
    {
      name: "chromium",
      use: { ...devices["Desktop Chrome"] },
      testIgnore: /tour\.spec\.ts/,
    },
    {
      name: "firefox",
      use: { ...devices["Desktop Firefox"] },
      testIgnore: /tour\.spec\.ts/,
    },
    {
      name: "webkit",
      use: { ...devices["Desktop Safari"] },
      testIgnore: /tour\.spec\.ts/,
    },
    {
      // The screenshot tour: every named state in both themes, with axe.
      name: "tour",
      use: { ...devices["Desktop Chrome"] },
      testMatch: /tour\.spec\.ts/,
    },
  ],
})
