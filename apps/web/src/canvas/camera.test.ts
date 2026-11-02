import { describe, expect, it } from "vitest"

import {
  MAX_ZOOM,
  MIN_ZOOM,
  applyWheel,
  between,
  centreOn,
  fitCamera,
  gridFor,
  panBy,
  toScreen,
  zoomAround,
  zoomBy,
} from "./camera"
import type { Camera, Point } from "./camera"

const at100: Camera = { x: 0, y: 0, zoom: 1 }
const page = { width: 1000, height: 800 }
const noMargins = { top: 0, right: 0, bottom: 0, left: 0 }

/** The tree point drawn at a screen point. */
function treePointAt(camera: Camera, screen: Point): Point {
  return {
    x: (screen.x - camera.x) / camera.zoom,
    y: (screen.y - camera.y) / camera.zoom,
  }
}

describe("panBy", () => {
  it("moves the tree by screen px", () => {
    expect(panBy({ x: 10, y: 20, zoom: 2 }, 5, -7)).toEqual({
      x: 15,
      y: 13,
      zoom: 2,
    })
  })
})

describe("zoomAround", () => {
  it("keeps the tree point under the pointer still", () => {
    const camera = { x: 40, y: -30, zoom: 1.5 }
    const pointer = { x: 300, y: 200 }
    const before = treePointAt(camera, pointer)
    const zoomed = zoomAround(camera, 3, pointer)
    expect(zoomed.zoom).toBe(3)
    const after = treePointAt(zoomed, pointer)
    expect(after.x).toBeCloseTo(before.x)
    expect(after.y).toBeCloseTo(before.y)
  })

  it("stays between 10% and 800%", () => {
    expect(zoomAround(at100, 50, { x: 0, y: 0 }).zoom).toBe(MAX_ZOOM)
    expect(zoomAround(at100, 0.001, { x: 0, y: 0 }).zoom).toBe(MIN_ZOOM)
    expect(MIN_ZOOM).toBe(0.1)
    expect(MAX_ZOOM).toBe(8)
  })

  it("does not drift when the zoom is already at a limit", () => {
    const camera = { x: 12, y: 34, zoom: MAX_ZOOM }
    expect(zoomBy(camera, 2, { x: 500, y: 500 })).toEqual(camera)
  })
})

describe("toScreen and centreOn", () => {
  it("draws a tree box through the camera", () => {
    expect(
      toScreen({ x: 10, y: 20, zoom: 2 }, { x: 5, y: 5, width: 50, height: 10 })
    ).toEqual({ x: 20, y: 30, width: 100, height: 20 })
  })

  it("puts a box's centre on a screen point, keeping the zoom", () => {
    const camera = centreOn(
      { x: 0, y: 0, zoom: 2 },
      { x: 100, y: 100, width: 40, height: 20 },
      { x: 500, y: 400 }
    )
    expect(camera.zoom).toBe(2)
    const screen = toScreen(camera, { x: 100, y: 100, width: 40, height: 20 })
    expect(screen.x + screen.width / 2).toBe(500)
    expect(screen.y + screen.height / 2).toBe(400)
  })
})

describe("fitCamera", () => {
  it("shrinks a large tree to fit the region, centred", () => {
    const region = { x: 600, y: 0, width: 800, height: 600 }
    const camera = fitCamera({ width: 4000, height: 1000 }, region, {
      top: 50,
      right: 50,
      bottom: 50,
      left: 50,
    })
    expect(camera.zoom).toBeCloseTo(700 / 4000)
    const screen = toScreen(camera, { x: 0, y: 0, width: 4000, height: 1000 })
    expect(screen.x).toBeCloseTo(650)
    expect(screen.x + screen.width).toBeCloseTo(1350)
    expect(screen.y + screen.height / 2).toBeCloseTo(300)
  })

  it("never enlarges a small tree past 100%", () => {
    const camera = fitCamera(
      { width: 100, height: 50 },
      { x: 0, y: 0, width: 1000, height: 800 },
      noMargins
    )
    expect(camera).toEqual({ x: 450, y: 375, zoom: 1 })
  })

  it("keeps to the minimum zoom for a vast tree", () => {
    const camera = fitCamera(
      { width: 1_000_000, height: 400 },
      { x: 0, y: 0, width: 1000, height: 800 },
      noMargins
    )
    expect(camera.zoom).toBe(MIN_ZOOM)
  })
})

describe("applyWheel", () => {
  const wheel = {
    deltaX: 0,
    deltaY: 0,
    deltaMode: 0,
    ctrlKey: false,
    metaKey: false,
  }

  it("pans against the scroll direction", () => {
    expect(
      applyWheel(
        at100,
        { ...wheel, deltaX: 10, deltaY: 30 },
        { x: 0, y: 0 },
        page
      )
    ).toEqual({ x: -10, y: -30, zoom: 1 })
  })

  it("counts lines and pages in px", () => {
    expect(
      applyWheel(
        at100,
        { ...wheel, deltaY: 3, deltaMode: 1 },
        { x: 0, y: 0 },
        page
      ).y
    ).toBe(-48)
    expect(
      applyWheel(
        at100,
        { ...wheel, deltaY: 1, deltaMode: 2 },
        { x: 0, y: 0 },
        page
      ).y
    ).toBe(-800)
  })

  it("zooms around the pointer when pinching or holding Ctrl or ⌘", () => {
    const pointer = { x: 250, y: 125 }
    for (const keys of [{ ctrlKey: true }, { metaKey: true }]) {
      const zoomedIn = applyWheel(
        at100,
        { ...wheel, ...keys, deltaY: -10 },
        pointer,
        page
      )
      expect(zoomedIn.zoom).toBeGreaterThan(1)
      expect(treePointAt(zoomedIn, pointer).x).toBeCloseTo(250)
      const zoomedOut = applyWheel(
        at100,
        { ...wheel, ...keys, deltaY: 10 },
        pointer,
        page
      )
      expect(zoomedOut.zoom).toBeLessThan(1)
    }
  })

  it("zooms a notched wheel in bounded steps", () => {
    const notch = applyWheel(
      at100,
      { ...wheel, ctrlKey: true, deltaY: -500 },
      { x: 0, y: 0 },
      page
    )
    expect(notch.zoom).toBeLessThan(1.6)
  })
})

describe("gridFor", () => {
  it("moves the dots with the camera", () => {
    expect(gridFor({ x: 25, y: -5, zoom: 1 })).toEqual({
      spacing: 20,
      x: 5,
      y: 15,
    })
  })

  it("keeps the spacing readable at every zoom", () => {
    for (const zoom of [0.1, 0.33, 1, 2.5, 8]) {
      const { spacing } = gridFor({ x: 0, y: 0, zoom })
      expect(spacing).toBeGreaterThanOrEqual(14)
      expect(spacing).toBeLessThan(28)
    }
  })
})

describe("between", () => {
  it("runs from one camera to the other", () => {
    const from = { x: 0, y: 0, zoom: 1 }
    const to = { x: 100, y: -50, zoom: 4 }
    expect(between(from, to, 0)).toEqual(from)
    expect(between(from, to, 1)).toEqual(to)
    expect(between(from, to, 0.5)).toEqual({ x: 50, y: -25, zoom: 2 })
  })
})
