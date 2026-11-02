import { describe, expect, it } from "vitest"

import { FileMirror } from "./file-mirror"

describe("FileMirror", () => {
  it("gives every text a new, increasing version", () => {
    const mirror = new FileMirror()
    const a1 = mirror.set("a.kul", "one")
    const b1 = mirror.set("b.kul", "two")
    const a2 = mirror.set("a.kul", "three")
    expect(b1).toBeGreaterThan(a1)
    expect(a2).toBeGreaterThan(b1)
    expect(mirror.get("a.kul")).toEqual({
      name: "a.kul",
      text: "three",
      version: a2,
    })
  })

  it("calls an answer current only when it saw every file's latest version", () => {
    const mirror = new FileMirror()
    const a = mirror.set("a.kul", "x")
    const b = mirror.set("b.kul", "y")
    expect(mirror.isCurrent({ "a.kul": a, "b.kul": b })).toBe(true)
    // An older text of one file.
    const a2 = mirror.set("a.kul", "x2")
    expect(mirror.isCurrent({ "a.kul": a, "b.kul": b })).toBe(false)
    expect(mirror.isCurrent({ "a.kul": a2, "b.kul": b })).toBe(true)
    // A file missing from the answer, or one the mirror no longer has.
    expect(mirror.isCurrent({ "a.kul": a2 })).toBe(false)
    mirror.delete("b.kul")
    expect(mirror.isCurrent({ "a.kul": a2, "b.kul": b })).toBe(false)
    expect(mirror.isCurrent({ "a.kul": a2 })).toBe(true)
  })

  it("checks an analysis's file list the same way", () => {
    const mirror = new FileMirror()
    const a = mirror.set("a.kul", "x")
    expect(mirror.isCurrentAnalysis([{ name: "a.kul", version: a }])).toBe(true)
    expect(mirror.isCurrentAnalysis([{ name: "a.kul", version: a - 1 }])).toBe(
      false
    )
    expect(mirror.isCurrentAnalysis([])).toBe(false)
  })

  it("replaces the whole set with fresh versions", () => {
    const mirror = new FileMirror()
    const old = mirror.set("old.kul", "")
    const files = mirror.replace([
      { name: "a.kul", text: "1" },
      { name: "b.kul", text: "2" },
    ])
    expect(files.map((file) => file.name)).toEqual(["a.kul", "b.kul"])
    expect(files.every((file) => file.version > old)).toBe(true)
    expect(mirror.get("old.kul")).toBeUndefined()
    expect(mirror.snapshot()).toEqual(files)
  })
})
