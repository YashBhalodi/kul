// The language worker: a module Web Worker that owns the Kul engine
// (kul-wasm) behind Comlink. The .wasm URL comes from a Vite `?url` import
// and is passed to `init` explicitly, so the bundler emits it as a hashed
// asset and nothing runs at module load (no top-level await).

import { expose } from "comlink"
import init, {
  KUL_CORE_VERSION,
  KUL_LANGUAGE_VERSION,
  Workspace,
} from "kul-wasm"
import wasmUrl from "kul-wasm/kul_wasm_bg.wasm?url"

export interface EngineInfo {
  /** The kul-core version the engine was built from. */
  core: string
  /** The Kul language version it implements. */
  language: string
}

export interface TreeRender {
  /** The theme-agnostic SVG, absent when the project can't be drawn. */
  svg?: string
  errors: number
}

let loading: Promise<unknown> | undefined

function load(): Promise<unknown> {
  loading ??= init({ module_or_path: wasmUrl })
  return loading
}

const api = {
  /** Load the engine (once) and report its versions. */
  async start(): Promise<EngineInfo> {
    await load()
    return { core: KUL_CORE_VERSION(), language: KUL_LANGUAGE_VERSION() }
  },

  /** Check a whole project and draw its tree. */
  async render(
    files: ReadonlyArray<{ name: string; text: string }>
  ): Promise<TreeRender> {
    await load()
    const workspace = new Workspace()
    try {
      for (const file of files) workspace.setFile(file.name, file.text, 0)
      const analysis = workspace.analyze()
      return {
        svg: analysis.svg,
        errors: analysis.diagnostics.filter((d) => d.severity === "error")
          .length,
      }
    } finally {
      workspace.free()
    }
  },
}

export type LanguageWorkerApi = typeof api

expose(api)
