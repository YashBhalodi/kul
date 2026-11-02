import { strToU8 } from "fflate"
import { describe, expect, it } from "vitest"

import {
  findCollisions,
  flattenImport,
  importSummary,
  importedProjectName,
  planImport,
} from "./import"
import type { ImportEntry } from "./import"
import { unzipEntries, zipProject } from "./zip"

const entry = (path: string, text: string): ImportEntry => ({
  path,
  bytes: strToU8(text),
})

describe("flattenImport", () => {
  it("keeps the .kul files from a folder, flattened to their names", () => {
    const batch = flattenImport([
      entry("family/kul.yml", "kul: 0.1"),
      entry("family/a.kul", "person a"),
      entry("family/branch/b.kul", "person b"),
      entry("family/photo.png", "\u0089PNG"),
      entry("family\\windows\\c.kul", "person c"),
    ])
    expect(batch.files).toEqual([
      { name: "a.kul", text: "person a" },
      { name: "b.kul", text: "person b" },
      { name: "c.kul", text: "person c" },
    ])
    expect(batch.ignored).toBe(2)
  })

  it("opens a .zip and reads the .kul files inside it", () => {
    const zip = zipProject([
      { name: "a.kul", text: "person a" },
      { name: "notes.txt", text: "" },
    ])
    const batch = flattenImport([
      { path: "backup.ZIP", bytes: zip },
      entry("b.kul", "person b"),
    ])
    expect(batch.files.map((file) => file.name)).toEqual(["a.kul", "b.kul"])
    expect(batch.ignored).toBe(1)
  })

  it("ignores a .zip inside a .zip, a broken .zip and macOS metadata", () => {
    const inner = zipProject([{ name: "deep.kul", text: "" }])
    const outer = zipProject([{ name: "x.kul", text: "person x" }])
    const withInner = flattenImport([
      { path: "outer.zip", bytes: outer },
      { path: "broken.zip", bytes: strToU8("not a zip") },
      entry("__MACOSX/._x.kul", "\u0000\u0005"),
    ])
    expect(withInner.files.map((file) => file.name)).toEqual(["x.kul"])
    expect(withInner.ignored).toBe(2)
    const nested = flattenImport([
      {
        path: "nested.zip",
        bytes: zipProject([{ name: "inner.zip", text: "" }]),
      },
    ])
    expect(nested.files).toEqual([])
    expect(nested.ignored).toBe(1)
    expect(unzipEntries(inner).map((e) => e.path)).toEqual(["deep.kul"])
  })

  it("ignores files that are not UTF-8 text or have unusable names", () => {
    const batch = flattenImport([
      { path: "binary.kul", bytes: new Uint8Array([0xff, 0xfe, 0x00]) },
      entry(".kul", ""),
      entry(`${"x".repeat(200)}.kul`, ""),
      entry("ok.kul", "person ok"),
    ])
    expect(batch.files.map((file) => file.name)).toEqual(["ok.kul"])
    expect(batch.ignored).toBe(3)
  })
})

describe("collisions", () => {
  const incoming = [
    { name: "Smith.kul", text: "new smith" },
    { name: "new.kul", text: "new" },
    { name: "NEW.kul", text: "second new" },
  ]
  const existing = ["smith.kul", "other.kul"]

  it("finds names that clash with the project or the batch, ignoring case", () => {
    expect(findCollisions(existing, incoming)).toEqual(["Smith.kul", "NEW.kul"])
    expect(findCollisions([], [{ name: "a.kul", text: "" }])).toEqual([])
  })

  it("keeps both by numbering the newcomer", () => {
    expect(planImport(existing, incoming, "keep-both")).toEqual({
      writes: [
        { name: "Smith-2.kul", text: "new smith" },
        { name: "new.kul", text: "new" },
        { name: "NEW-2.kul", text: "second new" },
      ],
      added: 3,
      replaced: 0,
      skipped: 0,
    })
  })

  it("replaces under the name already there", () => {
    expect(planImport(existing, incoming, "replace")).toEqual({
      writes: [
        { name: "smith.kul", text: "new smith" },
        { name: "new.kul", text: "second new" },
      ],
      added: 1,
      replaced: 1,
      skipped: 1,
    })
  })

  it("skips the newcomer", () => {
    expect(planImport(existing, incoming, "skip")).toEqual({
      writes: [{ name: "new.kul", text: "new" }],
      added: 1,
      replaced: 0,
      skipped: 2,
    })
  })
})

describe("the zip of a project", () => {
  it("round-trips its files at the archive's root", () => {
    const files = [
      { name: "01-founders.kul", text: '# Founders\nperson a name:"Ā"\n' },
      { name: "ઓકેકે.kul", text: "" },
    ]
    const zip = zipProject(files)
    expect(
      unzipEntries(zip).map(({ path, bytes }) => ({
        name: path,
        text: new TextDecoder().decode(bytes),
      }))
    ).toEqual(files)
    expect(flattenImport([{ path: "p.zip", bytes: zip }]).files).toEqual(files)
  })

  it("has the same bytes for the same files", () => {
    const files = [{ name: "a.kul", text: "person a" }]
    expect(zipProject(files)).toEqual(zipProject(files))
  })
})

describe("importSummary", () => {
  it("counts what was imported, skipped and ignored", () => {
    expect(importSummary({ imported: 6, skipped: 0, ignored: 2 })).toBe(
      "Imported 6 files, ignored 2"
    )
    expect(importSummary({ imported: 1, skipped: 1, ignored: 0 })).toBe(
      "Imported 1 file, skipped 1"
    )
    expect(importSummary({ imported: 0, skipped: 0, ignored: 3 })).toBe(
      "No .kul files imported, ignored 3"
    )
  })
})

describe("importedProjectName", () => {
  it("names the project after a lone folder or archive", () => {
    expect(importedProjectName(["okeke-family"])).toBe("okeke-family")
    expect(importedProjectName(["backup.zip"])).toBe("backup")
    expect(importedProjectName(["a.kul", "b.kul"])).toBe("Imported project")
    expect(importedProjectName([])).toBe("Imported project")
  })
})
