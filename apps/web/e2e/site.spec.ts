import { expect, test } from "./fixtures"

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
    await expect(page.locator("[data-kul-code]").first()).toContainText(
      "marriage m_ramesh_sita"
    )
    await page
      .getByRole("navigation", { name: "Docs", exact: true })
      .getByRole("link", { name: "Syntax cheat sheet" })
      .click()
    await expect(page).toHaveURL(/\/docs\/reference\/cheat-sheet\/$/)
    await expect(page.getByRole("heading", { level: 1 })).toHaveText(
      "Syntax cheat sheet"
    )
  })
})

test("each docs page links its Markdown variant", async ({ page, request }) => {
  await page.goto("/docs/reference/cheat-sheet/")
  const href = await page
    .locator('link[rel="alternate"][type="text/markdown"]')
    .getAttribute("href")
  expect(href).toBe("/docs/reference/cheat-sheet.md")
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

test("every link in llms.txt resolves on this site", async ({ request }) => {
  const text = await (await request.get("/llms.txt")).text()
  expect(text).toMatch(/^# Kul\n\n> \S/)
  expect(text).toContain("## Optional")
  const links = [...text.matchAll(/\]\((https:\/\/kullang\.dev[^)]+)\)/g)].map(
    (match) => new URL(match[1] ?? "").pathname
  )
  // Every docs page's Markdown variant, the full file and nothing missing.
  expect(links).toContain("/llms-full.txt")
  expect(links).toContain("/docs/reference/diagnostics.md")
  expect(links.filter((path) => path.endsWith(".md")).length).toBe(20)
  for (const path of links) {
    const response = await request.get(path)
    expect(response.status(), path).toBe(200)
    expect(response.headers()["content-type"], path).toMatch(
      path.endsWith(".md") ? /^text\/markdown/ : /^text\/plain/
    )
  }
  const full = await (await request.get("/llms-full.txt")).text()
  expect(full).toContain("# Syntax cheat sheet\n")
  expect(full).toContain("# Diagnostics\n")
  expect(full).toContain("```kul expect-error=KUL-R03")
})
