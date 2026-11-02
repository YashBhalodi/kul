// What the canvas marks: the selected entity (clicked on the canvas, or
// picked in the details panel) and the linked one (under the editor's
// cursor). Both are entities keyed by id, so they outlive the SVG they were
// drawn on: after every render they are applied again by id, and one the
// new tree no longer draws is dropped.

/** A person or a marriage, as the language service names it. */
export interface EntityRef {
  kind: "person" | "marriage"
  id: string
}

export interface CanvasMarks {
  /** The entity the details panel shows. */
  selected: EntityRef | undefined
  /** The entity under the editor's cursor, drawn more softly. */
  linked: EntityRef | undefined
}

export const NO_MARKS: CanvasMarks = { selected: undefined, linked: undefined }

/** What a rendered tree draws: the ids of its people and marriages. */
export interface TreeContents {
  persons: ReadonlySet<string>
  marriages: ReadonlySet<string>
}

export function sameEntity(
  a: EntityRef | undefined,
  b: EntityRef | undefined
): boolean {
  return a === b || (!!a && !!b && a.kind === b.kind && a.id === b.id)
}

/** Whether `tree` draws `entity`. */
export function draws(tree: TreeContents, entity: EntityRef): boolean {
  return entity.kind === "person"
    ? tree.persons.has(entity.id)
    : tree.marriages.has(entity.id)
}

// Each change below returns `marks` itself when nothing changes, so a
// subscriber sees only real transitions.

export function select(marks: CanvasMarks, entity: EntityRef): CanvasMarks {
  return sameEntity(marks.selected, entity)
    ? marks
    : { ...marks, selected: entity }
}

export function clearSelection(marks: CanvasMarks): CanvasMarks {
  return marks.selected ? { ...marks, selected: undefined } : marks
}

export function link(
  marks: CanvasMarks,
  entity: EntityRef | undefined
): CanvasMarks {
  return sameEntity(marks.linked, entity) ? marks : { ...marks, linked: entity }
}

/**
 * The marks after a new tree was drawn: each is kept if the tree still
 * draws its entity, and dropped if not (the person was deleted, say).
 */
export function reapply(marks: CanvasMarks, tree: TreeContents): CanvasMarks {
  const keep = (entity: EntityRef | undefined) =>
    entity && draws(tree, entity) ? entity : undefined
  const selected = keep(marks.selected)
  const linked = keep(marks.linked)
  return selected === marks.selected && linked === marks.linked
    ? marks
    : { selected, linked }
}
