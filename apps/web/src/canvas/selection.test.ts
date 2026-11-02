import { describe, expect, it } from "vitest"

import {
  NO_MARKS,
  clearSelection,
  draws,
  link,
  reapply,
  select,
} from "./selection"
import type { TreeContents } from "./selection"

const chinua = { kind: "person", id: "chinua" } as const
const amara = { kind: "person", id: "amara" } as const
const wedding = { kind: "marriage", id: "m_chinua_adaeze" } as const

const tree: TreeContents = {
  persons: new Set(["chinua", "adaeze", "amara"]),
  marriages: new Set(["m_chinua_adaeze"]),
}

describe("select, clearSelection and link", () => {
  it("select marks the entity and leaves the link", () => {
    const marks = select(link(NO_MARKS, amara), chinua)
    expect(marks).toEqual({ selected: chinua, linked: amara })
  })

  it("changes nothing when the entity is already marked", () => {
    const marks = select(link(NO_MARKS, amara), chinua)
    expect(select(marks, { kind: "person", id: "chinua" })).toBe(marks)
    expect(link(marks, { kind: "person", id: "amara" })).toBe(marks)
    expect(clearSelection(NO_MARKS)).toBe(NO_MARKS)
    expect(link(NO_MARKS, undefined)).toBe(NO_MARKS)
  })

  it("tells a person from a marriage with the same id", () => {
    const marks = select(NO_MARKS, { kind: "person", id: "x" })
    expect(select(marks, { kind: "marriage", id: "x" }).selected).toEqual({
      kind: "marriage",
      id: "x",
    })
  })

  it("clearSelection keeps the link", () => {
    expect(clearSelection(select(link(NO_MARKS, amara), chinua))).toEqual({
      selected: undefined,
      linked: amara,
    })
  })
})

describe("reapply", () => {
  it("keeps marks whose entities the new tree still draws", () => {
    const marks = select(link(NO_MARKS, wedding), chinua)
    expect(reapply(marks, tree)).toBe(marks)
  })

  it("drops a selection the new tree no longer draws", () => {
    const marks = select(link(NO_MARKS, wedding), {
      kind: "person",
      id: "emeka",
    })
    expect(reapply(marks, tree)).toEqual({
      selected: undefined,
      linked: wedding,
    })
  })

  it("drops a link to an entity that is gone", () => {
    const marks = link(NO_MARKS, { kind: "marriage", id: "m_gone" })
    expect(reapply(marks, tree)).toEqual(NO_MARKS)
  })

  it("matches by kind as well as id", () => {
    expect(draws(tree, { kind: "marriage", id: "chinua" })).toBe(false)
    expect(draws(tree, chinua)).toBe(true)
    expect(draws(tree, wedding)).toBe(true)
  })
})
