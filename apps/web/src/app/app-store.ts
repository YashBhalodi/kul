// The app's UI state. The editor window's layout, open tabs, selection and
// camera join it with the editor and the canvas.

import { createStore } from "@tanstack/react-store"

export interface AppState {
  paletteOpen: boolean
  shortcutsOpen: boolean
}

const initialState: AppState = { paletteOpen: false, shortcutsOpen: false }

export const appStore = createStore(initialState, ({ setState }) => ({
  setPaletteOpen: (paletteOpen: boolean) =>
    setState((state) => ({ ...state, paletteOpen })),
  togglePalette: () =>
    setState((state) => ({ ...state, paletteOpen: !state.paletteOpen })),
  setShortcutsOpen: (shortcutsOpen: boolean) =>
    setState((state) => ({ ...state, shortcutsOpen })),
  /** Close the palette and show the shortcuts dialog in its place. */
  showShortcuts: () =>
    setState((state) => ({
      ...state,
      paletteOpen: false,
      shortcutsOpen: true,
    })),
}))
