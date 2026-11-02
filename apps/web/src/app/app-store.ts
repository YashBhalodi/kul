// The app's UI state: which dialog is open. The editor keeps its own
// (editor/editor-store.ts), and the project's window state lives in the
// session.

import { createStore } from "@tanstack/react-store"

export type AppDialog =
  "palette" | "shortcuts" | "settings" | "rename-project" | "delete-project"

export interface AppState {
  /** The one dialog open, if any. */
  dialog: AppDialog | undefined
}

const initialState: AppState = { dialog: undefined }

export const appStore = createStore(initialState, ({ setState }) => ({
  /** Open `dialog`, closing any other. */
  show: (dialog: AppDialog) => setState(() => ({ dialog })),
  /** Close `dialog` if it is the one open. */
  hide: (dialog: AppDialog) =>
    setState((state) =>
      state.dialog === dialog ? { dialog: undefined } : state
    ),
  togglePalette: () =>
    setState((state) => ({
      dialog: state.dialog === "palette" ? undefined : "palette",
    })),
}))

/** Open state and setter for one dialog, for a dialog's props. */
export function dialogControl(dialog: AppDialog) {
  return {
    open: (state: AppState) => state.dialog === dialog,
    onOpenChange: (open: boolean) =>
      open ? appStore.actions.show(dialog) : appStore.actions.hide(dialog),
  }
}
