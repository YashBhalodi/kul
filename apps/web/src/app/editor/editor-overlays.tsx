import { useSelector } from "@tanstack/react-store"
import { LightbulbIcon, ListTreeIcon } from "lucide-react"
import { useEffect, useRef, useState } from "react"

import {
  Command,
  CommandDialog,
  CommandEmpty,
  CommandInput,
  CommandItem,
  CommandList,
} from "@/components/ui/command"
import {
  DropdownMenu,
  DropdownMenuContent,
  DropdownMenuGroup,
  DropdownMenuItem,
  DropdownMenuLabel,
} from "@/components/ui/dropdown-menu"
import type { OutlineSymbol } from "@/language/protocol"
import {
  applyQuickFix,
  focusEditor,
  showLocation,
  submitRename,
} from "./editor-controller"
import { closeOverlay, editorUiStore } from "./editor-store"
import { InlineCode } from "./inline-code"
import type { Anchor, EditorOverlay } from "./editor-store"

/** A point on screen for a popup to open from, below the text it is about. */
function virtualAnchor(anchor: Anchor) {
  return {
    getBoundingClientRect: () =>
      new DOMRect(anchor.x, anchor.y, 0, anchor.height),
  }
}

function dismiss() {
  closeOverlay()
  focusEditor()
}

/** The editor's popups: quick fixes, references, the rename box, symbols. */
export function EditorOverlays() {
  const overlay = useSelector(editorUiStore, (state) => state.overlay)
  if (!overlay) return null
  switch (overlay.kind) {
    case "quick-fixes":
      return <QuickFixMenu overlay={overlay} />
    case "references":
      return <ReferencesMenu overlay={overlay} />
    case "rename":
      return (
        <RenameBox
          key={`${overlay.file}:${overlay.offset}`}
          overlay={overlay}
        />
      )
    case "symbols":
      return <SymbolList overlay={overlay} />
  }
}

function QuickFixMenu({
  overlay,
}: {
  overlay: Extract<EditorOverlay, { kind: "quick-fixes" }>
}) {
  return (
    <DropdownMenu
      open
      onOpenChange={(open) => {
        if (!open) dismiss()
      }}
    >
      <DropdownMenuContent
        anchor={virtualAnchor(overlay.anchor)}
        className="w-auto max-w-md min-w-64"
        data-testid="quick-fix-menu"
      >
        <DropdownMenuGroup>
          <DropdownMenuLabel className="flex items-center gap-1.5">
            <LightbulbIcon className="size-3.5 text-warning" aria-hidden />
            Quick fixes
          </DropdownMenuLabel>
          {overlay.fixes.length === 0 && (
            <DropdownMenuItem disabled>No quick fixes here</DropdownMenuItem>
          )}
          {overlay.fixes.map((fix) => (
            <DropdownMenuItem
              key={fix.title}
              onClick={() => applyQuickFix(overlay.file, fix.edits)}
            >
              <InlineCode text={fix.title} />
            </DropdownMenuItem>
          ))}
        </DropdownMenuGroup>
      </DropdownMenuContent>
    </DropdownMenu>
  )
}

function ReferencesMenu({
  overlay,
}: {
  overlay: Extract<EditorOverlay, { kind: "references" }>
}) {
  const count = overlay.references.length
  return (
    <DropdownMenu
      open
      onOpenChange={(open) => {
        if (!open) dismiss()
      }}
    >
      <DropdownMenuContent
        anchor={virtualAnchor(overlay.anchor)}
        className="w-auto max-w-lg min-w-72"
        data-testid="references-menu"
      >
        <DropdownMenuGroup>
          <DropdownMenuLabel>
            {count === 1 ? "1 place uses" : `${count} places use`}{" "}
            <code className="font-mono">{overlay.id}</code>
          </DropdownMenuLabel>
          {overlay.references.map((reference) => (
            <DropdownMenuItem
              key={`${reference.location.file}:${reference.location.span.start}`}
              onClick={() => {
                closeOverlay()
                showLocation(reference.location)
              }}
              className="flex flex-col items-start gap-0.5"
            >
              <span className="font-mono text-xs text-muted-foreground">
                {reference.location.file}:{reference.line}
              </span>
              <span className="max-w-full truncate font-mono text-[0.8125rem]">
                {reference.preview}
              </span>
            </DropdownMenuItem>
          ))}
        </DropdownMenuGroup>
      </DropdownMenuContent>
    </DropdownMenu>
  )
}

function RenameBox({
  overlay,
}: {
  overlay: Extract<EditorOverlay, { kind: "rename" }>
}) {
  const input = useRef<HTMLInputElement>(null)
  const [problem, setProblem] = useState<string>()
  const busy = useRef(false)

  useEffect(() => {
    input.current?.focus()
    input.current?.select()
  }, [])

  const submit = async () => {
    const name = input.current?.value.trim() ?? ""
    if (name === overlay.current) {
      dismiss()
      return
    }
    busy.current = true
    const result = await submitRename(overlay.file, overlay.offset, name)
    busy.current = false
    if (result) {
      setProblem(result)
      input.current?.focus()
    }
  }

  return (
    <div
      data-testid="rename-box"
      className="pointer-events-auto fixed z-50 flex w-72 flex-col gap-1.5 rounded-lg border bg-popover p-2 text-popover-foreground shadow-lg"
      style={{
        left: overlay.anchor.x - 8,
        top: overlay.anchor.y + overlay.anchor.height + 4,
      }}
    >
      <label
        className="text-xs font-medium text-muted-foreground"
        htmlFor="rename-input"
      >
        Rename <code className="font-mono">{overlay.current}</code> everywhere
      </label>
      <input
        id="rename-input"
        ref={input}
        defaultValue={overlay.current}
        aria-invalid={problem ? true : undefined}
        aria-describedby={problem ? "rename-problem" : undefined}
        spellCheck={false}
        autoComplete="off"
        onKeyDown={(event) => {
          if (event.key === "Enter") {
            event.preventDefault()
            void submit()
          } else if (event.key === "Escape") {
            event.preventDefault()
            dismiss()
          }
        }}
        onBlur={() => {
          if (!busy.current) closeOverlay()
        }}
        className="h-8 rounded-md border border-input bg-background px-2 font-mono text-sm outline-none focus-visible:border-ring focus-visible:ring-3 focus-visible:ring-ring/50 aria-invalid:border-destructive"
      />
      {problem ? (
        <p
          id="rename-problem"
          role="alert"
          className="text-xs text-destructive"
        >
          {problem}
        </p>
      ) : (
        <p className="text-xs text-muted-foreground">
          Enter to rename in every file, Esc to cancel.
        </p>
      )}
    </div>
  )
}

interface SymbolRow {
  symbol: OutlineSymbol
  depth: number
}

function flatten(
  symbols: ReadonlyArray<OutlineSymbol>,
  depth = 0
): Array<SymbolRow> {
  return symbols.flatMap((symbol) => [
    { symbol, depth },
    ...flatten(symbol.children, depth + 1),
  ])
}

function SymbolList({
  overlay,
}: {
  overlay: Extract<EditorOverlay, { kind: "symbols" }>
}) {
  const input = useRef<HTMLInputElement>(null)
  const rows = flatten(overlay.symbols)
  return (
    <CommandDialog
      open
      onOpenChange={(open) => {
        if (!open) dismiss()
      }}
      title="Go to a person or marriage"
      description={`Everything declared in ${overlay.file}`}
      className="sm:max-w-lg"
      initialFocus={input}
    >
      <Command>
        <CommandInput
          ref={input}
          placeholder={`Go to a person or marriage in ${overlay.file}…`}
        />
        <CommandList className="max-h-96" data-testid="symbol-list">
          <CommandEmpty>Nothing declared matches that search.</CommandEmpty>
          {rows.map(({ symbol, depth }, index) => (
            <CommandItem
              key={`${symbol.selection.start}-${index}`}
              value={`${index}`}
              keywords={[symbol.name, symbol.kind, symbol.detail ?? ""]}
              onSelect={() => {
                closeOverlay()
                showLocation({ file: overlay.file, span: symbol.selection })
              }}
              style={{ paddingLeft: `${0.5 + depth * 1.25}rem` }}
            >
              <ListTreeIcon className="text-muted-foreground" aria-hidden />
              <span className="truncate">{symbol.name}</span>
              <span className="text-xs text-muted-foreground">
                {symbol.kind}
              </span>
              {symbol.detail && (
                <span className="ml-auto text-xs text-muted-foreground tabular-nums">
                  {symbol.detail}
                </span>
              )}
            </CommandItem>
          ))}
        </CommandList>
      </Command>
    </CommandDialog>
  )
}
