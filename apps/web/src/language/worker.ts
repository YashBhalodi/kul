// The language worker: a module Web Worker that owns the Kul engine
// (kul-wasm) behind Comlink. The .wasm URL comes from a Vite `?url` import
// and is passed to `init` explicitly, so the bundler emits it as a hashed
// asset and nothing runs at module load (no top-level await).

import { expose } from "comlink"
import init, { KUL_CORE_VERSION, KUL_LANGUAGE_VERSION } from "kul-wasm"
import wasmUrl from "kul-wasm/kul_wasm_bg.wasm?url"

export interface EngineInfo {
  /** The kul-core version the engine was built from. */
  core: string
  /** The Kul language version it implements. */
  language: string
}

let loading: Promise<unknown> | undefined

const api = {
  /** Load the engine (once) and report its versions. */
  async start(): Promise<EngineInfo> {
    loading ??= init({ module_or_path: wasmUrl })
    await loading
    return { core: KUL_CORE_VERSION(), language: KUL_LANGUAGE_VERSION() }
  },
}

export type LanguageWorkerApi = typeof api

expose(api)
