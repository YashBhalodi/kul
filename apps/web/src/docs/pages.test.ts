import { describe, expect, it } from "vitest"

import { llmsFullTxt, llmsTxt } from "./llms"
import {
  docsPages,
  htmlPath,
  markdownPath,
  pageForMarkdownName,
  pageMarkdown,
  parseInline,
} from "./pages"

describe("parseInline", () => {
  it("splits text into plain, code and link runs", () => {
    expect(parseInline("Use `born:` — see [dates](/docs/dates/).")).toEqual([
      { kind: "text", text: "Use " },
      { kind: "code", text: "born:" },
      { kind: "text", text: " — see " },
      { kind: "link", text: "dates", href: "/docs/dates/" },
      { kind: "text", text: "." },
    ])
  })
})

describe("docs paths", () => {
  it("gives every page a directory path and a Markdown variant", () => {
    for (const page of docsPages) {
      expect(htmlPath(page)).toMatch(/^\/docs\/([a-z0-9-]+\/)?$/)
      const name = markdownPath(page).slice("/docs/".length, -".md".length)
      expect(pageForMarkdownName(name)).toBe(page)
    }
  })
})

describe("pageMarkdown", () => {
  it("starts with the title and summary and fences code by language", () => {
    for (const page of docsPages) {
      const markdown = pageMarkdown(page)
      expect(
        markdown.startsWith(`# ${page.title}\n\n> ${page.description}`)
      ).toBe(true)
      for (const block of page.blocks) {
        if (block.kind === "code") {
          expect(markdown).toContain("```" + block.language + "\n" + block.code)
        }
      }
    }
  })
})

describe("llms files", () => {
  it("links every page's Markdown variant from llms.txt", () => {
    const text = llmsTxt()
    expect(text.startsWith("# Kul\n\n> ")).toBe(true)
    for (const page of docsPages) {
      expect(text).toContain(`https://kullang.dev${markdownPath(page)}`)
    }
    expect(text).toContain("## Optional")
  })

  it("includes every page in llms-full.txt", () => {
    const text = llmsFullTxt()
    for (const page of docsPages) expect(text).toContain(pageMarkdown(page))
  })
})
