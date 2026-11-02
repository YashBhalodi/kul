import type { ReactNode } from "react"

import { parseInline } from "./pages"
import type { Block, DocsPage } from "./pages"

/** A docs page as an article: its title, summary and blocks. */
export function DocsArticle({ page }: { page: DocsPage }) {
  return (
    <article className="flex min-w-0 flex-col gap-5">
      <header className="flex flex-col gap-3 pb-2">
        <h1 className="text-3xl font-semibold tracking-tight sm:text-4xl">
          {page.title}
        </h1>
        <p className="text-lg leading-relaxed text-muted-foreground">
          {page.description}
        </p>
      </header>
      {page.blocks.map((block, index) => (
        <DocsBlock key={index} block={block} />
      ))}
    </article>
  )
}

function DocsBlock({ block }: { block: Block }) {
  switch (block.kind) {
    case "paragraph":
      return (
        <p className="text-base leading-7">
          <InlineText text={block.text} />
        </p>
      )
    case "heading":
      return (
        <h2 className="pt-4 text-xl font-semibold tracking-tight">
          {block.text}
        </h2>
      )
    case "list":
      return (
        <ul className="flex list-disc flex-col gap-2 pl-6 text-base leading-7 marker:text-muted-foreground">
          {block.items.map((item) => (
            <li key={item}>
              <InlineText text={item} />
            </li>
          ))}
        </ul>
      )
    case "code":
      return (
        <pre
          tabIndex={0}
          className="overflow-x-auto rounded-lg border bg-code p-4 text-sm leading-relaxed text-code-foreground focus-visible:ring-3 focus-visible:ring-ring/50 focus-visible:outline-none"
        >
          <code data-language={block.language}>{block.code}</code>
        </pre>
      )
  }
}

function InlineText({ text }: { text: string }): ReactNode {
  return parseInline(text).map((run, index) => {
    switch (run.kind) {
      case "text":
        return run.text
      case "code":
        return (
          <code
            key={index}
            className="rounded bg-muted px-1.5 py-0.5 text-[0.9em] text-foreground"
          >
            {run.text}
          </code>
        )
      case "link":
        return (
          <a
            key={index}
            href={run.href}
            className="font-medium text-link underline underline-offset-4 hover:no-underline"
          >
            {run.text}
          </a>
        )
    }
  })
}
