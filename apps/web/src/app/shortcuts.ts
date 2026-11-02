// Every keyboard shortcut in the app, declared up front so the shortcuts
// dialog lists them whether or not their components are mounted. A
// component binds one with `useHotkey(appHotkeys[id], callback)`. The
// editor's own shortcuts (scope "editor") are matched inside the editor
// from this same table, and work while it has focus; the canvas's (scope
// "canvas") likewise, while the canvas has focus.

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
  "editor.toggle": {
    hotkey: "Mod+J",
    meta: { name: "Show or hide the editor", group: "General" },
  },
  "file.rename": {
    hotkey: "F2",
    meta: { name: "Rename the selected file", group: "Files" },
  },
  "tab.close": {
    hotkey: "Mod+W",
    meta: { name: "Close the file’s tab", group: "Files" },
  },
  "editor.complete": {
    hotkey: "Control+Space",
    scope: "editor",
    meta: { name: "Suggest completions", group: "Editor" },
  },
  "editor.definition": {
    hotkey: "F12",
    scope: "editor",
    meta: { name: "Go to the declaration", group: "Editor" },
  },
  "editor.references": {
    hotkey: "Shift+F12",
    scope: "editor",
    meta: { name: "Find every use", group: "Editor" },
  },
  "editor.rename": {
    hotkey: "F2",
    scope: "editor",
    meta: { name: "Rename a person or marriage id", group: "Editor" },
  },
  "editor.quickFix": {
    hotkey: "Mod+.",
    scope: "editor",
    meta: { name: "Show quick fixes", group: "Editor" },
  },
  "editor.format": {
    hotkey: "Alt+Shift+F",
    scope: "editor",
    meta: { name: "Format the file", group: "Editor" },
  },
  "editor.symbols": {
    hotkey: "Mod+Shift+O",
    scope: "editor",
    meta: { name: "Go to a person or marriage", group: "Editor" },
  },
  "editor.comment": {
    hotkey: "Mod+/",
    scope: "editor",
    meta: { name: "Comment or uncomment lines", group: "Editor" },
  },
  "editor.undo": {
    hotkey: "Mod+Z",
    scope: "editor",
    meta: { name: "Undo", group: "Editor" },
  },
  "editor.redo": {
    hotkey: "Mod+Shift+Z",
    scope: "editor",
    meta: { name: "Redo", group: "Editor" },
  },
  "editor.redoAgain": {
    hotkey: "Mod+Y",
    scope: "editor",
    meta: { name: "Redo", group: "Editor" },
  },
  "editor.find": {
    hotkey: "Mod+F",
    scope: "editor",
    meta: { name: "Find and replace", group: "Editor" },
  },
  "canvas.fit": {
    hotkey: "0",
    scope: "canvas",
    meta: { name: "Fit the tree to the canvas", group: "Canvas" },
  },
  "canvas.zoomIn": {
    hotkey: "+",
    scope: "canvas",
    meta: { name: "Zoom in", group: "Canvas" },
  },
  "canvas.zoomOut": {
    hotkey: "-",
    scope: "canvas",
    meta: { name: "Zoom out", group: "Canvas" },
  },
  "canvas.panLeft": {
    hotkey: "ArrowLeft",
    scope: "canvas",
    meta: { name: "Pan left", group: "Canvas" },
  },
  "canvas.panRight": {
    hotkey: "ArrowRight",
    scope: "canvas",
    meta: { name: "Pan right", group: "Canvas" },
  },
  "canvas.panUp": {
    hotkey: "ArrowUp",
    scope: "canvas",
    meta: { name: "Pan up", group: "Canvas" },
  },
  "canvas.panDown": {
    hotkey: "ArrowDown",
    scope: "canvas",
    meta: { name: "Pan down", group: "Canvas" },
  },
  "canvas.clearSelection": {
    hotkey: "Escape",
    scope: "canvas",
    meta: { name: "Clear the selection", group: "Canvas" },
  },
})

declare module "@tanstack/react-hotkeys" {
  interface Register {
    appHotkeys: typeof appHotkeys
  }
}

export type AppHotkeyId = keyof typeof appHotkeys
