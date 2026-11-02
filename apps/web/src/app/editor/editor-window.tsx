import { useHotkey } from "@tanstack/react-hotkeys"
import { useSelector } from "@tanstack/react-store"
import { cn } from "cn"
import {
  CodeIcon,
  EyeIcon,
  MinusIcon,
  PanelLeftIcon,
  PanelRightIcon,
} from "lucide-react"
import { useEffect, useLayoutEffect, useRef, useState } from "react"
import type { KeyboardEvent, PointerEvent, ReactNode } from "react"

import { Button } from "@/components/ui/button"
import {
  Tooltip,
  TooltipContent,
  TooltipTrigger,
} from "@/components/ui/tooltip"
import type { Dock, EditorWindowLayout } from "@/store/editor-window"
import { sessionStore, takeOverProject } from "../project-session"
import { appHotkeys } from "../shortcuts"
import { mountEditor } from "./editor-controller"
import { useEditorCommands } from "./editor-commands"
import {
  canvasSizeStore,
  currentLayout,
  setLayout,
  setMinimized,
  toggleEditor,
} from "./editor-layout"
import { EditorOverlays } from "./editor-overlays"
import { EditorPill } from "./editor-pill"
import { EditorRail } from "./editor-rail"
import { editorUiStore, setDockPreview } from "./editor-store"
import { EditorTabs } from "./editor-tabs"
import { ProblemsStrip } from "./problems-strip"
import {
  MIN_WIDTH,
  UNDOCK_DISTANCE,
  dockAt,
  dockedEdge,
  keepOnCanvas,
  moveBy,
  resizeBy,
  resizeDocked,
  undockUnder,
  windowRect,
} from "./window-geometry"
import type { Edge, Rect, Size } from "./window-geometry"

/**
 * The editor window over the canvas: the file list, tabs, the editor and
 * the problems strip. It floats, docks to either side, and minimizes to a
 * pill; its layout is saved with the project.
 */
export function EditorWindow() {
  const layer = useRef<HTMLDivElement>(null)
  const stored = useSelector(sessionStore, (state) => state.ui.editorWindow)
  const canvas = useSelector(canvasSizeStore)
  const layout = currentLayout(stored, canvas)
  const [live, setLive] = useState<Rect | null>(null)

  useEditorCommands()
  useHotkey(appHotkeys["editor.toggle"], toggleEditor)

  useLayoutEffect(() => {
    const element = layer.current
    if (!element) return
    const measure = () =>
      canvasSizeStore.setState(() => ({
        width: element.clientWidth,
        height: element.clientHeight,
      }))
    measure()
    const observer = new ResizeObserver(measure)
    observer.observe(element)
    return () => observer.disconnect()
  }, [])

  const rect = live ?? windowRect(layout, canvas)

  return (
    <div
      ref={layer}
      className="pointer-events-none absolute inset-0 z-10"
      data-testid="editor-layer"
    >
      <DockPreview canvas={canvas} />
      <section
        aria-label="Editor"
        data-testid="editor-window"
        data-dock={layout.dock ?? "none"}
        hidden={layout.minimized}
        style={{
          left: rect.x,
          top: rect.y,
          width: rect.width,
          height: rect.height,
        }}
        className={cn(
          "pointer-events-auto absolute flex flex-col overflow-hidden border bg-card text-card-foreground shadow-xl",
          layout.dock
            ? layout.dock === "left"
              ? "border-y-0 border-l-0"
              : "border-y-0 border-r-0"
            : "rounded-xl"
        )}
      >
        <TitleBar
          layout={layout}
          rect={rect}
          canvas={canvas}
          layer={layer}
          onLive={setLive}
        />
        <ReadOnlyNotice />
        <div className="flex min-h-0 flex-1">
          <EditorRail />
          <div className="flex min-w-0 flex-1 flex-col border-l">
            <EditorTabs />
            <EditorHost />
          </div>
        </div>
        <ProblemsStrip />
        <ResizeHandles
          layout={layout}
          rect={rect}
          canvas={canvas}
          onLive={setLive}
        />
      </section>
      <EditorPill />
      <EditorOverlays />
    </div>
  )
}

function EditorHost() {
  const host = useRef<HTMLDivElement>(null)
  const empty = useSelector(sessionStore, (state) => !state.ui.activeFile)
  useEffect(() => {
    const element = host.current
    if (!element) return
    return mountEditor(element)
  }, [])
  return (
    <div className="relative min-h-0 flex-1">
      <div ref={host} className="absolute inset-0" data-testid="editor" />
      {empty && (
        <div className="absolute inset-0 flex items-center justify-center bg-card p-6 text-center text-sm text-muted-foreground">
          Choose a file from the list to edit it.
        </div>
      )}
    </div>
  )
}

function ReadOnlyNotice() {
  const readOnly = useSelector(
    sessionStore,
    (state) => state.status === "open" && state.access === "read-only"
  )
  if (!readOnly) return null
  return (
    <div
      role="status"
      data-testid="read-only-banner"
      className="flex shrink-0 items-center gap-3 border-b bg-muted py-2 pr-2 pl-3 text-sm"
    >
      <EyeIcon className="size-4 shrink-0 text-muted-foreground" aria-hidden />
      <p className="flex-1">
        This project is open in another tab. Changes made there show here.
      </p>
      <Button size="sm" onClick={() => void takeOverProject()}>
        Use here instead
      </Button>
    </div>
  )
}

/** Where the window would dock, shown while it is dragged near a side. */
function DockPreview({ canvas }: { canvas: Size }) {
  const dock = useSelector(editorUiStore, (state) => state.dockPreview)
  if (!dock) return null
  const width = Math.min(currentLayout().width, canvas.width)
  return (
    <div
      aria-hidden
      data-testid="dock-preview"
      className="absolute top-0 h-full border-2 border-ring/60 bg-primary/10"
      style={{ width, left: dock === "left" ? 0 : canvas.width - width }}
    />
  )
}

interface DragProps {
  layout: EditorWindowLayout
  rect: Rect
  canvas: Size
  onLive: (rect: Rect | null) => void
}

function TitleBar({
  layout,
  rect,
  canvas,
  layer,
  onLive,
}: DragProps & { layer: React.RefObject<HTMLDivElement | null> }) {
  const activeFile = useSelector(sessionStore, (state) => state.ui.activeFile)
  const drag = useRef<{
    pointerId: number
    startX: number
    startY: number
    start: Rect
    floating: boolean
    dock: Dock | null
    rect: Rect
  } | null>(null)

  const local = (event: PointerEvent) => {
    const box = layer.current?.getBoundingClientRect()
    return {
      x: event.clientX - (box?.left ?? 0),
      y: event.clientY - (box?.top ?? 0),
    }
  }

  const onPointerDown = (event: PointerEvent<HTMLElement>) => {
    if (event.button !== 0) return
    if ((event.target as HTMLElement).closest("button")) return
    event.currentTarget.setPointerCapture(event.pointerId)
    drag.current = {
      pointerId: event.pointerId,
      startX: event.clientX,
      startY: event.clientY,
      start: rect,
      floating: layout.dock === null,
      dock: null,
      rect,
    }
  }

  const onPointerMove = (event: PointerEvent<HTMLElement>) => {
    const current = drag.current
    if (current?.pointerId !== event.pointerId) return
    const dx = event.clientX - current.startX
    const dy = event.clientY - current.startY
    if (!current.floating) {
      if (Math.hypot(dx, dy) < UNDOCK_DISTANCE) return
      // Pulled away from its side: float under the pointer.
      const pointer = local(event)
      current.start = undockUnder(layout, rect, pointer, canvas)
      current.startX = event.clientX
      current.startY = event.clientY
      current.floating = true
    }
    current.rect = moveBy(
      current.start,
      event.clientX - current.startX,
      event.clientY - current.startY,
      canvas
    )
    current.dock = dockAt(local(event).x, canvas)
    setDockPreview(current.dock)
    onLive(current.rect)
  }

  const onPointerUp = (event: PointerEvent<HTMLElement>) => {
    const current = drag.current
    if (current?.pointerId !== event.pointerId) return
    drag.current = null
    setDockPreview(null)
    onLive(null)
    if (!current.floating) return
    const moved = current.rect
    if (current.dock) {
      // Docked: keep the floating place to return to.
      const floating = layout.dock ? current.start : layout
      setLayout(() => ({
        ...layout,
        x: floating.x,
        y: floating.y,
        height: floating.height,
        dock: current.dock,
      }))
    } else {
      setLayout(() => ({ ...layout, ...moved, dock: null }))
    }
  }

  const dockTo = (dock: Dock | null) =>
    setLayout((current) => ({ ...current, dock }))

  return (
    <header
      data-testid="editor-title-bar"
      onPointerDown={onPointerDown}
      onPointerMove={onPointerMove}
      onPointerUp={onPointerUp}
      onPointerCancel={onPointerUp}
      className="flex h-10 shrink-0 cursor-grab touch-none items-center gap-2 border-b bg-muted/60 pr-1.5 pl-3 select-none active:cursor-grabbing"
    >
      <CodeIcon className="size-4 text-muted-foreground" aria-hidden />
      <h2 className="text-sm font-medium">Editor</h2>
      {activeFile && (
        <span className="min-w-0 truncate font-mono text-xs text-muted-foreground">
          {activeFile}
        </span>
      )}
      <div className="ml-auto flex items-center gap-0.5">
        <TitleButton
          label={
            layout.dock === "left" ? "Float the editor" : "Dock to the left"
          }
          pressed={layout.dock === "left"}
          onClick={() => dockTo(layout.dock === "left" ? null : "left")}
        >
          <PanelLeftIcon />
        </TitleButton>
        <TitleButton
          label={
            layout.dock === "right" ? "Float the editor" : "Dock to the right"
          }
          pressed={layout.dock === "right"}
          onClick={() => dockTo(layout.dock === "right" ? null : "right")}
        >
          <PanelRightIcon />
        </TitleButton>
        <TitleButton
          label="Minimize the editor"
          onClick={() => setMinimized(true)}
        >
          <MinusIcon />
        </TitleButton>
      </div>
    </header>
  )
}

function TitleButton({
  label,
  pressed,
  onClick,
  children,
}: {
  label: string
  pressed?: boolean
  onClick: () => void
  children: ReactNode
}) {
  return (
    <Tooltip>
      <TooltipTrigger
        render={
          <Button
            variant="ghost"
            size="icon-sm"
            aria-label={label}
            aria-pressed={pressed}
            onClick={onClick}
            className="aria-pressed:bg-accent aria-pressed:text-accent-foreground"
          />
        }
      >
        {children}
      </TooltipTrigger>
      <TooltipContent>{label}</TooltipContent>
    </Tooltip>
  )
}

const EDGE_CLASS: Readonly<Record<Edge, string>> = {
  n: "top-0 inset-x-3 h-1.5 cursor-ns-resize",
  s: "bottom-0 inset-x-3 h-1.5 cursor-ns-resize",
  e: "right-0 inset-y-3 w-1.5 cursor-ew-resize",
  w: "left-0 inset-y-3 w-1.5 cursor-ew-resize",
  ne: "top-0 right-0 size-3 cursor-nesw-resize",
  nw: "top-0 left-0 size-3 cursor-nwse-resize",
  se: "bottom-0 right-0 size-3 cursor-nwse-resize",
  sw: "bottom-0 left-0 size-3 cursor-nesw-resize",
}

/** How far one arrow key press resizes the window. */
const KEY_STEP = 16

function ResizeHandles({ layout, rect, canvas, onLive }: DragProps) {
  /** Save a resize: a docked window's width, or a floating window's rect. */
  const commit = (result: Rect, dx: number) => {
    if (layout.dock) setLayout(() => resizeDocked(layout, dx, canvas))
    else setLayout(() => ({ ...layout, ...keepOnCanvas(result, canvas) }))
  }
  const props = { rect, canvas, onLive, onCommit: commit }
  if (layout.dock) {
    return <ResizeHandle edge={dockedEdge(layout.dock)} keyboard {...props} />
  }
  return (
    <>
      {(["n", "s", "e", "w", "ne", "nw", "sw"] as const).map((edge) => (
        <ResizeHandle key={edge} edge={edge} keyboard={false} {...props} />
      ))}
      <ResizeHandle edge="se" keyboard {...props} />
    </>
  )
}

/**
 * One edge or corner to resize from. The ones that take the keyboard
 * (the bottom-right corner, or a docked window's inner edge) resize with
 * the arrow keys.
 */
function ResizeHandle({
  edge,
  keyboard,
  rect,
  canvas,
  onLive,
  onCommit,
}: {
  edge: Edge
  keyboard: boolean
  rect: Rect
  canvas: Size
  onLive: (rect: Rect | null) => void
  onCommit: (result: Rect, dx: number) => void
}) {
  const drag = useRef<{
    pointerId: number
    startX: number
    startY: number
    rect: Rect
    result: Rect
  } | null>(null)

  return (
    <div
      data-testid={`resize-${edge}`}
      role={keyboard ? "separator" : undefined}
      aria-label={keyboard ? "Resize the editor" : undefined}
      aria-orientation={keyboard ? "vertical" : undefined}
      aria-valuenow={keyboard ? Math.round(rect.width) : undefined}
      aria-valuemin={keyboard ? MIN_WIDTH : undefined}
      aria-valuemax={keyboard ? Math.round(canvas.width) : undefined}
      tabIndex={keyboard ? 0 : undefined}
      className={cn(
        "absolute z-10 touch-none focus-visible:bg-ring/50 focus-visible:outline-none",
        EDGE_CLASS[edge]
      )}
      onPointerDown={(event) => {
        if (event.button !== 0) return
        event.currentTarget.setPointerCapture(event.pointerId)
        drag.current = {
          pointerId: event.pointerId,
          startX: event.clientX,
          startY: event.clientY,
          rect,
          result: rect,
        }
      }}
      onPointerMove={(event) => {
        const current = drag.current
        if (current?.pointerId !== event.pointerId) return
        current.result = resizeBy(
          current.rect,
          edge,
          event.clientX - current.startX,
          event.clientY - current.startY,
          canvas
        )
        onLive(current.result)
      }}
      onPointerUp={(event) => {
        const current = drag.current
        if (current?.pointerId !== event.pointerId) return
        drag.current = null
        onLive(null)
        onCommit(current.result, event.clientX - current.startX)
      }}
      onKeyDown={(event: KeyboardEvent) => {
        const step = event.shiftKey ? KEY_STEP * 4 : KEY_STEP
        const dx =
          event.key === "ArrowRight"
            ? step
            : event.key === "ArrowLeft"
              ? -step
              : 0
        const dy =
          event.key === "ArrowDown" ? step : event.key === "ArrowUp" ? -step : 0
        if (!dx && !dy) return
        event.preventDefault()
        onCommit(resizeBy(rect, edge, dx, dy, canvas), dx)
      }}
    />
  )
}
