import { describe, expect, it } from "vitest"

import { toScreen } from "./camera"
import type { Box, Camera } from "./camera"
import {
  CHROME_GAP,
  REVEAL_MARGIN,
  centreInView,
  inView,
  revealCamera,
  visibleRegion,
} from "./reveal"

const canvas = { width: 1400, height: 800 }
const editorLeft: Box = { x: 16, y: 16, width: 600, height: 768 }
const panelRight: Box = { x: 1040, y: 0, width: 360, height: 800 }
const card: Box = { x: 0, y: 0, width: 160, height: 64 }
const at100: Camera = { x: 0, y: 0, zoom: 1 }

function centreX(box: Box) {
  return box.x + box.width / 2
}

describe("visibleRegion", () => {
  it("is the whole canvas with nothing over it", () => {
    expect(visibleRegion(canvas, [])).toEqual({
      x: 0,
      y: 0,
      width: 1400,
      height: 800,
    })
  })

  it("is the strip beside the editor window, a gap clear of it", () => {
    expect(visibleRegion(canvas, [editorLeft])).toEqual({
      x: 616 + CHROME_GAP,
      y: 0,
      width: 1400 - 616 - CHROME_GAP,
      height: 800,
    })
  })

  it("is the strip between the editor and the details panel", () => {
    expect(visibleRegion(canvas, [panelRight, editorLeft])).toMatchObject({
      x: 616 + CHROME_GAP,
      width: 1040 - 616 - 2 * CHROME_GAP,
    })
  })

  it("takes the wider side whichever side the occluder is on", () => {
    const right = visibleRegion(canvas, [
      { x: 200, y: 0, width: 300, height: 800 },
    ])
    expect(right).toMatchObject({
      x: 500 + CHROME_GAP,
      width: 900 - CHROME_GAP,
    })
    const left = visibleRegion(canvas, [
      { x: 900, y: 0, width: 300, height: 800 },
    ])
    expect(left).toMatchObject({ x: 0, width: 900 - CHROME_GAP })
  })

  it("ignores the parts of occluders off the canvas", () => {
    expect(
      visibleRegion(canvas, [{ x: -300, y: 0, width: 400, height: 50 }])
    ).toMatchObject({ x: 100 + CHROME_GAP, width: 1300 - CHROME_GAP })
    expect(
      visibleRegion(canvas, [{ x: 2000, y: 0, width: 400, height: 800 }])
    ).toMatchObject({ x: 0, width: 1400 })
  })

  it("falls back to the whole canvas when occluders cover all of it", () => {
    expect(
      visibleRegion(canvas, [{ x: -10, y: 0, width: 2000, height: 800 }])
    ).toMatchObject({ x: 0, width: 1400 })
  })
})

describe("inView", () => {
  it("needs the box on the canvas and clear of every occluder", () => {
    const free = { x: 700, y: 300, width: 160, height: 64 }
    expect(inView(free, canvas, [editorLeft, panelRight])).toBe(true)
    expect(inView({ ...free, x: 500 }, canvas, [editorLeft, panelRight])).toBe(
      false
    )
    expect(inView({ ...free, x: 1000 }, canvas, [editorLeft, panelRight])).toBe(
      false
    )
    expect(inView({ ...free, y: -20 }, canvas, [])).toBe(false)
    expect(inView({ ...free, x: 1300 }, canvas, [])).toBe(false)
  })

  it("needs a gap between the box and the chrome beside it", () => {
    const box = { x: 616, y: 300, width: 160, height: 64 }
    // Touching the editor's edge: not hidden, but hard against it.
    expect(inView(box, canvas, [editorLeft])).toBe(false)
    expect(
      inView({ ...box, x: 616 + CHROME_GAP - 1 }, canvas, [editorLeft])
    ).toBe(false)
    expect(inView({ ...box, x: 616 + CHROME_GAP }, canvas, [editorLeft])).toBe(
      true
    )
  })

  it("counts a box above a short floating window as seen", () => {
    const shortWindow = { x: 16, y: 500, width: 600, height: 280 }
    expect(
      inView({ x: 100, y: 100, width: 160, height: 64 }, canvas, [shortWindow])
    ).toBe(true)
  })
})

describe("revealCamera", () => {
  it("leaves a box that can be seen where it is", () => {
    expect(
      revealCamera({ x: 700, y: 300, zoom: 1 }, card, canvas, [editorLeft])
    ).toBeUndefined()
  })

  it("moves a box hidden behind the editor a gap clear of it", () => {
    const moved = revealCamera({ x: 100, y: 300, zoom: 1 }, card, canvas, [
      editorLeft,
    ])
    if (!moved) throw new Error("expected the camera to move")
    const screen = toScreen(moved, card)
    expect(screen.x).toBe(616 + CHROME_GAP)
    // Only across: it was already at a good height.
    expect(screen.y).toBe(300)
    expect(inView(screen, canvas, [editorLeft])).toBe(true)
  })

  it("moves a box hard against the editor a gap clear of it", () => {
    const moved = revealCamera({ x: 620, y: 300, zoom: 1 }, card, canvas, [
      editorLeft,
    ])
    if (!moved) throw new Error("expected the camera to move")
    expect(toScreen(moved, card).x).toBe(616 + CHROME_GAP)
  })

  it("moves a box behind a panel on the right a gap clear of it", () => {
    const moved = revealCamera({ x: 1100, y: 300, zoom: 1 }, card, canvas, [
      editorLeft,
      panelRight,
    ])
    if (!moved) throw new Error("expected the camera to move")
    const screen = toScreen(moved, card)
    expect(screen.x + screen.width).toBe(1040 - CHROME_GAP)
    expect(inView(screen, canvas, [editorLeft, panelRight])).toBe(true)
  })

  it("keeps a revealed card's spouse clear of the chrome when both fit", () => {
    // A couple, side by side, the right one under the panel that opened
    // for it: the least move leaves the left one clear of the editor.
    const left = card
    const right = { ...card, x: 190 }
    const camera = { x: 700, y: 300, zoom: 1 }
    const chrome = [editorLeft, panelRight]
    const moved = revealCamera(camera, right, canvas, chrome)
    if (!moved) throw new Error("expected the camera to move")
    expect(inView(toScreen(moved, right), canvas, chrome)).toBe(true)
    expect(inView(toScreen(moved, left), canvas, chrome)).toBe(true)
  })

  it("brings a box from below the canvas up into it, keeping the zoom", () => {
    const moved = revealCamera({ x: 700, y: 2000, zoom: 2 }, card, canvas, [])
    if (!moved) throw new Error("expected the camera to move")
    const screen = toScreen(moved, card)
    expect(moved.zoom).toBe(2)
    expect(screen.y + screen.height).toBe(800 - REVEAL_MARGIN)
    expect(screen.x).toBe(700)
  })

  it("keeps the margin from the canvas's own edges", () => {
    const moved = revealCamera({ x: 1300, y: 300, zoom: 1 }, card, canvas, [
      editorLeft,
    ])
    if (!moved) throw new Error("expected the camera to move")
    const screen = toScreen(moved, card)
    expect(screen.x + screen.width).toBe(1400 - REVEAL_MARGIN)
  })

  it("centres a box too wide for the visible region", () => {
    const wide = { x: 0, y: 0, width: 1000, height: 64 }
    const moved = revealCamera({ x: -2000, y: 300, zoom: 1 }, wide, canvas, [
      editorLeft,
    ])
    if (!moved) throw new Error("expected the camera to move")
    const room = { from: 616 + CHROME_GAP, to: 1400 - REVEAL_MARGIN }
    expect(centreX(toScreen(moved, wide))).toBe((room.from + room.to) / 2)
  })
})

describe("centreInView", () => {
  it("centres even a box already in view", () => {
    const camera = centreInView({ x: 650, y: 300, zoom: 1 }, card, canvas, [])
    expect(centreX(toScreen(camera, card))).toBe(700)
  })

  it("centres across the strip beside the chrome", () => {
    const camera = centreInView(at100, card, canvas, [editorLeft])
    const from = 616 + CHROME_GAP
    expect(centreX(toScreen(camera, card))).toBe((from + 1400) / 2)
  })

  it("keeps the zoom", () => {
    expect(centreInView({ ...at100, zoom: 3 }, card, canvas, []).zoom).toBe(3)
  })
})
