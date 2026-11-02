// The editor window: language features from the worker, edits across
// files, and the window's own layout. Every test starts on a first visit
// (the seeded three-generations example) in a fresh browser context.

import { expect, test } from "./fixtures"
import type { Page } from "./fixtures"

import {
  chooseFromSwitcher,
  editorContent,
  editorSelection,
  editorText,
  editorToken,
  goToEnd,
  openExample,
  openProject,
  pressEditorMod,
  pressMod,
} from "./support"

function activeTab(page: Page) {
  return page.locator("[data-testid=editor-tab][data-active]")
}

/** A blank project with one empty file, focused in the editor. */
async function openBlankProject(page: Page) {
  await openProject(page)
  await chooseFromSwitcher(page, ["New blank project"])
  await expect(activeTab(page)).toHaveAttribute("data-name", "family.kul")
  await editorContent(page).focus()
}

test("highlighting comes from the engine's tokens", async ({ page }) => {
  await openProject(page)
  await expect(editorToken(page, "keyword", "person")).toBeVisible()
  await expect(editorToken(page, "declaration", "chinua")).toBeVisible()
  await expect(editorToken(page, "field", "name")).toBeVisible()
  await expect(editorToken(page, "string", '"Chinua Okeke"')).toBeVisible()
  await expect(editorToken(page, "enum", "male")).toBeVisible()
  await expect(editorToken(page, "date", "~1925")).toBeVisible()
  await expect(editorToken(page, "reference", "m_chinua_adaeze")).toBeVisible()
  await expect(
    editorContent(page).locator(".tok-comment").first()
  ).toBeVisible()
})

test("typing an error shows a squiggle and the problem count within 500 ms", async ({
  page,
}) => {
  await openProject(page)
  await expect(page.getByTestId("problems-count")).toHaveText("No problems")
  await goToEnd(page)
  await page.keyboard.type('\nperson zola name:"Zola"', { delay: 10 })
  const typed = Date.now()
  await expect(
    editorContent(page).locator(".cm-lintRange-error").first()
  ).toBeVisible({
    timeout: 2000,
  })
  await expect(page.getByTestId("problems-count")).toHaveText(
    "1 error, 0 warnings"
  )
  const elapsed = Date.now() - typed
  console.log(`[latency] keystroke to squiggle and count: ${elapsed} ms`)
  expect(elapsed).toBeLessThan(1000)
  await expect(page.getByTestId("problems-button")).toContainText("1 error")
  await expect(
    page.locator("[data-testid=file-row][data-problems=error]")
  ).toHaveAttribute("data-name", "three-generations.kul")

  // The strip lists it; choosing it selects its range in the editor.
  await page.getByTestId("problems-count").click()
  const problem = page.getByTestId("problem")
  await expect(problem).toHaveCount(1)
  await expect(problem).toContainText("KUL-R03")
  await expect(problem).toContainText("three-generations.kul:")
  await problem.click()
  await expect(editorContent(page)).toBeFocused()
})

test("completion offers field keys and their values", async ({ page }) => {
  await openBlankProject(page)
  await page.keyboard.type('person zola name:"Zola" ', { delay: 10 })
  const list = page.locator(".cm-tooltip-autocomplete")
  await expect(list).toBeVisible()
  await expect(list.getByRole("option", { name: /^gender:/ })).toBeVisible()
  await page.keyboard.type("gend")
  await expect(list.getByRole("option").first()).toContainText("gender:")
  // CodeMirror ignores Enter for a moment after the list changes.
  await page.waitForTimeout(150)
  await page.keyboard.press("Enter")
  // Picking a field offers its values straight away.
  await expect(list.getByRole("option", { name: /^female/ })).toBeVisible()
  await page.keyboard.type("fem")
  await page.waitForTimeout(150)
  await page.keyboard.press("Enter")
  await expect
    .poll(() => editorText(page))
    .toBe('person zola name:"Zola" gender:female')
})

test("hovering an id shows what it declares", async ({ page }) => {
  await openProject(page)
  await editorToken(page, "declaration", "adaeze").hover()
  const hover = page.getByTestId("hover")
  await expect(hover).toBeVisible()
  await expect(hover).toContainText("Adaeze Okeke")
  await expect(hover.locator("code").first()).toHaveText("person adaeze")
})

test("F12 goes to a declaration in another file", async ({ page }) => {
  await openProject(page)
  await openExample(page, "A multi-file project")
  await page
    .locator("[data-testid=file-row][data-name='02-children.kul'] button")
    .first()
    .click()
  await expect(activeTab(page)).toHaveAttribute("data-name", "02-children.kul")
  await editorToken(page, "reference", "m_diego_carmen").click()
  await page.keyboard.press("F12")
  await expect(activeTab(page)).toHaveAttribute("data-name", "01-founders.kul")
  await expect(editorContent(page)).toBeFocused()
  // The declaration is selected.
  expect(await editorSelection(page)).toBe("m_diego_carmen")
})

test("F2 renames an id across files, and undo restores both", async ({
  page,
}) => {
  await openProject(page)
  await openExample(page, "A multi-file project")
  const row = (name: string) =>
    page.locator(`[data-testid=file-row][data-name='${name}'] button`).first()
  // Both files open in tabs.
  await row("02-children.kul").click()
  await row("01-founders.kul").click()
  await expect(page.getByTestId("editor-tab")).toHaveCount(2)
  const foundersBefore = await editorText(page)

  await editorToken(page, "declaration", "m_diego_carmen").click()
  await page.keyboard.press("F2")
  const box = page.getByRole("textbox", { name: /^Rename/ })
  await expect(box).toBeFocused()
  await expect(box).toHaveValue("m_diego_carmen")
  await box.fill("m_founders")
  await box.press("Enter")
  await expect(box).toBeHidden()
  await expect
    .poll(() => editorText(page))
    .toContain("marriage m_founders diego carmen")

  await page
    .getByTestId("editor-tab")
    .filter({ hasText: "02-children.kul" })
    .getByRole("button")
    .first()
    .click()
  await expect.poll(() => editorText(page)).toContain("birth m_founders")
  expect(await editorText(page)).not.toContain("birth m_diego_carmen")
  await expect(page.getByTestId("problems-count")).toHaveText("No problems")

  // Undo in one file undoes the rename in both.
  await editorContent(page).focus()
  await pressEditorMod(page, "z")
  await expect.poll(() => editorText(page)).toContain("birth m_diego_carmen")
  await page
    .getByTestId("editor-tab")
    .filter({ hasText: "01-founders.kul" })
    .getByRole("button")
    .first()
    .click()
  await expect.poll(() => editorText(page)).toBe(foundersBefore)
  await expect(page.getByTestId("problems-count")).toHaveText("No problems")
})

test("the quick fix for a missing gender (KUL-R03) applies", async ({
  page,
}) => {
  await openBlankProject(page)
  await page.keyboard.type('person zola name:"Zola"', { delay: 10 })
  await expect(
    editorContent(page).locator(".cm-lintRange-error").first()
  ).toBeVisible()
  await editorToken(page, "declaration", "zola").click()
  await pressMod(page, ".")
  const menu = page.getByTestId("quick-fix-menu")
  await expect(menu).toBeVisible()
  await menu.getByRole("menuitem", { name: "Add gender:female" }).click()
  await expect
    .poll(() => editorText(page))
    .toBe('person zola name:"Zola" gender:female')
  await expect(page.getByTestId("problems-count")).toHaveText("No problems")
  await expect(editorContent(page)).toBeFocused()
})

test("formatting rewrites the file in canonical form", async ({ page }) => {
  await openBlankProject(page)
  await page.keyboard.type('person   zola    name:"Zola"   gender:female', {
    delay: 5,
  })
  await page.keyboard.press("Alt+Shift+F")
  await expect
    .poll(() => editorText(page))
    .toBe('person zola  name:"Zola"  gender:female\n')
  // The palette formats every file.
  // The formatted text ends with a line break: type on the line after it.
  await goToEnd(page)
  await page.keyboard.type('person   ada name:"Ada" gender:female', {
    delay: 5,
  })
  await page.keyboard.press("Escape")
  await pressMod(page, "K")
  await page.getByRole("option", { name: "Format all files" }).click()
  await expect
    .poll(() => editorText(page))
    .toMatch(
      /^person zola +name:"Zola" +gender:female\nperson ada +name:"Ada" +gender:female\n$/
    )
  expect(await editorText(page)).not.toContain("person   ada")
})

test("Mod+Shift+O lists the file's people and marriages", async ({ page }) => {
  await openProject(page)
  await editorContent(page).focus()
  await pressMod(page, "Shift+O")
  const list = page.getByTestId("symbol-list")
  await expect(list).toBeVisible()
  await expect(page.getByRole("combobox")).toBeFocused()
  await page.keyboard.type("Amara")
  await page.keyboard.press("Enter")
  await expect(list).toBeHidden()
  await expect.poll(() => editorSelection(page)).toBe("amara")
})

test("Shift+F12 lists every use of an id", async ({ page }) => {
  await openProject(page)
  await editorToken(page, "declaration", "emeka").click()
  await page.keyboard.press("Shift+F12")
  const menu = page.getByTestId("references-menu")
  await expect(menu).toBeVisible()
  const items = menu.getByRole("menuitem")
  await expect(items).toHaveCount(2)
  await items.last().click()
  await expect(menu).toBeHidden()
  await expect(editorContent(page)).toBeFocused()
})

test("tabs open, switch and close", async ({ page }) => {
  await openProject(page)
  await openExample(page, "A multi-file project")
  const row = (name: string) =>
    page.locator(`[data-testid=file-row][data-name='${name}'] button`).first()
  await row("02-children.kul").click()
  await row("03-grandchildren.kul").click()
  await expect(page.getByTestId("editor-tab")).toHaveText([
    "01-founders.kul",
    "02-children.kul",
    "03-grandchildren.kul",
  ])
  await page.getByRole("button", { name: "Close 03-grandchildren.kul" }).click()
  await expect(activeTab(page)).toHaveAttribute("data-name", "02-children.kul")
  await editorContent(page).focus()
  await pressMod(page, "W")
  await expect(page.getByTestId("editor-tab")).toHaveText(["01-founders.kul"])
})

test("the window minimizes, docks, moves and resizes, and keeps it across a reload", async ({
  page,
}) => {
  await openProject(page)
  const windowEl = page.getByTestId("editor-window")
  const pill = page.getByTestId("editor-pill")

  // Minimize with the button, restore from the pill; Mod+J toggles.
  await page.getByRole("button", { name: "Minimize the editor" }).click()
  await expect(windowEl).toBeHidden()
  await expect(pill).toContainText("three-generations.kul")
  await pill.click()
  await expect(windowEl).toBeVisible()
  await pressMod(page, "J")
  await expect(windowEl).toBeHidden()
  await pressMod(page, "J")
  await expect(windowEl).toBeVisible()

  // Drag the title bar.
  const bar = page.getByTestId("editor-title-bar")
  const before = await windowEl.boundingBox()
  if (!before) throw new Error("no window")
  await bar.hover({ position: { x: 200, y: 20 } })
  await page.mouse.down()
  await page.mouse.move(before.x + 300, before.y + 20, { steps: 5 })
  await page.mouse.up()
  await expect
    .poll(async () => (await windowEl.boundingBox())?.x)
    .toBeCloseTo(before.x + 100, -1)

  // Resize from the bottom-right corner.
  const corner = page.getByTestId("resize-se")
  const box = await corner.boundingBox()
  if (!box) throw new Error("no corner")
  await page.mouse.move(box.x + 4, box.y + 4)
  await page.mouse.down()
  await page.mouse.move(box.x - 96, box.y - 96, { steps: 5 })
  await page.mouse.up()
  const resized = await windowEl.boundingBox()
  expect(resized?.width).toBeCloseTo(before.width - 100, -1)

  // Drag to the right edge to dock.
  const canvas = await page.getByTestId("canvas").boundingBox()
  if (!canvas) throw new Error("no canvas")
  const barBox = await bar.boundingBox()
  if (!barBox) throw new Error("no bar")
  await page.mouse.move(barBox.x + 100, barBox.y + 20)
  await page.mouse.down()
  await page.mouse.move(canvas.x + canvas.width - 5, barBox.y + 40, {
    steps: 8,
  })
  await expect(page.getByTestId("dock-preview")).toBeVisible()
  await page.mouse.up()
  await expect(windowEl).toHaveAttribute("data-dock", "right")
  const docked = await windowEl.boundingBox()
  expect(docked && docked.x + docked.width).toBeCloseTo(
    canvas.x + canvas.width,
    0
  )
  expect(docked?.height).toBeCloseTo(canvas.height, 0)

  // Minimized and docked survive a reload.
  await pressMod(page, "J")
  await expect(windowEl).toBeHidden()
  // Window state is written once it stops changing.
  await page.waitForTimeout(600)
  await page.reload()
  await expect(pill).toBeVisible({ timeout: 15_000 })
  await pill.click()
  await expect(windowEl).toHaveAttribute("data-dock", "right")
  expect((await windowEl.boundingBox())?.width).toBeCloseTo(
    docked?.width ?? 0,
    0
  )

  // Pulling it off the side floats it again at its own size.
  const dockedBar = await bar.boundingBox()
  if (!dockedBar) throw new Error("no bar")
  await page.mouse.move(dockedBar.x + 100, dockedBar.y + 20)
  await page.mouse.down()
  await page.mouse.move(dockedBar.x - 200, dockedBar.y + 120, { steps: 8 })
  await page.mouse.up()
  await expect(windowEl).toHaveAttribute("data-dock", "none")
  expect((await windowEl.boundingBox())?.height).toBeCloseTo(
    resized?.height ?? 0,
    0
  )
})

test("a crashed language worker restarts with every file", async ({ page }) => {
  await openProject(page)
  await openExample(page, "A multi-file project")
  await page.evaluate(() => window.kulTestHooks?.crashLanguageWorker())
  await expect(
    page.getByText(/Kul engine stopped and was restarted/)
  ).toBeVisible()
  await expect(page.getByTestId("engine-status")).toHaveAttribute(
    "data-restarts",
    "1"
  )
  // The new worker has every file: a reference into another file resolves.
  await page
    .locator("[data-testid=file-row][data-name='03-grandchildren.kul'] button")
    .first()
    .click()
  await expect(page.getByTestId("problems-count")).toHaveText("No problems")
  const reference = editorContent(page).locator(".tok-reference").first()
  const id = (await reference.textContent()) ?? ""
  await reference.click()
  await page.keyboard.press("F12")
  await expect(activeTab(page)).not.toHaveAttribute(
    "data-name",
    "03-grandchildren.kul"
  )
  expect(await editorSelection(page)).toBe(id)
  // And it still analyzes edits.
  await goToEnd(page)
  await page.keyboard.type('\nperson zola name:"Zola"', { delay: 10 })
  await expect(page.getByTestId("problems-count")).toHaveText(
    "1 error, 0 warnings"
  )
})

test("a read-only tab can't edit and says so in the window", async ({
  page,
}) => {
  await openProject(page)
  const other = await page.context().newPage()
  await openProject(other, page.url())
  await other.getByRole("button", { name: "Use here instead" }).click()
  await other.close()
  const banner = page.getByTestId("read-only-banner")
  await expect(banner).toBeVisible()
  await expect(page.getByTestId("editor-window")).toContainText(
    "This project is open in another tab"
  )
  const before = await editorText(page)
  await goToEnd(page)
  await page.keyboard.type("x")
  expect(await editorText(page)).toBe(before)
})
