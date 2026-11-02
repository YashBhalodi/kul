// "Add your first person": the empty project's way in. It writes the
// starter snippet at the end of the active file (or a new family.kul) and
// puts the editor's cursor on the new person's id.

import {
  STARTER_PERSON_ID,
  STARTER_SNIPPET,
  starterInsertion,
} from "@/project/starter"
import { applyFileEdits, showLocation } from "../editor/editor-controller"
import { setMinimized } from "../editor/editor-layout"
import { createFile, sessionStore } from "../project-session"

const NEW_FILE = "family.kul"

function textOf(name: string): string {
  return sessionStore.state.files.find((file) => file.name === name)?.text ?? ""
}

export async function addFirstPerson(): Promise<void> {
  const { files, ui, access } = sessionStore.state
  if (access !== "owner") return
  setMinimized(false)
  const name = ui.activeFile ?? files[0]?.name
  if (name === undefined) {
    await createFile(NEW_FILE, STARTER_SNIPPET)
  } else {
    const text = textOf(name)
    const end = text.length
    const applied = applyFileEdits(
      [
        {
          file: name,
          edits: [{ span: { start: end, end }, text: starterInsertion(text) }],
        },
      ],
      "input.starter"
    )
    if (!applied) return
  }
  const file = name ?? NEW_FILE
  const declaration = `person ${STARTER_PERSON_ID} `
  const at = textOf(file).lastIndexOf(declaration)
  if (at === -1) return
  const start = at + "person ".length
  showLocation({
    file,
    span: { start, end: start + STARTER_PERSON_ID.length },
  })
}
