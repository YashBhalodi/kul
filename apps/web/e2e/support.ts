import { expect } from "@playwright/test"
import type { Page } from "@playwright/test"

/** The app's top bar is up and the language engine has loaded. */
export async function expectAppReady(page: Page) {
  await expect(page.getByTestId("palette-trigger")).toBeVisible()
  await expect(page.getByTestId("engine-status")).toHaveAttribute(
    "data-state",
    "ready",
    { timeout: 15_000 }
  )
}

/**
 * Record CSP violations: `securitypolicyviolation` events in the page (every
 * engine fires them) plus console reports. Call before the first `goto`.
 */
export async function recordCspViolations(
  page: Page
): Promise<() => Promise<Array<string>>> {
  const consoleReports: Array<string> = []
  page.on("console", (message) => {
    const text = message.text()
    if (/content.security.policy|refused to/i.test(text))
      consoleReports.push(text)
  })
  await page.addInitScript(() => {
    const seen: Array<string> = []
    Object.defineProperty(window, "__cspViolations", { value: seen })
    document.addEventListener("securitypolicyviolation", (event) => {
      seen.push(`${event.violatedDirective} blocked ${event.blockedURI}`)
    })
  })
  return async () => {
    const inPage = await page.evaluate(
      () =>
        (window as unknown as { __cspViolations?: Array<string> })
          .__cspViolations ?? []
    )
    return [...inPage, ...consoleReports]
  }
}

/**
 * Press `Mod+<key>` the way the app reads it. The app (TanStack Hotkeys)
 * resolves Mod from the user agent, and Playwright's Desktop Safari profile
 * reports a Mac agent on every host, so ControlOrMeta (which follows the
 * host) is not enough.
 */
export async function pressMod(page: Page, key: string) {
  const mac = await page.evaluate(() =>
    /mac/i.test(`${navigator.platform} ${navigator.userAgent}`)
  )
  await page.keyboard.press(`${mac ? "Meta" : "Control"}+${key}`)
}

/**
 * Record the theme on <html> each time a script toggles its `dark` class
 * (theme-init.js before the first paint, the app afterwards), and whether
 * <body> had been parsed yet. Call before the first `goto`; the returned
 * function reads the record for the current page.
 */
export async function recordThemeTimeline(page: Page) {
  await page.addInitScript(() => {
    const timeline: Array<{ dark: boolean; bodyParsed: boolean }> = []
    Object.defineProperty(window, "__themeTimeline", { value: timeline })
    // Wrap the write itself: MutationObserver callbacks arrive too late to
    // tell whether <body> existed at the moment of the change.
    const toggle = Object.getOwnPropertyDescriptor(
      DOMTokenList.prototype,
      "toggle"
    )?.value as (this: DOMTokenList, token: string, force?: boolean) => boolean
    DOMTokenList.prototype.toggle = function (token, force) {
      const result = toggle.call(this, token, force)
      if (token === "dark" && this === document.documentElement.classList) {
        timeline.push({
          dark: this.contains("dark"),
          bodyParsed: !!document.body,
        })
      }
      return result
    }
  })
  return () =>
    page.evaluate(
      () =>
        (
          window as unknown as {
            __themeTimeline: Array<{ dark: boolean; bodyParsed: boolean }>
          }
        ).__themeTimeline
    )
}
