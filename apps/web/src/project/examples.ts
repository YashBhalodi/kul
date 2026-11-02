// The example projects, bundled at build time from the repository's
// examples/ folder, so the app offers exactly what the repo holds. Their
// comments explain the language, so each one doubles as a tutorial.

import { inFileOrder } from "@/store/file-names"
import type { ProjectFile } from "@/store/project-store"

export interface Example {
  /** The example's folder name, such as `02-three-generations`. */
  slug: string
  /** What it shows, from the first line of its first file. */
  title: string
  /** The name a project made from it gets: its family, when it has one. */
  projectName: string
  /** Its files, in file order. */
  files: Array<ProjectFile>
}

/** The example a first visit opens. */
export const FIRST_RUN_EXAMPLE = "02-three-generations"

const sources = import.meta.glob<string>("../../../../examples/*/*.kul", {
  query: "?raw",
  import: "default",
  eager: true,
})

/** The examples from `sources`, keyed by their paths, in folder order. */
export function examplesFrom(
  sourcesByPath: Record<string, string>
): Array<Example> {
  const folders = new Map<string, Array<ProjectFile>>()
  for (const [path, text] of Object.entries(sourcesByPath)) {
    const [slug, name] = path.split("/").slice(-2)
    if (slug === undefined || name === undefined) continue
    const files = folders.get(slug) ?? []
    files.push({ name, text })
    folders.set(slug, files)
  }
  return [...folders]
    .sort(([a], [b]) => a.localeCompare(b))
    .map(([slug, files]) => exampleFrom(slug, inFileOrder(files)))
}

function exampleFrom(slug: string, files: Array<ProjectFile>): Example {
  const title = titleOf(files) ?? slug
  const family = familyOf(files)
  return {
    slug,
    title,
    projectName: family ? `The ${family} family` : title,
    files,
  }
}

/**
 * The first line of the first file, which names what the example shows:
 * "# Three generations — depth, dates, …" gives "Three generations".
 */
function titleOf(files: ReadonlyArray<ProjectFile>): string | undefined {
  const first = files[0]?.text.split("\n", 1)[0] ?? ""
  if (!first.startsWith("#")) return undefined
  const title = first
    .replace(/^#\s*/, "")
    .split(/\s+—\s+/, 1)[0]
    ?.replace(/\.$/, "")
    .trim()
  return title === "" ? undefined : title
}

/** The surname most of the example's `family:` fields give. */
function familyOf(files: ReadonlyArray<ProjectFile>): string | undefined {
  const counts = new Map<string, number>()
  for (const file of files) {
    for (const match of file.text.matchAll(/\bfamily:"([^"]+)"/g)) {
      const family = match[1]
      if (family) counts.set(family, (counts.get(family) ?? 0) + 1)
    }
  }
  let best: string | undefined
  for (const [family, count] of counts) {
    if (best === undefined || count > (counts.get(best) ?? 0)) best = family
  }
  return best
}

export const EXAMPLES: ReadonlyArray<Example> = examplesFrom(sources)

export function findExample(slug: string): Example | undefined {
  return EXAMPLES.find((example) => example.slug === slug)
}
