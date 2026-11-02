import {
  mkdirSync,
  mkdtempSync,
  readFileSync,
  readdirSync,
  writeFileSync,
} from "node:fs"
import { tmpdir } from "node:os"
import { join } from "node:path"

import { expect, test } from "./fixtures"
import type { Page } from "./fixtures"
import { unzipSync } from "fflate"

import { chooseFromSwitcher, expectFiles, openProject } from "./support"

const EXAMPLE = join(
  import.meta.dirname,
  "../../../examples/02-three-generations"
)
const INTO_THIS_PROJECT = "Import into this project"
const AS_NEW_PROJECT = "Import as a new project"

function scratchDir(): string {
  return mkdtempSync(join(tmpdir(), "kul-import-"))
}

/** Pick `paths` in the file chooser a switcher item opens. */
async function importThrough(
  page: Page,
  menu: Array<string>,
  paths: string | Array<string>
) {
  const chooser = page.waitForEvent("filechooser")
  await chooseFromSwitcher(page, menu)
  await (await chooser).setFiles(paths)
}

/** Download the open project as a .zip and read the files in it. */
async function downloadZip(page: Page) {
  const download = page.waitForEvent("download")
  await page.getByTestId("export-menu").click()
  await page.getByRole("menuitem", { name: "Download project (.zip)" }).click()
  const file = await download
  // WebKit offers spaces in a download's name as underscores; keep the
  // name the app asked for.
  const name = file.suggestedFilename().replaceAll("_", " ")
  const path = join(scratchDir(), name)
  await file.saveAs(path)
  const files = Object.entries(unzipSync(readFileSync(path))).map(
    ([name, bytes]) => ({ name, text: new TextDecoder().decode(bytes) })
  )
  return { path, name, files }
}

test("importing a folder keeps only its .kul files", async ({ page }) => {
  await openProject(page)
  const dir = join(scratchDir(), "okeke-branch")
  mkdirSync(join(dir, "nested"), { recursive: true })
  writeFileSync(join(dir, "kul.yml"), "kul: 0.1\n")
  writeFileSync(join(dir, "photo.png"), new Uint8Array([0x89, 0x50, 0x4e]))
  writeFileSync(
    join(dir, "cousins.kul"),
    'person ike name:"Ike Okeke" gender:male\n'
  )
  writeFileSync(
    join(dir, "nested", "in-laws.kul"),
    'person bisi name:"Bisi Ade" gender:female\n'
  )

  await importThrough(page, [INTO_THIS_PROJECT, "A folder…"], dir)
  await expect(page.getByText("Imported 2 files, ignored 2")).toBeVisible()
  await expectFiles(page, [
    "cousins.kul",
    "in-laws.kul",
    "three-generations.kul",
  ])
  await expect(
    page.locator('[data-testid=tree] [data-person-id="bisi"]')
  ).toBeVisible()
})

test("a project's .zip imports as a new project with identical files", async ({
  page,
}) => {
  await openProject(page)

  // The archive holds the project's files at its root, byte for byte.
  const exported = await downloadZip(page)
  expect(exported.name).toBe("The Okeke family.zip")
  const onDisk = readdirSync(EXAMPLE)
    .filter((name) => name.endsWith(".kul"))
    .map((name) => ({ name, text: readFileSync(join(EXAMPLE, name), "utf8") }))
  expect(exported.files).toEqual(onDisk)

  // Into the same project, every file clashes.
  await importThrough(
    page,
    [INTO_THIS_PROJECT, "Files or a .zip…"],
    exported.path
  )
  const clash = page.getByRole("dialog", {
    name: "A file has a name already in use",
  })
  await clash.getByRole("button", { name: "Skip it" }).click()
  await expect(
    page.getByText("No .kul files imported, skipped 1")
  ).toBeVisible()

  const firstUrl = page.url()
  await importThrough(page, [AS_NEW_PROJECT, "Files or a .zip…"], exported.path)
  await expect(page).not.toHaveURL(firstUrl)
  // Named after the archive, and numbered: the name is taken.
  await expect(page.getByTestId("project-switcher")).toHaveText(
    "The Okeke family 2"
  )
  await expectFiles(page, ["three-generations.kul"])

  const again = await downloadZip(page)
  expect(again.files).toEqual(exported.files)
})

test("clashing names are replaced or kept side by side", async ({ page }) => {
  await openProject(page)
  const clashing = join(scratchDir(), "Three-Generations.kul")
  writeFileSync(clashing, 'person solo name:"Solo" gender:other\n')
  const dialog = page.getByRole("dialog", {
    name: "A file has a name already in use",
  })

  await importThrough(page, [INTO_THIS_PROJECT, "Files or a .zip…"], clashing)
  await expect(dialog).toContainText("Three-Generations.kul")
  await dialog.getByRole("button", { name: "Replace" }).click()
  await expect(page.getByText("Imported 1 file")).toBeVisible()
  // Replaced under the name already there, with the imported text.
  await expectFiles(page, ["three-generations.kul"])
  await expect(page.locator("[data-testid=tree] .kul-card")).toHaveCount(1)

  await importThrough(page, [INTO_THIS_PROJECT, "Files or a .zip…"], clashing)
  await dialog.getByRole("button", { name: "Keep both" }).click()
  await expectFiles(page, ["Three-Generations-2.kul", "three-generations.kul"])
})
