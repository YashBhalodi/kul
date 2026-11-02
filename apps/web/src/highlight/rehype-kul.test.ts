import type { Element, Root } from "hast"
import type { MdxJsxFlowElementHast } from "mdast-util-mdx-jsx"
import { VFile } from "vfile"
import { describe, expect, it } from "vitest"

import { rehypeKul } from "./rehype-kul"

/** A hast tree of ```kul blocks, as MDX's remark-rehype makes them. */
function tree(...blocks: Array<{ code: string; meta?: string }>): Root {
  return {
    type: "root",
    children: blocks.map(({ code, meta }): Element => ({
      type: "element",
      tagName: "pre",
      properties: {},
      children: [
        {
          type: "element",
          tagName: "code",
          properties: { className: ["language-kul"] },
          data: { meta: meta ?? null },
          children: [{ type: "text", value: `${code}\n` }],
        },
      ],
    })),
  }
}

function run(root: Root): Array<MdxJsxFlowElementHast> {
  rehypeKul()(root, new VFile({ path: "page.mdx" }))
  return root.children as unknown as Array<MdxJsxFlowElementHast>
}

function attributes(
  node: MdxJsxFlowElementHast | undefined
): Record<string, unknown> {
  if (!node) throw new Error("no node")
  return Object.fromEntries(
    node.attributes.map((attribute) =>
      attribute.type === "mdxJsxAttribute"
        ? [attribute.name, attribute.value]
        : ["?", undefined]
    )
  )
}

const ASHA = 'person asha  name:"Asha"  gender:female'

describe("rehypeKul", () => {
  it("turns a clean block into KulCode with its highlighted lines and files", () => {
    const [node] = run(tree({ code: ASHA }))
    expect(node?.type).toBe("mdxJsxFlowElement")
    expect(node?.name).toBe("KulCode")
    expect(JSON.parse(String(attributes(node).files))).toEqual([
      { name: "family.kul", text: `${ASHA}\n` },
    ])
    const line = node?.children[0] as Element
    expect(line.properties.className).toEqual(["line"])
    const first = line.children[0] as Element
    expect(first.properties.className).toEqual(["tok-keyword"])
  })

  it("fails the build on a block that doesn't check clean", () => {
    expect(() => run(tree({ code: 'person asha  name:"Asha"' }))).toThrow(
      /does not check clean:[\s\S]*KUL-R03/
    )
  })

  it("passes a block that shows the mistake it says, with the message", () => {
    const [node] = run(
      tree({ code: 'person asha  name:"Asha"', meta: "expect-error=KUL-R03" })
    )
    const shown = JSON.parse(String(attributes(node).diagnostics)) as Array<{
      code: string
    }>
    expect(shown.map((entry) => entry.code)).toEqual(["KUL-R03"])
    expect(attributes(node).files).toBeUndefined()
  })

  it("fails a block whose mistake is not the one it expects", () => {
    expect(() =>
      run(tree({ code: ASHA, meta: "expect-error=KUL-R03" }))
    ).toThrow(/expects KUL-R03 but checks clean/)
  })

  it("checks blocks of one project together, by file name", () => {
    const nodes = run(
      tree(
        {
          code: 'person a  name:"A"  gender:male\nperson b  name:"B"  gender:female\nmarriage m a b',
          meta: 'project="p" title="01-a.kul"',
        },
        {
          code: 'person c  name:"C"  gender:male\n  birth m',
          meta: 'project="p" title="02-c.kul"',
        }
      )
    )
    const files = JSON.parse(String(attributes(nodes[1]).files)) as Array<{
      name: string
    }>
    expect(files.map((file) => file.name)).toEqual(["01-a.kul", "02-c.kul"])
    expect(attributes(nodes[1]).title).toBe("02-c.kul")
  })

  it("fails a project block without a file name", () => {
    expect(() => run(tree({ code: ASHA, meta: 'project="p"' }))).toThrow(
      /needs title=/
    )
  })

  it("adds the drawn tree to a block marked tree", () => {
    const [node] = run(tree({ code: ASHA, meta: "tree" }))
    expect(String(attributes(node).tree)).toContain('class="kul-card"')
  })
})
