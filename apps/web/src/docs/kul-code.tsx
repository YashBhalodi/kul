import { CircleAlertIcon, PlayIcon } from "lucide-react"
import { useSyncExternalStore } from "react"
import type { ReactNode } from "react"

import { handOffSnippet } from "@/store/try-it"
import { CodeBlock } from "./code-block"
import type { TryItSnippet } from "@/store/try-it"

interface ShownDiagnostic {
  code: string
  message: string
  line?: number
}

export interface KulCodeProps {
  /** The files "Try it" opens, as JSON: this block's project. */
  files?: string
  /** What the engine says about a block that shows a mistake, as JSON. */
  diagnostics?: string
  /** A file name to show in the header. */
  title?: string
  /** The family tree the block draws (the engine's SVG), to show below it. */
  tree?: string
  /** The docs page the block is on. */
  page: string
  /** The highlighted lines (built by rehype-kul). */
  children?: ReactNode
}

const noSubscription = () => () => undefined

/** True once the page has hydrated; false in the prerendered HTML. */
function useHydrated(): boolean {
  return useSyncExternalStore(
    noSubscription,
    () => true,
    () => false
  )
}

/**
 * Opens the block's files in the app as a new project. It hands them over
 * through sessionStorage before going to /app/, so it is a button that
 * works once the page has hydrated, never a plain link that would open
 * the app without them.
 */
function TryIt({ page, files }: { page: string; files: string }) {
  const hydrated = useHydrated()
  return (
    <button
      type="button"
      data-testid="try-it"
      disabled={!hydrated}
      onClick={() => {
        handOffSnippet({
          page,
          files: JSON.parse(files) as TryItSnippet["files"],
        })
        window.location.assign("/app/")
      }}
      className="inline-flex h-7 items-center gap-1 rounded-md px-2 text-sm font-medium text-link transition-colors hover:bg-accent focus-visible:ring-3 focus-visible:ring-ring/50 focus-visible:outline-none disabled:opacity-60 [&_svg]:size-3.5"
    >
      <PlayIcon aria-hidden="true" />
      Try it
    </button>
  )
}

/** A message with its `code spans` set as code. */
function Message({ text }: { text: string }) {
  return text
    .split(/(`[^`]+`)/)
    .map((part, index) =>
      part.startsWith("`") && part.endsWith("`") && part.length > 1 ? (
        <code key={index}>{part.slice(1, -1)}</code>
      ) : (
        part
      )
    )
}

/**
 * A ```kul block: highlighted at build time from the engine's tokens. A
 * block that checks clean offers "Try it", which opens its files in the
 * app as a new project; a block that shows a mistake carries the engine's
 * own message for it.
 */
export function KulCode({
  files,
  diagnostics,
  title,
  tree,
  page,
  children,
}: KulCodeProps) {
  const shown =
    diagnostics === undefined
      ? undefined
      : (JSON.parse(diagnostics) as Array<ShownDiagnostic>)
  const codes = shown && [...new Set(shown.map((entry) => entry.code))]
  const attached = shown !== undefined || tree !== undefined
  return (
    <div className="my-4" data-kul-code="" data-expect-error={codes?.join(" ")}>
      <CodeBlock
        title={title ?? "Kul"}
        attached={attached}
        actions={files !== undefined && <TryIt page={page} files={files} />}
      >
        <code>{children}</code>
      </CodeBlock>
      {shown && (
        <ul
          aria-label="What Kul reports"
          className="flex flex-col gap-1.5 rounded-b-xl border border-t-0 bg-muted px-4 py-3 text-sm text-foreground"
        >
          {shown.map((entry, index) => (
            <li key={index} className="flex gap-2">
              <CircleAlertIcon
                aria-hidden="true"
                className="mt-0.5 size-4 shrink-0 text-destructive"
              />
              <span>
                <span className="font-mono font-medium">{entry.code}</span>
                {entry.line !== undefined && (
                  <span className="text-muted-foreground">
                    {" "}
                    · line {entry.line}
                  </span>
                )}{" "}
                <span className="whitespace-pre-line">
                  <Message text={entry.message} />
                </span>
              </span>
            </li>
          ))}
        </ul>
      )}
      {tree !== undefined && (
        <div
          role="img"
          aria-label="The family tree this code draws"
          tabIndex={0}
          className="kul-tree kul-tree-figure overflow-x-auto rounded-b-xl border border-t-0 bg-canvas p-4 focus-visible:ring-3 focus-visible:ring-ring/50 focus-visible:outline-none"
          // The engine's SVG, rendered at build time from the block above:
          // classes and data-* attributes only, coloured by the token sheet.
          dangerouslySetInnerHTML={{ __html: tree }}
        />
      )}
    </div>
  )
}
