// The language worker: a module Web Worker that owns the project's Kul
// `Workspace` (kul-wasm) behind Comlink. The .wasm URL comes from a Vite
// `?url` import and is passed to `init` explicitly, so the bundler emits it
// as a hashed asset and nothing runs at module load (no top-level await).
//
// Edits replace a file's whole text and answer with its lexical tokens at
// once. Analysis waits for edits to pause (ANALYSIS_WAIT_MS) and is pushed
// to the main thread; a feature that needs it (hover, completion, rename,
// ...) runs the waiting analysis first. Every answer carries the versions
// of the text it was computed from.

import { expose } from "comlink"
import init, {
  KUL_CORE_VERSION,
  KUL_LANGUAGE_VERSION,
  Workspace,
} from "kul-wasm"
import wasmUrl from "kul-wasm/kul_wasm_bg.wasm?url"

import { ANALYSIS_WAIT_MS } from "./protocol"
import type {
  AnalysisReport,
  Answer,
  EngineInfo,
  LexicalTokens,
  SvgTheme,
  Token,
  VersionedFile,
} from "./protocol"
import { createAnalysisScheduler } from "./scheduler"

let loading: Promise<unknown> | undefined
let workspace: Workspace | undefined
const versions = new Map<string, number>()
let listener: ((report: AnalysisReport) => void) | undefined

const scheduler = createAnalysisScheduler(publish, ANALYSIS_WAIT_MS)

function load(): Promise<unknown> {
  loading ??= init({ module_or_path: wasmUrl })
  return loading
}

function project(): Workspace {
  if (!workspace) throw new Error("The language worker has not started")
  return workspace
}

/** Analyze the project and push the report to the main thread. */
function publish() {
  const current = project()
  const analysis = current.analyze()
  const semantic: Record<string, Array<Token>> = {}
  for (const file of analysis.files) {
    semantic[file.name] = current.semanticTokens(file.name)
  }
  listener?.({ ...analysis, semantic })
}

/** Run `feature` against an up-to-date analysis, tagged with its versions. */
function answer<T>(feature: (workspace: Workspace) => T): Answer<T> {
  scheduler.flush()
  return { versions: Object.fromEntries(versions), value: feature(project()) }
}

function write(file: VersionedFile) {
  project().setFile(file.name, file.text, file.version)
  versions.set(file.name, file.version)
}

const api = {
  /** Load the engine (once), start an empty project and report versions. */
  async start(): Promise<EngineInfo> {
    await load()
    workspace ??= new Workspace()
    return { core: KUL_CORE_VERSION(), language: KUL_LANGUAGE_VERSION() }
  },

  /** Where to push each analysis. */
  connect(onAnalysis: (report: AnalysisReport) => void): void {
    listener = onAnalysis
  },

  /** Replace the whole file set. */
  reset(files: ReadonlyArray<VersionedFile>): void {
    for (const name of versions.keys()) project().deleteFile(name)
    versions.clear()
    for (const file of files) write(file)
    scheduler.request()
  },

  /** Add a file or replace its text; answers with its lexical tokens. */
  setFile(name: string, text: string, version: number): LexicalTokens {
    write({ name, text, version })
    scheduler.request()
    return { file: name, version, tokens: project().lexicalTokens(name) }
  },

  deleteFile(name: string): void {
    project().deleteFile(name)
    versions.delete(name)
    scheduler.request()
  },

  /** A file's lexical tokens, for its current text. */
  lexicalTokens(name: string): LexicalTokens | undefined {
    const version = versions.get(name)
    if (version === undefined) return undefined
    return { file: name, version, tokens: project().lexicalTokens(name) }
  },

  /** Analyze now, if edits are waiting. */
  flush(): void {
    scheduler.flush()
  },

  hover: (file: string, offset: number) => answer((w) => w.hover(file, offset)),
  complete: (file: string, offset: number) =>
    answer((w) => w.complete(file, offset)),
  definition: (file: string, offset: number) =>
    answer((w) => w.definition(file, offset)),
  references: (file: string, offset: number, includeDeclaration: boolean) =>
    answer((w) => w.references(file, offset, includeDeclaration)),
  prepareRename: (file: string, offset: number) =>
    answer((w) => w.prepareRename(file, offset)),
  rename: (file: string, offset: number, newName: string) =>
    answer((w) => w.rename(file, offset, newName)),
  quickFixes: (file: string, start: number, end: number) =>
    answer((w) => w.quickFixes(file, start, end)),
  format: (file: string) => answer((w) => w.format(file)),
  symbols: (file: string) => answer((w) => w.symbols(file)),
  entityAt: (file: string, offset: number) =>
    answer((w) => w.entityAt(file, offset)),
  locate: (id: string) => answer((w) => w.locate(id)),
  details: (id: string) => answer((w) => w.details(id)),
  renderSvg: (theme: SvgTheme) => answer((w) => w.renderSvg(theme)),

  /**
   * Fail the way a crash does: an error nothing catches, which the main
   * thread sees as the worker's `error` event. The app's test hook uses it
   * to prove the restart.
   */
  crash(): void {
    setTimeout(() => {
      throw new Error("The language worker was stopped")
    })
  },
}

export type LanguageWorkerApi = typeof api

expose(api)
