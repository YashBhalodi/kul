import {
  cpSync,
  mkdtempSync,
  readFileSync,
  rmSync,
  writeFileSync,
} from "node:fs"
import { tmpdir } from "node:os"
import { join } from "node:path"
import { fileURLToPath } from "node:url"

import { describe, expect, it } from "vitest"

import { highlightKul } from "@/highlight/highlight-kul"
import { FEATURES } from "./features"
import type { FeatureSource } from "./features"
import {
  SAMPLE_EXAMPLE,
  buildFeature,
  buildLandingContent,
  buildSample,
  withoutComments,
} from "./landing-content"

const EXAMPLES = fileURLToPath(new URL("../../../../examples", import.meta.url))
const text = (lines: ReturnType<typeof highlightKul>) =>
  lines.map((line) => line.map((run) => run.text).join(""))

describe("the landing sample", () => {
  const sample = buildSample()

  it("is the example without its comments, and draws its tree", () => {
    expect(sample.file).toBe("three-generations.kul")
    expect(sample.familyName).toBe("Okeke")
    const shown = text(sample.lines)
    expect(shown.some((line) => line.trimStart().startsWith("#"))).toBe(false)
    expect(shown[0]).toMatch(/^person chinua /)
    expect(sample.svg).toContain('class="kul-card"')
  })

  it("numbers the four callouts in line order, on the right tokens", () => {
    expect(sample.notes.map((note) => note.kind)).toEqual([
      "person",
      "partialDate",
      "marriage",
      "birth",
    ])
    expect(sample.notes.map((note) => note.number)).toEqual([1, 2, 3, 4])
    for (const note of sample.notes) {
      const run = sample.lines[note.line]?.[note.run]
      expect(run?.kind).toBe(note.kind === "partialDate" ? "date" : "keyword")
    }
    expect(sample.facts).toEqual({
      personId: "chinua",
      spouses: ["chinua", "adaeze"],
      birthMarriage: "m_chinua_adaeze",
      partialDate: "1930-03",
      circaDate: "~1925",
    })
  })

  it("fails the build when the example stops showing a circa date", () => {
    const root = mkdtempSync(join(tmpdir(), "kul-landing-"))
    try {
      cpSync(join(EXAMPLES, SAMPLE_EXAMPLE), join(root, SAMPLE_EXAMPLE), {
        recursive: true,
      })
      const file = join(root, SAMPLE_EXAMPLE, "three-generations.kul")
      writeFileSync(
        file,
        readFileSync(file, "utf8").replaceAll("~1925", "1925")
      )
      expect(() => buildSample(root)).toThrow(/circa date/)
    } finally {
      rmSync(root, { recursive: true, force: true })
    }
  })
})

describe("withoutComments", () => {
  it("drops comment lines and keeps one blank line between code", () => {
    const lines = highlightKul(
      '# title\n\n# note\nperson a  name:"A"  gender:male\n\n\n# more\n\nperson b  name:"B"  gender:male  # kept\n'
    )
    expect(withoutComments(lines)).toBe(
      'person a  name:"A"  gender:male\n\nperson b  name:"B"  gender:male  # kept\n'
    )
  })
})

describe("the feature examples", () => {
  it("all check as their copy says, and one shows a real message", () => {
    const { features } = buildLandingContent()
    expect(features.map((feature) => feature.id)).toEqual(
      FEATURES.map((feature) => feature.id)
    )
    const mistakes = features.find((feature) => feature.id === "mistakes")
    expect(mistakes?.diagnostics).toEqual([
      expect.objectContaining({ code: "KUL-R02", line: 2 }),
    ])
    expect(mistakes?.diagnostics[0]?.message).toContain("m_chinua_adeze")
    for (const feature of features) {
      expect(feature.lines.length).toBeGreaterThan(0)
      expect(feature.lines.length).toBeLessThanOrEqual(3)
    }
  })

  const feature = (patch: Partial<FeatureSource>): FeatureSource => ({
    id: "probe",
    title: "Probe",
    body: "Probe.",
    files: [{ name: "a.kul", shown: 'person a  name:"A"  gender:male' }],
    ...patch,
  })

  it("shows only the shown lines, highlighted with the hidden ones", () => {
    const built = buildFeature(
      feature({
        files: [
          {
            name: "a.kul",
            before:
              'person a  name:"A"  gender:male\nperson b  name:"B"  gender:female',
            shown: "marriage m a b",
          },
        ],
      })
    )
    expect(text(built.lines)).toEqual(["marriage m a b"])
    expect(
      built.lines[0]?.filter((run) => run.kind === "reference")
    ).toHaveLength(2)
  })

  it("fails on an example that doesn't check clean", () => {
    expect(() =>
      buildFeature(feature({ files: [{ name: "a.kul", shown: "person a" }] }))
    ).toThrow(/probe.*does not check clean/s)
  })

  it("fails when the expected error is a different one", () => {
    expect(() => buildFeature(feature({ expectErrors: ["KUL-R02"] }))).toThrow(
      /expects KUL-R02 but checks clean/
    )
  })

  it("fails when the error is on a hidden line", () => {
    expect(() =>
      buildFeature(
        feature({
          expectErrors: ["KUL-R03"],
          files: [
            {
              name: "a.kul",
              before: "person z",
              shown: 'person a  name:"A"  gender:male',
            },
          ],
        })
      )
    ).toThrow(/outside the shown lines/)
  })

  it("fails unless exactly one file shows lines", () => {
    expect(() =>
      buildFeature(
        feature({
          files: [
            { name: "a.kul", shown: 'person a  name:"A"  gender:male' },
            { name: "b.kul", shown: 'person b  name:"B"  gender:male' },
          ],
        })
      )
    ).toThrow(/one file/)
  })
})
