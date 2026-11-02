// Hooks the end-to-end suite drives through `window.kulTestHooks`. They
// reach states a person can't produce on demand, such as a crashed
// language worker, and change nothing else.

import { shownSelection, shownText } from "./editor/editor-controller"
import { crashLanguageWorker } from "./engine-status"

export interface KulTestHooks {
  /** Make the language worker fail as a crash would; the app restarts it. */
  crashLanguageWorker: () => void
  /**
   * The whole text of the file the editor shows. (The editor draws only the
   * lines in view, so the page's text is not all of it.)
   */
  editorText: () => string | undefined
  /** The text selected in it. */
  editorSelection: () => string | undefined
}

declare global {
  interface Window {
    kulTestHooks?: KulTestHooks
  }
}

export function installTestHooks(): () => void {
  window.kulTestHooks = {
    crashLanguageWorker,
    editorText: shownText,
    editorSelection: shownSelection,
  }
  return () => {
    delete window.kulTestHooks
  }
}
