// Updates: a page that a service worker controls finds a new build when it
// checks (here, on becoming visible), says so in a toast, and moves to it
// only when Reload is pressed, after the project's pending writes. Each
// test serves the build under test on an origin of its own, then the next
// build (sites.ts) on the same origin, as a deploy does.

import { expect, test } from "./fixtures"
import type { Page } from "./fixtures"
import {
  BUILD_DIR,
  CURRENT_VERSION,
  NEXT_VERSION,
  deployNextBuild,
  startSite,
} from "./sites"
import type { SiteServer } from "./sites"
import {
  editorText,
  expectAppReady,
  goToEnd,
  openProject,
  treeCard,
  waitForOfflineCopy,
} from "./support"

let site: SiteServer

test.beforeEach(async () => {
  site = await startSite(BUILD_DIR)
})

test.afterEach(async () => {
  await site.close()
})

/** Load `path` under the service worker the first visit installed. */
async function visitControlled(page: Page, path: string) {
  await page.goto(`${site.origin}${path}`)
  await waitForOfflineCopy(page)
  // The first visit installs the worker; from the next load on, it serves.
  await page.reload()
  expect(await page.evaluate(() => !!navigator.serviceWorker.controller)).toBe(
    true
  )
}

const updateToast = (page: Page) =>
  page.getByText("A new version of Kul is ready")

const reloadButton = (page: Page) =>
  page.getByRole("button", { name: "Reload", exact: true })

test("Reload moves the page to the new build, which then works offline", async ({
  page,
  context,
}) => {
  const footer = page.getByRole("contentinfo")
  await visitControlled(page, "/")
  await expect(footer).toContainText(`Kul ${CURRENT_VERSION}`)

  await deployNextBuild(page, site)
  await expect(updateToast(page)).toBeVisible()
  // Nothing moves until the person asks.
  await expect(footer).toContainText(`Kul ${CURRENT_VERSION}`)

  await reloadButton(page).click()
  await expect(footer).toContainText(`Kul ${NEXT_VERSION}`)
  await expect(updateToast(page)).toBeHidden()
  const worker = await page.evaluate(async () => {
    const registration = await navigator.serviceWorker.getRegistration()
    return {
      controlled: !!navigator.serviceWorker.controller,
      waiting: !!registration?.waiting,
    }
  })
  expect(worker).toEqual({ controlled: true, waiting: false })

  // The new worker's precache now answers: offline, the page is the new one.
  await context.setOffline(true)
  await page.reload()
  await expect(footer).toContainText(`Kul ${NEXT_VERSION}`)
})

test("Reload in the app writes the pending edit first", async ({ page }) => {
  await openProject(page, `${site.origin}/app/`)
  await waitForOfflineCopy(page)
  await page.reload()
  await expectAppReady(page)

  await deployNextBuild(page, site)
  await expect(updateToast(page)).toBeVisible()

  await goToEnd(page)
  await page.keyboard.type('\n\nperson zola name:"Zola" gender:female', {
    delay: 5,
  })
  // The edit waits for the save debounce when Reload is pressed.
  await expect(page.getByTestId("save-status")).toHaveAttribute(
    "data-state",
    "saving"
  )
  await reloadButton(page).click()

  await expect(updateToast(page)).toBeHidden()
  await expectAppReady(page)
  await expect(treeCard(page, "zola")).toBeVisible()
  expect(await editorText(page)).toContain('person zola name:"Zola"')
})
