// The docs: search that works offline, and "Try it", which opens a docs
// example in the app as a new project.

import { expect, test } from "./fixtures"
import {
  PROJECT_URL,
  expectAppReady,
  expectFiles,
  openDocsSearch,
  treeCard,
  waitForOfflineCopy,
} from "./support"

test("search finds a page offline", async ({ page, context }) => {
  await page.goto("/docs/")
  await waitForOfflineCopy(page)
  await context.setOffline(true)
  await page.reload()

  const dialog = await openDocsSearch(page)
  await dialog.getByRole("combobox", { name: "Search" }).fill("adoption")
  const result = dialog.getByText("Children", { exact: true }).first()
  await expect(result).toBeVisible()
  await result.click()
  await expect(page).toHaveURL(/\/docs\/guide\/children\/(#.*)?$/)
  await expect(page.getByRole("heading", { level: 1 })).toHaveText("Children")
})

test("Try it opens an example as a new project that draws", async ({
  page,
}) => {
  await page.goto("/docs/guide/children/")
  const example = page
    .locator("[data-kul-code]")
    .filter({ hasText: "person bayani" })
  await example.getByTestId("try-it").click()

  await expect(page).toHaveURL(PROJECT_URL)
  // The snippet travels in sessionStorage, never in the address.
  expect(new URL(page.url()).search).toBe("")
  await expectAppReady(page)
  await expect(page.getByTestId("project-switcher")).toHaveText(
    "Try it: Children"
  )
  await expectFiles(page, ["family.kul"])
  await expect(treeCard(page, "bayani")).toBeVisible()
  await expect(treeCard(page, "carlos")).toBeVisible()

  // The hand-off is used once: /app/ now opens the same project again.
  await page.goto("/app/")
  await expect(page).toHaveURL(PROJECT_URL)
  await expect(page.getByTestId("project-switcher")).toHaveText(
    "Try it: Children"
  )
})

test("Try it on a multi-file example opens every file", async ({ page }) => {
  await page.goto("/docs/guide/files/")
  await page
    .locator("[data-kul-code]")
    .filter({ hasText: "person alice" })
    .getByTestId("try-it")
    .click()

  await expect(page).toHaveURL(PROJECT_URL)
  await expectAppReady(page)
  await expectFiles(page, ["01-founders.kul", "02-children.kul"])
  await expect(treeCard(page, "alice")).toBeVisible()
  await expect(treeCard(page, "ramesh")).toBeVisible()
})

test("an example that shows a mistake carries the engine's message", async ({
  page,
}) => {
  await page.goto("/docs/reference/diagnostics/")
  const example = page.locator('[data-kul-code][data-expect-error="KUL-R03"]')
  await expect(
    example.getByRole("list", { name: "What Kul reports" })
  ).toContainText("needs a `gender:` field".replaceAll("`", ""))
  await expect(example.getByTestId("try-it")).toHaveCount(0)
})
