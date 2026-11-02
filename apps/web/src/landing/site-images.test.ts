import { readFileSync } from "node:fs"
import { createRequire } from "node:module"

import { describe, expect, it } from "vitest"

import { pngSize } from "../../scripts/check-landing.mjs"
import { exampleFiles, SAMPLE_EXAMPLE } from "./landing-content"
import {
  faviconImages,
  icoFromPng,
  ogImage,
  ogImageSvg,
  resolveCssVariables,
} from "./site-images"
import { sitemapXml } from "./vite-plugin"
import { woffToSfnt } from "./woff"

const require = createRequire(import.meta.url)
const FAVICON = readFileSync(
  new URL("../../public/favicon.svg", import.meta.url),
  "utf8"
)

describe("woffToSfnt", () => {
  it("unpacks Inter into an OpenType font with every table", () => {
    const woff = readFileSync(
      require.resolve("@fontsource/inter/files/inter-latin-600-normal.woff")
    )
    const font = woffToSfnt(woff)
    const view = new DataView(font.buffer)
    const tables = new DataView(woff.buffer, woff.byteOffset).getUint16(12)
    expect(view.getUint16(4)).toBe(tables)
    const tags = Array.from({ length: tables }, (_, index) =>
      String.fromCharCode(...font.subarray(12 + index * 16, 16 + index * 16))
    )
    expect(tags).toEqual(
      expect.arrayContaining(["cmap", "glyf", "head", "name"])
    )
  })

  it("refuses a file that isn't WOFF", () => {
    expect(() => woffToSfnt(new Uint8Array(64))).toThrow(/not a WOFF/)
  })
})

describe("resolveCssVariables", () => {
  it("replaces custom properties, nested ones too, and drops them", () => {
    const svg =
      "<style>svg { --a: 1.5; --b: var(--a); } .x { stroke-width: var(--b); }</style>"
    const resolved = resolveCssVariables(svg)
    expect(resolved).toContain(".x { stroke-width: 1.5; }")
    expect(resolved).not.toContain("--")
  })
})

describe("the Open Graph image", () => {
  const files = exampleFiles(SAMPLE_EXAMPLE)

  it("draws the sample's tree with resolved colours and the description", () => {
    const svg = ogImageSvg(files, FAVICON)
    expect(svg).toContain('data-person-id="chinua"')
    expect(svg).not.toContain("var(--")
    expect(svg).toContain("Kul draws the tree.")
    // The legend is cropped off: the tree keeps the page render's box.
    expect(svg).toContain('viewBox="0 0 400 432" overflow="hidden"')
  })

  it("is a 1200×630 PNG", async () => {
    expect(pngSize(await ogImage(files, FAVICON))).toEqual({
      width: 1200,
      height: 630,
    })
  })

  it("comes with favicon.ico and a 180 px touch icon", async () => {
    const { ico, touchIcon } = await faviconImages(FAVICON, files)
    expect([...ico.subarray(0, 4)]).toEqual([0, 0, 1, 0])
    expect(pngSize(ico.subarray(22))).toEqual({ width: 32, height: 32 })
    expect(pngSize(touchIcon)).toEqual({ width: 180, height: 180 })
  })
})

describe("icoFromPng", () => {
  it("points its one entry at the PNG after the header", () => {
    const ico = icoFromPng(new Uint8Array([1, 2, 3]), 32)
    const view = new DataView(ico.buffer)
    expect(view.getUint16(4, true)).toBe(1)
    expect(view.getUint32(14, true)).toBe(3)
    expect(view.getUint32(18, true)).toBe(22)
    expect([...ico.subarray(22)]).toEqual([1, 2, 3])
  })
})

describe("sitemapXml", () => {
  it("lists each page on the origin", () => {
    expect(sitemapXml("https://kullang.dev", ["/", "/docs/"])).toBe(
      '<?xml version="1.0" encoding="UTF-8"?>\n<urlset xmlns="http://www.sitemaps.org/schemas/sitemap/0.9">\n  <url><loc>https://kullang.dev/</loc></url>\n  <url><loc>https://kullang.dev/docs/</loc></url>\n</urlset>\n'
    )
  })
})
