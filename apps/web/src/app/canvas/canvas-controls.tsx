import { useSelector } from "@tanstack/react-store"
import { ListIcon, MinusIcon, PlusIcon, ScanIcon } from "lucide-react"
import { useMemo } from "react"
import type { ReactNode } from "react"

import { legendRows } from "@/canvas/legend"
import type { LegendSwatch } from "@/canvas/legend"
import { Button } from "@/components/ui/button"
import {
  Tooltip,
  TooltipContent,
  TooltipTrigger,
} from "@/components/ui/tooltip"
import { EngineStatusBadge } from "../canvas-surface"
import { canvasSizeStore, currentLayout } from "../editor/editor-layout"
import { sessionStore } from "../project-session"
import { useCanvasCommands } from "./canvas-commands"
import {
  cameraStore,
  controlsRightInset,
  drawnTreeStore,
  fitTree,
  legendStore,
  marksStore,
  toggleLegend,
  zoomStep,
  zoomToActualSize,
} from "./canvas-store"

/**
 * The canvas's corner: the engine's status, the legend toggle and the
 * zoom controls, with the legend above them when it is open. It stays
 * clear of the details panel and of an editor docked on the right.
 */
export function CanvasControls() {
  useCanvasCommands()
  const canvas = useSelector(canvasSizeStore)
  const editorWindow = useSelector(
    sessionStore,
    (state) => state.ui.editorWindow
  )
  const panelOpen = useSelector(
    marksStore,
    (marks) => marks.selected !== undefined
  )
  const right = controlsRightInset(
    canvas,
    currentLayout(editorWindow, canvas),
    panelOpen
  )
  return (
    <div
      style={{ right }}
      className="pointer-events-none absolute bottom-4 z-[5] flex flex-col items-end gap-2"
    >
      <EngineStatusBadge />
      <Legend />
      <div className="flex items-center gap-2">
        <LegendToggle />
        <ZoomControls />
      </div>
    </div>
  )
}

function ControlButton({
  label,
  iconOnly = false,
  shortcut,
  onClick,
  children,
  testId,
  pressed,
  disabled,
}: {
  /** What the tooltip says; also the button's name when it shows only an icon. */
  label: string
  iconOnly?: boolean
  shortcut?: string
  onClick: () => void
  children: ReactNode
  testId?: string
  pressed?: boolean
  disabled?: boolean
}) {
  return (
    <Tooltip>
      <TooltipTrigger
        render={
          <Button
            variant="ghost"
            size="sm"
            aria-label={iconOnly ? label : undefined}
            aria-pressed={pressed}
            disabled={disabled}
            data-testid={testId}
            onClick={onClick}
            className="h-8 min-w-8 px-2 aria-pressed:bg-muted aria-pressed:text-foreground"
          />
        }
      >
        {children}
      </TooltipTrigger>
      <TooltipContent>
        {label}
        {shortcut && <span className="ml-1.5 opacity-70">{shortcut}</span>}
      </TooltipContent>
    </Tooltip>
  )
}

function ZoomControls() {
  const zoom = useSelector(cameraStore, (camera) => camera.zoom)
  const hasTree = useSelector(drawnTreeStore, (svg) => svg !== undefined)
  return (
    <div
      role="group"
      aria-label="Zoom"
      className="pointer-events-auto flex items-center gap-0.5 rounded-xl border bg-card p-0.5 text-card-foreground shadow-md"
    >
      <ControlButton
        label="Zoom out"
        iconOnly
        shortcut="−"
        onClick={() => zoomStep(-1)}
      >
        <MinusIcon />
      </ControlButton>
      <output
        data-testid="zoom-level"
        aria-label="Zoom level"
        className="w-12 text-center text-xs font-medium tabular-nums"
      >
        {Math.round(zoom * 100)}%
      </output>
      <ControlButton
        label="Zoom in"
        iconOnly
        shortcut="+"
        onClick={() => zoomStep(1)}
      >
        <PlusIcon />
      </ControlButton>
      <span className="mx-0.5 h-5 w-px bg-border" aria-hidden />
      <ControlButton
        label="Fit the tree"
        shortcut="0"
        onClick={fitTree}
        disabled={!hasTree}
        testId="zoom-fit"
      >
        <ScanIcon />
        <span className="text-xs">Fit</span>
      </ControlButton>
      <ControlButton
        label="Show the tree at its actual size"
        onClick={zoomToActualSize}
        testId="zoom-actual"
      >
        <span className="text-xs">100%</span>
      </ControlButton>
    </div>
  )
}

/** The legend's rows for the tree on screen. */
function useLegendRows() {
  const svg = useSelector(drawnTreeStore)
  return useMemo(
    () =>
      svg ? legendRows((selector) => svg.querySelector(selector) !== null) : [],
    [svg]
  )
}

function LegendToggle() {
  const open = useSelector(legendStore, (state) => state.open)
  const rows = useLegendRows()
  return (
    <div className="pointer-events-auto rounded-xl border bg-card p-0.5 text-card-foreground shadow-md">
      <ControlButton
        label={open ? "Hide the legend" : "Show the legend"}
        onClick={toggleLegend}
        pressed={open}
        disabled={rows.length === 0}
        testId="legend-toggle"
      >
        <ListIcon />
        <span className="text-xs">Legend</span>
      </ControlButton>
    </div>
  )
}

function Legend() {
  const open = useSelector(legendStore, (state) => state.open)
  const rows = useLegendRows()
  if (!open || rows.length === 0) return null
  return (
    <section
      aria-label="Legend"
      data-testid="legend"
      className="pointer-events-auto w-48 rounded-xl border bg-card px-3 py-2.5 text-card-foreground shadow-md"
    >
      <h2 className="mb-1.5 text-xs font-medium text-muted-foreground">
        Legend
      </h2>
      <ul className="flex flex-col gap-1">
        {rows.map((row) => (
          <li
            key={row.key}
            data-row={row.key}
            className="flex items-center gap-2.5 text-sm"
          >
            <Swatch swatch={row.swatch} />
            {row.label}
          </li>
        ))}
      </ul>
    </section>
  )
}

/** A miniature of a card or a line, painted by the tree's own CSS. */
function Swatch({ swatch }: { swatch: LegendSwatch }) {
  return (
    <svg
      viewBox="0 0 32 18"
      width="32"
      height="18"
      aria-hidden
      className="kul-tree kul-legend-swatch shrink-0"
    >
      {swatch.element === "card" ? (
        <g
          className="kul-card"
          data-kind={swatch.kind}
          data-gender={swatch.gender}
        >
          <rect
            x="1"
            y="2"
            width="30"
            height="14"
            rx="3"
            ry="3"
            strokeDasharray={swatch.kind === "ghost" ? "3 2" : undefined}
          />
        </g>
      ) : (
        <path
          className="kul-edge"
          data-link-kind={swatch.linkKind}
          data-is-ended={swatch.ended ? "true" : undefined}
          fill="none"
          d="M 1 9 L 31 9"
          strokeDasharray={swatch.linkKind === "adoption" ? "6 4" : undefined}
        />
      )}
    </svg>
  )
}
