// The main thread's handle on the language worker. Only the app route
// imports this module, through a dynamic import, so the worker and the
// .wasm never load on the landing page or the docs, nor during prerender.

import { wrap } from "comlink"
import type { Remote } from "comlink"

import type { EngineInfo, LanguageWorkerApi } from "./worker"

export type { EngineInfo }

export interface Engine {
  info: EngineInfo
  api: Remote<LanguageWorkerApi>
  terminate: () => void
}

/** Start the language worker and wait until the engine has loaded. */
export async function startEngine(): Promise<Engine> {
  const worker = new Worker(new URL("./worker.ts", import.meta.url), {
    type: "module",
    name: "kul-language",
  })
  const failed = new Promise<never>((_, reject) => {
    worker.addEventListener("error", (event) =>
      reject(new Error(event.message || "The language worker failed to load"))
    )
  })
  const api = wrap<LanguageWorkerApi>(worker)
  try {
    const info = await Promise.race([api.start(), failed])
    return { info, api, terminate: () => worker.terminate() }
  } catch (error) {
    worker.terminate()
    throw error
  }
}
