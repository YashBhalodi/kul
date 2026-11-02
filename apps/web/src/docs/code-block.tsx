import { cn } from "cn"
import { CheckIcon, CopyIcon } from "lucide-react"
import { useRef, useState } from "react"
import type { HTMLAttributes, ReactNode, RefObject } from "react"

/** Copies the text of the code block it sits in. */
function CopyButton({ target }: { target: RefObject<HTMLElement | null> }) {
  const [copied, setCopied] = useState(false)
  return (
    <button
      type="button"
      aria-label={copied ? "Copied" : "Copy code"}
      aria-live="polite"
      onClick={() => {
        const text = target.current?.querySelector("pre")?.textContent ?? ""
        void navigator.clipboard.writeText(text).then(() => {
          setCopied(true)
          setTimeout(() => setCopied(false), 1500)
        })
      }}
      className="inline-flex size-7 items-center justify-center rounded-md text-muted-foreground transition-colors hover:bg-accent hover:text-foreground focus-visible:ring-3 focus-visible:ring-ring/50 focus-visible:outline-none [&_svg]:size-3.5"
    >
      {copied ? (
        <CheckIcon aria-hidden="true" />
      ) : (
        <CopyIcon aria-hidden="true" />
      )}
    </button>
  )
}

export interface CodeBlockProps extends HTMLAttributes<HTMLElement> {
  /** The file name or language shown in the header. */
  title: string
  /** Buttons for the header, before the copy button. */
  actions?: ReactNode
  /** Square off the bottom corners for something attached below. */
  attached?: boolean
}

/**
 * A code block: a header with its title, actions and a copy button, and
 * the code in a region that scrolls sideways. The `shiki` class lets
 * highlighted lines take their colours from --shiki-light/--shiki-dark.
 */
export function CodeBlock({
  title,
  actions,
  attached = false,
  className,
  children,
  ...props
}: CodeBlockProps) {
  const area = useRef<HTMLDivElement>(null)
  return (
    <figure
      {...props}
      className={cn(
        "shiki my-0 overflow-hidden border bg-card text-sm",
        attached ? "rounded-t-xl" : "rounded-xl",
        className
      )}
    >
      <div className="flex h-10 items-center gap-2 border-b pr-2 pl-4 text-muted-foreground">
        <figcaption className="flex-1 truncate font-mono text-[0.8125rem]">
          {title}
        </figcaption>
        {actions}
        <CopyButton target={area} />
      </div>
      <div
        ref={area}
        role="region"
        aria-label={`${title} code`}
        tabIndex={0}
        className="overflow-x-auto py-3.5 text-[0.8125rem] focus-visible:ring-2 focus-visible:ring-ring focus-visible:outline-none focus-visible:ring-inset"
      >
        <pre className="w-max min-w-full font-mono leading-relaxed *:flex *:flex-col">
          {children}
        </pre>
      </div>
    </figure>
  )
}
