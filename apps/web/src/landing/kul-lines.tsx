import { cn } from "cn"

import { TOKEN_CLASS } from "@/highlight/tokens"
import type { CodeLine } from "@/highlight/highlight-kul"

/** Text with its `code spans` set as code. */
export function Prose({ text }: { text: string }) {
  return text.split(/(`[^`]+`)/).map((part, index) =>
    part.length > 1 && part.startsWith("`") && part.endsWith("`") ? (
      <code key={index} className="landing-code">
        {part.slice(1, -1)}
      </code>
    ) : (
      part
    )
  )
}

/** A token or a callout's marker. */
export interface LineMark {
  line: number
  run: number
  number: number
}

/**
 * Kul code highlighted from the engine's tokens (built with the site).
 * Long lines wrap, indented under their start, rather than scroll; line
 * numbers, when shown, make the wrapping plain. A marked token carries its
 * callout's number in the gutter.
 */
export function KulLines({
  lines,
  numbered = false,
  marks = [],
  className,
}: {
  lines: ReadonlyArray<CodeLine>
  numbered?: boolean
  marks?: ReadonlyArray<LineMark>
  className?: string
}) {
  return (
    <pre className={cn("kul-lines", className)}>
      <code>
        {lines.map((line, index) => {
          const marked = marks.filter((mark) => mark.line === index)
          return (
            <span key={index} className="kul-line" data-line={index + 1}>
              {marks.length > 0 && (
                <span className="kul-line-mark" aria-hidden="true">
                  {marked.map((mark) => (
                    <span key={mark.number} className="kul-note-number">
                      {mark.number}
                    </span>
                  ))}
                </span>
              )}
              {numbered && (
                <span className="kul-line-number" aria-hidden="true">
                  {index + 1}
                </span>
              )}
              <span className="kul-line-text">
                {line.length === 0
                  ? " "
                  : line.map((run, runIndex) => {
                      const note = marked.find((mark) => mark.run === runIndex)
                      return run.kind === undefined ? (
                        run.text
                      ) : (
                        <span
                          key={runIndex}
                          className={cn(
                            TOKEN_CLASS[run.kind],
                            note && "kul-noted"
                          )}
                          data-note={note?.number}
                        >
                          {run.text}
                        </span>
                      )
                    })}
              </span>
            </span>
          )
        })}
      </code>
    </pre>
  )
}
