// The landing page's layout: the calls to action above the fold on a
// laptop screen, nothing wider than a phone's screen, and the tree drawn
// whole at every width.

import { expect, test } from "./fixtures"

test("both calls to action are above the fold at 1440 × 900", async ({
  page,
}) => {
  await page.setViewportSize({ width: 1440, height: 900 })
  await page.goto("/")
  const main = page.getByRole("main")
  for (const name of ["Open Kul", "Learn the language"]) {
    await expect(main.getByRole("link", { name }).first()).toBeInViewport({
      ratio: 1,
    })
  }
})

for (const width of [360, 390, 768, 1440]) {
  test(`nothing overflows and the tree is whole at ${String(width)} px`, async ({
    page,
  }) => {
    await page.setViewportSize({ width, height: 900 })
    await page.goto("/")
    await page.evaluate(() => document.fonts.ready)
    const overflow = await page.evaluate(
      () => document.documentElement.scrollWidth - window.innerWidth
    )
    expect(overflow, "horizontal overflow in px").toBeLessThanOrEqual(0)

    // Every element inside the main content stays within the screen.
    const outside = await page.evaluate(() =>
      [...document.querySelectorAll("main *")]
        .filter((element) => {
          const box = element.getBoundingClientRect()
          return (
            box.width > 0 &&
            (box.left < -0.5 || box.right > window.innerWidth + 0.5)
          )
        })
        .map((element) => element.outerHTML.slice(0, 80))
    )
    expect(outside).toEqual([])

    // The tree fits inside its figure: no card is cut off.
    const tree = page.locator(".landing-tree")
    const frame = await tree.boundingBox()
    const svg = await tree.locator("svg").boundingBox()
    expect(frame).not.toBeNull()
    expect(svg).not.toBeNull()
    if (frame && svg) {
      expect(svg.x).toBeGreaterThanOrEqual(frame.x)
      expect(svg.y).toBeGreaterThanOrEqual(frame.y)
      expect(svg.x + svg.width).toBeLessThanOrEqual(frame.x + frame.width + 0.5)
      expect(svg.y + svg.height).toBeLessThanOrEqual(
        frame.y + frame.height + 0.5
      )
    }
    await expect(tree.locator(".kul-card")).toHaveCount(6)
  })
}

test("the page describes itself to link previews and crawlers", async ({
  page,
  request,
}) => {
  await page.goto("/")
  await expect(page.locator('link[rel="canonical"]')).toHaveAttribute(
    "href",
    "https://kullang.dev/"
  )
  await expect(page.locator('meta[property="og:image"]')).toHaveAttribute(
    "content",
    "https://kullang.dev/og.png"
  )
  for (const [path, type] of [
    ["/og.png", "image/png"],
    ["/apple-touch-icon.png", "image/png"],
    ["/favicon.ico", "image/x-icon"],
    ["/sitemap.xml", "application/xml"],
    ["/robots.txt", "text/plain"],
  ] as const) {
    const response = await request.get(path)
    expect(response.status(), path).toBe(200)
    expect(response.headers()["content-type"], path).toContain(type)
  }
  const sitemap = await (await request.get("/sitemap.xml")).text()
  expect(sitemap).toContain("<loc>https://kullang.dev/</loc>")
  expect(sitemap).toContain("<loc>https://kullang.dev/docs/quick-start/</loc>")
})
