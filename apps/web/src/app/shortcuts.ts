// Every keyboard shortcut in the app, declared up front so the shortcuts
// dialog lists them whether or not their components are mounted. A
// component binds one with `useHotkey(appHotkeys[id], callback)`.

import { defineAppHotkeys } from "@tanstack/react-hotkeys"

export const appHotkeys = defineAppHotkeys({
  "palette.open": {
    hotkey: "Mod+K",
    meta: { name: "Open the command palette", group: "General" },
  },
  "shortcuts.open": {
    hotkey: "?",
    meta: { name: "Show keyboard shortcuts", group: "General" },
  },
  "file.rename": {
    hotkey: "F2",
    meta: { name: "Rename the selected file", group: "Files" },
  },
})

declare module "@tanstack/react-hotkeys" {
  interface Register {
    appHotkeys: typeof appHotkeys
  }
}

export type AppHotkeyId = keyof typeof appHotkeys
