// Checks that hold the docs to the code they describe. The build runs them
// (vite.config.ts) and so do the unit tests:
//
// - the diagnostics reference has one entry per code the engine defines,
//   and none for a code it doesn't; every L, P and R entry shows an example
//   that produces its code (the example itself is checked by rehype-kul);
// - the keyboard shortcuts page lists every shortcut the app declares, with
//   its key, and nothing else.
//
// Node-safe: relative imports only, so vite.config.ts can load it.

/** A diagnostics-reference entry: `### KUL-R03: … [#kul-r03]`. */
const ENTRY = /^### (KUL-[A-Z]\d+[a-z]?)\b/

/** The codes the diagnostics reference documents, in page order. */
export function documentedCodes(mdx: string): Array<string> {
  return mdx
    .split("\n")
    .map((line) => ENTRY.exec(line)?.[1])
    .filter((code): code is string => code !== undefined)
}

/** Each entry's text, by code. */
function entries(mdx: string): Map<string, string> {
  const sections = new Map<string, string>()
  let current: string | undefined
  for (const line of mdx.split("\n")) {
    const code = ENTRY.exec(line)?.[1]
    if (code !== undefined) {
      current = code
      sections.set(code, "")
    } else if (/^#{1,3} /.test(line)) {
      current = undefined
    } else if (current !== undefined) {
      sections.set(current, `${sections.get(current) ?? ""}${line}\n`)
    }
  }
  return sections
}

/** Codes whose mistakes a Kul file can show (lexer, parser, rules). */
function hasFileExample(code: string): boolean {
  return /^KUL-[LPR]/.test(code)
}

/**
 * What is wrong with the diagnostics reference `mdx` against the engine's
 * catalogue of `codes`, as human-readable lines.
 */
export function diagnosticsDrift(
  mdx: string,
  codes: ReadonlyArray<string>
): Array<string> {
  const problems: Array<string> = []
  const documented = documentedCodes(mdx)
  const catalogue = new Set(codes)
  const seen = new Set<string>()
  for (const code of documented) {
    if (seen.has(code)) problems.push(`${code} is documented twice`)
    seen.add(code)
    if (!catalogue.has(code)) {
      problems.push(
        `${code} is documented, but the engine defines no such code`
      )
    }
  }
  for (const code of codes) {
    if (!seen.has(code)) problems.push(`${code} has no entry in the reference`)
  }
  for (const [code, text] of entries(mdx)) {
    if (!hasFileExample(code)) continue
    const fence = /^```kul expect-error=(\S+)/m.exec(text)?.[1]
    if (fence?.split(",")[0] !== code) {
      problems.push(
        `${code}'s entry needs an example fenced \`\`\`kul expect-error=${code}`
      )
    }
  }
  return problems
}

/** A declared shortcut: its key (TanStack Hotkeys syntax) and its name. */
export interface DeclaredShortcut {
  hotkey: string
  name: string
}

const KEY_NAMES: Readonly<Record<string, string>> = {
  Mod: "Ctrl",
  Control: "Ctrl",
  Escape: "Esc",
  ArrowLeft: "←",
  ArrowRight: "→",
  ArrowUp: "↑",
  ArrowDown: "↓",
}

/** How the docs write a hotkey: `Mod+Shift+O` → `<kbd>Ctrl</kbd>+…`. */
export function hotkeyMarkup(hotkey: string): string {
  const keys = hotkey === "+" ? ["+"] : hotkey.split("+")
  return keys.map((key) => `<kbd>${KEY_NAMES[key] ?? key}</kbd>`).join("+")
}

/** The rows of the shortcut tables: `| <keys> | <name> |`. */
export function shortcutRows(mdx: string): Array<DeclaredShortcut> {
  const rows: Array<DeclaredShortcut> = []
  for (const line of mdx.split("\n")) {
    const [, hotkey, name] =
      /^\|\s*(<kbd>.*?)\s*\|\s*(.+?)\s*\|$/.exec(line) ?? []
    if (hotkey !== undefined && name !== undefined) rows.push({ hotkey, name })
  }
  return rows
}

/**
 * What is wrong with the shortcuts page `mdx` against the app's declared
 * shortcuts, as human-readable lines.
 */
export function shortcutsDrift(
  mdx: string,
  declared: ReadonlyArray<DeclaredShortcut>
): Array<string> {
  const problems: Array<string> = []
  const rows = shortcutRows(mdx)
  const key = (row: DeclaredShortcut) => `${row.hotkey} ${row.name}`
  const listed = new Set(rows.map(key))
  const expected = new Set(
    declared.map((shortcut) =>
      key({ hotkey: hotkeyMarkup(shortcut.hotkey), name: shortcut.name })
    )
  )
  for (const shortcut of expected) {
    if (!listed.has(shortcut)) problems.push(`not listed: ${shortcut}`)
  }
  for (const row of listed) {
    if (!expected.has(row)) problems.push(`listed but not declared: ${row}`)
  }
  return problems
}
