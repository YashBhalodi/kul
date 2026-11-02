// How long a keystroke takes to become a squiggle, on the largest example
// (09, a family across a century). Each sample marks the keydown with
// `performance.mark` and measures to the moment the page shows the error's
// underline, or stops showing it. Typing a letter asks for completions,
// which run the waiting analysis at once; deleting one waits out the
// analysis debounce. Both medians are printed for the record and held to
// the issue's "about 500 ms".

import { expect, test } from "./fixtures"
import type { Page } from "./fixtures"

import { editorContent, goToEnd, openExample, openProject } from "./support"

const SAMPLES = 7

/** Press `key` and resolve with the ms until the page shows an error squiggle (or none). */
async function timeToSquiggle(page: Page, key: string, present: boolean) {
  const measuring = page.evaluate((present) => {
    return new Promise<number>((resolve) => {
      const content = document.querySelector(".cm-content")
      if (!content) throw new Error("no editor")
      document.addEventListener(
        "keydown",
        () => performance.mark("kul:keystroke"),
        { capture: true, once: true }
      )
      const settled = () =>
        !!content.querySelector(".cm-lintRange-error") === present
      const observer = new MutationObserver(() => {
        if (!settled()) return
        observer.disconnect()
        performance.mark("kul:squiggle")
        resolve(
          performance.measure(
            "kul:keystroke-to-squiggle",
            "kul:keystroke",
            "kul:squiggle"
          ).duration
        )
      })
      observer.observe(content, {
        subtree: true,
        childList: true,
        attributes: true,
        characterData: true,
      })
    })
  }, present)
  // The observer is in place before the key is pressed.
  await page.evaluate(
    () => new Promise((resolve) => requestAnimationFrame(resolve))
  )
  await page.keyboard.press(key)
  return measuring
}

test("keystroke to squiggle on example 09", async ({ page }, testInfo) => {
  await openProject(page)
  await openExample(page, "A family across a century", "The Adeyemi family")
  await goToEnd(page)
  await page.keyboard.type('person zz name:"Z" gender:male', { delay: 5 })
  await page.keyboard.press("Escape")
  await expect(page.getByTestId("problems-count")).toHaveText("No problems")

  const shown: Array<number> = []
  const cleared: Array<number> = []
  for (let sample = 0; sample < SAMPLES; sample++) {
    // "malex" is not a gender: an error, then its fix.
    shown.push(await timeToSquiggle(page, "x", true))
    await page.keyboard.press("Escape")
    cleared.push(await timeToSquiggle(page, "Backspace", false))
  }
  await expect(editorContent(page).locator(".cm-lintRange-error")).toHaveCount(
    0
  )

  const report = (label: string, samples: Array<number>) => {
    const sorted = [...samples].sort((a, b) => a - b)
    const median = sorted[Math.floor(sorted.length / 2)] ?? Number.NaN
    console.log(
      `[latency] ${testInfo.project.name}: ${label} on example 09, median ${median.toFixed(0)} ms of ${SAMPLES} (${sorted.map((ms) => ms.toFixed(0)).join(", ")})`
    )
    testInfo.annotations.push({ type: label, description: median.toFixed(0) })
    return median
  }
  expect(report("keystroke to squiggle", shown)).toBeLessThan(500)
  expect(report("keystroke to squiggle cleared", cleared)).toBeLessThan(500)
})
