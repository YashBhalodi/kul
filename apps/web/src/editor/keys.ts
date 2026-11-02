// The app declares its shortcuts in TanStack Hotkeys' notation
// (`Mod+Shift+Z`); CodeMirror's keymaps take their own (`Mod-Shift-z`).

/**
 * A key in CodeMirror's notation for one in TanStack Hotkeys'. CodeMirror
 * resolves `Mod` by platform: Cmd on a Mac, Ctrl elsewhere.
 */
export function codeMirrorKey(hotkey: string): string {
  const parts = hotkey.split("+")
  const key = parts.pop() ?? ""
  return [...parts, key.length === 1 ? key.toLowerCase() : key].join("-")
}
