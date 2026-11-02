import init, { Workspace } from "kul-wasm"
import wasmUrl from "kul-wasm/kul_wasm_bg.wasm?url"
import { beforeAll, describe, expect, it } from "vitest"

import { LEGEND_ROWS, legendRows } from "./legend"
import { EVERY_CATEGORY } from "./every-category"

beforeAll(async () => {
  await init({ module_or_path: wasmUrl })
})

function render(source: string, theme: "agnostic" | "fileExport") {
  const workspace = new Workspace()
  try {
    workspace.setFile("family.kul", source, 1)
    const svg = workspace.renderSvg(theme)
    if (!svg) throw new Error("the project did not render")
    return new DOMParser().parseFromString(svg, "image/svg+xml")
  } finally {
    workspace.free()
  }
}

describe("the canvas legend", () => {
  it("has the downloaded SVG's legend rows, in its order", () => {
    const baked = render(EVERY_CATEGORY, "fileExport")
    const labels = [...baked.querySelectorAll(".kul-legend-label")].map(
      (label) => label.textContent
    )
    expect(LEGEND_ROWS.map((row) => row.label)).toEqual(labels)
  })

  it("finds every category in a tree that draws them all", () => {
    const tree = render(EVERY_CATEGORY, "agnostic")
    const rows = legendRows((selector) => tree.querySelector(selector) !== null)
    expect(rows).toEqual(LEGEND_ROWS)
  })

  it("lists only the categories a tree draws", () => {
    const tree = render(
      `person ann name:"Ann" gender:female
person bob name:"Bob" gender:male
marriage m ann bob
person cy name:"Cy" gender:male
  birth m
`,
      "agnostic"
    )
    const rows = legendRows((selector) => tree.querySelector(selector) !== null)
    expect(rows.map((row) => row.key)).toEqual([
      "gender-male",
      "gender-female",
      "birth",
      "marriage",
    ])
  })
})
