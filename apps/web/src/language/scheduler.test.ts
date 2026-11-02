import { afterEach, beforeEach, describe, expect, it, vi } from "vitest"

import { createAnalysisScheduler } from "./scheduler"

describe("createAnalysisScheduler", () => {
  beforeEach(() => {
    vi.useFakeTimers()
  })
  afterEach(() => {
    vi.useRealTimers()
  })

  it("runs once, `wait` ms after the last of a burst of requests", () => {
    const run = vi.fn()
    const scheduler = createAnalysisScheduler(run, 200)
    scheduler.request()
    vi.advanceTimersByTime(150)
    scheduler.request()
    vi.advanceTimersByTime(150)
    scheduler.request()
    expect(run).not.toHaveBeenCalled()
    vi.advanceTimersByTime(199)
    expect(run).not.toHaveBeenCalled()
    vi.advanceTimersByTime(1)
    expect(run).toHaveBeenCalledTimes(1)
    expect(scheduler.pending).toBe(false)
  })

  it("flushes a waiting request at once, and only once", () => {
    const run = vi.fn()
    const scheduler = createAnalysisScheduler(run, 200)
    expect(scheduler.flush()).toBe(false)
    scheduler.request()
    expect(scheduler.pending).toBe(true)
    expect(scheduler.flush()).toBe(true)
    expect(run).toHaveBeenCalledTimes(1)
    vi.advanceTimersByTime(500)
    expect(run).toHaveBeenCalledTimes(1)
    expect(scheduler.flush()).toBe(false)
  })

  it("drops a cancelled request", () => {
    const run = vi.fn()
    const scheduler = createAnalysisScheduler(run, 200)
    scheduler.request()
    scheduler.cancel()
    vi.advanceTimersByTime(500)
    expect(run).not.toHaveBeenCalled()
  })

  it("takes new requests after running", () => {
    const run = vi.fn()
    const scheduler = createAnalysisScheduler(run, 50)
    scheduler.request()
    vi.advanceTimersByTime(50)
    scheduler.request()
    vi.advanceTimersByTime(50)
    expect(run).toHaveBeenCalledTimes(2)
  })
})
