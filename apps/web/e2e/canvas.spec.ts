// The canvas: the live tree, its camera, selection and the details panel,
// the two-way link with the editor, past records' return badges, the error
// state, the empty project, and the SVG download.

import { readFile } from "node:fs/promises"

import { expect, test } from "./fixtures"
import type { Page } from "./fixtures"
import type { Locator } from "@playwright/test"

import {
  cameraOf,
  chooseFromSwitcher,
  editorContent,
  editorSelection,
  editorToken,
  goToEnd,
  openExample,
  openFile,
  openProject,
  treeCard,
  treeMarriage,
} from "./support"

interface Box {
  x: number
  y: number
  width: number
  height: number
}

async function boxOf(locator: Locator): Promise<Box> {
  const box = await locator.boundingBox()
  if (!box) throw new Error("not on screen")
  return box
}

const overlaps = (a: Box, b: Box) =>
  a.x < b.x + b.width &&
  b.x < a.x + a.width &&
  a.y < b.y + b.height &&
  b.y < a.y + a.height

/** `target` is on the canvas, and neither the editor nor the panel hides it. */
async function expectInView(page: Page, target: Locator) {
  await expect(async () => {
    const box = await boxOf(target)
    const canvas = await boxOf(page.getByTestId("tree-viewport"))
    expect(box.x).toBeGreaterThanOrEqual(canvas.x)
    expect(box.y).toBeGreaterThanOrEqual(canvas.y)
    expect(box.x + box.width).toBeLessThanOrEqual(canvas.x + canvas.width)
    expect(box.y + box.height).toBeLessThanOrEqual(canvas.y + canvas.height)
    for (const chrome of [
      page.getByTestId("editor-window"),
      page.getByTestId("details-panel"),
    ]) {
      if (await chrome.isVisible()) {
        expect(overlaps(box, await boxOf(chrome))).toBe(false)
      }
    }
  }).toPass({ timeout: 5000 })
}

/** `target` lies wholly off the canvas. */
async function expectOutOfView(page: Page, target: Locator) {
  await expect(async () => {
    const box = await boxOf(target)
    const canvas = await boxOf(page.getByTestId("tree-viewport"))
    expect(overlaps(box, canvas)).toBe(false)
  }).toPass({ timeout: 5000 })
}

/** Pan the canvas with its arrow keys: `presses` steps towards `key`. */
async function panWithKeys(page: Page, key: string, presses: number) {
  await page.getByTestId("tree-viewport").focus()
  for (let press = 0; press < presses; press++) await page.keyboard.press(key)
}

test("clicking a card opens its details and puts the editor on its declaration", async ({
  page,
}) => {
  await openProject(page)
  await treeCard(page, "adaeze").click()

  const panel = page.getByTestId("details-panel")
  await expect(panel).toBeVisible()
  await expect(page.getByTestId("details-title")).toHaveText("Adaeze Okeke")
  await expect(panel).toContainText("1930-03")
  await expect(treeCard(page, "adaeze")).toHaveAttribute("data-selected", "")
  await expect.poll(() => editorSelection(page)).toBe("adaeze")
  // The canvas keeps the keyboard; the editor only moved its cursor.
  await expect(page.getByTestId("tree-viewport")).toBeFocused()

  // A related person in the panel selects that person.
  await panel.getByRole("button", { name: "Emeka Okeke" }).click()
  await expect(page.getByTestId("details-title")).toHaveText("Emeka Okeke")
  await expect.poll(() => editorSelection(page)).toBe("emeka")

  // Esc clears the selection; so does a click on the background.
  await page.getByTestId("tree-viewport").focus()
  await page.keyboard.press("Escape")
  await expect(panel).toBeHidden()
  await treeCard(page, "chidi").click()
  await expect(panel).toBeVisible()
  const canvas = await boxOf(page.getByTestId("tree-viewport"))
  await page.mouse.click(canvas.x + 720, canvas.y + 20)
  await expect(panel).toBeHidden()
  await expect(page.locator("[data-testid=tree] [data-selected]")).toHaveCount(
    0
  )
})

test("clicking a marriage line selects the marriage", async ({ page }) => {
  await openProject(page)
  await page
    .locator('[data-testid=tree] .kul-hit[data-marriage-id="m_emeka_ngozi"]')
    .click({ force: true })
  await expect(page.getByTestId("details-title")).toHaveText(
    "Emeka Okeke & Ngozi Okeke"
  )
  await expect(treeMarriage(page, "m_emeka_ngozi")).toHaveAttribute(
    "data-selected",
    ""
  )
  await expect.poll(() => editorSelection(page)).toBe("m_emeka_ngozi")
})

test("the cursor on a reference in another file marks and reveals its entity", async ({
  page,
}) => {
  await openProject(page)
  await openExample(page, "A multi-file project")
  const wedding = treeMarriage(page, "m_diego_carmen")
  await expect(wedding).toHaveCount(1)
  // Look up, past the top of the tree, until it is out of sight.
  await panWithKeys(page, "ArrowUp", 20)
  await expectOutOfView(page, wedding)

  await openFile(page, "02-children.kul")
  await editorToken(page, "reference", "m_diego_carmen").click()
  await expect(wedding).toHaveAttribute("data-linked", "")
  await expectInView(page, wedding)
  // Linking leaves the keyboard in the editor.
  await expect(editorContent(page)).toBeFocused()
  await expect(page.getByTestId("details-panel")).toBeHidden()
})

test("editing another person keeps the selection and the camera", async ({
  page,
}) => {
  await openProject(page)
  await treeCard(page, "chidi").click()
  await expect(page.getByTestId("details-title")).toHaveText("Chidi Okeke")
  await page.getByTestId("zoom-fit").click()
  await expect.poll(() => cameraOf(page).then((c) => c.zoom)).toBeLessThan(1)
  // Let the fit settle before reading the camera.
  await page.waitForTimeout(400)
  const camera = await cameraOf(page)

  // Adaeze dies in 2001.
  await editorToken(page, "declaration", "adaeze").click()
  await page.keyboard.press("End")
  await page.keyboard.type(" died:2001", { delay: 10 })
  await expect(treeCard(page, "adaeze")).toHaveAttribute("data-died", "2001")

  await expect(treeCard(page, "chidi")).toHaveAttribute("data-selected", "")
  await expect(page.getByTestId("details-title")).toHaveText("Chidi Okeke")
  expect(await cameraOf(page)).toEqual(camera)
})

test("deleting the selected person clears the selection", async ({ page }) => {
  await openProject(page)
  await treeCard(page, "amara").click()
  await expect(page.getByTestId("details-title")).toHaveText("Amara Okeke")
  await expect.poll(() => editorSelection(page)).toBe("amara")

  // Delete Amara's two lines: the declaration and her birth.
  await editorContent(page).focus()
  await page.keyboard.press("Home")
  await page.keyboard.press("Shift+ArrowDown")
  await page.keyboard.press("Shift+ArrowDown")
  await page.keyboard.press("Backspace")

  await expect(treeCard(page, "amara")).toHaveCount(0)
  await expect(page.getByTestId("details-panel")).toBeHidden()
  await expect(page.locator("[data-testid=tree] [data-selected]")).toHaveCount(
    0
  )
})

test("a past record's badge glides to the person's own card", async ({
  page,
}) => {
  await openProject(page)
  await openExample(page, "Divorce, remarriage, and a blended family")
  const badge = page.locator(
    '[data-testid=tree] .kul-card[data-kind="ghost"][data-person-id="erik"] .kul-ghost-badge'
  )
  await expect(badge).toHaveCount(1)
  // Zoom in on the past record until Erik's own card is out of sight.
  const at = await boxOf(badge)
  await page.mouse.move(at.x + at.width / 2, at.y + at.height / 2)
  await page.keyboard.down("Control")
  for (let step = 0; step < 4; step++) await page.mouse.wheel(0, -200)
  await page.keyboard.up("Control")
  const own = treeCard(page, "erik")
  await expectOutOfView(page, own)

  await badge.click()
  await expectInView(page, own)
  // Jumping is navigation only: nothing is selected.
  await expect(page.getByTestId("details-panel")).toBeHidden()
})

test("Fit shows the whole tree, 100% shows it at its own size, and the camera is saved", async ({
  page,
}) => {
  await openProject(page)
  await openExample(page, "A family across a century", "The Adeyemi family")
  await expect(treeCard(page, "oludare")).toHaveCount(1)
  await page.getByTestId("zoom-actual").click()
  await expect(page.getByTestId("zoom-level")).toHaveText("100%")
  await expect.poll(() => cameraOf(page).then((c) => c.zoom)).toBe(1)

  await page.getByRole("button", { name: "Zoom in" }).click()
  await expect(page.getByTestId("zoom-level")).toHaveText("125%")

  // 0 on the canvas fits the tree beside the editor window.
  await page.getByTestId("tree-viewport").focus()
  await page.keyboard.press("0")
  await expect(page.getByTestId("zoom-level")).not.toHaveText("125%")
  await expectInView(page, page.locator("[data-testid=tree] > svg"))
  expect((await cameraOf(page)).zoom).toBeLessThan(1)

  // The camera is the project's: a reload opens on the same view.
  await page.waitForTimeout(1200)
  const camera = await cameraOf(page)
  await page.reload()
  await expect(
    page.locator("[data-testid=tree] .kul-card").first()
  ).toBeVisible()
  await expect.poll(() => cameraOf(page)).toEqual(camera)
})

test("dragging pans, and Ctrl+scroll zooms around the pointer", async ({
  page,
}) => {
  await openProject(page)
  const canvas = await boxOf(page.getByTestId("tree-viewport"))
  const start = await cameraOf(page)
  const from = { x: canvas.x + canvas.width - 100, y: canvas.y + 60 }
  await page.mouse.move(from.x, from.y)
  await page.mouse.down()
  await page.mouse.move(from.x - 60, from.y + 40, { steps: 5 })
  await page.mouse.up()
  const dragged = await cameraOf(page)
  expect(dragged.x - start.x).toBeCloseTo(-60, 0)
  expect(dragged.y - start.y).toBeCloseTo(40, 0)
  // A drag is not a click: nothing was selected.
  await expect(page.getByTestId("details-panel")).toBeHidden()

  const card = await boxOf(treeCard(page, "emeka"))
  const pointer = { x: card.x + 10, y: card.y + 10 }
  await page.mouse.move(pointer.x, pointer.y)
  await page.keyboard.down("Control")
  await page.mouse.wheel(0, -100)
  await page.keyboard.up("Control")
  await expect.poll(() => cameraOf(page).then((c) => c.zoom)).toBeGreaterThan(1)
  // The point under the pointer stayed put.
  const zoomed = await boxOf(treeCard(page, "emeka"))
  const zoom = (await cameraOf(page)).zoom
  // (Within a few px: the card's box includes its outline.)
  expect(Math.abs(zoomed.x + 10 * zoom - pointer.x)).toBeLessThan(3)
  expect(Math.abs(zoomed.y + 10 * zoom - pointer.y)).toBeLessThan(3)
})

test("with errors the canvas keeps the last valid tree, dimmed", async ({
  page,
}) => {
  await openProject(page)
  await goToEnd(page)
  await page.keyboard.type('\n\nperson zola name:"Zola"', { delay: 5 })
  await page.keyboard.press("Escape")

  const tree = page.getByTestId("tree")
  await expect(tree).toHaveAttribute("data-stale", "true")
  await expect(treeCard(page, "adaeze")).toBeVisible()
  const banner = page.getByTestId("stale-tree-banner")
  await expect(banner).toHaveText("1 error — showing the last valid tree")
  await banner.click()
  await expect(page.getByTestId("problem")).toHaveCount(1)

  // Fixing it draws the new tree, no longer dimmed.
  await editorContent(page).focus()
  await page.keyboard.type(" gender:female", { delay: 5 })
  await expect(treeCard(page, "zola")).toBeVisible()
  await expect(tree).not.toHaveAttribute("data-stale")
  await expect(banner).toBeHidden()
})

test("an empty project offers to add the first person", async ({ page }) => {
  await openProject(page)
  await chooseFromSwitcher(page, ["New blank project"])
  const empty = page.getByTestId("tree-empty")
  await expect(empty).toBeVisible()
  await empty.getByRole("button", { name: "Add your first person" }).click()
  await expect(treeCard(page, "me")).toBeVisible()
  await expect(empty).toBeHidden()
  await expect.poll(() => editorSelection(page)).toBe("me")
})

test("the legend lists what the tree draws", async ({ page }) => {
  await openProject(page)
  await page.getByTestId("legend-toggle").click()
  const legend = page.getByTestId("legend")
  await expect(legend.getByRole("listitem")).toHaveText([
    "Male",
    "Female",
    "Birth",
    "Marriage",
  ])
  await page.getByTestId("legend-toggle").click()
  await expect(legend).toBeHidden()
})

test("the downloaded tree is a self-contained SVG with a legend", async ({
  page,
}) => {
  await openProject(page)
  await page.getByTestId("export-menu").click()
  const download = page.waitForEvent("download")
  await page.getByTestId("download-tree").click()
  const file = await download
  expect(file.suggestedFilename()).toMatch(/^The[ _]Okeke[ _]family\.svg$/)
  const path = await file.path()
  const text = await readFile(path, "utf8")

  const report = await page.evaluate((source) => {
    const doc = new DOMParser().parseFromString(source, "image/svg+xml")
    const root = doc.documentElement
    const external: Array<string> = []
    for (const element of doc.querySelectorAll("*")) {
      for (const attribute of element.attributes) {
        const value = attribute.value.trim()
        if (
          (attribute.name === "href" || attribute.name === "xlink:href") &&
          !value.startsWith("#")
        ) {
          external.push(`${attribute.name}=${value}`)
        }
        for (const match of value.matchAll(/url\(\s*['"]?([^'")]*)/g)) {
          if (!match[1]?.startsWith("#")) external.push(match[0])
        }
      }
    }
    for (const style of doc.querySelectorAll("style")) {
      for (const match of style.textContent.matchAll(
        /url\(\s*['"]?([^'")]*)|@import/g
      )) {
        if (!match[1]?.startsWith("#")) external.push(match[0])
      }
    }
    return {
      parsed: doc.querySelector("parsererror") === null,
      root: root.localName,
      namespace: root.namespaceURI,
      legend: doc.querySelectorAll(".kul-legend .kul-legend-label").length,
      cards: doc.querySelectorAll(".kul-card[data-person-id]").length,
      external,
    }
  }, text)

  expect(report).toEqual({
    parsed: true,
    root: "svg",
    namespace: "http://www.w3.org/2000/svg",
    legend: 4,
    cards: 6,
    external: [],
  })
})
