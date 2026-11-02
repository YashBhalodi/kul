import { describe, expect, it, vi } from "vitest"

import { createLanguageClient } from "./client"
import type { ClientStatus, WorkerApi, WorkerHandle } from "./client"
import type { AnalysisReport, VersionedFile } from "./protocol"

/**
 * An in-memory stand-in for the worker: it keeps files and versions the
 * way the real one does, answers hover with the file's text, and lets a
 * test push an analysis, fail the worker, or make a call trap.
 */
class FakeWorker {
  files = new Map<string, VersionedFile>()
  listener: ((report: AnalysisReport) => void) | undefined
  failure: ((reason: string) => void) | undefined
  terminated = false
  trapNext = false
  resets: Array<Array<string>> = []
  /** Calls that never answer until released. */
  held: Array<() => void> = []
  hold = false

  handle(): WorkerHandle {
    const versions = () =>
      Object.fromEntries(
        [...this.files.values()].map((file) => [file.name, file.version])
      )
    const gate = async () => {
      if (this.hold) {
        await new Promise<void>((resolve) => this.held.push(resolve))
      }
      if (this.trapNext) {
        this.trapNext = false
        const trap = new Error("unreachable")
        trap.name = "RuntimeError"
        throw trap
      }
    }
    const api = {
      start: () => Promise.resolve({ core: "0.0.0", language: "0.1" }),
      connect: (listener: (report: AnalysisReport) => void) => {
        this.listener = listener
        return Promise.resolve()
      },
      reset: (files: ReadonlyArray<VersionedFile>) => {
        this.files = new Map(files.map((file) => [file.name, file]))
        this.resets.push(files.map((file) => `${file.name}:${file.text}`))
        return Promise.resolve()
      },
      setFile: (name: string, text: string, version: number) => {
        this.files.set(name, { name, text, version })
        return Promise.resolve({ file: name, version, tokens: [] })
      },
      deleteFile: (name: string) => {
        this.files.delete(name)
        return Promise.resolve()
      },
      lexicalTokens: (name: string) => {
        const file = this.files.get(name)
        return Promise.resolve(
          file && { file: name, version: file.version, tokens: [] }
        )
      },
      hover: async (file: string) => {
        // The answer is computed from the files as they are now; a held
        // call delivers it late, as a busy worker would.
        const answer = {
          versions: versions(),
          value: {
            markdown: this.files.get(file)?.text ?? "",
            span: { start: 0, end: 0 },
          },
        }
        await gate()
        return answer
      },
      crash: () => {
        this.fail("crashed")
        return Promise.resolve()
      },
    }
    return {
      api: api as unknown as WorkerApi,
      terminate: () => {
        this.terminated = true
      },
      onFailure: (listener) => {
        this.failure = listener
      },
    }
  }

  publish() {
    this.listener?.({
      files: [...this.files.values()].map(({ name, version }) => ({
        name,
        version,
      })),
      diagnostics: [],
      semantic: {},
    })
  }

  fail(reason: string) {
    this.failure?.(reason)
  }
}

function setup(options: { maxCrashes?: number } = {}) {
  const workers: Array<FakeWorker> = []
  const reports: Array<AnalysisReport> = []
  const statuses: Array<ClientStatus> = []
  const onRestart = vi.fn()
  const client = createLanguageClient({
    spawn: () => {
      const worker = new FakeWorker()
      workers.push(worker)
      return Promise.resolve(worker.handle())
    },
    onAnalysis: (report) => reports.push(report),
    onStatus: (status) => statuses.push(status),
    onRestart,
    ...options,
  })
  const latest = () => {
    const worker = workers.at(-1)
    if (!worker) throw new Error("no worker")
    return worker
  }
  return { client, workers, latest, reports, statuses, onRestart }
}

const settle = () => new Promise((resolve) => setTimeout(resolve, 0))

describe("createLanguageClient", () => {
  it("starts a worker, gives it the files and reports ready", async () => {
    const { client, latest, statuses } = setup()
    client.replaceFiles([{ name: "a.kul", text: "A" }])
    expect(await client.ready).toEqual({ core: "0.0.0", language: "0.1" })
    await settle()
    expect(latest().files.get("a.kul")?.text).toBe("A")
    expect(statuses.at(-1)).toMatchObject({ state: "ready", restarts: 0 })
  })

  it("tags each edit with a new version and sends the whole text", async () => {
    const { client, latest } = setup()
    await client.ready
    const first = client.setFile("a.kul", "one")
    const second = client.setFile("a.kul", "two")
    expect(second.version).toBeGreaterThan(first.version)
    expect(await second.tokens).toEqual({
      file: "a.kul",
      version: second.version,
      tokens: [],
    })
    expect(latest().files.get("a.kul")).toEqual({
      name: "a.kul",
      text: "two",
      version: second.version,
    })
  })

  it("hands on an analysis only while it is current", async () => {
    const { client, latest, reports } = setup()
    await client.ready
    client.setFile("a.kul", "one")
    await settle()
    latest().publish()
    expect(reports).toHaveLength(1)

    // An edit the worker has not seen yet makes the analysis stale.
    const worker = latest()
    const report = {
      files: [...worker.files.values()].map(({ name, version }) => ({
        name,
        version,
      })),
      diagnostics: [],
      semantic: {},
    }
    client.setFile("a.kul", "two")
    worker.listener?.(report)
    expect(reports).toHaveLength(1)
  })

  it("drops an answer an edit overtook", async () => {
    const { client, latest } = setup()
    await client.ready
    client.setFile("a.kul", "one")
    await settle()
    latest().hold = true
    const hover = client.query("hover", "a.kul", 0)
    await settle()
    client.setFile("a.kul", "two")
    latest().hold = false
    // The held answer was computed from "one", which the edit replaced.
    for (const release of latest().held) release()
    expect(await hover).toBeUndefined()
    expect(await client.query("hover", "a.kul", 0)).toMatchObject({
      markdown: "two",
    })
  })

  it("restarts a failed worker and replays the file set", async () => {
    const { client, workers, latest, onRestart, statuses } = setup()
    client.replaceFiles([
      { name: "a.kul", text: "A" },
      { name: "b.kul", text: "B" },
    ])
    await client.ready
    client.setFile("b.kul", "B2")
    await settle()
    const first = latest()
    first.hold = true
    const waiting = client.query("hover", "a.kul", 0)
    await settle()

    first.fail("boom")
    // The call waiting on the dead worker settles.
    expect(await waiting).toBeUndefined()
    expect(first.terminated).toBe(true)
    await settle()
    expect(workers).toHaveLength(2)
    expect(onRestart).toHaveBeenCalledTimes(1)
    expect(latest().resets.at(-1)).toEqual(["a.kul:A", "b.kul:B2"])
    expect(statuses.at(-1)).toMatchObject({ state: "ready", restarts: 1 })
    expect(await client.query("hover", "b.kul", 0)).toMatchObject({
      markdown: "B2",
    })
  })

  it("keeps edits made while the new worker starts", async () => {
    const { client, latest } = setup()
    client.replaceFiles([{ name: "a.kul", text: "A" }])
    await client.ready
    latest().fail("boom")
    client.setFile("a.kul", "edited during the restart")
    await settle()
    await settle()
    expect(latest().files.get("a.kul")?.text).toBe("edited during the restart")
    expect(await client.query("hover", "a.kul", 0)).toMatchObject({
      markdown: "edited during the restart",
    })
  })

  it("treats a WebAssembly trap as a crash", async () => {
    const { client, workers, latest, onRestart } = setup()
    client.replaceFiles([{ name: "a.kul", text: "A" }])
    await client.ready
    latest().trapNext = true
    expect(await client.query("hover", "a.kul", 0)).toBeUndefined()
    await settle()
    expect(workers).toHaveLength(2)
    expect(onRestart).toHaveBeenCalledTimes(1)
    expect(await client.query("hover", "a.kul", 0)).toMatchObject({
      markdown: "A",
    })
  })

  it("restarts through the crash hook", async () => {
    const { client, workers } = setup()
    await client.ready
    client.crashWorker()
    await settle()
    await settle()
    expect(workers).toHaveLength(2)
  })

  it("gives up after too many crashes", async () => {
    const { client, latest, statuses, workers } = setup({ maxCrashes: 2 })
    await client.ready
    for (let crash = 0; crash < 3; crash++) {
      latest().fail("boom")
      await settle()
    }
    expect(workers).toHaveLength(3)
    expect(statuses.at(-1)).toEqual({ state: "failed", message: "boom" })
    expect(await client.query("hover", "a.kul", 0)).toBeUndefined()
  })
})
