// The main thread's side of the language worker. It sends every edit as
// the file's whole text with a new version, keeps the latest text of every
// file (FileMirror), and hands on only the answers that are still current.
//
// When the worker crashes (its `error` event, or a WebAssembly trap in a
// call), the client starts a new one and gives it the file set back. Calls
// waiting on the dead worker settle with `undefined`; calls made during the
// restart wait for the new worker. The editor never waits on any of this.

import { proxy } from "comlink"

import { FileMirror } from "./file-mirror"
import type {
  AnalysisReport,
  Answer,
  EngineInfo,
  LexicalTokens,
  VersionedFile,
} from "./protocol"
import type { LanguageWorkerApi } from "./worker"

/** The worker's API as the main thread calls it: every method is async. */
export type WorkerApi = {
  [K in keyof LanguageWorkerApi]: LanguageWorkerApi[K] extends (
    ...args: infer A
  ) => infer R
    ? (...args: A) => Promise<Awaited<R>>
    : never
}

/** A running worker. */
export interface WorkerHandle {
  api: WorkerApi
  terminate: () => void
  /** Call `listener` once if the worker fails on its own. */
  onFailure: (listener: (reason: string) => void) => void
}

export type ClientStatus =
  | { state: "starting" }
  | { state: "ready"; info: EngineInfo; restarts: number }
  | { state: "failed"; message: string }

export interface LanguageClientOptions {
  spawn: () => Promise<WorkerHandle>
  onAnalysis: (report: AnalysisReport) => void
  onStatus?: (status: ClientStatus) => void
  /** Called after a crashed worker has been replaced and given the files. */
  onRestart?: () => void
  /** Give up after this many crashes within `crashWindowMs`. */
  maxCrashes?: number
  crashWindowMs?: number
  now?: () => number
}

type Queries = Omit<
  WorkerApi,
  | "start"
  | "connect"
  | "reset"
  | "setFile"
  | "deleteFile"
  | "lexicalTokens"
  | "flush"
  | "crash"
>

type QueryValue<K extends keyof Queries> =
  Awaited<ReturnType<Queries[K]>> extends Answer<infer T> ? T : never

export interface LanguageClient {
  /** Settles once the first worker has started. */
  readonly ready: Promise<EngineInfo>
  /** Replace the whole file set (a project was opened). */
  replaceFiles: (files: ReadonlyArray<{ name: string; text: string }>) => void
  /**
   * Send a file's new text. The version is assigned at once; the lexical
   * tokens follow (undefined if the worker crashed first).
   */
  setFile: (
    name: string,
    text: string
  ) => { version: number; tokens: Promise<LexicalTokens | undefined> }
  deleteFile: (name: string) => void
  /** The latest text and version sent for `name`. */
  file: (name: string) => VersionedFile | undefined
  /** Every file, as last sent. */
  files: () => Array<VersionedFile>
  /** A file's lexical tokens for its latest text. */
  lexicalTokens: (name: string) => Promise<LexicalTokens | undefined>
  /**
   * Ask the worker a question. The answer is `undefined` when an edit
   * overtook it, or when the worker failed.
   */
  query: <K extends keyof Queries>(
    feature: K,
    ...args: Parameters<Queries[K]>
  ) => Promise<QueryValue<K> | undefined>
  /** Make the worker fail as a crash would (a test hook). */
  crashWorker: () => void
  dispose: () => void
}

interface Generation {
  handle: WorkerHandle | undefined
  /** No worker: the engine gave up after too many crashes. */
  stopped?: boolean
  api: Promise<WorkerApi>
  /** Settles when this worker is replaced or fails. */
  ended: Promise<undefined>
  end: () => void
}

const NEVER_READY = "The language engine is not running"

export function createLanguageClient(
  options: LanguageClientOptions
): LanguageClient {
  const {
    spawn,
    onAnalysis,
    onStatus,
    onRestart,
    maxCrashes = 3,
    crashWindowMs = 60_000,
    now = () => Date.now(),
  } = options
  const mirror = new FileMirror()
  const crashes: Array<number> = []
  let restarts = 0
  let disposed = false
  let resolveReady: (info: EngineInfo) => void = () => undefined
  let rejectReady: (error: unknown) => void = () => undefined
  const ready = new Promise<EngineInfo>((resolve, reject) => {
    resolveReady = resolve
    rejectReady = reject
  })
  // A start failure is reported through `ready` and the status.
  ready.catch(() => undefined)

  function receive(report: AnalysisReport) {
    if (!disposed && mirror.isCurrentAnalysis(report.files)) onAnalysis(report)
  }

  function begin(restart: boolean): Generation {
    let end: () => void = () => undefined
    const ended = new Promise<undefined>((resolve) => {
      end = () => resolve(undefined)
    })
    const generation: Generation = {
      handle: undefined,
      api: Promise.resolve(undefined as never),
      ended,
      end,
    }
    generation.api = (async () => {
      const handle = await spawn()
      generation.handle = handle
      if (disposed || current !== generation) {
        handle.terminate()
        throw new Error(NEVER_READY)
      }
      handle.onFailure((reason) => crashed(generation, reason))
      const info = await handle.api.start()
      await handle.api.connect(proxy(receive))
      await handle.api.reset(mirror.snapshot())
      if (restart) {
        restarts++
        onRestart?.()
      } else {
        resolveReady(info)
      }
      onStatus?.({ state: "ready", info, restarts })
      return handle.api
    })()
    generation.api.catch((error: unknown) => {
      if (current !== generation || disposed) return
      const message = error instanceof Error ? error.message : String(error)
      if (restart) {
        crashed(generation, message)
      } else {
        rejectReady(error)
        onStatus?.({ state: "failed", message })
      }
    })
    return generation
  }

  function crashed(generation: Generation, reason: string) {
    if (current !== generation || disposed) return
    generation.end()
    generation.handle?.terminate()
    const time = now()
    crashes.push(time)
    while (crashes.length > 0 && (crashes[0] ?? 0) < time - crashWindowMs) {
      crashes.shift()
    }
    if (crashes.length > maxCrashes) {
      current = stopped(reason)
      rejectReady(new Error(reason))
      onStatus?.({ state: "failed", message: reason })
      return
    }
    onStatus?.({ state: "starting" })
    current = begin(true)
  }

  function stopped(reason: string): Generation {
    const api = Promise.reject(new Error(reason))
    api.catch(() => undefined)
    return {
      handle: undefined,
      stopped: true,
      api,
      ended: Promise.resolve(undefined),
      end: () => undefined,
    }
  }

  onStatus?.({ state: "starting" })
  let current = begin(false)

  /**
   * Run `call` on the current worker. Settles with undefined if the worker
   * fails first; a WebAssembly trap counts as a crash.
   */
  function run<T>(
    call: (api: WorkerApi) => Promise<T>
  ): Promise<T | undefined> {
    const generation = current
    const result = generation.api.then(call).catch((error: unknown) => {
      if (isTrap(error)) {
        crashed(generation, error instanceof Error ? error.message : "trap")
      } else if (current === generation && !generation.stopped) {
        console.error("The language worker failed a request", error)
      }
      return undefined
    })
    return Promise.race([result, generation.ended])
  }

  return {
    ready,
    replaceFiles(files) {
      const versioned = mirror.replace(files)
      void run((api) => api.reset(versioned))
    },
    setFile(name, text) {
      const version = mirror.set(name, text)
      return {
        version,
        tokens: run((api) => api.setFile(name, text, version)),
      }
    },
    deleteFile(name) {
      if (mirror.delete(name)) void run((api) => api.deleteFile(name))
    },
    file: (name) => mirror.get(name),
    files: () => mirror.snapshot(),
    lexicalTokens: (name) =>
      run((api) => api.lexicalTokens(name)).then((tokens) =>
        tokens && tokens.version === mirror.version(name) ? tokens : undefined
      ),
    query<K extends keyof Queries>(
      feature: K,
      ...args: Parameters<Queries[K]>
    ) {
      return run((api) => {
        const method = api[feature] as unknown as (
          ...a: Parameters<Queries[K]>
        ) => Promise<Answer<QueryValue<K>>>
        return method(...args)
      }).then((answer) =>
        answer && mirror.isCurrent(answer.versions) ? answer.value : undefined
      )
    },
    crashWorker() {
      void run((api) => api.crash())
    },
    dispose() {
      disposed = true
      current.end()
      current.handle?.terminate()
    },
  }
}

/** Whether `error` is a WebAssembly trap (a Rust panic), which poisons the module. */
function isTrap(error: unknown): boolean {
  return (
    typeof error === "object" &&
    error !== null &&
    (error as { name?: unknown }).name === "RuntimeError"
  )
}
