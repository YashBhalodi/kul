// Highlighting from the engine's tokens: no grammar lives in the app. The
// lexer's tokens arrive on every edit; the semantic tokens refine them
// after each analysis. A token answer is for one version of the text, so
// edits made since are mapped over it until the next answer arrives.

import { ChangeSet } from "@codemirror/state"
import type { ChangeDesc } from "@codemirror/state"

import { TOKEN_CLASS } from "@/highlight/tokens"
import type { Token } from "@/language/protocol"

/** A class to paint over `[from, to)`. */
export interface TokenRange {
  from: number
  to: number
  className: string
}

/**
 * Token ranges for a text, mapped through `changes` made to it since.
 * A token that an edit deleted is dropped; one an edit typed into grows.
 */
export function tokenRanges(
  tokens: ReadonlyArray<Token>,
  length: number,
  changes?: ChangeDesc
): Array<TokenRange> {
  const ranges: Array<TokenRange> = []
  for (const token of tokens) {
    let from = Math.min(token.span.start, length)
    let to = Math.min(token.span.end, length)
    if (changes) {
      from = changes.mapPos(from, 1)
      to = changes.mapPos(to, -1)
    }
    if (to > from) {
      ranges.push({ from, to, className: TOKEN_CLASS[token.kind] })
    }
  }
  return ranges.sort((a, b) => a.from - b.from || a.to - b.to)
}

/**
 * The edits made to one file's text since each version was sent, so an
 * answer for an older version can be mapped onto the text as it is now.
 */
export class EditTrail {
  #since: Array<{ version: number; changes: ChangeDesc }> = []

  /**
   * Record that the text, now `length` long after `changes`, was sent as
   * `version`.
   */
  sent(version: number, length: number, changes?: ChangeDesc): void {
    if (changes) {
      for (const entry of this.#since) {
        entry.changes = entry.changes.composeDesc(changes)
      }
    }
    this.#since.push({ version, changes: ChangeSet.empty(length).desc })
  }

  /**
   * The edits made since `version` was sent, or undefined when that version
   * is unknown (too old, or never sent from here).
   */
  since(version: number): ChangeDesc | undefined {
    return this.#since.find((entry) => entry.version === version)?.changes
  }

  /** Forget versions older than `version`. */
  forgetBefore(version: number): void {
    this.#since = this.#since.filter((entry) => entry.version >= version)
  }

  /** The latest version sent. */
  get latest(): number | undefined {
    return this.#since.at(-1)?.version
  }
}
