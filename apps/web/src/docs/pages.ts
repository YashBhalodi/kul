// The docs pages as the routes load them. The collection module awaits its
// entries at the top level, so the routes import it only from their
// loaders, dynamically: a static import would pull it into the graph of
// every page of the site and split the shared code into many more files.

import type { TOCItemType } from "fumadocs-core/toc"
import { use } from "react"
import type { MDXContent } from "mdx/types"

import { navSections, slugsOfFile } from "./page-tree"
import type { NavSection } from "./page-tree"

export interface LoadedPage {
  /** The page's file under content/docs, which names it. */
  path: string
  slugs: Array<string>
  title: string
  description: string | undefined
}

const bodies = new Map<string, { Body: MDXContent; toc: TOCItemType[] }>()

const collection = () => import("./collection").then((module) => module.docs)

/** The sidebar's sections. */
export async function loadNav(): Promise<Array<NavSection>> {
  const docs = await collection()
  return navSections(docs.docs, docs.meta)
}

/**
 * The page at `slugs`, with its compiled body loaded for `pageBody`, or
 * undefined when there is no such page.
 */
export async function loadPage(
  slugs: ReadonlyArray<string>
): Promise<LoadedPage | undefined> {
  const docs = await collection()
  const entry = docs.docs.find(
    (page) => slugsOfFile(page.info.path).join("/") === slugs.join("/")
  )
  if (!entry) return undefined
  const { body, toc } = await entry.load()
  bodies.set(entry.info.path, { Body: body, toc })
  return {
    path: entry.info.path,
    slugs: [...slugs],
    title: entry.title,
    description: entry.description,
  }
}

const pending = new Map<
  string,
  Promise<{ Body: MDXContent; toc: TOCItemType[] }>
>()

/**
 * A page's body and outline. The loader has usually loaded them; when the
 * page hydrates from its prerendered HTML it hasn't (loaders don't run
 * again), so this suspends until they load, under a Suspense boundary.
 */
export function usePageBody(path: string): {
  Body: MDXContent
  toc: TOCItemType[]
} {
  const loaded = bodies.get(path)
  if (loaded) return loaded
  let promise = pending.get(path)
  if (!promise) {
    promise = collection().then(async (docs) => {
      const entry = docs.getPage(path)
      if (!entry) throw new Error(`no docs page ${path}`)
      const { body, toc } = await entry.load()
      const result = { Body: body, toc }
      bodies.set(path, result)
      return result
    })
    pending.set(path, promise)
  }
  return use(promise)
}
