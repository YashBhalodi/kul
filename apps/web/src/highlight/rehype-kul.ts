// A rehype plugin for ```kul blocks: highlights each from the engine's
// tokens and checks it, at build time, so no Kul code on the site can be
// wrong without the build failing.
//
// - A block is checked as a project of one file, and must check clean.
// - A block fenced ```kul expect-error=KUL-R03 must report that code and
//   no other: it shows a mistake on purpose, and the engine's message is
//   shown with it. A mistake that always brings a second one lists both:
//   expect-error=KUL-L01,KUL-P07.
// - Blocks that share `project="<name>"` on one page are the files of one
//   project, each named by its `title="<file>.kul"`, and are checked
//   together, so a page can show references across files.
// - A block with `tree` in its meta also shows the family tree it draws.
//
// Each block becomes a <KulCode> element (the MDX components supply it)
// with its highlighted lines as children. A block that checks clean also
// carries its project's files, so the page can open them in the app. The plugin runs before the default
// code highlighter, which then never sees a Kul block.

import type { Element, ElementContent, Root } from "hast"
import type { MdxJsxAttribute, MdxJsxFlowElementHast } from "mdast-util-mdx-jsx"
import type { VFile } from "vfile"

import { analyzeKul, expectedErrors, snippetProblem } from "./highlight-kul.ts"
import type { CodeLine, KulFile } from "./highlight-kul.ts"
import { TOKEN_CLASS } from "./tokens.ts"

/** The name of the MDX component a Kul block becomes. */
export const KUL_CODE_COMPONENT = "KulCode"

/** The file a lone block is checked (and opened in the app) as. */
export const SNIPPET_FILE = "family.kul"

interface KulBlock {
  node: Element
  source: string
  meta: string
  title: string | undefined
  project: string | undefined
  expected: Array<string> | undefined
  tree: boolean
}

function textOf(node: ElementContent): string {
  if (node.type === "text") return node.value
  if (node.type === "element") return node.children.map(textOf).join("")
  return ""
}

function kulCode(node: Element): Element | undefined {
  if (node.tagName !== "pre" || node.children.length !== 1) return undefined
  const code = node.children[0]
  if (code?.type !== "element" || code.tagName !== "code") return undefined
  const classes = code.properties.className
  return Array.isArray(classes) && classes.includes("language-kul")
    ? code
    : undefined
}

function metaValue(meta: string, key: string): string | undefined {
  return new RegExp(`(?:^|\\s)${key}="([^"]+)"`).exec(meta)?.[1]
}

/** Highlighted lines as hast: one `span.line` per line, tokens inside. */
export function linesToHast(
  lines: ReadonlyArray<CodeLine>
): Array<ElementContent> {
  const out: Array<ElementContent> = []
  lines.forEach((line, index) => {
    if (index > 0) out.push({ type: "text", value: "\n" })
    out.push({
      type: "element",
      tagName: "span",
      properties: { className: ["line"] },
      children: line.map((run): ElementContent =>
        run.kind === undefined
          ? { type: "text", value: run.text }
          : {
              type: "element",
              tagName: "span",
              properties: { className: [TOKEN_CLASS[run.kind]] },
              children: [{ type: "text", value: run.text }],
            }
      ),
    })
  })
  return out
}

function attribute(name: string, value: string): MdxJsxAttribute {
  return { type: "mdxJsxAttribute", name, value }
}

function collect(parent: Root | Element, blocks: Array<KulBlock>): void {
  for (const child of parent.children) {
    if (child.type !== "element") continue
    const code = kulCode(child)
    if (!code) {
      collect(child, blocks)
      continue
    }
    // MDX keeps the fence's info string after the language as `data.meta`.
    const data: { meta?: unknown } | undefined = code.data
    const meta = typeof data?.meta === "string" ? data.meta : ""
    blocks.push({
      node: child,
      source: textOf(code).replace(/\n$/, ""),
      meta,
      title: metaValue(meta, "title"),
      project: metaValue(meta, "project"),
      expected: expectedErrors(meta),
      tree: /(?:^|\s)tree(?:\s|$)/.test(meta),
    })
  }
}

function replace(
  parent: Root | Element,
  replacements: Map<Element, MdxJsxFlowElementHast>
): void {
  parent.children = parent.children.map((child) => {
    if (child.type !== "element") return child
    const replacement = replacements.get(child)
    if (replacement) return replacement
    replace(child, replacements)
    return child
  })
}

/** The plugin: `rehypePlugins: (plugins) => [rehypeKul, ...plugins]`. */
export function rehypeKul() {
  return (tree: Root, file: VFile) => {
    const blocks: Array<KulBlock> = []
    collect(tree, blocks)

    // Lone blocks are projects of one; `project=` blocks are grouped.
    const groups = new Map<string, Array<KulBlock>>()
    blocks.forEach((block, index) => {
      const key = block.project ?? `\u0000${String(index)}`
      groups.set(key, [...(groups.get(key) ?? []), block])
    })

    const replacements = new Map<Element, MdxJsxFlowElementHast>()
    for (const group of groups.values()) {
      const grouped = group[0]?.project !== undefined
      const files: Array<KulFile> = group.map((block) => ({
        name: block.title ?? SNIPPET_FILE,
        text: `${block.source}\n`,
      }))
      if (grouped) {
        for (const block of group) {
          if (block.title === undefined || !block.title.endsWith(".kul")) {
            file.fail(
              `Kul code block in project "${String(block.project)}" needs title="<name>.kul"`,
              block.node
            )
          }
          if (block.expected !== undefined) {
            file.fail(
              "a Kul code block in a project can't expect an error",
              block.node
            )
          }
        }
        const names = files.map((entry) => entry.name)
        if (new Set(names).size !== names.length) {
          file.fail(
            `Kul project "${String(group[0]?.project)}" names a file twice`,
            group[0]?.node
          )
        }
      }
      const { lines, diagnostics, svg } = analyzeKul(files)
      const expected = grouped ? undefined : group[0]?.expected
      const problem = snippetProblem(diagnostics, expected)
      if (problem !== undefined) {
        const source = group.map((block) => block.source).join("\n\n")
        file.fail(`Kul code block: ${problem}\n\n${source}`, group[0]?.node)
      }
      const filesJson = JSON.stringify(files)
      group.forEach((block, index) => {
        const name = files[index]?.name ?? SNIPPET_FILE
        const attributes: Array<MdxJsxAttribute> = []
        if (expected === undefined) {
          attributes.push(attribute("files", filesJson))
        } else {
          // What the engine says about the mistake, shown under the code.
          const messages = diagnostics.map(({ code, message, line }) => ({
            code,
            message,
            line,
          }))
          attributes.push(attribute("diagnostics", JSON.stringify(messages)))
        }
        if (block.title !== undefined) {
          attributes.push(attribute("title", block.title))
        }
        if (block.tree) {
          if (svg === undefined) {
            file.fail("a Kul code block with `tree` draws no tree", block.node)
          } else {
            attributes.push(attribute("tree", svg))
          }
        }
        replacements.set(block.node, {
          type: "mdxJsxFlowElement",
          name: KUL_CODE_COMPONENT,
          attributes,
          children: linesToHast(lines.get(name) ?? []),
          position: block.node.position,
        })
      })
    }
    replace(tree, replacements)
  }
}
