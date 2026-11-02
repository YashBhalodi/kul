// The screenshot tour: every named state in both themes, written to
// test-results/shots/<state>-<theme>-<width>.png for a person (or an agent)
// to review against the design checklist. axe runs on every state, and a
// serious or critical violation fails the tour. The PNGs are artefacts, not
// golden images.

import { mkdirSync, writeFileSync } from "node:fs"
import { join } from "node:path"

import AxeBuilder from "@axe-core/playwright"
import { expect, test } from "./fixtures"
import type { Page } from "./fixtures"

import {
  chooseFromSwitcher,
  createFile,
  editorContent,
  editorToken,
  fileRows,
  goToEnd,
  openProject,
  openSwitcher,
  pressMod,
  treeCard,
} from "./support"

const FIXTURES = join(import.meta.dirname, "../test-results/tour-fixtures")

/** Add a person with no gender (an error) at the end of the open file. */
async function typeError(page: Page) {
  await goToEnd(page)
  await page.keyboard.type('\n\nperson zola name:"Zola"', { delay: 5 })
  await page.keyboard.press("Escape")
  await expect(
    editorContent(page).locator(".cm-lintRange-error").first()
  ).toBeVisible()
}

/** A .kul file on disk for the tour to import. */
function fixture(name: string, text: string): string {
  mkdirSync(FIXTURES, { recursive: true })
  const path = join(FIXTURES, name)
  writeFileSync(path, text)
  return path
}

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
  { name: "app-shell", path: "/app/", widths: [1440], enter: openProject },
  {
    name: "canvas-person",
    path: "/app/",
    widths: [1440],
    enter: async (page) => {
      await openProject(page)
      await treeCard(page, "adaeze").click()
      await expect(page.getByTestId("details-title")).toHaveText("Adaeze Okeke")
    },
  },
  {
    name: "canvas-marriage",
    path: "/app/",
    widths: [1440],
    enter: async (page) => {
      await openProject(page)
      await page
        .locator(
          '[data-testid=tree] .kul-hit[data-marriage-id="m_emeka_ngozi"]'
        )
        .click({ force: true })
      await expect(page.getByTestId("details-title")).toHaveText(
        "Emeka Okeke & Ngozi Okeke"
      )
    },
  },
  {
    name: "canvas-linked",
    path: "/app/",
    widths: [1440],
    enter: async (page) => {
      await openProject(page)
      await editorToken(page, "declaration", "ngozi").click()
      await expect(treeCard(page, "ngozi")).toHaveAttribute("data-linked", "")
    },
  },
  {
    name: "canvas-error",
    path: "/app/",
    widths: [1440],
    enter: async (page) => {
      await openProject(page)
      await typeError(page)
      await expect(page.getByTestId("stale-tree-banner")).toBeVisible()
    },
  },
  {
    name: "canvas-empty",
    path: "/app/",
    widths: [1440],
    enter: async (page) => {
      await openProject(page)
      await chooseFromSwitcher(page, ["New blank project"])
      await expect(page.getByTestId("tree-empty")).toBeVisible()
    },
  },
  {
    name: "canvas-legend",
    path: "/app/",
    widths: [1440],
    enter: async (page) => {
      await openProject(page)
      await page.getByTestId("legend-toggle").click()
      await expect(page.getByTestId("legend")).toBeVisible()
    },
  },
  {
    name: "editor-docked-left",
    path: "/app/",
    widths: [1440],
    enter: async (page) => {
      await openProject(page)
      await page.getByRole("button", { name: "Dock to the left" }).click()
      await expect(page.getByTestId("editor-window")).toHaveAttribute(
        "data-dock",
        "left"
      )
    },
  },
  {
    name: "editor-minimized",
    path: "/app/",
    widths: [1440],
    enter: async (page) => {
      await openProject(page)
      await typeError(page)
      await page.getByRole("button", { name: "Minimize the editor" }).click()
      await expect(page.getByTestId("editor-pill")).toBeVisible()
    },
  },
  {
    name: "editor-completion",
    path: "/app/",
    widths: [1440],
    enter: async (page) => {
      await openProject(page)
      await goToEnd(page)
      await page.keyboard.type('\n\nperson zola name:"Zola" ', { delay: 5 })
      await expect(page.locator(".cm-tooltip-autocomplete")).toBeVisible()
    },
  },
  {
    name: "editor-hover",
    path: "/app/",
    widths: [1440],
    enter: async (page) => {
      await openProject(page)
      await editorToken(page, "declaration", "adaeze").hover()
      await expect(page.getByTestId("hover")).toBeVisible()
    },
  },
  {
    name: "editor-rename",
    path: "/app/",
    widths: [1440],
    enter: async (page) => {
      await openProject(page)
      await editorToken(page, "declaration", "emeka").click()
      await page.keyboard.press("F2")
      await expect(page.getByTestId("rename-box")).toBeVisible()
    },
  },
  {
    name: "editor-quick-fix",
    path: "/app/",
    widths: [1440],
    enter: async (page) => {
      await openProject(page)
      await typeError(page)
      await editorToken(page, "declaration", "zola").click()
      await pressMod(page, ".")
      await expect(page.getByTestId("quick-fix-menu")).toBeVisible()
    },
  },
  {
    name: "editor-problems",
    path: "/app/",
    widths: [1440],
    enter: async (page) => {
      await openProject(page)
      await typeError(page)
      await page.getByTestId("problems-count").click()
      await expect(page.getByTestId("problem")).toHaveCount(1)
    },
  },
  {
    name: "project-switcher",
    path: "/app/",
    widths: [1440],
    enter: async (page) => {
      await openProject(page)
      await openSwitcher(page)
      await page.getByRole("menuitem", { name: "New from example" }).hover()
      await expect(page.getByRole("menu")).toHaveCount(2)
    },
  },
  {
    name: "file-name-error",
    path: "/app/",
    widths: [1440],
    enter: async (page) => {
      await openProject(page)
      await page.getByRole("button", { name: "New file" }).click()
      await page
        .getByRole("textbox", { name: "New file name" })
        .fill("Three-Generations")
      await expect(page.getByRole("alert")).toBeVisible()
    },
  },
  {
    name: "delete-file",
    path: "/app/",
    widths: [1440],
    enter: async (page) => {
      await openProject(page)
      const row = fileRows(page).first()
      await row.hover()
      await row.getByRole("button", { name: /^Actions for/ }).click()
      await page.getByRole("menuitem", { name: "Delete…" }).click()
      await expect(page.getByRole("alertdialog")).toBeVisible()
    },
  },
  {
    name: "import-collision",
    path: "/app/",
    widths: [1440],
    enter: async (page) => {
      await openProject(page)
      const chooser = page.waitForEvent("filechooser")
      await chooseFromSwitcher(page, [
        "Import into this project",
        "Files or a .zip…",
      ])
      await (
        await chooser
      ).setFiles([
        fixture("three-generations.kul", 'person a name:"A" gender:other\n'),
        fixture("Notes.kul", 'person b name:"B" gender:other\n'),
      ])
      await expect(page.getByRole("dialog")).toBeVisible()
    },
  },
  {
    name: "persistence-banner",
    path: "/app/",
    widths: [1440],
    enter: async (page) => {
      await openProject(page)
      await page.evaluate(() =>
        Object.assign(navigator.storage, {
          persist: () => Promise.resolve(false),
        })
      )
      await createFile(page, "notes")
      await expect(page.getByTestId("persistence-banner")).toBeVisible()
    },
  },
  {
    name: "read-only-banner",
    path: "/app/",
    widths: [1440],
    enter: async (page) => {
      await openProject(page)
      // Another tab takes the project over; this one turns read-only.
      const other = await page.context().newPage()
      await openProject(other, page.url())
      await other.getByRole("button", { name: "Use here instead" }).click()
      await other.close()
      await expect(page.getByTestId("read-only-banner")).toBeVisible()
    },
  },
  {
    name: "settings",
    path: "/app/",
    widths: [1440],
    enter: async (page) => {
      await openProject(page)
      await pressMod(page, "K")
      await page.getByRole("option", { name: "Settings", exact: true }).click()
      await expect(page.getByTestId("storage-usage")).toContainText("using")
    },
  },
  {
    name: "command-palette",
    path: "/app/",
    widths: [1440],
    enter: async (page) => {
      await openProject(page)
      await pressMod(page, "K")
      await expect(page.getByRole("dialog")).toBeVisible()
    },
  },
  {
    name: "shortcuts",
    path: "/app/",
    widths: [1440],
    enter: async (page) => {
      await openProject(page)
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

        // Let entry animations (dialogs, menus) settle before the capture.
        // (Endless ones, such as the editor's blinking cursor, are left
        // running, and one replaced while waiting no longer counts.)
        await page.evaluate(() =>
          Promise.all(
            document
              .getAnimations()
              .filter(
                (animation) =>
                  animation.effect?.getComputedTiming().endTime !== Infinity
              )
              .map((animation) => animation.finished.catch(() => undefined))
          )
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
