import { describe, expect, it } from "vitest"

import { parseCanvasCamera } from "./canvas-camera"

describe("parseCanvasCamera", () => {
  it("reads a stored camera", () => {
    expect(parseCanvasCamera({ x: -12.5, y: 40, zoom: 0.75 })).toEqual({
      x: -12.5,
      y: 40,
      zoom: 0.75,
    })
  })

  it("refuses anything that isn't a camera", () => {
    expect(parseCanvasCamera(null)).toBeNull()
    expect(parseCanvasCamera("camera")).toBeNull()
    expect(parseCanvasCamera({ x: 1, y: 2 })).toBeNull()
    expect(parseCanvasCamera({ x: 1, y: Number.NaN, zoom: 1 })).toBeNull()
    expect(parseCanvasCamera({ x: 1, y: 2, zoom: 0 })).toBeNull()
  })
})
