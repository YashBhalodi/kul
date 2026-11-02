import type { MDXComponents } from "mdx/types"
import { isValidElement } from "react"
import type { HTMLAttributes, TableHTMLAttributes } from "react"

import { CodeBlock } from "./code-block"
import { DocsLink } from "./docs-link"
import { KulCode } from "./kul-code"
import type { KulCodeProps } from "./kul-code"

/** A table that scrolls sideways on a narrow screen. */
function Table(props: TableHTMLAttributes<HTMLTableElement>) {
  return (
    <div className="relative my-6 overflow-x-auto" tabIndex={0}>
      <table {...props} />
    </div>
  )
}

const LANGUAGE_TITLES: Readonly<Record<string, string>> = {
  sh: "Shell",
  text: "Syntax",
}

/**
 * A highlighted block in another language (shell): the highlighter's
 * <pre> becomes a code block, keeping its classes so its colours apply.
 */
function Pre({
  className,
  children,
  ...props
}: HTMLAttributes<HTMLPreElement> & { title?: string }) {
  const codeClass = isValidElement<{ className?: string }>(children)
    ? children.props.className
    : undefined
  const language = /language-(\w+)/.exec(codeClass ?? "")?.[1]
  return (
    <div className="my-4">
      <CodeBlock
        title={props.title ?? LANGUAGE_TITLES[language ?? ""] ?? "Code"}
        className={className}
      >
        {children}
      </CodeBlock>
    </div>
  )
}

type HeadingProps = HTMLAttributes<HTMLHeadingElement>

/**
 * The components a docs page's MDX renders with: only what the docs use,
 * so a page loads nothing it doesn't show.
 */
export function docsMdxComponents(page: string): MDXComponents {
  return {
    a: DocsLink,
    pre: Pre,
    h2: (props: HeadingProps) => <h2 className="scroll-mt-20" {...props} />,
    h3: (props: HeadingProps) => <h3 className="scroll-mt-20" {...props} />,
    table: Table,
    KulCode: (props: Omit<KulCodeProps, "page">) => (
      <KulCode {...props} page={page} />
    ),
  }
}
