import { expect, test } from "./fixtures"
import type { Page } from "./fixtures"

import { recordThemeTimeline } from "./support"

async function chooseTheme(page: Page, name: "System" | "Light" | "Dark") {
  const option = page
    .getByRole("group", { name: "Theme" })
    .getByRole("button", { name: `${name} theme` })
  await option.click()
  await expect(option).toHaveAttribute("aria-pressed", "true")
}

const html = (page: Page) => page.locator("html")

/**
 * The page set `dark` (or not) while <head> was parsing, before any of
 * <body> existed to paint, and kept it.
 */
async function expectThemeBeforePaint(
  timeline: () => Promise<Array<{ dark: boolean; bodyParsed: boolean }>>,
  dark: boolean
) {
  const entries = await timeline()
  expect(entries[0]).toEqual({ dark, bodyParsed: false })
  expect(entries.every((entry) => entry.dark === dark)).toBe(true)
}

test("the system preference applies with no saved choice", async ({ page }) => {
  await page.emulateMedia({ colorScheme: "dark" })
  const timeline = await recordThemeTimeline(page)
  await page.goto("/")
  await expectThemeBeforePaint(timeline, true)
  await expect(html(page)).toHaveClass(/\bdark\b/)

  await page.emulateMedia({ colorScheme: "light" })
  await expect(html(page)).not.toHaveClass(/\bdark\b/)
})

test("a chosen theme survives a reload without a flash", async ({ page }) => {
  // The system says dark; the choice says light, then dark, then system.
  await page.emulateMedia({ colorScheme: "dark" })
  const timeline = await recordThemeTimeline(page)
  await page.goto("/")
  await expect(page.getByTestId("theme-toggle")).toBeVisible()

  await chooseTheme(page, "Light")
  await expect(html(page)).not.toHaveClass(/\bdark\b/)
  await page.reload()
  await expectThemeBeforePaint(timeline, false)
  await expect(html(page)).not.toHaveClass(/\bdark\b/)

  await page.emulateMedia({ colorScheme: "light" })
  await chooseTheme(page, "Dark")
  await expect(html(page)).toHaveClass(/\bdark\b/)
  await page.goto("/docs/")
  await expectThemeBeforePaint(timeline, true)
  await page.goto("/app/")
  await expectThemeBeforePaint(timeline, true)
  await expect(html(page)).toHaveClass(/\bdark\b/)

  await page.goto("/")
  await chooseTheme(page, "System")
  await expect(html(page)).not.toHaveClass(/\bdark\b/)
  await page.reload()
  await expectThemeBeforePaint(timeline, false)
})
