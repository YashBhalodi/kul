// The editor's CodeMirror extensions for Kul. Nothing here knows Kul's
// grammar: highlighting comes from the engine's tokens, and completion,
// hover, diagnostics and the rest come from the language worker through
// `KulFeatures`. Each open file has its own EditorState built from these.

import {
  autocompletion,
  closeBrackets,
  closeBracketsKeymap,
  completionKeymap,
  pickedCompletion,
} from "@codemirror/autocomplete"
import type {
  Completion,
  CompletionContext,
  CompletionResult,
} from "@codemirror/autocomplete"
import {
  defaultKeymap,
  history,
  historyKeymap,
  indentWithTab,
} from "@codemirror/commands"
import { indentOnInput, indentService, indentUnit } from "@codemirror/language"
import { lintGutter, lintKeymap } from "@codemirror/lint"
import {
  highlightSelectionMatches,
  search,
  searchKeymap,
} from "@codemirror/search"
import {
  Compartment,
  EditorState,
  RangeSetBuilder,
  StateEffect,
  StateField,
} from "@codemirror/state"
import type { Extension } from "@codemirror/state"
import {
  Decoration,
  EditorView,
  ViewPlugin,
  crosshairCursor,
  drawSelection,
  dropCursor,
  highlightActiveLine,
  highlightActiveLineGutter,
  highlightSpecialChars,
  hoverTooltip,
  keymap,
  lineNumbers,
  rectangularSelection,
} from "@codemirror/view"
import type { DecorationSet, ViewUpdate } from "@codemirror/view"

import type { CompletionItem, Hover } from "@/language/protocol"
import { KUL_INDENT, holdsStatement, kulIndentation } from "./indent"
import { renderMarkdown } from "./markdown"
import type { TokenRange } from "./tokens"

/** What the editor asks the language worker. */
export interface KulFeatures {
  complete: (
    file: string,
    offset: number
  ) => Promise<Array<CompletionItem> | undefined>
  hover: (file: string, offset: number) => Promise<Hover | undefined>
  /** Go to the declaration of the id at `offset` (Mod+Click). */
  goToDefinitionAt: (view: EditorView, offset: number) => void
}

// ---- The file a state edits -------------------------------------------

/** Rename the file a state edits. */
export const renameEditedFile = StateEffect.define<string>()

/** The name of the project file a state edits. */
export const editedFile = StateField.define<string>({
  create: () => "",
  update(name, transaction) {
    for (const effect of transaction.effects) {
      if (effect.is(renameEditedFile)) name = effect.value
    }
    return name
  },
})

// ---- Highlighting from tokens -----------------------------------------

/** Replace the highlighting with token ranges for the current text. */
export const setTokenRanges = StateEffect.define<ReadonlyArray<TokenRange>>()

const markCache = new Map<string, Decoration>()
function mark(className: string): Decoration {
  let decoration = markCache.get(className)
  if (!decoration) {
    decoration = Decoration.mark({ class: className })
    markCache.set(className, decoration)
  }
  return decoration
}

/**
 * The highlighting. Between token answers it is mapped through every
 * edit, so typed text never flashes unstyled.
 */
export const tokenHighlights = StateField.define<DecorationSet>({
  create: () => Decoration.none,
  update(decorations, transaction) {
    let next = decorations.map(transaction.changes)
    for (const effect of transaction.effects) {
      if (effect.is(setTokenRanges)) {
        const builder = new RangeSetBuilder<Decoration>()
        for (const range of effect.value) {
          builder.add(range.from, range.to, mark(range.className))
        }
        next = builder.finish()
      }
    }
    return next
  },
  provide: (field) => EditorView.decorations.from(field),
})

/** The highlighted string around `pos` when the cursor touches one of its quotes. */
function stringAt(state: EditorState, pos: number) {
  let found: { from: number; to: number } | undefined
  state
    .field(tokenHighlights)
    .between(Math.max(0, pos - 1), pos + 1, (from, to, value) => {
      const spec = value.spec as { class?: string }
      if (spec.class === "tok-string" && (pos === from || pos === to)) {
        found = { from, to }
        return false
      }
      return undefined
    })
  if (!found || found.to - found.from < 2) return undefined
  const text = state.sliceDoc(found.from, found.to)
  return text.startsWith('"') && text.endsWith('"') ? found : undefined
}

const matchingQuote = Decoration.mark({ class: "cm-matchingBracket" })

/** Highlight both quotes of a string while the cursor sits at either. */
const quoteMatching = ViewPlugin.fromClass(
  class {
    decorations: DecorationSet
    constructor(view: EditorView) {
      this.decorations = this.build(view.state)
    }
    update(update: ViewUpdate) {
      this.decorations = this.build(update.state)
    }
    build(state: EditorState): DecorationSet {
      const range = state.selection.main
      if (!range.empty) return Decoration.none
      const found = stringAt(state, range.head)
      if (!found) return Decoration.none
      return Decoration.set([
        matchingQuote.range(found.from, found.from + 1),
        matchingQuote.range(found.to - 1, found.to),
      ])
    }
  },
  { decorations: (plugin) => plugin.decorations }
)

// ---- Completion ----------------------------------------------------------

const WORD_BEFORE = /[\p{L}\p{N}_-]+$/u
const COMPLETION_ICON: Readonly<Record<CompletionItem["kind"], string>> = {
  keyword: "keyword",
  field: "property",
  enumValue: "enum",
  value: "text",
  marriage: "namespace",
  person: "variable",
}

function toCompletion(item: CompletionItem): Completion {
  const { insert } = item
  return {
    label: item.label,
    detail: item.detail,
    type: COMPLETION_ICON[item.kind],
    boost: item.preselect ? 50 : 0,
    apply: insert
      ? (view, completion, from, to) => {
          view.dispatch({
            changes: { from, to, insert: insert.text },
            selection: { anchor: from + insert.cursor },
            annotations: pickedCompletion.of(completion),
            userEvent: "input.complete",
          })
        }
      : undefined,
  }
}

function completionSource(features: KulFeatures) {
  return async (
    context: CompletionContext
  ): Promise<CompletionResult | null> => {
    const line = context.state.doc.lineAt(context.pos)
    const typed = WORD_BEFORE.exec(line.text.slice(0, context.pos - line.from))
    const from = context.pos - (typed?.[0].length ?? 0)
    const before = context.state.sliceDoc(from - 1, from)
    // Offer completions while a word is typed, after `:` and after a space.
    if (
      !context.explicit &&
      from === context.pos &&
      before !== ":" &&
      before !== " "
    ) {
      return null
    }
    const items = await features.complete(
      context.state.field(editedFile),
      context.pos
    )
    if (context.aborted || !items || items.length === 0) return null
    return {
      from,
      options: items.map(toCompletion),
      validFor: /^[\p{L}\p{N}_-]*$/u,
    }
  }
}

// ---- Hover -----------------------------------------------------------------

function hoverSource(features: KulFeatures) {
  return hoverTooltip(
    async (view, pos) => {
      const hover = await features.hover(view.state.field(editedFile), pos)
      if (!hover) return null
      const length = view.state.doc.length
      return {
        pos: Math.min(hover.span.start, length),
        end: Math.min(hover.span.end, length),
        above: true,
        create() {
          const dom = document.createElement("div")
          dom.className = "cm-kul-hover"
          dom.setAttribute("data-testid", "hover")
          renderMarkdown(dom, hover.markdown)
          return { dom }
        },
      }
    },
    { hoverTime: 250 }
  )
}

// ---- Indentation, comments, quotes ---------------------------------------

const kulIndent = indentService.of((context, pos) => {
  const line = context.state.doc.lineAt(pos)
  let above: string | undefined
  for (let number = line.number - 1; number >= 1; number--) {
    const text = context.lineAt(context.state.doc.line(number).from).text
    if (holdsStatement(text)) {
      above = text
      break
    }
  }
  return kulIndentation(context.lineAt(pos).text, above)
})

const kulLanguageData = EditorState.languageData.of(() => [
  {
    commentTokens: { line: "#" },
    closeBrackets: { brackets: ['"'] },
    // Typing the space after a top-level keyword re-indents its line.
    indentOnInput: /^\s*(person|marriage)\s$/,
  },
])

// ---- Read-only mode ------------------------------------------------------

const readOnlyMode = new Compartment()

function readOnlyExtensions(readOnly: boolean): Extension {
  return readOnly ? [EditorState.readOnly.of(true)] : []
}

/** Switch a state between editable and read-only. */
export function setReadOnly(readOnly: boolean) {
  return readOnlyMode.reconfigure(readOnlyExtensions(readOnly))
}

// ---- Putting it together ---------------------------------------------------

export interface KulEditorOptions {
  file: string
  readOnly: boolean
  features: KulFeatures
  /** The editor's own commands (go to definition, rename, ...). */
  commands: Extension
  /** Reports each update, for the app to follow edits and the cursor. */
  onUpdate: (update: ViewUpdate) => void
  theme: Extension
}

/** Every extension one file's EditorState needs. */
export function kulEditorExtensions(options: KulEditorOptions): Extension {
  const { features } = options
  return [
    editedFile.init(() => options.file),
    tokenHighlights,
    readOnlyMode.of(readOnlyExtensions(options.readOnly)),
    lineNumbers(),
    lintGutter(),
    highlightActiveLineGutter(),
    highlightSpecialChars(),
    history(),
    drawSelection(),
    dropCursor(),
    EditorState.allowMultipleSelections.of(true),
    // Mod+Click goes to a declaration; Alt+Click adds a cursor.
    EditorView.clickAddsSelectionRange.of((event) => event.altKey),
    rectangularSelection(),
    crosshairCursor(),
    highlightActiveLine(),
    highlightSelectionMatches(),
    quoteMatching,
    indentUnit.of(KUL_INDENT),
    kulIndent,
    indentOnInput(),
    kulLanguageData,
    closeBrackets(),
    autocompletion({
      override: [completionSource(features)],
      activateOnTypingDelay: 50,
      activateOnCompletion: (completion) => completion.type === "property",
      icons: true,
    }),
    hoverSource(features),
    search({ top: true }),
    EditorView.domEventHandlers({
      mousedown(event, view) {
        const mod = /Mac/.test(navigator.platform)
          ? event.metaKey
          : event.ctrlKey
        if (!mod || event.button !== 0) return false
        const pos = view.posAtCoords({ x: event.clientX, y: event.clientY })
        if (pos === null) return false
        event.preventDefault()
        features.goToDefinitionAt(view, pos)
        return true
      },
    }),
    options.commands,
    keymap.of([
      ...closeBracketsKeymap,
      ...defaultKeymap,
      ...searchKeymap,
      ...historyKeymap,
      ...completionKeymap,
      ...lintKeymap,
      indentWithTab,
    ]),
    EditorView.updateListener.of(options.onUpdate),
    EditorView.contentAttributes.compute([editedFile], (state) => ({
      "aria-label": `Contents of ${state.field(editedFile)}`,
      // In the tab order explicitly, so the scrolling area reads as
      // keyboard-reachable.
      tabindex: "0",
    })),
    options.theme,
  ]
}
