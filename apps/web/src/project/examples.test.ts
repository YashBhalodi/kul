import { readFileSync, readdirSync } from "node:fs"
import { join } from "node:path"

import { describe, expect, it } from "vitest"

import { EXAMPLES, FIRST_RUN_EXAMPLE, findExample } from "./examples"

const EXAMPLES_DIR = join(import.meta.dirname, "../../../../examples")

describe("the bundled examples", () => {
  it("are exactly the repository's example projects, byte for byte", () => {
    const onDisk = readdirSync(EXAMPLES_DIR, { withFileTypes: true })
      .filter((entry) => entry.isDirectory())
      .map((dir) => ({
        slug: dir.name,
        files: readdirSync(join(EXAMPLES_DIR, dir.name))
          .filter((name) => name.endsWith(".kul"))
          .map((name) => ({
            name,
            text: readFileSync(join(EXAMPLES_DIR, dir.name, name), "utf8"),
          })),
      }))
      .filter((example) => example.files.length > 0)
    expect(onDisk.length).toBeGreaterThan(0)
    expect(
      EXAMPLES.map(({ slug, files }) => ({
        slug,
        files: [...files].sort((a, b) => a.name.localeCompare(b.name)),
      }))
    ).toEqual(
      onDisk.map(({ slug, files }) => ({
        slug,
        files: files.sort((a, b) => a.name.localeCompare(b.name)),
      }))
    )
  })

  it("take their titles from their first lines", () => {
    expect(
      EXAMPLES.map((example) => [example.slug, example.title])
    ).toContainEqual(["01-nuclear-family", "A nuclear family"])
    expect(findExample("08-multi-file-project")?.title).toBe(
      "A multi-file project"
    )
  })

  it("name a project after the example's family", () => {
    const firstRun = findExample(FIRST_RUN_EXAMPLE)
    expect(firstRun?.projectName).toBe("The Okeke family")
    expect(firstRun?.files.map((file) => file.name)).toEqual([
      "three-generations.kul",
    ])
    expect(findExample("09-family-across-a-century")?.projectName).toBe(
      "The Adeyemi family"
    )
    // Without family names, the project takes the example's title.
    expect(findExample("01-nuclear-family")?.projectName).toBe(
      "A nuclear family"
    )
  })

  it("list a multi-file example's files in file order", () => {
    expect(
      findExample("08-multi-file-project")?.files.map((file) => file.name)
    ).toEqual(["01-founders.kul", "02-children.kul", "03-grandchildren.kul"])
  })
})
