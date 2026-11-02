import { expect, test } from "@playwright/test"

test.describe("with JavaScript disabled", () => {
  test.use({ javaScriptEnabled: false })

  test("the landing page reads fully and its calls to action are links", async ({
    page,
  }) => {
    await page.goto("/")
    await expect(page.getByRole("heading", { level: 1 })).toHaveText(
      "Describe a family in plain text. Kul draws the tree."
    )
    await expect(
      page.getByRole("heading", { name: "Why a language" })
    ).toBeVisible()
    await expect(page.locator("pre code")).toContainText(
      "marriage m_ramesh_sita"
    )
    await expect(
      page.getByRole("main").getByRole("link", { name: "Open Kul" })
    ).toHaveAttribute("href", "/app")
    await expect(
      page.getByRole("link", { name: "Learn the language" })
    ).toHaveAttribute("href", "/docs/")
  })

  test("the docs read fully and link to each other", async ({ page }) => {
    await page.goto("/docs/")
    await expect(page.getByRole("heading", { level: 1 })).toHaveText(
      "Introduction"
    )
    await page
      .getByRole("navigation", { name: "Docs" })
      .getByRole("link", { name: "Syntax cheat sheet" })
      .click()
    await expect(page).toHaveURL(/\/docs\/cheat-sheet\/$/)
    await expect(page.getByRole("heading", { level: 1 })).toHaveText(
      "Syntax cheat sheet"
    )
  })
})

test("each docs page links its Markdown variant", async ({ page, request }) => {
  await page.goto("/docs/cheat-sheet/")
  const href = await page
    .locator('link[rel="alternate"][type="text/markdown"]')
    .getAttribute("href")
  expect(href).toBe("/docs/cheat-sheet.md")
  const markdown = await request.get(href ?? "")
  expect(markdown.headers()["content-type"]).toMatch(/^text\/markdown/)
  expect(await markdown.text()).toMatch(/^# Syntax cheat sheet\n/)
})

test("llms.txt and llms-full.txt are plain text", async ({ request }) => {
  for (const path of ["/llms.txt", "/llms-full.txt"]) {
    const response = await request.get(path)
    expect(response.status()).toBe(200)
    expect(response.headers()["content-type"]).toMatch(/^text\/plain/)
  }
})
