import { expect, test } from "./fixtures"

import { createFile, openProject, pressMod } from "./support"

test.describe("when the browser won't promise to keep storage", () => {
  test.beforeEach(async ({ page }) => {
    await page.addInitScript(() => {
      Object.assign(navigator.storage, {
        persist: () => Promise.resolve(false),
        persisted: () => Promise.resolve(false),
      })
    })
  })

  test("the first edit shows a reminder to back up, until dismissed", async ({
    page,
  }) => {
    await openProject(page)
    const banner = page.getByTestId("persistence-banner")
    await expect(banner).toBeHidden()

    await createFile(page, "first-edit")
    await expect(banner).toHaveText(
      /Your browser may clear these files\. Export a backup\./
    )
    const download = page.waitForEvent("download")
    await banner.getByRole("button", { name: "Download backup" }).click()
    expect((await download).suggestedFilename()).toMatch(
      /^The[ _]Okeke[ _]family\.zip$/
    )

    await banner.getByRole("button", { name: "Dismiss" }).click()
    await expect(banner).toBeHidden()

    // Dismissed for good in this browser.
    await page.reload()
    await createFile(page, "second-edit")
    await expect(page.getByTestId("save-status")).toHaveText("Saved")
    await expect(banner).toBeHidden()
  })

  test("settings show the storage used and that it may be cleared", async ({
    page,
  }) => {
    await openProject(page)
    await pressMod(page, "K")
    await page.getByRole("option", { name: "Settings", exact: true }).click()
    const dialog = page.getByRole("dialog", { name: "Settings" })
    await expect(dialog.getByTestId("storage-usage")).toHaveText(
      /^1 project, using .+ of about .+ available\.$/
    )
    await expect(dialog.getByTestId("persistence")).toContainText(
      "may clear your projects"
    )
    await expect(
      dialog.getByRole("button", { name: "Ask the browser to keep them" })
    ).toBeVisible()
  })
})

test("where the browser can't store files, the app works from memory and says so", async ({
  browser,
  baseURL,
}) => {
  // An ordinary browser context: Playwright's WebKit has no origin private
  // file system in one, and the other browsers are told to refuse it.
  const context = await browser.newContext({ baseURL })
  await context.addInitScript(() => {
    Object.assign(navigator.storage, {
      getDirectory: () =>
        Promise.reject(new DOMException("No storage", "SecurityError")),
    })
  })
  const page = await context.newPage()
  try {
    await openProject(page)
    await expect(page.getByTestId("storage-banner")).toContainText(
      "can’t save files"
    )
    await createFile(page, "kept-for-now")
    await expect(page.getByTestId("file-row")).toHaveCount(2)
    // Edits don't ask the browser to keep storage it doesn't have.
    await expect(page.getByTestId("persistence-banner")).toBeHidden()
  } finally {
    await context.close()
  }
})
