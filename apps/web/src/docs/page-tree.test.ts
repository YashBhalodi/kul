import { describe, expect, it } from "vitest"

import { navSections, neighbours, readingOrder, slugsOfFile } from "./page-tree"

const page = (path: string, title: string) => ({ info: { path }, title })
const meta = (path: string, title: string, pages: Array<string>) => ({
  info: { path },
  title,
  pages,
})

const PAGES = [
  page("index.mdx", "Introduction"),
  page("quick-start.mdx", "Quick start"),
  page("guide/dates.mdx", "Dates"),
  page("guide/people.mdx", "People"),
  page("guide/extra.mdx", "Extra"),
  page("command-line.mdx", "Command line"),
]
const METAS = [
  meta("meta.json", "Learn Kul", [
    "index",
    "quick-start",
    "guide",
    "command-line",
  ]),
  meta("guide/meta.json", "Guide", ["people", "dates"]),
]

describe("slugsOfFile", () => {
  it("maps a file to its page's slugs, index files to their folder", () => {
    expect(slugsOfFile("index.mdx")).toEqual([])
    expect(slugsOfFile("guide/dates.mdx")).toEqual(["guide", "dates"])
    expect(slugsOfFile("guide/index.mdx")).toEqual(["guide"])
  })
})

describe("navSections", () => {
  it("follows meta.json, with unlisted pages last, and folders as sections", () => {
    expect(navSections(PAGES, METAS)).toEqual([
      {
        title: undefined,
        pages: [
          { title: "Introduction", url: "/docs/" },
          { title: "Quick start", url: "/docs/quick-start/" },
        ],
      },
      {
        title: "Guide",
        pages: [
          { title: "People", url: "/docs/guide/people/" },
          { title: "Dates", url: "/docs/guide/dates/" },
          { title: "Extra", url: "/docs/guide/extra/" },
        ],
      },
      {
        title: undefined,
        pages: [{ title: "Command line", url: "/docs/command-line/" }],
      },
    ])
  })
})

describe("neighbours", () => {
  const sections = navSections(PAGES, METAS)

  it("gives the pages before and after one, across sections", () => {
    expect(readingOrder(sections)).toHaveLength(6)
    const { previous, next } = neighbours(sections, "/docs/guide/people/")
    expect(previous?.title).toBe("Quick start")
    expect(next?.title).toBe("Dates")
  })

  it("has no previous for the first page and no next for the last", () => {
    expect(neighbours(sections, "/docs/").previous).toBeUndefined()
    expect(neighbours(sections, "/docs/command-line/").next).toBeUndefined()
  })
})
