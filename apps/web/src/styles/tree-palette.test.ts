// The canvas paints the tree from the token sheet; the downloaded SVG
// bakes its own light palette (kul-svg). Their categories must stay in
// step: a category the engine starts drawing needs a hue in both themes.

import { readFileSync } from "node:fs"

import { describe, expect, it } from "vitest"

import { EVERY_CATEGORY } from "@/canvas/every-category"
import { workspaceWith } from "@/canvas/node-engine"

const tokens = readFileSync(new URL("./tokens.css", import.meta.url), "utf8")

/** The declarations inside the first `selector { … }` block of `css`. */
function block(css: string, selector: string): string {
  const start = css.indexOf(`${selector} {`)
  if (start === -1) throw new Error(`no ${selector} block`)
  const open = css.indexOf("{", start)
  return css.slice(open + 1, css.indexOf("}", open))
}

/** The names after `prefix` of the custom properties declared in `css`. */
function names(css: string, prefix: string): Array<string> {
  const pattern = new RegExp(`${prefix}([a-z-]+)\\s*:`, "g")
  return [...css.matchAll(pattern)].map((match) => match[1] ?? "").sort()
}

function bakedPalette(): string {
  const workspace = workspaceWith({ "family.kul": EVERY_CATEGORY })
  try {
    const svg = workspace.renderSvg("fileExport")
    if (!svg) throw new Error("the project did not render")
    return svg.slice(svg.indexOf("<style>"), svg.indexOf("</style>"))
  } finally {
    workspace.free()
  }
}

describe("the tree's token sheet", () => {
  const baked = names(bakedPalette(), "--kul-hue-")

  it("reads the downloaded SVG's palette categories", () => {
    expect(baked).toEqual([
      "adoption",
      "birth",
      "female",
      "male",
      "marriage",
      "other",
      "past",
    ])
  })

  it("has a hue for every one of them in the light theme", () => {
    expect(names(block(tokens, ":root"), "--tree-hue-")).toEqual(baked)
  })

  it("has a hue for every one of them in the dark theme", () => {
    expect(names(block(tokens, ".dark"), "--tree-hue-")).toEqual(baked)
  })

  it("defines every tree token in both themes", () => {
    expect(names(block(tokens, ".dark"), "--tree-")).toEqual(
      names(block(tokens, ":root"), "--tree-")
    )
  })
})
