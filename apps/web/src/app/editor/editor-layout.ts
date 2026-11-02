// The editor window's layout: read from the open project's window state
// (a default until the window is first moved) and changed through it, so
// it is saved with the project.

import { createStore } from "@tanstack/react-store"

import type { EditorWindowLayout } from "@/store/editor-window"
import { sessionStore, updateUi } from "../project-session"
import { setProblemsOpen } from "./editor-store"
import { defaultLayout } from "./window-geometry"
import type { Size } from "./window-geometry"

/** The canvas the window sits on, as last measured. */
export const canvasSizeStore = createStore<Size>({ width: 1280, height: 800 })

/** The layout in effect: the project's own, or the default for the canvas. */
export function currentLayout(
  stored: EditorWindowLayout | null = sessionStore.state.ui.editorWindow,
  canvas: Size = canvasSizeStore.state
): EditorWindowLayout {
  return stored ?? defaultLayout(canvas)
}

export function setLayout(
  change: (layout: EditorWindowLayout) => EditorWindowLayout
): void {
  updateUi((ui) => ({
    ...ui,
    editorWindow: change(currentLayout(ui.editorWindow)),
  }))
}

export function setMinimized(minimized: boolean): void {
  setLayout((layout) =>
    layout.minimized === minimized ? layout : { ...layout, minimized }
  )
}

/** Minimize the editor, or restore it (Mod+J). */
export function toggleEditor(): void {
  setMinimized(!currentLayout().minimized)
}

/** Restore the editor and open its problems strip. */
export function showProblems(): void {
  setMinimized(false)
  setProblemsOpen(true)
}
