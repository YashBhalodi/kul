import { describe, expect, it } from "vitest"

import { parseEditorWindow } from "./editor-window"

describe("parseEditorWindow", () => {
  it("reads a stored layout", () => {
    expect(
      parseEditorWindow({
        x: 1,
        y: 2,
        width: 400,
        height: 300,
        dock: "right",
        minimized: true,
      })
    ).toEqual({
      x: 1,
      y: 2,
      width: 400,
      height: 300,
      dock: "right",
      minimized: true,
    })
  })

  it("defaults an unknown dock and minimized state", () => {
    expect(
      parseEditorWindow({ x: 0, y: 0, width: 1, height: 1, dock: "top" })
    ).toMatchObject({ dock: null, minimized: false })
  })

  it("refuses anything without a position and a size", () => {
    expect(parseEditorWindow(null)).toBeNull()
    expect(parseEditorWindow({ x: 10, docked: "left" })).toBeNull()
    expect(
      parseEditorWindow({ x: Number.NaN, y: 0, width: 1, height: 1 })
    ).toBeNull()
  })
})
