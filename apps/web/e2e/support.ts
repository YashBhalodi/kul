import { expect } from "@playwright/test"
import type { Page } from "@playwright/test"

/** The app's top bar is up and the language engine has loaded. */
export async function expectAppReady(page: Page) {
  await expect(page.getByTestId("palette-trigger")).toBeVisible()
  await expect(page.getByTestId("engine-status")).toHaveAttribute(
    "data-state",
    "ready",
    { timeout: 15_000 }
  )
}

/** A project's address: /app/<uuid>/. */
export const PROJECT_URL = /\/app\/[0-9a-f-]{36}\/$/

/**
 * Open the app and wait for a project to be open with its tree drawn. On
 * a fresh browser context that is the first-run example.
 */
export async function openProject(page: Page, path = "/app/") {
  await page.goto(path)
  await expect(page).toHaveURL(PROJECT_URL)
  await expectAppReady(page)
  await expect(
    page.getByTestId("files-panel").or(page.getByTestId("editor-pill"))
  ).toBeVisible()
  await expect(
    page.locator("[data-testid=tree] .kul-card").first()
  ).toBeVisible()
}

/** The file list's rows, in the order shown. */
export function fileRows(page: Page) {
  return page.getByTestId("file-row")
}

/** The file list shows exactly `names`, in this order. */
export async function expectFiles(page: Page, names: Array<string>) {
  await expect(fileRows(page)).toHaveText(names)
}

/** Open the project switcher's menu. */
export async function openSwitcher(page: Page) {
  await page.getByTestId("project-switcher").click()
  await expect(page.getByRole("menu").first()).toBeVisible()
}

/** Pick a project-switcher item, through submenus: ["Import", "A folder…"]. */
export async function chooseFromSwitcher(page: Page, path: Array<string>) {
  await openSwitcher(page)
  for (const [index, name] of path.entries()) {
    const item = page.getByRole("menuitem", { name, exact: true }).last()
    if (index < path.length - 1) await item.hover()
    await item.click()
  }
}

/** Add a file through the file list's inline input. */
export async function createFile(page: Page, stem: string) {
  await page.getByRole("button", { name: "New file" }).click()
  const input = page.getByRole("textbox", { name: "New file name" })
  await input.fill(stem)
  await input.press("Enter")
  await expect(input).toBeHidden()
}

/**
 * Record CSP violations: `securitypolicyviolation` events in the page (every
 * engine fires them) plus console reports. Call before the first `goto`.
 */
export async function recordCspViolations(
  page: Page
): Promise<() => Promise<Array<string>>> {
  const consoleReports: Array<string> = []
  page.on("console", (message) => {
    const text = message.text()
    if (/content.security.policy|refused to/i.test(text))
      consoleReports.push(text)
  })
  await page.addInitScript(() => {
    const seen: Array<string> = []
    Object.defineProperty(window, "__cspViolations", { value: seen })
    document.addEventListener("securitypolicyviolation", (event) => {
      seen.push(`${event.violatedDirective} blocked ${event.blockedURI}`)
    })
  })
  return async () => {
    const inPage = await page.evaluate(
      () =>
        (window as unknown as { __cspViolations?: Array<string> })
          .__cspViolations ?? []
    )
    return [...inPage, ...consoleReports]
  }
}

/**
 * Press `Mod+<key>` the way the app reads it. The app (TanStack Hotkeys)
 * resolves Mod from the user agent, and Playwright's Desktop Safari profile
 * reports a Mac agent on every host, so ControlOrMeta (which follows the
 * host) is not enough.
 */
export async function pressMod(page: Page, key: string) {
  const mac = await page.evaluate(() =>
    /mac/i.test(`${navigator.platform} ${navigator.userAgent}`)
  )
  await page.keyboard.press(`${mac ? "Meta" : "Control"}+${key}`)
}

/**
 * Record the theme on <html> each time a script toggles its `dark` class
 * (theme-init.js before the first paint, the app afterwards), and whether
 * <body> had been parsed yet. Call before the first `goto`; the returned
 * function reads the record for the current page.
 */
export async function recordThemeTimeline(page: Page) {
  await page.addInitScript(() => {
    const timeline: Array<{ dark: boolean; bodyParsed: boolean }> = []
    Object.defineProperty(window, "__themeTimeline", { value: timeline })
    // Wrap the write itself: MutationObserver callbacks arrive too late to
    // tell whether <body> existed at the moment of the change.
    const toggle = Object.getOwnPropertyDescriptor(
      DOMTokenList.prototype,
      "toggle"
    )?.value as (this: DOMTokenList, token: string, force?: boolean) => boolean
    DOMTokenList.prototype.toggle = function (token, force) {
      const result = toggle.call(this, token, force)
      if (token === "dark" && this === document.documentElement.classList) {
        timeline.push({
          dark: this.contains("dark"),
          bodyParsed: !!document.body,
        })
      }
      return result
    }
  })
  return () =>
    page.evaluate(
      () =>
        (
          window as unknown as {
            __themeTimeline: Array<{ dark: boolean; bodyParsed: boolean }>
          }
        ).__themeTimeline
    )
}

/**
 * Press `Mod+<key>` the way the editor (CodeMirror) reads it: Meta on a Mac
 * platform, Control elsewhere. CodeMirror reads `navigator.platform`, which
 * Playwright's Desktop Safari profile leaves as the host's.
 */
export async function pressEditorMod(page: Page, key: string) {
  const mac = await page.evaluate(() => /Mac/.test(navigator.platform))
  await page.keyboard.press(`${mac ? "Meta" : "Control"}+${key}`)
}

/** The editor's text area. */
export function editorContent(page: Page) {
  return page.getByTestId("editor").locator(".cm-content")
}

/** The whole text of the file the editor shows. */
export async function editorText(page: Page): Promise<string> {
  return page.evaluate(() => window.kulTestHooks?.editorText() ?? "")
}

/** The text selected in the editor. */
export async function editorSelection(page: Page): Promise<string> {
  return page.evaluate(() => window.kulTestHooks?.editorSelection() ?? "")
}

/** Put the cursor at the end of the editor's text. */
export async function goToEnd(page: Page) {
  // Focus rather than click: the content is wider than the editor, so a
  // click scrolls it sideways or lands under the line numbers.
  await editorContent(page).focus()
  await pressEditorMod(page, "End")
}

/** A highlighted token in the editor, by its kind's class and exact text. */
export function editorToken(page: Page, kind: string, text: string) {
  return editorContent(page)
    .locator(`.tok-${kind}`, { hasText: new RegExp(`^${text}$`) })
    .first()
}

/** Open a new project from an example, by its title. */
export async function openExample(page: Page, title: string, name = title) {
  await chooseFromSwitcher(page, ["New from example", title])
  await expect(page.getByTestId("project-switcher")).toHaveText(name)
  await expect(editorContent(page)).toBeVisible()
}

/** A person's own card on the canvas. */
export function treeCard(page: Page, id: string) {
  return page.locator(
    `[data-testid=tree] .kul-card[data-person-id="${id}"][data-kind="canonical"]`
  )
}

/** A marriage's line on the canvas. */
export function treeMarriage(page: Page, id: string) {
  return page.locator(
    `[data-testid=tree] .kul-edge[data-link-kind="marriage"][data-marriage-id="${id}"]`
  )
}

/** The canvas camera, read from the tree layer's transform. */
export async function cameraOf(
  page: Page
): Promise<{ x: number; y: number; zoom: number }> {
  return page.getByTestId("tree-layer").evaluate((layer) => {
    const matrix = new DOMMatrix(getComputedStyle(layer).transform)
    return { x: matrix.e, y: matrix.f, zoom: matrix.a }
  })
}

/** Open a file of the open project in the editor, from the file list. */
export async function openFile(page: Page, name: string) {
  await page
    .locator(`[data-testid=file-row][data-name='${name}'] button`)
    .first()
    .click()
}

/**
 * Wait until the service worker has precached the site and is active, so
 * the next load works offline.
 */
export async function waitForOfflineCopy(page: Page) {
  await page.evaluate(async () => {
    const registration = await navigator.serviceWorker.ready
    const worker = registration.active
    if (!worker || worker.state === "activated") return
    await new Promise<void>((resolve) => {
      worker.addEventListener("statechange", () => {
        if (worker.state === "activated") resolve()
      })
    })
  })
}

/**
 * Open the docs search dialog. The trigger works once the page has
 * hydrated, so a click before that is retried.
 */
export async function openDocsSearch(page: Page) {
  const dialog = page.getByRole("dialog")
  await expect(async () => {
    await page.getByTestId("docs-search").click()
    await expect(dialog).toBeVisible({ timeout: 1000 })
  }).toPass()
  return dialog
}
