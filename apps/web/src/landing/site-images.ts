// The site's raster images, drawn when the site is built: the Open Graph
// image (the landing page's sample tree, as link previews show it) and the
// favicons for places that don't take the SVG one. resvg (WASM) rasterises
// them; Inter, unpacked from its WOFF files, sets their text.
//
// Colours come from the engine's downloadable SVG, whose palette is baked
// in, and from the favicon: this file names none of its own.

import { readFileSync } from "node:fs"
import { createRequire } from "node:module"

import { Resvg, initWasm } from "@resvg/resvg-wasm"

import { workspaceWith } from "../highlight/node-engine.ts"
import type { KulFile } from "../highlight/highlight-kul.ts"
import { woffToSfnt } from "./woff.ts"

const require = createRequire(import.meta.url)

export const OG_WIDTH = 1200
export const OG_HEIGHT = 630

let ready: Promise<void> | undefined

/** Load resvg's WASM once per process. */
function loadResvg(): Promise<void> {
  ready ??= initWasm(
    readFileSync(require.resolve("@resvg/resvg-wasm/index_bg.wasm"))
  )
  return ready
}

let fonts: Array<Uint8Array> | undefined

/** Inter at the weights the images use, as OpenType. */
function interFonts(): Array<Uint8Array> {
  fonts ??= [500, 600, 700].map((weight) =>
    woffToSfnt(
      readFileSync(
        require.resolve(
          `@fontsource/inter/files/inter-latin-${String(weight)}-normal.woff`
        )
      )
    )
  )
  return fonts
}

/** An SVG as a PNG, `width` pixels wide. */
export async function rasterise(
  svg: string,
  width: number
): Promise<Uint8Array> {
  await loadResvg()
  const image = new Resvg(svg, {
    font: {
      fontBuffers: interFonts(),
      defaultFontFamily: "Inter",
      sansSerifFamily: "Inter",
    },
    fitTo: { mode: "width", value: width },
  })
  try {
    return image.render().asPng()
  } finally {
    image.free()
  }
}

/**
 * The SVG with every `var(--x)` replaced by the custom property's value,
 * and the declarations dropped: resvg reads CSS but not custom properties.
 */
export function resolveCssVariables(svg: string): string {
  const values = new Map<string, string>()
  for (const [, name, value] of svg.matchAll(/(--[\w-]+)\s*:\s*([^;]+);/g)) {
    if (name && value) values.set(name, value.trim())
  }
  let text = svg.replace(/--[\w-]+\s*:[^;]+;\s*/g, "")
  for (let pass = 0; pass < 10 && text.includes("var(--"); pass += 1) {
    text = text.replace(
      /var\((--[\w-]+)\)/g,
      (_, name: string) => values.get(name) ?? ""
    )
  }
  if (text.includes("var(--")) {
    throw new Error("the tree's SVG has a custom property with no value")
  }
  return text
}

/** The value a custom property has in the engine's baked palette. */
function paletteColour(svg: string, name: string): string {
  const value = new RegExp(`${name}\\s*:\\s*(#[0-9a-fA-F]{3,8})\\s*;`).exec(
    svg
  )?.[1]
  if (!value) throw new Error(`the tree's palette has no ${name}`)
  return value
}

/** The favicon's light-theme colour, the brand teal. */
function markColour(favicon: string): string {
  const value = /color:\s*(#[0-9a-fA-F]{3,8})/.exec(favicon)?.[1]
  if (!value) throw new Error("the favicon names no colour")
  return value
}

/**
 * A project's tree as the download draws it (palette baked in, with its
 * legend) and as the page draws it (theme-agnostic, without one).
 */
function renders(files: ReadonlyArray<KulFile>): {
  tree: string
  drawn: string
} {
  const workspace = workspaceWith(
    Object.fromEntries(files.map((file) => [file.name, file.text]))
  )
  try {
    const tree = workspace.renderSvg("fileExport")
    const drawn = workspace.analyze().svg
    if (!tree || !drawn) throw new Error("the project's tree does not draw")
    return { tree, drawn }
  } finally {
    workspace.free()
  }
}

/** An SVG document's root element, re-placed as a nested <svg>. */
function nested(
  svg: string,
  box: { x: number; y: number; width: number; height: number },
  viewBox: string
): string {
  const body = svg.replace(/^\s*<svg\b[^>]*>/, "").replace(/<\/svg>\s*$/, "")
  return `<svg x="${String(box.x)}" y="${String(box.y)}" width="${String(box.width)}" height="${String(box.height)}" viewBox="${viewBox}" overflow="hidden">${body}</svg>`
}

function viewBoxOf(svg: string): [number, number, number, number] {
  const box = /viewBox="([^"]+)"/.exec(svg)?.[1]?.split(/\s+/).map(Number)
  if (box?.length !== 4 || box.some(Number.isNaN)) {
    throw new Error("an SVG without a viewBox")
  }
  return [box[0] ?? 0, box[1] ?? 0, box[2] ?? 0, box[3] ?? 0]
}

/**
 * The Open Graph image's SVG: the mark, the page's one-line description
 * and the address on the left; the project's tree, as the downloaded SVG
 * draws it (without its legend), on a card on the right.
 */
export function ogImageSvg(
  files: ReadonlyArray<KulFile>,
  favicon: string
): string {
  const { tree, drawn } = renders(files)

  const surface = paletteColour(tree, "--kul-surface")
  const raised = paletteColour(tree, "--kul-raised")
  const ink = paletteColour(tree, "--kul-on-surface")
  const muted = paletteColour(tree, "--kul-muted")
  const border = paletteColour(tree, "--kul-border")
  const accent = markColour(favicon)

  // The tree without the legend: the agnostic render's box.
  const [x, y, width, height] = viewBoxOf(drawn)
  const card = { x: 668, y: 56, width: 476, height: 518 }
  const pad = 36
  const scale = Math.min(
    (card.width - 2 * pad) / width,
    (card.height - 2 * pad) / height
  )
  const treeBox = {
    width: Math.round(width * scale),
    height: Math.round(height * scale),
    x: 0,
    y: 0,
  }
  treeBox.x = Math.round(card.x + (card.width - treeBox.width) / 2)
  treeBox.y = Math.round(card.y + (card.height - treeBox.height) / 2)

  const text = (
    content: string,
    at: { x: number; y: number },
    size: number,
    weight: number,
    fill: string
  ) =>
    `<text x="${String(at.x)}" y="${String(at.y)}" font-family="Inter" font-size="${String(size)}" font-weight="${String(weight)}" fill="${fill}">${content}</text>`

  return [
    `<svg xmlns="http://www.w3.org/2000/svg" width="${String(OG_WIDTH)}" height="${String(OG_HEIGHT)}" viewBox="0 0 ${String(OG_WIDTH)} ${String(OG_HEIGHT)}">`,
    `<rect width="${String(OG_WIDTH)}" height="${String(OG_HEIGHT)}" fill="${raised}"/>`,
    nested(favicon, { x: 80, y: 84, width: 60, height: 60 }, "0 0 32 32"),
    text("Kul", { x: 160, y: 132 }, 52, 700, ink),
    text("Describe a family", { x: 80, y: 290 }, 54, 700, ink),
    text("in plain text.", { x: 80, y: 356 }, 54, 700, ink),
    text("Kul draws the tree.", { x: 80, y: 422 }, 54, 700, accent),
    text("kullang.dev", { x: 80, y: 540 }, 28, 600, muted),
    `<rect x="${String(card.x)}" y="${String(card.y)}" width="${String(card.width)}" height="${String(card.height)}" rx="24" fill="${surface}" stroke="${border}" stroke-width="2"/>`,
    nested(
      resolveCssVariables(tree),
      treeBox,
      [x, y, width, height].map(String).join(" ")
    ),
    "</svg>",
  ].join("")
}

/** The Open Graph image, as a PNG. */
export async function ogImage(
  files: ReadonlyArray<KulFile>,
  favicon: string
): Promise<Uint8Array> {
  return rasterise(ogImageSvg(files, favicon), OG_WIDTH)
}

/**
 * The favicon on a square ground of the palette's surface colour, which
 * home-screen icons need (a transparent one shows black on iOS).
 */
export function touchIconSvg(favicon: string, ground: string): string {
  return `<svg xmlns="http://www.w3.org/2000/svg" width="180" height="180" viewBox="0 0 180 180"><rect width="180" height="180" fill="${ground}"/>${nested(favicon, { x: 30, y: 30, width: 120, height: 120 }, "0 0 32 32")}</svg>`
}

/** An .ico holding one PNG image (Vista and later read PNG entries). */
export function icoFromPng(png: Uint8Array, size: number): Uint8Array {
  const header = new Uint8Array(6 + 16)
  const view = new DataView(header.buffer)
  view.setUint16(0, 0, true) // reserved
  view.setUint16(2, 1, true) // type: icon
  view.setUint16(4, 1, true) // one image
  view.setUint8(6, size >= 256 ? 0 : size) // width
  view.setUint8(7, size >= 256 ? 0 : size) // height
  view.setUint8(8, 0) // no palette
  view.setUint8(9, 0) // reserved
  view.setUint16(10, 1, true) // colour planes
  view.setUint16(12, 32, true) // bits per pixel
  view.setUint32(14, png.length, true) // image bytes
  view.setUint32(18, header.length, true) // image offset
  const ico = new Uint8Array(header.length + png.length)
  ico.set(header)
  ico.set(png, header.length)
  return ico
}

/** favicon.ico (32 px) and apple-touch-icon.png (180 px). */
export async function faviconImages(
  favicon: string,
  files: ReadonlyArray<KulFile>
): Promise<{ ico: Uint8Array; touchIcon: Uint8Array }> {
  const ground = paletteColour(renders(files).tree, "--kul-surface")
  return {
    ico: icoFromPng(await rasterise(favicon, 32), 32),
    touchIcon: await rasterise(touchIconSvg(favicon, ground), 180),
  }
}
