import { expect, test } from "./fixtures"

import { PROJECT_URL, expectAppReady, pressMod } from "./support"

test("a deep link into the app is served the shell", async ({ page }) => {
  const response = await page.goto("/app/3f2c9a1e-project")
  expect(response?.status()).toBe(200)
  await expect(page).toHaveURL(/\/app\/3f2c9a1e-project\/?$/)
  await expectAppReady(page)
  await expect(page.getByTestId("canvas")).toBeVisible()
  await expect(page).toHaveTitle("Kul")
  // No project has that id in this browser.
  await expect(page.getByTestId("project-missing")).toContainText(
    "This project isn’t in this browser"
  )
})

test("/app opens the app at a project", async ({ page }) => {
  await page.goto("/app")
  await expect(page).toHaveURL(PROJECT_URL)
  await expectAppReady(page)
})

test("Mod+K opens the command palette and runs a command", async ({ page }) => {
  await page.goto("/app/")
  await expectAppReady(page)
  await pressMod(page, "K")
  const palette = page.getByRole("dialog", { name: "Command palette" })
  await expect(palette).toBeVisible()
  await expect(
    palette.getByRole("option", { name: "Toggle theme" })
  ).toBeVisible()

  await expect(palette.getByRole("combobox")).toBeFocused()
  await page.keyboard.type("dark theme")
  await page.keyboard.press("Enter")
  await expect(palette).toBeHidden()
  await expect(page.locator("html")).toHaveClass(/\bdark\b/)

  await pressMod(page, "K")
  await expect(palette).toBeVisible()
  await page.keyboard.press("Escape")
  await expect(palette).toBeHidden()
})

test("? lists the keyboard shortcuts", async ({ page }) => {
  await page.goto("/app/")
  await expectAppReady(page)
  await page.keyboard.press("?")
  const dialog = page.getByRole("dialog", { name: "Keyboard shortcuts" })
  await expect(dialog).toBeVisible()
  await expect(dialog).toContainText("Open the command palette")
})

test.describe("on a phone-sized screen", () => {
  test.use({ viewport: { width: 390, height: 844 } })

  test("the app shows the desktop-only notice", async ({ page }) => {
    await page.goto("/app/")
    const gate = page.getByTestId("desktop-gate")
    await expect(gate).toBeVisible()
    await expect(
      page.getByRole("heading", { name: "Kul’s editor needs a larger screen" })
    ).toBeVisible()
    await expect(
      gate.getByRole("link", { name: "Learn the language" })
    ).toHaveAttribute("href", "/docs/")
    await expect(page.getByTestId("palette-trigger")).toHaveCount(0)
  })
})
