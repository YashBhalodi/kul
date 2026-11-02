// The canvas camera as a project keeps it in project.json: where the tree
// is drawn and at what zoom, so a project opens on the view it was left on.

export interface CanvasCamera {
  /** Where the tree's origin is drawn, in px from the canvas's corner. */
  x: number
  y: number
  /** Screen px per tree unit: 1 is 100%. */
  zoom: number
}

function isFiniteNumber(value: unknown): value is number {
  return typeof value === "number" && Number.isFinite(value)
}

/** A stored camera, or null when the value isn't one. */
export function parseCanvasCamera(value: unknown): CanvasCamera | null {
  if (typeof value !== "object" || value === null) return null
  const { x, y, zoom } = value as Record<string, unknown>
  if (!isFiniteNumber(x) || !isFiniteNumber(y) || !isFiniteNumber(zoom)) {
    return null
  }
  if (zoom <= 0) return null
  return { x, y, zoom }
}
