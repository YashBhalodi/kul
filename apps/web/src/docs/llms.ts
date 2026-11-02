// llms.txt (https://llmstxt.org) and llms-full.txt, built from the docs pages.

import { SITE_ORIGIN, REPO_URL } from "@/site"
import { docsPages, markdownPath, pageMarkdown } from "./pages"

const SUMMARY =
  "Kul is a small plain-text language for describing families: persons, marriages, births and adoptions, with partial and approximate dates. Kul checks the text and draws the family tree."

export function llmsTxt(): string {
  const pages = docsPages.map(
    (page) =>
      `- [${page.title}](${SITE_ORIGIN}${markdownPath(page)}): ${page.description}`
  )
  return [
    "# Kul",
    `> ${SUMMARY}`,
    "## Docs",
    [
      ...pages,
      `- [All docs in one file](${SITE_ORIGIN}/llms-full.txt): every docs page as Markdown.`,
      `- [Authoring skill](${REPO_URL}/tree/main/skills/kul-authoring): how to turn a family narrative into Kul.`,
    ].join("\n"),
    "## Optional",
    `- [Language specification](${REPO_URL}/tree/main/spec): the normative definition of Kul.`,
  ]
    .join("\n\n")
    .concat("\n")
}

export function llmsFullTxt(): string {
  return docsPages.map(pageMarkdown).join("\n")
}
