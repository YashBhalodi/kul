import { describe, expect, it } from "vitest"

import { detailsModel, formatDate, sentenceCase } from "./details"
import { EVERY_CATEGORY } from "./every-category"
import { workspaceWith } from "@/highlight/node-engine"

function modelFor(source: string, id: string) {
  const workspace = workspaceWith({ "family.kul": source })
  try {
    const detail = workspace.details(id)
    if (!detail) throw new Error(`no details for ${id}`)
    return detailsModel(detail)
  } finally {
    workspace.free()
  }
}

const OKEKE = `
person chinua  name:"Chinua Okeke"  gender:male  family:"Okeke"  given:"Chinua"  born:~1925  died:1998-05-12
person adaeze  name:"Adaeze Okeke"  gender:female  born:1930-03
marriage m_chinua_adaeze chinua adaeze  start:1948-11-02
person emeka  name:"Emeka Okeke"  gender:male  born:1950
  birth m_chinua_adaeze
`

describe("formatDate and sentenceCase", () => {
  it("shows dates as they are written", () => {
    expect(formatDate({ value: "1925", precision: "year", circa: true })).toBe(
      "~1925"
    )
    expect(
      formatDate({ value: "1930-03", precision: "month", circa: false })
    ).toBe("1930-03")
  })

  it("turns language words into labels", () => {
    expect(sentenceCase("female")).toBe("Female")
    expect(sentenceCase("end_reason")).toBe("End reason")
  })
})

describe("detailsModel", () => {
  it("describes a person: names, dates and the people around them", () => {
    expect(modelFor(OKEKE, "chinua")).toEqual({
      entity: { kind: "person", id: "chinua" },
      kind: "Person",
      title: "Chinua Okeke",
      facts: [
        { label: "Gender", value: "Male" },
        { label: "Born", value: "~1925" },
        { label: "Died", value: "1998-05-12" },
        { label: "Family name", value: "Okeke" },
        { label: "Given name", value: "Chinua" },
      ],
      groups: [
        {
          title: "Marriages",
          rows: [
            {
              link: {
                entity: { kind: "person", id: "adaeze" },
                label: "Adaeze Okeke",
              },
              note: "married 1948-11-02",
              also: {
                entity: { kind: "marriage", id: "m_chinua_adaeze" },
                label: "Marriage",
              },
            },
          ],
        },
        {
          title: "Children",
          rows: [
            {
              link: {
                entity: { kind: "person", id: "emeka" },
                label: "Emeka Okeke",
              },
            },
          ],
        },
      ],
    })
  })

  it("lists birth parents apart from adoptive ones, with the adoption's dates", () => {
    const model = modelFor(EVERY_CATEGORY, "gus")
    expect(model?.groups.map((group) => group.title)).toEqual(["Adopted by"])
    expect(model?.groups[0]?.rows).toEqual([
      {
        link: { entity: { kind: "person", id: "ana" }, label: "Ana" },
        note: "from 1990",
      },
      {
        link: { entity: { kind: "person", id: "cal" }, label: "Cal" },
        note: "from 1990",
      },
    ])
  })

  it("says how a marriage ended, and marks adopted children", () => {
    const ana = modelFor(EVERY_CATEGORY, "ana")
    const marriages = ana?.groups.find((group) => group.title === "Marriages")
    expect(marriages?.rows.map((row) => row.note)).toEqual([
      "married 1970, ended 1980 (divorce)",
      "married 1982",
    ])
    const children = ana?.groups.find((group) => group.title === "Children")
    expect(
      children?.rows.map((row) => [row.link.label, row.note ?? ""])
    ).toEqual([
      ["Eve", ""],
      ["Fay", ""],
      ["Gus", "adopted from 1990"],
    ])
  })

  it("describes a marriage: its spouses, dates and children", () => {
    expect(modelFor(EVERY_CATEGORY, "m_ana_ben")).toEqual({
      entity: { kind: "marriage", id: "m_ana_ben" },
      kind: "Marriage",
      title: "Ana & Ben",
      facts: [
        { label: "Started", value: "1970" },
        { label: "Ended", value: "1980" },
        { label: "End reason", value: "Divorce" },
      ],
      groups: [
        {
          title: "Spouses",
          rows: [
            { link: { entity: { kind: "person", id: "ana" }, label: "Ana" } },
            { link: { entity: { kind: "person", id: "ben" }, label: "Ben" } },
          ],
        },
        {
          title: "Children",
          rows: [
            { link: { entity: { kind: "person", id: "eve" }, label: "Eve" } },
          ],
        },
      ],
    })
  })
})
