// Downloads: the project as a .zip, one file, or the tree as an SVG.
// Everything is built in the page; nothing leaves the machine.

import { zipProject } from "@/project/zip"

import { languageClient } from "./engine-status"
import { flushProject, sessionStore } from "./project-session"

/** Save `data` as a file named `name` through the browser's download. */
export function saveFile(name: string, data: BlobPart, type: string): void {
  const url = URL.createObjectURL(new Blob([data], { type }))
  const link = document.createElement("a")
  link.href = url
  link.download = name
  link.style.display = "none"
  document.body.append(link)
  link.click()
  link.remove()
  // Give the browser a moment to start the download before letting go.
  setTimeout(() => URL.revokeObjectURL(url), 60_000)
}

/** A project name made safe for a file name. */
export function fileNameFor(projectName: string): string {
  // Characters that some file systems refuse become a hyphen.
  const safe = projectName
    .replace(/[/\\:*?"<>|]/g, "-")
    .split("")
    .filter((character) => character >= " ")
    .join("")
    .trim()
  return safe === "" ? "project" : safe
}

/** Download the open project's files as a .zip, after any unsaved edits. */
export async function downloadProject(): Promise<void> {
  await flushProject()
  const { project, files } = sessionStore.state
  if (!project) return
  saveFile(
    `${fileNameFor(project.name)}.zip`,
    zipProject(files) as Uint8Array<ArrayBuffer>,
    "application/zip"
  )
}

/** Download one file of the open project. */
export function downloadFile(name: string): void {
  const file = sessionStore.state.files.find((f) => f.name === name)
  if (file) saveFile(file.name, file.text, "text/plain;charset=utf-8")
}

/**
 * Download the tree as a self-contained SVG: its own light palette and a
 * legend baked in, so it reads the same wherever it is opened. Nothing
 * downloads while the project has errors.
 */
export async function downloadTree(): Promise<void> {
  const { project } = sessionStore.state
  const svg = await languageClient()?.query("renderSvg", "fileExport")
  if (!project || !svg) return
  saveFile(`${fileNameFor(project.name)}.svg`, svg, "image/svg+xml")
}
