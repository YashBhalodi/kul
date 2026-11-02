// The app's language engine: one language worker per page, kept fed with
// the open project's files, and the latest analysis of them.

import { createStore } from "@tanstack/react-store"
import { toast } from "sonner"

import type { ClientStatus, LanguageClient } from "@/language/client"
import { createLanguageClient } from "@/language/client"
import type { AnalysisReport } from "@/language/protocol"
import { sessionStore } from "./project-session"
import type { SessionState } from "./project-session"

export type EngineStatus = ClientStatus

export const engineStatusStore = createStore<EngineStatus>({
  state: "starting",
})

export interface AnalysisState {
  /** The latest analysis that matched every edit sent when it arrived. */
  report: AnalysisReport | undefined
  /** The tree from the latest analysis that could draw one. */
  lastSvg: string | undefined
}

export const analysisStore = createStore<AnalysisState>({
  report: undefined,
  lastSvg: undefined,
})

let client: LanguageClient | undefined
let stopSync: (() => void) | undefined

/**
 * Start the language engine once per page and keep it in step with the
 * open project. The worker module is imported dynamically, so only the
 * app route ever loads it.
 */
export function bootEngine(): void {
  if (client) return
  const started = createLanguageClient({
    spawn: () =>
      import("@/language/engine").then((module) =>
        module.spawnLanguageWorker()
      ),
    onAnalysis: (report) =>
      analysisStore.setState((state) => ({
        report,
        lastSvg: report.svg ?? state.lastSvg,
      })),
    onStatus: (status) => {
      engineStatusStore.setState(() => status)
      if (status.state === "failed" && client === started) {
        stopSync?.()
        stopSync = undefined
        client = undefined
        started.dispose()
      }
    },
    onRestart: () =>
      toast("The Kul engine stopped and was restarted. Your files are safe.", {
        id: "engine-restarted",
      }),
  })
  client = started
  stopSync = syncFiles(started)
}

/** The running language client, if the engine has been booted. */
export function languageClient(): LanguageClient | undefined {
  return client
}

/**
 * Send the open project's files to the worker: the whole set when a
 * project opens, then each file whose text differs from what was sent.
 * The editor sends its own edits first, so those are already in step.
 */
function syncFiles(target: LanguageClient): () => void {
  let projectId: string | undefined
  let files: SessionState["files"] | undefined

  const sync = (state: SessionState) => {
    if (state.status !== "open") {
      if (projectId !== undefined) {
        projectId = undefined
        files = undefined
        target.replaceFiles([])
        analysisStore.setState(() => ({
          report: undefined,
          lastSvg: undefined,
        }))
      }
      return
    }
    if (state.projectId !== projectId) {
      projectId = state.projectId
      files = state.files
      analysisStore.setState(() => ({ report: undefined, lastSvg: undefined }))
      target.replaceFiles(state.files)
      return
    }
    if (state.files === files) return
    files = state.files
    const names = new Set(state.files.map((file) => file.name))
    for (const sent of target.files()) {
      if (!names.has(sent.name)) target.deleteFile(sent.name)
    }
    for (const file of state.files) {
      if (target.file(file.name)?.text !== file.text) {
        target.setFile(file.name, file.text)
      }
    }
  }

  sync(sessionStore.state)
  const subscription = sessionStore.subscribe(sync)
  return () => subscription.unsubscribe()
}

/** Make the language worker fail as a crash would (the app's test hook). */
export function crashLanguageWorker(): void {
  client?.crashWorker()
}
