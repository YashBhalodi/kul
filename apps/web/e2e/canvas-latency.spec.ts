// How long a keystroke takes to reach the canvas: from the keydown
// (`performance.mark`) to the moment the new tree is swapped into the page,
// debounce included. Measured on the largest example (09, a family across
// a century), where the median is held to 300 ms, and on a generated
// 2,000-person project, which is recorded only. Each sample types a letter
// into a person's name or deletes it again; both change the drawing. The
// app measures its own swap (`kul:canvas-swap`: the new SVG parsed into the
// page and marked), and the spec adds the frame that follows (style,
// layout, paint): together, how long a swap holds the main thread.

import { expect, test } from "./fixtures"
import type { Page, TestInfo } from "@playwright/test"

import {
  chooseFromSwitcher,
  editorToken,
  openExample,
  openProject,
  treeCard,
} from "./support"

const SAMPLES = 7
const CEILING_MS = 300

/** Press `key` and resolve with the ms until the canvas swaps in a new tree. */
async function timeToSwap(page: Page, key: string) {
  const measuring = page.evaluate(() => {
    return new Promise<{ latency: number; swap: number; frame: number }>(
      (resolve) => {
        const tree = document.querySelector("[data-testid=tree]")
        if (!tree) throw new Error("no tree")
        document.addEventListener(
          "keydown",
          () => performance.mark("kul:keystroke"),
          { capture: true, once: true }
        )
        const observer = new MutationObserver(() => {
          observer.disconnect()
          performance.mark("kul:swapped")
          const latency = performance.measure(
            "kul:keystroke-to-canvas",
            "kul:keystroke",
            "kul:swapped"
          ).duration
          const swaps = performance.getEntriesByName("kul:canvas-swap")
          const swap = swaps.at(-1)?.duration ?? Number.NaN
          // The frame that shows the new tree: style, layout and paint too.
          requestAnimationFrame(() =>
            setTimeout(() => {
              const frame = performance.measure(
                "kul:swap-to-frame",
                "kul:swapped"
              ).duration
              resolve({ latency, swap, frame: swap + frame })
            })
          )
        })
        observer.observe(tree, { childList: true })
      }
    )
  })
  await page.evaluate(
    () => new Promise((resolve) => requestAnimationFrame(resolve))
  )
  await page.keyboard.press(key)
  return measuring
}

function median(samples: ReadonlyArray<number>): number {
  const sorted = [...samples].sort((a, b) => a - b)
  return sorted[Math.floor(sorted.length / 2)] ?? Number.NaN
}

/** Time `SAMPLES` letters typed into the name at `name`, and their deletion. */
async function sample(page: Page, name: string) {
  await editorToken(page, "string", name).click()
  await page.keyboard.press("ArrowLeft")
  const typing: Array<number> = []
  const deleting: Array<number> = []
  const swaps: Array<number> = []
  const frames: Array<number> = []
  for (let round = 0; round < SAMPLES; round++) {
    const typed = await timeToSwap(page, "x")
    const deleted = await timeToSwap(page, "Backspace")
    typing.push(typed.latency)
    deleting.push(deleted.latency)
    swaps.push(typed.swap, deleted.swap)
    frames.push(typed.frame, deleted.frame)
  }
  return { typing, deleting, swaps, frames }
}

function report(
  testInfo: TestInfo,
  project: string,
  { typing, deleting, swaps, frames }: Awaited<ReturnType<typeof sample>>
) {
  const line = (label: string, samples: ReadonlyArray<number>) => {
    const value = median(samples)
    console.log(
      `[canvas] ${testInfo.project.name}: ${label} on ${project}, median ${value.toFixed(0)} ms (${[
        ...samples,
      ]
        .sort((a, b) => a - b)
        .map((ms) => ms.toFixed(0))
        .join(", ")})`
    )
    testInfo.annotations.push({
      type: `${project}: ${label}`,
      description: value.toFixed(0),
    })
    return value
  }
  return {
    typing: line("keystroke to canvas (typing)", typing),
    deleting: line("keystroke to canvas (deleting)", deleting),
    swap: line("DOM swap", swaps),
    frame: line("DOM swap to the next frame", frames),
  }
}

test("keystroke to canvas on example 09", async ({ page }, testInfo) => {
  await openProject(page)
  await openExample(page, "A family across a century", "The Adeyemi family")
  await expect(treeCard(page, "oludare")).toHaveCount(1)
  const medians = report(
    testInfo,
    "example 09",
    await sample(page, '"Oludare Adeyemi"')
  )
  expect(medians.typing).toBeLessThanOrEqual(CEILING_MS)
  expect(medians.deleting).toBeLessThanOrEqual(CEILING_MS)
})

/**
 * A project of `count` people over several generations: founding couples,
 * then children who each marry someone from outside the family and have
 * children of their own.
 */
function generatedProject(count: number): string {
  const lines: Array<string> = []
  let people = 0
  const person = (gender: string, birth?: string) => {
    people++
    const id = `p${people}`
    lines.push(`person ${id} name:"Person ${people}" gender:${gender}`)
    if (birth) lines.push(`  birth ${birth}`)
    return id
  }
  const marry = (a: string, b: string) => {
    const id = `m_${a}_${b}`
    lines.push(`marriage ${id} ${a} ${b}`)
    return id
  }
  const families: Array<string> = []
  for (let couple = 0; couple < 8; couple++) {
    families.push(marry(person("male"), person("female")))
  }
  while (people < count) {
    const family = families.shift()
    if (!family) break
    for (let child = 0; child < 2 && people < count; child++) {
      const born = person(child === 0 ? "male" : "female", family)
      if (people >= count) break
      const spouse = person(child === 0 ? "female" : "male")
      families.push(marry(born, spouse))
    }
  }
  return `${lines.join("\n")}\n`
}

test("keystroke to canvas on a generated 2,000-person project", async ({
  page,
}, testInfo) => {
  test.setTimeout(180_000)
  await openProject(page)
  const chooser = page.waitForEvent("filechooser")
  await chooseFromSwitcher(page, [
    "Import as a new project",
    "Files or a .zip…",
  ])
  await (
    await chooser
  ).setFiles({
    name: "generated.kul",
    mimeType: "text/plain",
    buffer: Buffer.from(generatedProject(2000)),
  })
  await expect(treeCard(page, "p2000")).toHaveCount(1, { timeout: 60_000 })
  await expect(
    page.locator("[data-testid=tree] .kul-card[data-kind=canonical]")
  ).toHaveCount(2000)
  const medians = report(
    testInfo,
    "2,000 people",
    await sample(page, '"Person 1"')
  )
  // Recorded, not gated: the swap's main-thread time is a follow-up when
  // it goes past 100 ms.
  expect(medians.typing).toBeGreaterThan(0)
})
