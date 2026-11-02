import { afterEach, beforeEach, describe, expect, it, vi } from "vitest"

import { createSaveQueue } from "./save-queue"
import type { SaveStatus } from "./save-queue"

describe("the save queue", () => {
  let log: Array<string>
  let statuses: Array<SaveStatus>

  beforeEach(() => {
    vi.useFakeTimers()
    log = []
    statuses = []
  })
  afterEach(() => vi.useRealTimers())

  function queue(
    writeTexts?: (texts: ReadonlyMap<string, string>) => Promise<void>
  ) {
    return createSaveQueue({
      wait: 400,
      writeTexts:
        writeTexts ??
        ((texts) => {
          log.push(`write ${[...texts].map(([n, t]) => `${n}=${t}`).join(" ")}`)
          return Promise.resolve()
        }),
      onStatus: (status) => statuses.push(status),
    })
  }

  it("writes text edits once typing pauses, latest text per file", async () => {
    const saves = queue()
    saves.change("a.kul", "1")
    await vi.advanceTimersByTimeAsync(300)
    saves.change("a.kul", "12")
    saves.change("b.kul", "x")
    await vi.advanceTimersByTimeAsync(300)
    expect(log).toEqual([])
    await vi.advanceTimersByTimeAsync(100)
    expect(log).toEqual(["write a.kul=12 b.kul=x"])
    expect(statuses.at(0)).toBe("saving")
    expect(statuses.at(-1)).toBe("saved")
  })

  it("writes waiting text before a change that runs at once", async () => {
    const saves = queue()
    saves.change("a.kul", "typed")
    const result = saves.run(() => {
      log.push("rename a.kul")
      return Promise.resolve(42)
    })
    expect(await result).toBe(42)
    expect(log).toEqual(["write a.kul=typed", "rename a.kul"])
    await vi.advanceTimersByTimeAsync(1000)
    expect(log).toHaveLength(2)
  })

  it("flushes everything now", async () => {
    const saves = queue()
    saves.change("a.kul", "1")
    await saves.flush()
    expect(log).toEqual(["write a.kul=1"])
    expect(statuses.at(-1)).toBe("saved")
  })

  it("runs one write at a time, in order", async () => {
    const saves = queue()
    let release: () => void = () => undefined
    const first = saves.run(
      () =>
        new Promise<void>((resolve) => {
          log.push("first starts")
          release = () => {
            log.push("first ends")
            resolve()
          }
        })
    )
    const second = saves.run(() => {
      log.push("second")
      return Promise.resolve()
    })
    await vi.advanceTimersByTimeAsync(0)
    expect(log).toEqual(["first starts"])
    release()
    await Promise.all([first, second])
    expect(log).toEqual(["first starts", "first ends", "second"])
  })

  it("reports a failed write and recovers on the next one", async () => {
    let fail = true
    const saves = queue(() =>
      fail ? Promise.reject(new Error("disk full")) : Promise.resolve()
    )
    saves.change("a.kul", "1")
    await saves.flush()
    expect(statuses.at(-1)).toBe("failed")
    fail = false
    saves.change("a.kul", "2")
    await saves.flush()
    expect(statuses.at(-1)).toBe("saved")
  })

  it("drops what is waiting on discard", async () => {
    const saves = queue()
    saves.change("a.kul", "1")
    saves.discard()
    await vi.advanceTimersByTimeAsync(1000)
    expect(log).toEqual([])
  })
})
