// The worker's analysis scheduler: a trailing debounce that coalesces.
// Each edit asks for an analysis; a request replaces the one still
// waiting, so a burst of edits costs one analysis after the last of them.
// A feature that needs the analysis now flushes the waiting request.

export interface Timers {
  set: (callback: () => void, ms: number) => unknown
  clear: (handle: unknown) => void
}

export interface AnalysisScheduler {
  /** Ask for an analysis after `wait` ms, replacing any still waiting. */
  request: () => void
  /** Run the waiting analysis now. Returns whether one was waiting. */
  flush: () => boolean
  /** Drop the waiting analysis. */
  cancel: () => void
  /** Whether an analysis is waiting. */
  readonly pending: boolean
}

export const systemTimers: Timers = {
  set: (callback, ms) => setTimeout(callback, ms),
  clear: (handle) => clearTimeout(handle as ReturnType<typeof setTimeout>),
}

export function createAnalysisScheduler(
  run: () => void,
  wait: number,
  timers: Timers = systemTimers
): AnalysisScheduler {
  let handle: unknown
  let pending = false

  function fire() {
    pending = false
    handle = undefined
    run()
  }

  return {
    request() {
      if (pending) timers.clear(handle)
      pending = true
      handle = timers.set(fire, wait)
    },
    flush() {
      if (!pending) return false
      timers.clear(handle)
      fire()
      return true
    },
    cancel() {
      if (!pending) return
      timers.clear(handle)
      pending = false
      handle = undefined
    },
    get pending() {
      return pending
    },
  }
}
