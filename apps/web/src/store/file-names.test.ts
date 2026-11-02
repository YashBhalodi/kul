import { describe, expect, it } from "vitest"

import {
  compareFileNames,
  fileNameProblem,
  inFileOrder,
  sameFileName,
  uniqueFileName,
  withKulExtension,
} from "./file-names"

describe("fileNameProblem", () => {
  it("accepts a .kul name", () => {
    expect(fileNameProblem("family.kul", [])).toBeUndefined()
    expect(fileNameProblem("ઓકેકે પરિવાર.kul", [])).toBeUndefined()
  })

  it("requires the .kul extension and a name before it", () => {
    expect(fileNameProblem("family", [])).toBe("extension")
    expect(fileNameProblem("family.KUL", [])).toBe("extension")
    expect(fileNameProblem(".kul", [])).toBe("empty")
  })

  it("refuses path separators and control characters", () => {
    expect(fileNameProblem("a/b.kul", [])).toBe("separator")
    expect(fileNameProblem("a\\b.kul", [])).toBe("separator")
    expect(fileNameProblem("a\nb.kul", [])).toBe("control")
  })

  it("allows at most 128 characters, counting the extension", () => {
    const longest = `${"x".repeat(124)}.kul`
    expect(fileNameProblem(longest, [])).toBeUndefined()
    expect(fileNameProblem(`x${longest}`, [])).toBe("too-long")
    // Characters, not UTF-16 code units.
    expect(fileNameProblem(`${"😀".repeat(124)}.kul`, [])).toBeUndefined()
  })

  it("refuses a name another file has, ignoring case", () => {
    expect(fileNameProblem("Smith.kul", ["smith.kul"])).toBe("taken")
    expect(fileNameProblem("smith.kul", ["smith.kul"])).toBe("taken")
    expect(fileNameProblem("smith2.kul", ["smith.kul"])).toBeUndefined()
  })

  it("lets a renamed file keep its name or change its case", () => {
    const files = ["smith.kul", "jones.kul"]
    expect(fileNameProblem("smith.kul", files, "smith.kul")).toBeUndefined()
    expect(fileNameProblem("Smith.kul", files, "smith.kul")).toBeUndefined()
    expect(fileNameProblem("JONES.kul", files, "smith.kul")).toBe("taken")
  })
})

describe("sameFileName", () => {
  it("ignores case and Unicode normalisation", () => {
    expect(sameFileName("ÉMILE.kul", "émile.kul")).toBe(true)
    expect(sameFileName("é.kul", "é.kul")).toBe(true)
    expect(sameFileName("a.kul", "b.kul")).toBe(false)
  })
})

describe("withKulExtension", () => {
  it("appends .kul to what was typed", () => {
    expect(withKulExtension(" family ")).toBe("family.kul")
    expect(withKulExtension("family.kul")).toBe("family.kul")
    expect(withKulExtension("family.KUL")).toBe("family.kul")
    expect(withKulExtension("")).toBe(".kul")
  })
})

describe("file order", () => {
  it("compares the UTF-8 bytes of the names", () => {
    const names = ["😀.kul", "ｚ.kul", "b.kul", "B.kul", "a.kul", "é.kul"]
    expect(names.slice().sort(compareFileNames)).toEqual([
      "B.kul",
      "a.kul",
      "b.kul",
      "é.kul",
      "ｚ.kul",
      "😀.kul",
    ])
    expect(compareFileNames("a.kul", "a.kul")).toBe(0)
    expect(compareFileNames("ab", "abc")).toBeLessThan(0)
  })

  it("sorts records by name", () => {
    expect(
      inFileOrder([{ name: "b.kul" }, { name: "a.kul" }]).map((f) => f.name)
    ).toEqual(["a.kul", "b.kul"])
  })
})

describe("uniqueFileName", () => {
  it("numbers a name that is taken", () => {
    expect(uniqueFileName("a.kul", [])).toBe("a.kul")
    expect(uniqueFileName("a.kul", ["A.kul"])).toBe("a-2.kul")
    expect(uniqueFileName("a.kul", ["a.kul", "a-2.kul"])).toBe("a-3.kul")
  })

  it("shortens the stem to stay within the length limit", () => {
    const long = `${"x".repeat(124)}.kul`
    const next = uniqueFileName(long, [long])
    expect(next).toBe(`${"x".repeat(122)}-2.kul`)
    expect(fileNameProblem(next, [long])).toBeUndefined()
  })
})
