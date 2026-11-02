// Offline use: one visit to the app installs the service worker, which
// precaches the whole site. With the network gone, the app boots, edits are
// analysed, drawn and saved, and the landing page, the docs and their text
// files still load.

import { expect, test } from "./fixtures"
import { CURRENT_VERSION } from "./sites"
import {
  editorText,
  expectAppReady,
  goToEnd,
  openProject,
  treeCard,
  waitForOfflineCopy,
} from "./support"

test("after one visit to the app, the app and the site work offline", async ({
  page,
  context,
}) => {
  await openProject(page)
  await waitForOfflineCopy(page)
  await context.setOffline(true)

  await page.reload()
  await expectAppReady(page)
  await expect(treeCard(page, "adaeze")).toBeVisible()

  // An edit is analysed, drawn and saved.
  await goToEnd(page)
  await page.keyboard.type('\n\nperson zola name:"Zola" gender:female', {
    delay: 5,
  })
  await page.keyboard.press("Escape")
  await expect(treeCard(page, "zola")).toBeVisible()
  await expect(page.getByTestId("save-status")).toHaveText("Saved")

  const project = page.url()
  await page.reload()
  await expect(page).toHaveURL(project)
  await expectAppReady(page)
  await expect(treeCard(page, "zola")).toBeVisible()
  expect(await editorText(page)).toContain('person zola name:"Zola"')

  for (const path of ["/", "/docs/", "/docs/reference/cheat-sheet/"]) {
    await page.goto(path)
    await expect(page.getByRole("heading", { level: 1 })).toBeVisible()
    await expect(page.getByRole("contentinfo")).toContainText(
      `Kul ${CURRENT_VERSION}`
    )
  }

  const answers = await page.evaluate(() =>
    Promise.all(
      [
        "/llms.txt",
        "/llms-full.txt",
        "/docs/index.md",
        "/docs/search.json",
      ].map(async (path) => {
        const response = await fetch(path)
        const size = (await response.text()).length
        return { path, status: response.status, size }
      })
    )
  )
  for (const answer of answers) {
    expect(answer.status, answer.path).toBe(200)
    expect(answer.size, answer.path).toBeGreaterThan(0)
  }
})
