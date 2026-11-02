import { describe, expect, it } from "vitest"

import { toScreen } from "./camera"
import type { Box, Camera } from "./camera"
import {
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

  it("is the strip beside the editor window", () => {
    expect(visibleRegion(canvas, [editorLeft])).toEqual({
      x: 616,
      y: 0,
      width: 784,
      height: 800,
    })
  })

  it("is the strip between the editor and the details panel", () => {
    expect(visibleRegion(canvas, [panelRight, editorLeft])).toMatchObject({
      x: 616,
      width: 424,
    })
  })

  it("takes the wider side whichever side the occluder is on", () => {
    const right = visibleRegion(canvas, [
      { x: 200, y: 0, width: 300, height: 800 },
    ])
    expect(right).toMatchObject({ x: 500, width: 900 })
    const left = visibleRegion(canvas, [
      { x: 900, y: 0, width: 300, height: 800 },
    ])
    expect(left).toMatchObject({ x: 0, width: 900 })
  })

  it("ignores the parts of occluders off the canvas", () => {
    expect(
      visibleRegion(canvas, [{ x: -300, y: 0, width: 400, height: 50 }])
    ).toMatchObject({ x: 100, width: 1300 })
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

  it("moves a box hidden behind the editor just clear of it", () => {
    const moved = revealCamera({ x: 100, y: 300, zoom: 1 }, card, canvas, [
      editorLeft,
    ])
    if (!moved) throw new Error("expected the camera to move")
    const screen = toScreen(moved, card)
    expect(screen.x).toBe(616 + REVEAL_MARGIN)
    // Only across: it was already at a good height.
    expect(screen.y).toBe(300)
    expect(inView(screen, canvas, [editorLeft])).toBe(true)
  })

  it("moves a box behind a panel on the right just clear of it", () => {
    const moved = revealCamera({ x: 1100, y: 300, zoom: 1 }, card, canvas, [
      editorLeft,
      panelRight,
    ])
    if (!moved) throw new Error("expected the camera to move")
    const screen = toScreen(moved, card)
    expect(screen.x + screen.width).toBe(1040 - REVEAL_MARGIN)
    expect(inView(screen, canvas, [editorLeft, panelRight])).toBe(true)
  })

  it("brings a box from below the canvas up into it, keeping the zoom", () => {
    const moved = revealCamera({ x: 700, y: 2000, zoom: 2 }, card, canvas, [])
    if (!moved) throw new Error("expected the camera to move")
    const screen = toScreen(moved, card)
    expect(moved.zoom).toBe(2)
    expect(screen.y + screen.height).toBe(800 - REVEAL_MARGIN)
    expect(screen.x).toBe(700)
  })

  it("centres a box too wide for the visible region", () => {
    const wide = { x: 0, y: 0, width: 1000, height: 64 }
    const moved = revealCamera({ x: -2000, y: 300, zoom: 1 }, wide, canvas, [
      editorLeft,
    ])
    if (!moved) throw new Error("expected the camera to move")
    expect(centreX(toScreen(moved, wide))).toBe(616 + 784 / 2)
  })
})

describe("centreInView", () => {
  it("centres even a box already in view", () => {
    const camera = centreInView({ x: 650, y: 300, zoom: 1 }, card, canvas, [])
    expect(centreX(toScreen(camera, card))).toBe(700)
  })

  it("keeps the zoom", () => {
    expect(centreInView({ ...at100, zoom: 3 }, card, canvas, []).zoom).toBe(3)
  })
})
