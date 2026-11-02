import { describe, expect, it } from "vitest"

import type { EditorWindowLayout } from "@/store/editor-window"
import {
  KEEP_VISIBLE,
  MIN_HEIGHT,
  MIN_WIDTH,
  defaultLayout,
  dockAt,
  freeCanvasInsets,
  keepOnCanvas,
  moveBy,
  resizeBy,
  resizeDocked,
  undockUnder,
  windowRect,
} from "./window-geometry"

const canvas = { width: 1440, height: 852 }

const floating: EditorWindowLayout = {
  x: 100,
  y: 50,
  width: 600,
  height: 500,
  dock: null,
  minimized: false,
}

describe("defaultLayout", () => {
  it("floats at the left, inside the canvas", () => {
    const layout = defaultLayout(canvas)
    expect(layout).toMatchObject({ x: 16, y: 16, dock: null, minimized: false })
    expect(layout.width).toBeGreaterThanOrEqual(MIN_WIDTH)
    expect(layout.y + layout.height).toBeLessThanOrEqual(canvas.height)
  })

  it("never goes below the minimum size", () => {
    const layout = defaultLayout({ width: 500, height: 200 })
    expect(layout.width).toBe(MIN_WIDTH)
    expect(layout.height).toBe(MIN_HEIGHT)
  })
})

describe("keepOnCanvas", () => {
  it("keeps 48 px on the canvas and the title bar below its top", () => {
    expect(
      keepOnCanvas({ x: -2000, y: -50, width: 600, height: 400 }, canvas)
    ).toEqual({ x: KEEP_VISIBLE - 600, y: 0, width: 600, height: 400 })
    expect(
      keepOnCanvas({ x: 5000, y: 5000, width: 600, height: 400 }, canvas)
    ).toEqual({
      x: canvas.width - KEEP_VISIBLE,
      y: canvas.height - KEEP_VISIBLE,
      width: 600,
      height: 400,
    })
  })

  it("enforces the minimum size and the canvas's", () => {
    expect(
      keepOnCanvas({ x: 0, y: 0, width: 10, height: 10 }, canvas)
    ).toMatchObject({ width: MIN_WIDTH, height: MIN_HEIGHT })
    expect(
      keepOnCanvas({ x: 0, y: 0, width: 9000, height: 9000 }, canvas)
    ).toMatchObject({ width: canvas.width, height: canvas.height })
  })
})

describe("windowRect", () => {
  it("fills the canvas's height when docked, on its side", () => {
    expect(windowRect({ ...floating, dock: "left" }, canvas)).toEqual({
      x: 0,
      y: 0,
      width: 600,
      height: canvas.height,
    })
    expect(windowRect({ ...floating, dock: "right" }, canvas)).toEqual({
      x: canvas.width - 600,
      y: 0,
      width: 600,
      height: canvas.height,
    })
  })

  it("keeps a stored floating window on a smaller canvas", () => {
    const rect = windowRect(
      { ...floating, x: 1400 },
      { width: 800, height: 600 }
    )
    expect(rect.x).toBe(800 - KEEP_VISIBLE)
  })
})

describe("dragging", () => {
  it("docks near either side", () => {
    expect(dockAt(10, canvas)).toBe("left")
    expect(dockAt(canvas.width - 5, canvas)).toBe("right")
    expect(dockAt(700, canvas)).toBeNull()
  })

  it("moves within the canvas", () => {
    expect(moveBy(floating, 40, -20, canvas)).toEqual({
      x: 140,
      y: 30,
      width: 600,
      height: 500,
    })
    expect(moveBy(floating, 0, -500, canvas).y).toBe(0)
  })

  it("undocks under the pointer at the window's own size", () => {
    const docked = windowRect({ ...floating, dock: "left" }, canvas)
    const rect = undockUnder(floating, docked, { x: 300, y: 10 }, canvas)
    expect(rect.width).toBe(600)
    expect(rect.height).toBe(500)
    // Grabbed at the middle of the bar, it stays held at its middle.
    expect(rect.x).toBe(0)
    expect(rect.y).toBe(0)
  })
})

describe("resizing", () => {
  const start = { x: 100, y: 100, width: 600, height: 500 }

  it("grows from the east and south edges", () => {
    expect(resizeBy(start, "se", 50, 60, canvas)).toEqual({
      x: 100,
      y: 100,
      width: 650,
      height: 560,
    })
  })

  it("moves the west and north edges, keeping the other sides", () => {
    expect(resizeBy(start, "nw", -50, -40, canvas)).toEqual({
      x: 50,
      y: 60,
      width: 650,
      height: 540,
    })
  })

  it("stops at the minimum size and the canvas", () => {
    const small = resizeBy(start, "nw", 1000, 1000, canvas)
    expect(small).toMatchObject({ width: MIN_WIDTH, height: MIN_HEIGHT })
    expect(small.x + small.width).toBe(700)
    expect(small.y + small.height).toBe(600)
    expect(resizeBy(start, "e", 5000, 0, canvas).width).toBe(
      canvas.width - start.x
    )
  })

  it("widens a docked window from its inner edge", () => {
    expect(resizeDocked({ ...floating, dock: "left" }, 40, canvas).width).toBe(
      640
    )
    expect(resizeDocked({ ...floating, dock: "right" }, 40, canvas).width).toBe(
      560
    )
  })
})

describe("freeCanvasInsets", () => {
  it("keeps chrome beside the window, on its wider side", () => {
    expect(freeCanvasInsets(floating, canvas)).toEqual({ left: 716, right: 0 })
    expect(freeCanvasInsets({ ...floating, dock: "right" }, canvas)).toEqual({
      left: 0,
      right: 616,
    })
  })

  it("uses the whole width when the window is minimized or leaves no room", () => {
    expect(freeCanvasInsets({ ...floating, minimized: true }, canvas)).toEqual({
      left: 0,
      right: 0,
    })
    expect(
      freeCanvasInsets({ ...floating, x: 300, width: 900 }, canvas)
    ).toEqual({ left: 0, right: 0 })
  })
})
