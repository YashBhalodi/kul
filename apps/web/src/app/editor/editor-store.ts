// The editor window's passing UI state: whether the problems strip is
// open, which popup the editor shows (quick fixes, references, the rename
// box, the symbol list), and the pill's flash. Its layout is project
// window state, kept in the session (`ui.editorWindow`).

import { createStore } from "@tanstack/react-store"

import type { Location, OutlineSymbol, QuickFix } from "@/language/protocol"
import type { Dock } from "@/store/editor-window"

/** A place on screen, in viewport px, that a popup opens from. */
export interface Anchor {
  x: number
  y: number
  /** The height of the text the popup belongs to, to open below it. */
  height: number
}

export interface ReferenceItem {
  location: Location
  line: number
  preview: string
}

export type EditorOverlay =
  | {
      kind: "quick-fixes"
      file: string
      anchor: Anchor
      fixes: Array<QuickFix>
    }
  | {
      kind: "references"
      anchor: Anchor
      id: string
      references: Array<ReferenceItem>
    }
  | {
      kind: "rename"
      file: string
      offset: number
      anchor: Anchor
      current: string
      problem?: string
    }
  | { kind: "symbols"; file: string; symbols: Array<OutlineSymbol> }

export interface EditorUiState {
  problemsOpen: boolean
  overlay: EditorOverlay | undefined
  /** Counts up each time the minimized pill should flash a file name. */
  pillFlash: number
  pillFile: string | undefined
  /** The side a drag would dock the window to, while dragging. */
  dockPreview: Dock | null
}

export const editorUiStore = createStore<EditorUiState>({
  problemsOpen: false,
  overlay: undefined,
  pillFlash: 0,
  pillFile: undefined,
  dockPreview: null,
})

export function setProblemsOpen(open: boolean): void {
  editorUiStore.setState((state) => ({ ...state, problemsOpen: open }))
}

export function showOverlay(overlay: EditorOverlay): void {
  editorUiStore.setState((state) => ({ ...state, overlay }))
}

export function closeOverlay(): void {
  editorUiStore.setState((state) =>
    state.overlay ? { ...state, overlay: undefined } : state
  )
}

/** Flash `file` on the minimized editor's pill. */
export function flashPill(file: string): void {
  editorUiStore.setState((state) => ({
    ...state,
    pillFlash: state.pillFlash + 1,
    pillFile: file,
  }))
}

export function setDockPreview(dockPreview: Dock | null): void {
  editorUiStore.setState((state) =>
    state.dockPreview === dockPreview ? state : { ...state, dockPreview }
  )
}

/** Where the editor's cursor is: the file and the UTF-16 offset in it. */
export interface EditorCursor {
  file: string
  offset: number
}

/**
 * The cursor in the active file, updated as it moves. The canvas follows
 * it to the entity under it (`entityAt`), debouncing as it likes.
 */
export const editorCursorStore = createStore<EditorCursor | undefined>(
  undefined
)
