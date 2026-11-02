// The docs pages: one source for the prerendered HTML, the Markdown
// variants (/docs/<slug>.md), llms.txt, llms-full.txt and the search index.
//
// Inline text is a small Markdown subset: `code` and [label](href).

export type Block =
  | { kind: "paragraph"; text: string }
  | { kind: "heading"; text: string }
  | { kind: "list"; items: Array<string> }
  | { kind: "code"; language: "kul" | "sh"; code: string }

export interface DocsPage {
  /** "" for the docs home, otherwise the path segment under /docs/. */
  slug: string
  title: string
  description: string
  blocks: Array<Block>
}

const FAMILY = `person ramesh  name:"Ramesh Sharma"  gender:male    born:1925-03-10  died:2005-08-22
person sita    name:"Sita Sharma"    gender:female  born:~1928
person alice   name:"Alice Sharma"   gender:female  born:1950-04
  birth m_ramesh_sita

marriage m_ramesh_sita ramesh sita  start:1948-06-10`

/** The docs home, /docs/. */
export const docsHome: DocsPage = {
  slug: "",
  title: "Introduction",
  description:
    "Kul is a small plain-text language for describing families. You write who people are and how they are related, and Kul draws the family tree.",
  blocks: [
    {
      kind: "paragraph",
      text: "Kul is a small plain-text language for describing families. You write down the people, the marriages between them and the children of each marriage, and Kul works out the structure and draws the tree.",
    },
    {
      kind: "paragraph",
      text: "Three statements carry everything: `person`, `marriage`, and a `birth` or `adoption` line under a child. Parents, siblings, cousins and in-laws are never written down; they follow from those links.",
    },
    { kind: "code", language: "kul", code: FAMILY },
    {
      kind: "paragraph",
      text: "Alice's parents come from her `birth m_ramesh_sita` line and the two spouses of that marriage. Sita's birth year is only known approximately, so it is written `~1928`; Alice's birth is known to the month.",
    },
    { kind: "heading", text: "Why plain text" },
    {
      kind: "list",
      items: [
        "The text is the record. It reads like notes, it can be versioned and compared line by line, and it stays yours.",
        "Real records are uneven. A date can be a full day, a month, a year or an approximate year, so you never invent precision you don't have.",
        "Adding what you learn later never means rewriting what you already wrote: a new child, a second marriage or an adoption is one more line.",
      ],
    },
    { kind: "heading", text: "Next" },
    {
      kind: "paragraph",
      text: "The [syntax cheat sheet](/docs/cheat-sheet/) lists every statement and field. The full normative definition is the [language specification](https://github.com/YashBhalodi/kul/tree/main/spec) on GitHub.",
    },
  ],
}

const cheatSheet: DocsPage = {
  slug: "cheat-sheet",
  title: "Syntax cheat sheet",
  description:
    "Every Kul statement and field on one page: persons, marriages, births, adoptions and dates.",
  blocks: [
    {
      kind: "paragraph",
      text: "A Kul project is one or more `.kul` files in one folder. Each line starting with `#` is a comment.",
    },
    { kind: "heading", text: "Person" },
    {
      kind: "code",
      language: "kul",
      code: `person asha  name:"Asha Rao"  gender:female  family:"Rao"  given:"Asha"  born:1961-02  died:~2019`,
    },
    {
      kind: "list",
      items: [
        "`name` and `gender` (`male`, `female` or `other`) are required.",
        "`family`, `given`, `born` and `died` are optional. Leave out what you don't know; there is no unknown value.",
      ],
    },
    { kind: "heading", text: "Marriage" },
    {
      kind: "code",
      language: "kul",
      code: `marriage m_ravi_asha ravi asha  start:1984-05-20  end:1999  end_reason:divorce`,
    },
    {
      kind: "list",
      items: [
        "The two spouses come right after the marriage id. The first one is the host, whose family the marriage is drawn in.",
        "`start` is optional. `end` always comes with `end_reason`, and `divorce` is the only reason. A death does not end a marriage.",
      ],
    },
    { kind: "heading", text: "Children" },
    {
      kind: "code",
      language: "kul",
      code: `person kiran  name:"Kiran Rao"  gender:other  born:1987
  birth m_ravi_asha
person meera  name:"Meera Rao"  gender:female  born:1990
  adoption m_ravi_asha start:1992`,
    },
    {
      kind: "list",
      items: [
        "A `birth` line under a person names the marriage they were born into.",
        "An `adoption` line names the adopting marriage and needs a `start` date. A child can have both.",
      ],
    },
    { kind: "heading", text: "Dates" },
    {
      kind: "list",
      items: [
        "`1961-02-14`: a full date.",
        "`1961-02`: a month.",
        "`1961`: a year.",
        "`~1961`: an approximate date, in any of the forms above.",
      ],
    },
  ],
}

export const docsPages: ReadonlyArray<DocsPage> = [docsHome, cheatSheet]

/** The page's HTML path: /docs/ or /docs/<slug>/. */
export function htmlPath(page: DocsPage): string {
  return page.slug === "" ? "/docs/" : `/docs/${page.slug}/`
}

/** The page's Markdown variant: /docs/index.md or /docs/<slug>.md. */
export function markdownPath(page: DocsPage): string {
  return `/docs/${page.slug === "" ? "index" : page.slug}.md`
}

/** The page for a Markdown-variant name ("index" or a slug). */
export function pageForMarkdownName(name: string): DocsPage | undefined {
  const slug = name === "index" ? "" : name
  return docsPages.find((page) => page.slug === slug)
}

/** The page for a slug under /docs/, if any. */
export function pageForSlug(slug: string): DocsPage | undefined {
  return docsPages.find((page) => page.slug === slug)
}

/** The page as Markdown, with an H1 title and a summary blockquote. */
export function pageMarkdown(page: DocsPage): string {
  const parts = [`# ${page.title}`, `> ${page.description}`]
  for (const block of page.blocks) {
    switch (block.kind) {
      case "paragraph":
        parts.push(block.text)
        break
      case "heading":
        parts.push(`## ${block.text}`)
        break
      case "list":
        parts.push(block.items.map((item) => `- ${item}`).join("\n"))
        break
      case "code":
        parts.push("```" + block.language + "\n" + block.code + "\n```")
        break
    }
  }
  return parts.join("\n\n") + "\n"
}

/** The page's text without Markdown syntax, for the search index. */
export function pagePlainText(page: DocsPage): string {
  return page.blocks
    .map((block) => {
      switch (block.kind) {
        case "paragraph":
        case "heading":
          return stripInline(block.text)
        case "list":
          return block.items.map(stripInline).join(" ")
        case "code":
          return block.code
      }
    })
    .join("\n")
}

/** A run of inline text: plain text, `code` or a [label](href) link. */
export type Inline =
  | { kind: "text"; text: string }
  | { kind: "code"; text: string }
  | { kind: "link"; text: string; href: string }

const INLINE = /`([^`]+)`|\[([^\]]+)\]\(([^)\s]+)\)/g

/** Split inline text into plain, code and link runs. */
export function parseInline(text: string): Array<Inline> {
  const runs: Array<Inline> = []
  let last = 0
  for (const match of text.matchAll(INLINE)) {
    if (match.index > last) {
      runs.push({ kind: "text", text: text.slice(last, match.index) })
    }
    const [, code, label, href] = match
    if (code !== undefined) runs.push({ kind: "code", text: code })
    else if (label !== undefined && href !== undefined) {
      runs.push({ kind: "link", text: label, href })
    }
    last = match.index + match[0].length
  }
  if (last < text.length) runs.push({ kind: "text", text: text.slice(last) })
  return runs
}

function stripInline(text: string): string {
  return parseInline(text)
    .map((run) => run.text)
    .join("")
}
