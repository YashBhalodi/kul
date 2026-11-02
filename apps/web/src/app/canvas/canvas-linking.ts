// The canvas and the editor, linked both ways. Choosing an entity on the
// canvas puts the editor's cursor on its declaration (without taking the
// keyboard from wherever it is); the editor's cursor resting on a
// declaration or a reference marks that entity on the canvas, softly, and
// brings it into view. The selected entity's details come from the
// language service, and stay as they were while the project has errors.

import { Debouncer } from "@tanstack/pacer"

import { detailsModel } from "@/canvas/details"
import { clearSelection, link, sameEntity, select } from "@/canvas/selection"
import type { EntityRef } from "@/canvas/selection"
import { showLocation } from "../editor/editor-controller"
import { setMinimized } from "../editor/editor-layout"
import { editorCursorStore } from "../editor/editor-store"
import type { EditorCursor } from "../editor/editor-store"
import { analysisStore, languageClient } from "../engine-status"
import {
  centreEntity,
  detailsStore,
  drawnTreeStore,
  marksStore,
  revealEntity,
} from "./canvas-store"
import { canonicalCard, pulse } from "./tree-dom"

/** How long the editor's cursor rests before the canvas follows it. */
export const CURSOR_REST_MS = 150

/**
 * Select `entity` (a click on the canvas, or a link in the details panel):
 * the panel shows it, the canvas brings it into view, and the editor's
 * cursor goes to its declaration.
 */
export function chooseEntity(entity: EntityRef): void {
  marksStore.setState((marks) => select(marks, entity))
  revealEntity(entity)
  void languageClient()
    ?.query("locate", entity.id)
    .then((location) => {
      if (location && sameEntity(marksStore.state.selected, entity)) {
        showLocation(location, { focus: false })
      }
    })
}

export function clearCanvasSelection(): void {
  marksStore.setState(clearSelection)
}

/** Open the editor at `entity`'s declaration, and give it the keyboard. */
export function goToSource(entity: EntityRef): void {
  void languageClient()
    ?.query("locate", entity.id)
    .then((location) => {
      if (!location) return
      setMinimized(false)
      showLocation(location)
    })
}

/** Glide from a past record to the person's own card, and pulse it. */
export function jumpToOwnCard(personId: string): void {
  centreEntity({ kind: "person", id: personId })
  const svg = drawnTreeStore.state
  const card = svg && canonicalCard(svg, personId)
  if (card) setTimeout(() => pulse(card), 280)
}

let detailsRequest = 0

/**
 * Ask for the selected entity's details. An answer that doesn't come
 * (the project has errors, or an edit overtook the question) leaves the
 * panel as it was.
 */
function refreshDetails() {
  const entity = marksStore.state.selected
  const request = ++detailsRequest
  if (!entity) {
    detailsStore.setState(() => undefined)
    return
  }
  void languageClient()
    ?.query("details", entity.id)
    .then((detail) => {
      if (request !== detailsRequest || !detail) return
      const model = detailsModel(detail)
      if (sameEntity(model.entity, entity)) {
        detailsStore.setState(() => model)
      }
    })
}

let linkRequest = 0

/** Mark the entity under the editor's cursor, and bring it into view. */
function followCursor(cursor: EditorCursor | undefined) {
  const request = ++linkRequest
  if (!cursor) return
  void languageClient()
    ?.query("entityAt", cursor.file, cursor.offset)
    .then((entity) => {
      // An edit that overtakes the question also moves the cursor, so a
      // newer question is already on its way; only the latest one counts.
      if (request !== linkRequest) return
      const linked: EntityRef | undefined = entity
      const before = marksStore.state.linked
      marksStore.setState((marks) => link(marks, linked))
      if (linked && !sameEntity(before, linked)) revealEntity(linked)
    })
}

let connected = false

/** Link the canvas to the editor and the language service, once per page. */
export function connectLinking(): void {
  if (connected) return
  connected = true
  const cursorRest = new Debouncer(followCursor, { wait: CURSOR_REST_MS })
  editorCursorStore.subscribe((cursor) => cursorRest.maybeExecute(cursor))
  let selected = marksStore.state.selected
  marksStore.subscribe((marks) => {
    if (sameEntity(marks.selected, selected)) return
    selected = marks.selected
    refreshDetails()
  })
  analysisStore.subscribe(({ report }) => {
    if (report?.svg !== undefined) refreshDetails()
  })
}
