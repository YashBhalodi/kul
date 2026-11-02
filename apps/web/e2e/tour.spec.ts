// The screenshot tour: every named state in both themes, written to
// test-results/shots/<state>-<theme>-<width>.png for a person (or an agent)
// to review against the design checklist. axe runs on every state, and a
// serious or critical violation fails the tour. The PNGs are artefacts, not
// golden images.

import AxeBuilder from "@axe-core/playwright"
import { expect, test } from "@playwright/test"
import type { Page } from "@playwright/test"

import { expectAppReady, pressMod } from "./support"

type Theme = "light" | "dark"

interface State {
  name: string
  path: string
  widths: ReadonlyArray<number>
  /** Bring the page into the state once it has loaded. */
  enter?: (page: Page) => Promise<void>
  /** Capture the whole scrolling page rather than the viewport. */
  fullPage?: boolean
}

const STATES: ReadonlyArray<State> = [
  { name: "landing", path: "/", widths: [390, 768, 1440], fullPage: true },
  { name: "docs", path: "/docs/", widths: [390, 1440], fullPage: true },
  {
    name: "docs-cheat-sheet",
    path: "/docs/cheat-sheet/",
    widths: [1440],
    fullPage: true,
  },
  { name: "app-shell", path: "/app/", widths: [1440], enter: expectAppReady },
  {
    name: "command-palette",
    path: "/app/",
    widths: [1440],
    enter: async (page) => {
      await expectAppReady(page)
      await pressMod(page, "K")
      await expect(page.getByRole("dialog")).toBeVisible()
    },
  },
  {
    name: "shortcuts",
    path: "/app/",
    widths: [1440],
    enter: async (page) => {
      await expectAppReady(page)
      await page.keyboard.press("?")
      await expect(page.getByRole("dialog")).toBeVisible()
    },
  },
  {
    name: "gate",
    path: "/app/",
    widths: [390],
    enter: async (page) => {
      await expect(page.getByTestId("desktop-gate")).toBeVisible()
    },
  },
]

const THEMES: ReadonlyArray<Theme> = ["light", "dark"]

for (const state of STATES) {
  for (const theme of THEMES) {
    for (const width of state.widths) {
      test(`${state.name} ${theme} ${width}`, async ({ page }) => {
        await page.setViewportSize({ width, height: width < 768 ? 844 : 900 })
        await page.emulateMedia({ colorScheme: theme, reducedMotion: "reduce" })
        await page.goto(state.path)
        await page.evaluate(() => document.fonts.ready)
        await state.enter?.(page)
        await expect(page.locator("html")).toHaveClass(
          theme === "dark" ? /\bdark\b/ : /^(?!.*\bdark\b)/
        )

        const overflow = await page.evaluate(
          () => document.documentElement.scrollWidth - window.innerWidth
        )
        expect(overflow, "horizontal overflow in px").toBeLessThanOrEqual(0)

        await page.screenshot({
          path: `test-results/shots/${state.name}-${theme}-${width}.png`,
          fullPage: state.fullPage ?? false,
        })

        const { violations } = await new AxeBuilder({ page }).analyze()
        const blocking = violations.filter(
          (violation) =>
            violation.impact === "serious" || violation.impact === "critical"
        )
        expect(
          blocking.map(
            (violation) =>
              `${violation.id}: ${violation.help} (${violation.nodes
                .map((node) => node.target.join(" "))
                .join(", ")})`
          )
        ).toEqual([])
      })
    }
  }
}
