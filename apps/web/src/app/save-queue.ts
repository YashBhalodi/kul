// Autosave. Text edits wait for a pause in typing (about 400 ms) and are
// then written together; every other change (a new file, a rename, an
// import) is written at once, after any text still waiting. Writes run one
// at a time in the order they were asked for, and the queue reports
// whether anything is still unsaved.

import { Debouncer } from "@tanstack/pacer"

export type SaveStatus = "idle" | "saving" | "saved" | "failed"

export const AUTOSAVE_WAIT_MS = 400

export interface SaveQueue {
  /** Record the latest text of a file; it is written after a pause. */
  change: (name: string, text: string) => void
  /** Write a change now, after the text edits waiting before it. */
  run: <T>(operation: () => Promise<T>) => Promise<T>
  /** Write everything waiting, now. */
  flush: () => Promise<void>
  /** Drop everything not yet written. */
  discard: () => void
}

export interface SaveQueueOptions {
  writeTexts: (texts: ReadonlyMap<string, string>) => Promise<void>
  onStatus: (status: SaveStatus) => void
  wait?: number
}

export function createSaveQueue({
  writeTexts,
  onStatus,
  wait = AUTOSAVE_WAIT_MS,
}: SaveQueueOptions): SaveQueue {
  let texts = new Map<string, string>()
  let tail: Promise<unknown> = Promise.resolve()
  let running = 0
  let failed = false

  function settle() {
    if (running === 0 && texts.size === 0) onStatus(failed ? "failed" : "saved")
  }

  function enqueue<T>(operation: () => Promise<T>): Promise<T> {
    running++
    onStatus("saving")
    const result = tail.then(operation)
    tail = result.then(
      () => {
        running--
        failed = false
        settle()
      },
      () => {
        running--
        failed = true
        settle()
      }
    )
    return result
  }

  function writePending(): Promise<void> {
    if (texts.size === 0) return Promise.resolve()
    const batch = texts
    texts = new Map()
    return enqueue(() => writeTexts(batch))
  }

  const debouncer = new Debouncer(() => void writePending().catch(() => {}), {
    wait,
  })

  return {
    change(name, text) {
      texts.set(name, text)
      onStatus("saving")
      debouncer.maybeExecute()
    },
    run(operation) {
      debouncer.cancel()
      void writePending().catch(() => {})
      return enqueue(operation)
    },
    async flush() {
      debouncer.cancel()
      await writePending().catch(() => {})
      await tail
    },
    discard() {
      debouncer.cancel()
      texts = new Map()
      settle()
    },
  }
}
