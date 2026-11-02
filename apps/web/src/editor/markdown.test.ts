import { describe, expect, it } from "vitest"

import { parseInline, parseMarkdown } from "./markdown"

describe("parseMarkdown", () => {
  it("reads a person's hover: a heading line and a list", () => {
    const markdown =
      "**Alice** — `person alice`\n\n- gender: female\n- born: `1900-01-01`\n\nHost of marriage `m`."
    expect(parseMarkdown(markdown)).toEqual([
      {
        type: "paragraph",
        children: [
          { type: "strong", children: [{ type: "text", text: "Alice" }] },
          { type: "text", text: " — " },
          { type: "code", text: "person alice" },
        ],
      },
      {
        type: "list",
        items: [
          [{ type: "text", text: "gender: female" }],
          [
            { type: "text", text: "born: " },
            { type: "code", text: "1900-01-01" },
          ],
        ],
      },
      {
        type: "paragraph",
        children: [
          { type: "text", text: "Host of marriage " },
          { type: "code", text: "m" },
          { type: "text", text: "." },
        ],
      },
    ])
  })

  it("reads fenced code and links", () => {
    const markdown =
      '**`person`** — declares an individual.\n\n```kul\nperson a name:"A"\n```\n\n[Top-level statements →](https://example.org/04.md)'
    const blocks = parseMarkdown(markdown)
    expect(blocks[0]).toEqual({
      type: "paragraph",
      children: [
        { type: "strong", children: [{ type: "code", text: "person" }] },
        { type: "text", text: " — declares an individual." },
      ],
    })
    expect(blocks[1]).toEqual({ type: "code", text: 'person a name:"A"' })
    expect(blocks[2]).toEqual({
      type: "paragraph",
      children: [
        {
          type: "link",
          href: "https://example.org/04.md",
          children: [{ type: "text", text: "Top-level statements →" }],
        },
      ],
    })
  })
})

describe("parseInline", () => {
  it("unescapes backslashes and reads emphasis", () => {
    expect(parseInline('say \\"hi\\" *(not declared)*')).toEqual([
      { type: "text", text: 'say "hi" ' },
      { type: "em", children: [{ type: "text", text: "(not declared)" }] },
    ])
  })

  it("leaves unmatched markers as text", () => {
    expect(parseInline("a * b ` c")).toEqual([
      { type: "text", text: "a * b ` c" },
    ])
  })
})
