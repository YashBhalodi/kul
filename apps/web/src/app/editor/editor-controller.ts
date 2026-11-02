// The editor: one CodeMirror view showing the active file, and one
// EditorState per open tab (each with its own undo history), kept in step
// with the open project, the language worker and its analysis.
//
// Every edit sends the file's whole text to the worker with a new version
// and records it in the session (which saves it). Token answers and
// diagnostics are applied to the state of the file they belong to, mapped
// through any edits made since. A change that spans files (a rename, or
// formatting every file) edits open files through their states and writes
// closed ones through the session; undoing it in one open file undoes it
// in all of them.

import {
  isolateHistory,
  redo,
  redoDepth,
  undo,
  undoDepth,
} from "@codemirror/commands"
import { setDiagnostics } from "@codemirror/lint"
import type { Diagnostic as LintDiagnostic } from "@codemirror/lint"
import {
  EditorSelection,
  EditorState,
  Prec,
  Transaction,
} from "@codemirror/state"
import type { ChangeSet, Extension, TransactionSpec } from "@codemirror/state"
import { EditorView, keymap } from "@codemirror/view"
import type { ViewUpdate } from "@codemirror/view"
import { matchesKeyboardEvent } from "@tanstack/react-hotkeys"
import { toast } from "sonner"

import {
  editedFile,
  kulEditorExtensions,
  renameEditedFile,
  setReadOnly,
  setTokenRanges,
} from "@/editor/kul-editor"
import type { KulFeatures } from "@/editor/kul-editor"
import { kulEditorTheme } from "@/editor/theme"
import { EditTrail, overlaySemantic, tokenRanges } from "@/editor/tokens"
import type {
  AnalysisReport,
  Diagnostic,
  FileEdit,
  LexicalTokens,
  Location,
  TextEdit,
  Token,
} from "@/language/protocol"
import { analysisStore, languageClient } from "../engine-status"
import {
  renameFile,
  selectFile,
  sessionStore,
  setFileText,
  writeFiles,
} from "../project-session"
import type { SessionState } from "../project-session"
import { appHotkeys } from "../shortcuts"
import type { AppHotkeyId } from "../shortcuts"
import { currentLayout } from "./editor-layout"
import {
  closeOverlay,
  editorCursorStore,
  flashPill,
  showOverlay,
} from "./editor-store"
import type { Anchor } from "./editor-store"
import { lineAndColumn } from "./problems"

interface OpenFile {
  state: EditorState
  trail: EditTrail
  lexical: LexicalTokens | undefined
  semantic: { version: number; tokens: Array<Token> } | undefined
  scroll: ReturnType<EditorView["scrollSnapshot"]> | undefined
}

/** One change across files, as undo sees it. */
interface LinkedEdit {
  open: Array<{ name: string; undoDepth: number; redoDepth: number }>
  closed: Array<{ name: string; before: string; after: string }>
  undone: boolean
}

const open = new Map<string, OpenFile>()
const linked: Array<LinkedEdit> = []
let view: EditorView | undefined
let active: string | undefined
let projectId: string | undefined
let readOnly = false

// ---- Files and states ---------------------------------------------------

function sessionText(name: string): string | undefined {
  return sessionStore.state.files.find((file) => file.name === name)?.text
}

function fileText(name: string): string | undefined {
  return open.get(name)?.state.doc.toString() ?? sessionText(name)
}

function createState(name: string, text: string): EditorState {
  return EditorState.create({
    doc: text,
    extensions: kulEditorExtensions({
      file: name,
      readOnly,
      features,
      commands: editorCommands,
      onUpdate,
      theme: kulEditorTheme,
    }),
  })
}

/** The open file `name`, creating its state from the project if needed. */
function openFile(name: string): OpenFile | undefined {
  const existing = open.get(name)
  if (existing) return existing
  const text = sessionText(name)
  if (text === undefined) return undefined
  const file: OpenFile = {
    state: createState(name, text),
    trail: new EditTrail(),
    lexical: undefined,
    semantic: undefined,
    scroll: undefined,
  }
  open.set(name, file)
  followText(name, file, undefined)
  applyReport(name, analysisStore.state.report)
  return file
}

/**
 * Apply a change to a file's state: through the view when it is the one
 * shown, directly otherwise. Either way edits reach `edited`.
 */
function dispatchTo(
  name: string,
  change:
    TransactionSpec | ((state: EditorState) => TransactionSpec | Transaction)
) {
  const file = open.get(name)
  if (!file) return
  const spec = typeof change === "function" ? change(file.state) : change
  if (view && active === name && view.state === file.state) {
    view.dispatch(spec as TransactionSpec)
    return
  }
  const transaction =
    spec instanceof Transaction ? spec : file.state.update(spec)
  file.state = transaction.state
  if (transaction.docChanged) edited(name, file, transaction.changes)
}

function onUpdate(update: ViewUpdate) {
  const name = update.state.field(editedFile)
  const file = open.get(name)
  if (!file || view !== update.view) return
  file.state = update.state
  if (update.docChanged) edited(name, file, update.changes)
  if (update.selectionSet || update.docChanged) {
    editorCursorStore.setState(() => ({
      file: name,
      offset: update.state.selection.main.head,
    }))
  }
}

/** A file's text changed: send it, record it, and follow its tokens. */
function edited(name: string, file: OpenFile, changes: ChangeSet) {
  followText(name, file, changes.desc)
  const text = file.state.doc.toString()
  if (sessionText(name) !== text) setFileText(name, text)
}

/** Send a file's current text (if the worker lacks it) and ask for tokens. */
function followText(
  name: string,
  file: OpenFile,
  changes: Parameters<EditTrail["sent"]>[2]
) {
  const client = languageClient()
  if (!client) return
  const text = file.state.doc.toString()
  const sent = client.file(name)
  if (sent?.text === text) {
    file.trail.sent(sent.version, text.length, changes)
    void client.lexicalTokens(name).then((answer) => {
      if (answer) receiveLexical(name, answer)
    })
    return
  }
  const { version, tokens } = client.setFile(name, text)
  file.trail.sent(version, text.length, changes)
  void tokens.then((answer) => {
    if (answer) receiveLexical(name, answer)
  })
}

function receiveLexical(name: string, answer: LexicalTokens) {
  const file = open.get(name)
  if (!file?.trail.since(answer.version)) return
  if (file.lexical && file.lexical.version > answer.version) return
  file.lexical = answer
  file.trail.forgetBefore(answer.version)
  paintTokens(name, file)
}

function paintTokens(name: string, file: OpenFile) {
  const lexical = file.lexical
  if (!lexical) return
  const changes = file.trail.since(lexical.version)
  if (!changes) return
  const tokens =
    file.semantic?.version === lexical.version
      ? overlaySemantic(lexical.tokens, file.semantic.tokens)
      : lexical.tokens
  const ranges = tokenRanges(tokens, changes.length, changes)
  dispatchTo(name, { effects: setTokenRanges.of(ranges) })
}

// ---- Analysis ---------------------------------------------------------------

const SEVERITY = { error: "error", warning: "warning", note: "info" } as const

function lintDiagnostics(
  diagnostics: ReadonlyArray<Diagnostic>,
  name: string,
  length: number
): Array<LintDiagnostic> {
  return diagnostics.flatMap((diagnostic) => {
    const location = diagnostic.location
    if (location?.file !== name) return []
    const from = Math.min(location.span.start, length)
    const to = Math.min(Math.max(location.span.end, from), length)
    return [
      {
        from,
        to,
        severity: SEVERITY[diagnostic.severity],
        message: diagnostic.message,
        source: diagnostic.code,
      },
    ]
  })
}

/** Show an analysis's diagnostics and semantic tokens in one open file. */
function applyReport(name: string, report: AnalysisReport | undefined) {
  const file = open.get(name)
  const client = languageClient()
  if (!file || !report || !client) return
  const version = report.files.find((entry) => entry.name === name)?.version
  // The analysis must be of the text the state holds.
  if (version === undefined || version !== client.file(name)?.version) return
  if (client.file(name)?.text !== file.state.doc.toString()) return
  dispatchTo(name, (state) =>
    setDiagnostics(
      state,
      lintDiagnostics(report.diagnostics, name, state.doc.length)
    )
  )
  const semantic = report.semantic[name]
  if (semantic) {
    file.semantic = { version, tokens: semantic }
    paintTokens(name, file)
  }
}

// ---- Following the project --------------------------------------------------

/** The smallest change that turns `from` into `to`. */
function difference(from: string, to: string) {
  let start = 0
  const limit = Math.min(from.length, to.length)
  while (start < limit && from.charCodeAt(start) === to.charCodeAt(start)) {
    start++
  }
  let end = 0
  while (
    end < limit - start &&
    from.charCodeAt(from.length - 1 - end) ===
      to.charCodeAt(to.length - 1 - end)
  ) {
    end++
  }
  return {
    from: start,
    to: from.length - end,
    insert: to.slice(start, to.length - end),
  }
}

function followSession(state: SessionState) {
  if (state.status !== "open" || state.projectId !== projectId) {
    open.clear()
    linked.length = 0
    active = undefined
    projectId = state.status === "open" ? state.projectId : undefined
  }
  if (state.status !== "open") {
    showFile(undefined)
    return
  }

  const nextReadOnly = state.access === "read-only"
  if (nextReadOnly !== readOnly) {
    readOnly = nextReadOnly
    for (const name of open.keys()) {
      dispatchTo(name, { effects: setReadOnly(readOnly) })
    }
  }

  const texts = new Map(state.files.map((file) => [file.name, file.text]))
  for (const [name, file] of [...open]) {
    const text = texts.get(name)
    if (text === undefined || !state.ui.openTabs.includes(name)) {
      open.delete(name)
      continue
    }
    const current = file.state.doc.toString()
    if (text !== current) {
      // Changed outside this editor: another tab, or an import.
      dispatchTo(name, {
        changes: difference(current, text),
        annotations: [
          Transaction.addToHistory.of(false),
          Transaction.remote.of(true),
        ],
      })
    }
  }
  showFile(state.ui.activeFile ?? undefined)
}

/** Show `name` in the view (nothing when undefined). */
function showFile(name: string | undefined) {
  if (!view || name === active) return
  const previous = active ? open.get(active) : undefined
  if (previous && view.state === previous.state) {
    previous.scroll = view.scrollSnapshot()
  }
  const file = name ? openFile(name) : undefined
  active = file ? name : undefined
  closeOverlay()
  if (!file || !name) {
    view.setState(EditorState.create({ doc: "", extensions: kulEditorTheme }))
    editorCursorStore.setState(() => undefined)
    return
  }
  view.setState(file.state)
  if (file.scroll) view.dispatch({ effects: file.scroll })
  editorCursorStore.setState(() => ({
    file: name,
    offset: file.state.selection.main.head,
  }))
}

/**
 * Mount the editor in `parent`. Returns the function that unmounts it;
 * the open files' states survive, so remounting finds them as they were.
 *
 * The view is made once the editor's font has loaded: CodeMirror measures
 * character and line sizes when it starts, and a font that arrives later
 * would leave clicks and hovers landing beside the text.
 */
export function mountEditor(parent: HTMLElement): () => void {
  let unmounted = false
  let unmount: () => void = () => undefined
  const font = getComputedStyle(parent).getPropertyValue("--font-mono").trim()
  void document.fonts
    .load(`13px ${font || "monospace"}`)
    .catch(() => undefined)
    .then(() => {
      if (!unmounted) unmount = attachView(parent)
    })
  return () => {
    unmounted = true
    unmount()
  }
}

function attachView(parent: HTMLElement): () => void {
  // Made with the editor's theme, so its first measurements are of the
  // font and size every file shows in.
  view = new EditorView({
    parent,
    state: EditorState.create({ extensions: kulEditorTheme }),
  })
  active = undefined
  followSession(sessionStore.state)
  const session = sessionStore.subscribe(followSession)
  const analysis = analysisStore.subscribe(({ report }) => {
    for (const name of open.keys()) applyReport(name, report)
  })
  return () => {
    session.unsubscribe()
    analysis.unsubscribe()
    const current = active ? open.get(active) : undefined
    if (current && view) current.state = view.state
    view?.destroy()
    view = undefined
    active = undefined
  }
}

/** The text of the file the editor shows. */
export function shownText(): string | undefined {
  return active ? open.get(active)?.state.doc.toString() : undefined
}

/** The text selected in the file the editor shows. */
export function shownSelection(): string | undefined {
  const state = active ? open.get(active)?.state : undefined
  if (!state) return undefined
  const { from, to } = state.selection.main
  return state.sliceDoc(from, to)
}

/** Give the editor keyboard focus. */
export function focusEditor(): void {
  view?.focus()
}

/** Move an open file's state to its new name, before the file is renamed. */
export function renameProjectFile(from: string, to: string): Promise<void> {
  const file = open.get(from)
  if (file) {
    open.delete(from)
    open.set(to, file)
    if (active === from) active = to
    dispatchTo(to, { effects: renameEditedFile.of(to) })
  }
  return renameFile(from, to).then(() => {
    const moved = open.get(to)
    if (moved) followText(to, moved, undefined)
  })
}

// ---- Going places -----------------------------------------------------------

export interface ShowLocationOptions {
  /** Focus the editor (the default). The canvas reveals without focus. */
  focus?: boolean
}

/**
 * Open `location`'s file in a tab and select the range. When the editor is
 * minimized it stays so, and its pill flashes the file's name.
 */
export function showLocation(
  location: Location,
  { focus = true }: ShowLocationOptions = {}
): void {
  if (sessionText(location.file) === undefined) return
  void selectFile(location.file)
  openFile(location.file)
  showFile(location.file)
  dispatchTo(location.file, (state) => {
    const length = state.doc.length
    const anchor = Math.min(location.span.start, length)
    const head = Math.min(location.span.end, length)
    return {
      selection: EditorSelection.single(anchor, head),
      effects: EditorView.scrollIntoView(anchor, { y: "center" }),
    }
  })
  if (currentLayout().minimized) {
    flashPill(location.file)
  } else if (focus) {
    view?.focus()
  }
}

// ---- Edits across files -----------------------------------------------------

function toChanges(edits: ReadonlyArray<TextEdit>) {
  return edits.map((edit) => ({
    from: edit.span.start,
    to: edit.span.end,
    insert: edit.text,
  }))
}

function applyToText(text: string, edits: ReadonlyArray<TextEdit>): string {
  let out = ""
  let at = 0
  for (const edit of edits) {
    out += text.slice(at, edit.span.start) + edit.text
    at = edit.span.end
  }
  return out + text.slice(at)
}

function editsFit(text: string, edits: ReadonlyArray<TextEdit>): boolean {
  let at = 0
  for (const edit of edits) {
    if (edit.span.start < at || edit.span.end < edit.span.start) return false
    if (edit.span.end > text.length) return false
    at = edit.span.end
  }
  return true
}

/**
 * Apply edits to several files as one change: all of them or none. Open
 * files take one transaction each; closed files are written through the
 * session. Returns whether it applied.
 */
export function applyFileEdits(
  fileEdits: ReadonlyArray<FileEdit>,
  userEvent: string
): boolean {
  const changes = fileEdits.filter((fileEdit) => fileEdit.edits.length > 0)
  for (const fileEdit of changes) {
    const text = fileText(fileEdit.file)
    if (text === undefined || !editsFit(text, fileEdit.edits)) {
      toast.error("The change couldn’t be applied. Nothing was changed.")
      return false
    }
  }
  const group: LinkedEdit = { open: [], closed: [], undone: false }
  const closed: Array<{ name: string; text: string }> = []
  for (const fileEdit of changes) {
    const name = fileEdit.file
    if (open.has(name)) {
      dispatchTo(name, {
        changes: toChanges(fileEdit.edits),
        userEvent,
        annotations: isolateHistory.of("full"),
      })
      const state = open.get(name)?.state
      if (state) {
        group.open.push({
          name,
          undoDepth: undoDepth(state),
          redoDepth: redoDepth(state),
        })
      }
    } else {
      const before = sessionText(name) ?? ""
      const after = applyToText(before, fileEdit.edits)
      closed.push({ name, text: after })
      group.closed.push({ name, before, after })
    }
  }
  if (closed.length > 0) void writeFiles(closed)
  if (group.open.length + group.closed.length > 1) linked.push(group)
  return true
}

/** Undo (or redo) a change across files from any of its open files. */
function linkedHistory(direction: "undo" | "redo") {
  return (target: EditorView): boolean => {
    const name = target.state.field(editedFile)
    const depthOf = direction === "undo" ? undoDepth : redoDepth
    const group = [...linked].reverse().find((candidate) => {
      if (candidate.undone !== (direction === "redo")) return false
      const member = candidate.open.find((entry) => entry.name === name)
      if (!member) return false
      const depth = direction === "undo" ? member.undoDepth : member.redoDepth
      return depth === depthOf(target.state)
    })
    if (!group) return false
    const command = direction === "undo" ? undo : redo
    for (const member of group.open) {
      const file = open.get(member.name)
      if (!file) continue
      const depth = direction === "undo" ? member.undoDepth : member.redoDepth
      if (depthOf(file.state) !== depth) continue
      command({
        state: file.state,
        dispatch: (transaction) => dispatchTo(member.name, transaction),
      })
      member.undoDepth = undoDepth(file.state)
      member.redoDepth = redoDepth(file.state)
    }
    const restore = group.closed.flatMap((entry) => {
      const [from, to] =
        direction === "undo"
          ? [entry.after, entry.before]
          : [entry.before, entry.after]
      return sessionText(entry.name) === from
        ? [{ name: entry.name, text: to }]
        : []
    })
    if (restore.length > 0) void writeFiles(restore)
    group.undone = direction === "undo"
    return true
  }
}

// ---- The language features ------------------------------------------------

function anchorAt(target: EditorView, pos: number): Anchor {
  const coords = target.coordsAtPos(pos)
  const rect = target.contentDOM.getBoundingClientRect()
  return coords
    ? { x: coords.left, y: coords.top, height: coords.bottom - coords.top }
    : { x: rect.left, y: rect.top, height: 0 }
}

function query() {
  return languageClient()
}

const features: KulFeatures = {
  complete: (file, offset) =>
    query()?.query("complete", file, offset) ?? Promise.resolve(undefined),
  hover: (file, offset) =>
    query()?.query("hover", file, offset) ?? Promise.resolve(undefined),
  goToDefinitionAt: (target, offset) => {
    void goToDefinition(target.state.field(editedFile), offset)
  },
}

async function goToDefinition(file: string, offset: number) {
  const location = await query()?.query("definition", file, offset)
  if (location) showLocation(location)
}

function lineOf(text: string, offset: number) {
  const { line } = lineAndColumn(text, offset)
  const preview = text.split("\n")[line - 1]?.trim() ?? ""
  return { line, preview }
}

async function findReferences(target: EditorView) {
  const file = target.state.field(editedFile)
  const offset = target.state.selection.main.head
  const locations = await query()?.query("references", file, offset, true)
  if (!locations || view !== target) return
  const word = target.state.wordAt(offset)
  showOverlay({
    kind: "references",
    anchor: anchorAt(target, offset),
    id: word ? target.state.sliceDoc(word.from, word.to) : "",
    references: locations.map((location) => ({
      location,
      ...lineOf(fileText(location.file) ?? "", location.span.start),
    })),
  })
}

async function startRename(target: EditorView) {
  if (readOnly) return
  const file = target.state.field(editedFile)
  const offset = target.state.selection.main.head
  const span = await query()?.query("prepareRename", file, offset)
  if (view !== target) return
  if (!span) {
    toast("Put the cursor on a person or marriage id to rename it.")
    return
  }
  showOverlay({
    kind: "rename",
    file,
    offset,
    anchor: anchorAt(target, span.start),
    current: target.state.sliceDoc(span.start, span.end),
  })
}

/** Rename the id the rename box was opened on. Resolves to a problem, if any. */
export async function submitRename(
  file: string,
  offset: number,
  newName: string
): Promise<string | undefined> {
  const result = await query()?.query("rename", file, offset, newName)
  if (!result) return "The rename couldn’t be worked out. Try again."
  if (!result.ok) return result.message
  closeOverlay()
  applyFileEdits(result.edits, "rename")
  view?.focus()
  return undefined
}

async function showQuickFixes(target: EditorView) {
  const file = target.state.field(editedFile)
  const { from, to } = target.state.selection.main
  const fixes = await query()?.query("quickFixes", file, from, to)
  if (view !== target) return
  showOverlay({
    kind: "quick-fixes",
    file,
    anchor: anchorAt(target, from),
    fixes: readOnly ? [] : (fixes ?? []),
  })
}

/** Apply a quick fix's edits to its file. */
export function applyQuickFix(
  file: string,
  edits: ReadonlyArray<TextEdit>
): void {
  closeOverlay()
  applyFileEdits([{ file, edits: [...edits] }], "quickfix")
  view?.focus()
}

/** Format one file. */
export async function formatFile(file: string): Promise<void> {
  if (readOnly) return
  const edits = await query()?.query("format", file)
  if (edits === undefined) {
    toast.error(`Fix the syntax errors in ${file}, then format it.`)
    return
  }
  if (edits.length === 0) {
    toast(`${file} is already formatted.`)
    return
  }
  applyFileEdits([{ file, edits }], "format")
}

/** Format every file of the project, as one change. */
export async function formatAllFiles(): Promise<void> {
  if (readOnly) return
  const client = query()
  if (!client) return
  const names = sessionStore.state.files.map((file) => file.name)
  const results = await Promise.all(
    names.map(async (file) => ({
      file,
      edits: await client.query("format", file),
    }))
  )
  const skipped = results.filter((result) => result.edits === undefined)
  const fileEdits = results.flatMap((result) =>
    result.edits && result.edits.length > 0
      ? [{ file: result.file, edits: result.edits }]
      : []
  )
  if (fileEdits.length > 0) applyFileEdits(fileEdits, "format")
  if (skipped.length > 0) {
    toast.error(
      `Not formatted, because of syntax errors: ${skipped.map((s) => s.file).join(", ")}.`
    )
  } else if (fileEdits.length === 0) {
    toast("Every file is already formatted.")
  }
}

async function showSymbols(target: EditorView) {
  const file = target.state.field(editedFile)
  const symbols = await query()?.query("symbols", file)
  if (!symbols || view !== target) return
  showOverlay({ kind: "symbols", file, symbols })
}

/** Run an editor command on the shown file (from the palette). */
export function runOnEditor(command: (target: EditorView) => unknown): void {
  if (view && active) command(view)
}

/** The editor's own shortcuts, from the app's table. */
const EDITOR_SHORTCUTS: ReadonlyArray<
  readonly [AppHotkeyId, (target: EditorView) => unknown]
> = [
  [
    "editor.definition",
    (target) =>
      goToDefinition(
        target.state.field(editedFile),
        target.state.selection.main.head
      ),
  ],
  ["editor.references", findReferences],
  ["editor.rename", startRename],
  ["editor.quickFix", showQuickFixes],
  ["editor.format", (target) => formatFile(target.state.field(editedFile))],
  ["editor.symbols", showSymbols],
]

/**
 * The editor's commands. Language features match keys the way the app's
 * shortcuts do (by physical key where a layout types a character, as
 * Option+Shift+F does on a Mac); undo and redo follow CodeMirror's own.
 */
const editorCommands: Extension = [
  Prec.highest(
    EditorView.domEventHandlers({
      keydown(event, target) {
        for (const [id, command] of EDITOR_SHORTCUTS) {
          if (matchesKeyboardEvent(event, appHotkeys[id].defaultHotkey)) {
            event.preventDefault()
            void command(target)
            return true
          }
        }
        return false
      },
    })
  ),
  Prec.high(
    keymap.of([
      { key: "Mod-z", run: linkedHistory("undo"), preventDefault: true },
      { key: "Mod-y", run: linkedHistory("redo"), preventDefault: true },
      { key: "Mod-Shift-z", run: linkedHistory("redo"), preventDefault: true },
    ])
  ),
]

export const editorFeatureCommands = {
  goToDefinition: (target: EditorView) =>
    goToDefinition(
      target.state.field(editedFile),
      target.state.selection.main.head
    ),
  findReferences,
  startRename,
  showQuickFixes,
  showSymbols,
}
