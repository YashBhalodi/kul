// Where the editor window sits on the canvas, and how dragging and
// resizing move it. Positions are in px from the canvas's top-left corner.
// A window never shrinks below 360×240, and at least 48 px of it, title
// bar included, stays on the canvas.

import type { Dock, EditorWindowLayout } from "@/store/editor-window"

export const MIN_WIDTH = 360
export const MIN_HEIGHT = 240
/** How much of the window always stays on the canvas. */
export const KEEP_VISIBLE = 48
/** How close to a side the pointer must come to dock the window there. */
export const DOCK_DISTANCE = 24
/** How far a docked window is dragged before it floats again. */
export const UNDOCK_DISTANCE = 12
/** The gap kept around a floating window by default. */
const MARGIN = 16

export interface Size {
  width: number
  height: number
}

export interface Rect extends Size {
  x: number
  y: number
}

export type Edge = "n" | "s" | "e" | "w" | "ne" | "nw" | "se" | "sw"

const clamp = (value: number, low: number, high: number) =>
  Math.min(Math.max(value, low), Math.max(low, high))

/** The layout a project starts with: floating at the left of the canvas. */
export function defaultLayout(canvas: Size): EditorWindowLayout {
  const width = clamp(Math.round(canvas.width * 0.46), MIN_WIDTH, 720)
  const height = Math.max(MIN_HEIGHT, canvas.height - 2 * MARGIN)
  return {
    x: MARGIN,
    y: MARGIN,
    width,
    height,
    dock: null,
    minimized: false,
  }
}

/** A floating window's size, kept within its minimum and the canvas. */
function fitSize(size: Size, canvas: Size): Size {
  return {
    width: clamp(size.width, MIN_WIDTH, canvas.width),
    height: clamp(size.height, MIN_HEIGHT, canvas.height),
  }
}

/** A floating rectangle kept far enough on the canvas to grab again. */
export function keepOnCanvas(rect: Rect, canvas: Size): Rect {
  const size = fitSize(rect, canvas)
  return {
    ...size,
    x: clamp(rect.x, KEEP_VISIBLE - size.width, canvas.width - KEEP_VISIBLE),
    y: clamp(rect.y, 0, canvas.height - KEEP_VISIBLE),
  }
}

/** Where a layout puts the window on a canvas of this size. */
export function windowRect(layout: EditorWindowLayout, canvas: Size): Rect {
  if (layout.dock) {
    const width = clamp(layout.width, MIN_WIDTH, canvas.width)
    return {
      x: layout.dock === "left" ? 0 : canvas.width - width,
      y: 0,
      width,
      height: canvas.height,
    }
  }
  return keepOnCanvas(layout, canvas)
}

/** The side a pointer at `x` docks to, if it is close enough to one. */
export function dockAt(x: number, canvas: Size): Dock | null {
  if (x <= DOCK_DISTANCE) return "left"
  if (x >= canvas.width - DOCK_DISTANCE) return "right"
  return null
}

/** The window moved by a drag of `dx, dy` from `start`. */
export function moveBy(
  start: Rect,
  dx: number,
  dy: number,
  canvas: Size
): Rect {
  return keepOnCanvas({ ...start, x: start.x + dx, y: start.y + dy }, canvas)
}

/**
 * A docked window pulled away from its side: it floats at its own size,
 * under the pointer, held at the same fraction of its title bar.
 */
export function undockUnder(
  layout: EditorWindowLayout,
  docked: Rect,
  pointer: { x: number; y: number },
  canvas: Size
): Rect {
  const size = fitSize(layout, canvas)
  const grab = (pointer.x - docked.x) / docked.width
  return keepOnCanvas(
    {
      ...size,
      x: pointer.x - grab * size.width,
      y: pointer.y - Math.min(pointer.y - docked.y, 16),
    },
    canvas
  )
}

/** The window resized by dragging `edge` by `dx, dy` from `start`. */
export function resizeBy(
  start: Rect,
  edge: Edge,
  dx: number,
  dy: number,
  canvas: Size
): Rect {
  let { x, y, width, height } = start
  const right = start.x + start.width
  const bottom = start.y + start.height
  if (edge.includes("e")) {
    width = clamp(start.width + dx, MIN_WIDTH, canvas.width - start.x)
  }
  if (edge.includes("w")) {
    width = clamp(start.width - dx, MIN_WIDTH, right)
    x = right - width
  }
  if (edge.includes("s")) {
    height = clamp(start.height + dy, MIN_HEIGHT, canvas.height - start.y)
  }
  if (edge.startsWith("n")) {
    height = clamp(start.height - dy, MIN_HEIGHT, bottom)
    y = bottom - height
  }
  return { x, y, width, height }
}

/** The edge a docked window resizes from: the one facing the canvas. */
export function dockedEdge(dock: Dock): Edge {
  return dock === "left" ? "e" : "w"
}

/** The layout after a docked window's width changed. */
export function resizeDocked(
  layout: EditorWindowLayout,
  dx: number,
  canvas: Size
): EditorWindowLayout {
  const growth = layout.dock === "left" ? dx : -dx
  return {
    ...layout,
    width: clamp(layout.width + growth, MIN_WIDTH, canvas.width),
  }
}

/** The room canvas chrome keeps from the editor window, in px. */
const CLEARANCE = 16
/** The narrowest strip of canvas worth placing chrome in beside the window. */
const MIN_FREE_WIDTH = 360

/**
 * Where canvas chrome (its banners, say) can sit without covering the
 * editor window: insets from the canvas's left and right edges. The whole
 * width when the window is minimized, or when it leaves too little room.
 */
export function freeCanvasInsets(
  layout: EditorWindowLayout,
  canvas: Size
): { left: number; right: number } {
  const none = { left: 0, right: 0 }
  if (layout.minimized) return none
  const rect = windowRect(layout, canvas)
  const leftOfWindow = rect.x
  const rightOfWindow = canvas.width - (rect.x + rect.width)
  if (rightOfWindow >= leftOfWindow && rightOfWindow >= MIN_FREE_WIDTH) {
    return { left: rect.x + rect.width + CLEARANCE, right: 0 }
  }
  if (leftOfWindow >= MIN_FREE_WIDTH) {
    return { left: 0, right: canvas.width - rect.x + CLEARANCE }
  }
  return none
}
