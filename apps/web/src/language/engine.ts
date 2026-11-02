// Starting the real language worker. Only the app route imports this
// module, through a dynamic import, so the worker and the .wasm never load
// on the landing page or the docs, nor during prerender.

import { wrap } from "comlink"

import type { WorkerHandle } from "./client"
import type { LanguageWorkerApi } from "./worker"

/** Start a language worker. */
export function spawnLanguageWorker(): Promise<WorkerHandle> {
  const worker = new Worker(new URL("./worker.ts", import.meta.url), {
    type: "module",
    name: "kul-language",
  })
  let failed: ((reason: string) => void) | undefined
  let failure: string | undefined
  const fail = (reason: string) => {
    if (failure !== undefined) return
    failure = reason
    failed?.(reason)
  }
  worker.addEventListener("error", (event) => {
    event.preventDefault()
    fail(event.message || "The language worker failed")
  })
  worker.addEventListener("messageerror", () =>
    fail("The language worker sent a message that could not be read")
  )
  return Promise.resolve({
    api: wrap<LanguageWorkerApi>(worker),
    terminate: () => worker.terminate(),
    onFailure: (listener) => {
      failed = listener
      if (failure !== undefined) listener(failure)
    },
  })
}
