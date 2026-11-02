// No request leaves the origin. A listener on the browser context records
// every request a full session makes (pages, a docs search, scripts,
// workers, the service worker where the browser reports its fetches,
// downloads, a Try it) and every one must be same-origin.
//
// `data:` and `blob:` URLs also count as staying home: they name bytes the
// page already holds in memory (an inlined image, an export about to be
// saved) and fetching them never touches the network.

import { mkdtempSync, writeFileSync } from "node:fs"
import { tmpdir } from "node:os"
import { join } from "node:path"

import { expect, test } from "./fixtures"
import type { Page } from "./fixtures"
import {
  PROJECT_URL,
  chooseFromSwitcher,
  expectAppReady,
  expectFiles,
  goToEnd,
  openDocsSearch,
  openProject,
  treeCard,
} from "./support"

const IN_MEMORY = new Set(["data:", "blob:"])

async function download(page: Page, open: () => Promise<void>) {
  const saved = page.waitForEvent("download")
  await open()
  const file = await saved
  expect(await file.failure()).toBeNull()
  return file.suggestedFilename()
}

test("a whole session makes only same-origin requests", async ({
  page,
  context,
  baseURL,
}) => {
  // A whole session in one test: the site, a search, the app, an import,
  // two downloads, a reload and a Try it. Give it more than one step's time.
  test.setTimeout(90_000)
  const origin = new URL(baseURL ?? "").origin
  const requests: Array<string> = []
  context.on("request", (request) => requests.push(request.url()))
  page.on("websocket", (socket) => requests.push(socket.url()))

  // The landing page and the docs, with their text variants.
  await page.goto("/")
  await expect(page.getByRole("heading", { level: 1 })).toBeVisible()
  for (const path of ["/docs/", "/docs/reference/cheat-sheet/"]) {
    await page.goto(path)
    await expect(page.getByRole("heading", { level: 1 })).toBeVisible()
  }
  const statuses = await page.evaluate(() =>
    Promise.all(
      ["/docs/index.md", "/llms.txt"].map(
        async (path) => (await fetch(path)).status
      )
    )
  )
  expect(statuses).toEqual([200, 200])

  // A docs search: the index loads and answers in the page.
  const search = await openDocsSearch(page)
  await search.getByRole("combobox", { name: "Search" }).fill("adoption")
  await expect(
    search.getByText("Children", { exact: true }).first()
  ).toBeVisible()
  await page.keyboard.press("Escape")

  // The app: first run, an edit, an import, both exports, a reload.
  await openProject(page)
  await goToEnd(page)
  await page.keyboard.type('\n\nperson zola name:"Zola" gender:female', {
    delay: 5,
  })
  await page.keyboard.press("Escape")
  await expect(treeCard(page, "zola")).toBeVisible()

  const dir = mkdtempSync(join(tmpdir(), "kul-egress-"))
  const imported = join(dir, "notes.kul")
  writeFileSync(imported, 'person tomi name:"Tomi" gender:male\n')
  const chooser = page.waitForEvent("filechooser")
  await chooseFromSwitcher(page, [
    "Import into this project",
    "Files or a .zip…",
  ])
  await (await chooser).setFiles(imported)
  await expectFiles(page, ["notes.kul", "three-generations.kul"])
  await expect(treeCard(page, "tomi")).toBeVisible()
  await expect(page.getByTestId("save-status")).toHaveText("Saved")

  expect(
    await download(page, async () => {
      await page.getByTestId("export-menu").click()
      await page
        .getByRole("menuitem", { name: "Download project (.zip)" })
        .click()
    })
  ).toMatch(/\.zip$/)
  expect(
    await download(page, async () => {
      await page.getByTestId("export-menu").click()
      await page.getByTestId("download-tree").click()
    })
  ).toMatch(/\.svg$/)

  await page.reload()
  await expectAppReady(page)
  await expect(treeCard(page, "tomi")).toBeVisible()

  // Try it: a docs example opens in the app as a new project.
  await page.goto("/docs/guide/children/")
  await page
    .locator("[data-kul-code]")
    .filter({ hasText: "person bayani" })
    .getByTestId("try-it")
    .click()
  await expect(page).toHaveURL(PROJECT_URL)
  await expectAppReady(page)
  await expect(treeCard(page, "bayani")).toBeVisible()

  // The record is real: it holds the pages, their text files, scripts and
  // fonts.
  const paths = requests.map((url) => new URL(url).pathname)
  for (const expected of [
    "/",
    "/docs/",
    "/docs/index.md",
    "/llms.txt",
    "/docs/search.json",
    "/docs/guide/children/",
  ]) {
    expect(paths, expected).toContain(expected)
  }
  expect(paths.some((path) => /^\/assets\/.+\.js$/.test(path))).toBe(true)
  expect(paths.some((path) => /^\/assets\/.+\.woff2$/.test(path))).toBe(true)

  const away = requests.filter((url) => {
    const { protocol, origin: from } = new URL(url)
    return !IN_MEMORY.has(protocol) && from !== origin
  })
  expect(away).toEqual([])
})
