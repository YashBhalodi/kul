// The docs as Markdown for language models: llms.txt (https://llmstxt.org),
// llms-full.txt and each page's /docs/<slug>.md variant. Pages appear in
// the sidebar's order, and links between pages point at Markdown variants.

import type * as PageTree from "fumadocs-core/page-tree"

import { REPO_URL, SITE_ORIGIN } from "@/site"
import { markdownUrl, source } from "./source"
import type { DocsPage } from "./source"

export const SUMMARY =
  "Kul is a small plain-text language for describing families: people, marriages, births and adoptions, with dates as exact or as vague as the records are. The Kul app checks the text as you type and draws the family tree."

/** Every docs page in sidebar order. */
export function pagesInOrder(): Array<DocsPage> {
  const pages: Array<DocsPage> = []
  const walk = (nodes: ReadonlyArray<PageTree.Node>) => {
    for (const node of nodes) {
      if (node.type === "page") {
        const page = source.getNodePage(node)
        if (page) pages.push(page)
      } else if (node.type === "folder") {
        if (node.index) {
          const page = source.getNodePage(node.index)
          if (page) pages.push(page)
        }
        walk(node.children)
      }
    }
  }
  walk(source.getPageTree().children)
  return pages
}

/** `/docs/a/b/#x` → the absolute URL of page a/b's Markdown variant. */
export function toMarkdownLinks(markdown: string): string {
  return markdown.replace(
    /\]\(\/docs\/([a-z0-9/-]*?)\/?(?:#[^)\s]*)?\)/g,
    (_match, path: string) =>
      `](${SITE_ORIGIN}${markdownUrl(path === "" ? [] : path.split("/"))})`
  )
}

/** One page as Markdown: its title, summary and processed body. */
export async function pageMarkdown(page: DocsPage): Promise<string> {
  const body = await page.data.getText("processed")
  return [
    `# ${page.data.title}`,
    `> ${page.data.description ?? ""}`.trimEnd(),
    toMarkdownLinks(body.trim()),
  ]
    .join("\n\n")
    .concat("\n")
}

function link(page: DocsPage): string {
  const description = page.data.description ? `: ${page.data.description}` : ""
  return `- [${page.data.title}](${SITE_ORIGIN}${markdownUrl(page.slugs)})${description}`
}

/** llms.txt: a summary, then the docs by section, then optional extras. */
export function llmsTxt(): string {
  const sections: Array<string> = [`# Kul`, `> ${SUMMARY}`]
  const loose: Array<string> = []
  const folders: Array<string> = []
  for (const node of source.getPageTree().children) {
    if (node.type === "page") {
      const page = source.getNodePage(node)
      if (page) loose.push(link(page))
    } else if (node.type === "folder") {
      const pages: Array<string> = []
      const collect = (nodes: ReadonlyArray<PageTree.Node>) => {
        for (const child of nodes) {
          if (child.type === "page") {
            const page = source.getNodePage(child)
            if (page) pages.push(link(page))
          } else if (child.type === "folder") collect(child.children)
        }
      }
      if (node.index) {
        const page = source.getNodePage(node.index)
        if (page) pages.push(link(page))
      }
      collect(node.children)
      const title = typeof node.name === "string" ? node.name : "Docs"
      folders.push(`## ${title}\n\n${pages.join("\n")}`)
    }
  }
  sections.push(`## Docs\n\n${loose.join("\n")}`, ...folders)
  sections.push(
    [
      "## All in one",
      "",
      `- [Every docs page in one file](${SITE_ORIGIN}/llms-full.txt): the pages above, in order, as one Markdown file.`,
      `- [Kul authoring skill](${REPO_URL}/tree/main/skills/kul-authoring): an agent skill for turning family narratives into Kul.`,
    ].join("\n"),
    [
      "## Optional",
      "",
      `- [Language specification](${REPO_URL}/tree/main/spec): the normative definition of Kul, for implementers.`,
    ].join("\n")
  )
  return sections.join("\n\n").concat("\n")
}

/** llms-full.txt: every page's Markdown, in sidebar order. */
export async function llmsFullTxt(): Promise<string> {
  const pages = await Promise.all(pagesInOrder().map(pageMarkdown))
  return pages.join("\n")
}
