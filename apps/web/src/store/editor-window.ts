// The editor window's layout as a project keeps it in project.json:
// where it floats, how big it is, whether it is docked to a side of the
// canvas, and whether it is minimized to its pill.

export type Dock = "left" | "right"

export interface EditorWindowLayout {
  /** The floating window's top-left corner, from the canvas's, in px. */
  x: number
  y: number
  /** Its size in px; a docked window keeps the width and fills the height. */
  width: number
  height: number
  dock: Dock | null
  minimized: boolean
}

function isFiniteNumber(value: unknown): value is number {
  return typeof value === "number" && Number.isFinite(value)
}

/** A stored layout, or null when the value isn't one. */
export function parseEditorWindow(value: unknown): EditorWindowLayout | null {
  if (typeof value !== "object" || value === null) return null
  const record = value as Record<string, unknown>
  const { x, y, width, height, dock, minimized } = record
  if (
    !isFiniteNumber(x) ||
    !isFiniteNumber(y) ||
    !isFiniteNumber(width) ||
    !isFiniteNumber(height)
  ) {
    return null
  }
  return {
    x,
    y,
    width,
    height,
    dock: dock === "left" || dock === "right" ? dock : null,
    minimized: minimized === true,
  }
}
