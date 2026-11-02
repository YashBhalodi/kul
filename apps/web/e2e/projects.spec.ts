import { expect, test } from "./fixtures"

import {
  PROJECT_URL,
  chooseFromSwitcher,
  createFile,
  expectFiles,
  fileRows,
  openProject,
  openSwitcher,
} from "./support"

// Every test starts in a fresh browser context: an empty origin private
// file system, so a first visit.

test("a first visit seeds the example project and draws its tree", async ({
  page,
}) => {
  await openProject(page)
  await expect(page.getByTestId("project-switcher")).toHaveText(
    "The Okeke family"
  )
  await expectFiles(page, ["three-generations.kul"])
  await expect(fileRows(page).first()).toHaveAttribute("data-active", "true")
  const tree = page.getByTestId("tree")
  for (const id of ["chinua", "adaeze", "emeka", "amara"]) {
    await expect(tree.locator(`[data-person-id="${id}"]`)).toBeVisible()
  }
  // The fitted tree sits inside the canvas.
  const canvas = await page.getByTestId("canvas").boundingBox()
  const drawing = await tree.locator("svg").boundingBox()
  expect(canvas && drawing).toBeTruthy()
  if (canvas && drawing) {
    expect(drawing.x).toBeGreaterThanOrEqual(canvas.x)
    expect(drawing.y).toBeGreaterThanOrEqual(canvas.y)
    expect(drawing.x + drawing.width).toBeLessThanOrEqual(
      canvas.x + canvas.width
    )
    expect(drawing.y + drawing.height).toBeLessThanOrEqual(
      canvas.y + canvas.height
    )
  }
})

test("projects are created, renamed, duplicated and deleted", async ({
  page,
}) => {
  await openProject(page)
  const firstUrl = page.url()
  const switcher = page.getByTestId("project-switcher")

  await chooseFromSwitcher(page, ["New blank project"])
  await expect(page).not.toHaveURL(firstUrl)
  await expect(page).toHaveURL(PROJECT_URL)
  await expect(switcher).toHaveText("Untitled project")
  await expectFiles(page, ["family.kul"])

  await chooseFromSwitcher(page, ["Rename project…"])
  const dialog = page.getByRole("dialog", { name: "Rename project" })
  const name = dialog.getByRole("textbox", { name: "Project name" })
  await name.fill("   ")
  await expect(dialog.getByText("Enter a name.")).toBeVisible()
  await name.fill("The Mehta family")
  await name.press("Enter")
  await expect(dialog).toBeHidden()
  await expect(switcher).toHaveText("The Mehta family")

  await chooseFromSwitcher(page, ["Duplicate project"])
  await expect(switcher).toHaveText("The Mehta family (copy)")
  await expectFiles(page, ["family.kul"])

  await chooseFromSwitcher(page, ["New from example", "A multi-file project"])
  await expect(switcher).toHaveText("A multi-file project")
  await expectFiles(page, [
    "01-founders.kul",
    "02-children.kul",
    "03-grandchildren.kul",
  ])

  await openSwitcher(page)
  await expect(page.getByTestId("project-option")).toHaveCount(4)
  await page.getByTestId("project-option").filter({ hasText: "(copy)" }).click()
  await expect(switcher).toHaveText("The Mehta family (copy)")

  await chooseFromSwitcher(page, ["Delete project…"])
  const confirm = page.getByRole("alertdialog")
  await expect(confirm).toContainText("Delete “The Mehta family (copy)”?")
  await confirm.getByRole("button", { name: "Delete project" }).click()
  await expect(switcher).not.toHaveText("The Mehta family (copy)")
  await expect(page).toHaveURL(PROJECT_URL)

  await openSwitcher(page)
  const options = page.getByTestId("project-option")
  await expect(options).toHaveCount(3)
  await expect(options.filter({ hasText: "(copy)" })).toHaveCount(0)
})

test("files are created, renamed and deleted", async ({ page }) => {
  await openProject(page)

  await createFile(page, "cousins")
  await expectFiles(page, ["cousins.kul", "three-generations.kul"])
  await expect(fileRows(page).first()).toHaveAttribute("data-active", "true")

  // Names are checked as you type, ignoring case.
  await page.getByRole("button", { name: "New file" }).click()
  const input = page.getByRole("textbox", { name: "New file name" })
  await input.fill("Three-Generations")
  await expect(page.getByRole("alert")).toHaveText(
    "Another file already has this name."
  )
  await input.fill("a/b")
  await expect(page.getByRole("alert")).toHaveText(
    "Names can’t contain / or \\."
  )
  await input.press("Escape")
  await expect(input).toBeHidden()

  // Rename by double-clicking, and with F2 on the selected file.
  await fileRows(page).filter({ hasText: "cousins.kul" }).dblclick()
  const rename = page.getByRole("textbox", { name: "New name for cousins.kul" })
  await rename.fill("in-laws")
  await rename.press("Enter")
  await expectFiles(page, ["in-laws.kul", "three-generations.kul"])

  await fileRows(page)
    .filter({ hasText: "in-laws.kul" })
    .getByRole("button")
    .first()
    .click()
  await page.keyboard.press("F2")
  const again = page.getByRole("textbox", { name: "New name for in-laws.kul" })
  await again.fill("In-Laws")
  await again.press("Enter")
  await expectFiles(page, ["In-Laws.kul", "three-generations.kul"])

  const row = fileRows(page).filter({ hasText: "In-Laws.kul" })
  await row.hover()
  await row.getByRole("button", { name: "Actions for In-Laws.kul" }).click()
  await page.getByRole("menuitem", { name: "Delete…" }).click()
  const confirm = page.getByRole("alertdialog")
  await expect(confirm).toContainText("Delete In-Laws.kul?")
  await confirm.getByRole("button", { name: "Delete file" }).click()
  await expectFiles(page, ["three-generations.kul"])
})

test("a reload keeps projects, files and the open file", async ({ page }) => {
  await openProject(page)
  await createFile(page, "notes")
  await chooseFromSwitcher(page, ["Rename project…"])
  const name = page.getByRole("textbox", { name: "Project name" })
  await name.fill("Okeke archive")
  await name.press("Enter")
  await expect(page.getByTestId("save-status")).toHaveText("Saved")
  const url = page.url()

  await page.reload()
  await expect(page).toHaveURL(url)
  await expect(page.getByTestId("project-switcher")).toHaveText("Okeke archive")
  await expectFiles(page, ["notes.kul", "three-generations.kul"])
  await expect(fileRows(page).first()).toHaveAttribute("data-active", "true")

  // /app returns to the project opened last.
  await chooseFromSwitcher(page, ["New blank project"])
  await expect(page.getByTestId("project-switcher")).toHaveText(
    "Untitled project"
  )
  await openSwitcher(page)
  await page
    .getByTestId("project-option")
    .filter({ hasText: "Okeke archive" })
    .click()
  await expect(page).toHaveURL(url)
  await expectFiles(page, ["notes.kul", "three-generations.kul"])
  await page.goto("/app/")
  await expect(page).toHaveURL(url)
  await expectFiles(page, ["notes.kul", "three-generations.kul"])
})
