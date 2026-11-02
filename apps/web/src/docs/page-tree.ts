// The docs' navigation, read from the collection: the sidebar's sections
// in meta.json's order, and the reading order the previous/next links
// follow. Page addresses come from file paths: `guide/dates.mdx` is
// /docs/guide/dates/, and `index.mdx` the folder's own page.

import { docsUrl } from "./urls"

/** A page in the sidebar. */
export interface NavPage {
  title: string
  url: string
}

/** A group of pages under a heading, or top-level pages (no title). */
export interface NavSection {
  title: string | undefined
  pages: Array<NavPage>
}

/** What the navigation needs of a page file and a meta.json file. */
export interface PageFile {
  info: { path: string }
  title: string
}
export interface MetaFile {
  info: { path: string }
  title?: string
  pages?: Array<string>
}

/** A page file's slugs: `guide/dates.mdx` → ["guide", "dates"]. */
export function slugsOfFile(path: string): Array<string> {
  const parts = path.replace(/\.mdx?$/, "").split("/")
  if (parts.at(-1) === "index") parts.pop()
  return parts
}

function folderOf(path: string): string {
  return path.includes("/") ? path.slice(0, path.lastIndexOf("/")) : ""
}

/**
 * The sidebar: top-level pages and one section per folder, each in the
 * order its meta.json lists (files it doesn't list follow, by name).
 */
export function navSections(
  pages: ReadonlyArray<PageFile>,
  metas: ReadonlyArray<MetaFile>
): Array<NavSection> {
  const metaFor = (folder: string) =>
    metas.find(
      (meta) =>
        folderOf(meta.info.path) === folder &&
        meta.info.path.endsWith("meta.json")
    )
  const entriesOf = (folder: string) => {
    const pagesHere = pages.filter(
      (page) => folderOf(page.info.path) === folder
    )
    const folders = [
      ...new Set(
        pages
          .map((page) => folderOf(page.info.path))
          .filter((path) => folderOf(path) === folder && path !== folder)
      ),
    ]
    const names = [
      ...pagesHere.map((page) =>
        page.info.path
          .slice(folder ? folder.length + 1 : 0)
          .replace(/\.mdx?$/, "")
      ),
      ...folders.map((path) => path.slice(folder ? folder.length + 1 : 0)),
    ]
    const listed = metaFor(folder)?.pages ?? []
    const order = [
      ...listed.filter((name) => names.includes(name)),
      ...names.filter((name) => !listed.includes(name)).sort(),
    ]
    return order.map((name) => {
      const path = folder ? `${folder}/${name}` : name
      const page = pagesHere.find(
        (entry) => entry.info.path.replace(/\.mdx?$/, "") === path
      )
      return page
        ? { kind: "page" as const, page }
        : { kind: "folder" as const, path }
    })
  }
  const pageOf = (page: PageFile): NavPage => ({
    title: page.title,
    url: docsUrl(slugsOfFile(page.info.path)),
  })
  const flatten = (folder: string): Array<NavPage> =>
    entriesOf(folder).flatMap((entry) =>
      entry.kind === "page" ? [pageOf(entry.page)] : flatten(entry.path)
    )

  const sections: Array<NavSection> = []
  for (const entry of entriesOf("")) {
    if (entry.kind === "folder") {
      sections.push({
        title: metaFor(entry.path)?.title ?? entry.path,
        pages: flatten(entry.path),
      })
    } else {
      const last = sections.at(-1)
      if (last && last.title === undefined) last.pages.push(pageOf(entry.page))
      else sections.push({ title: undefined, pages: [pageOf(entry.page)] })
    }
  }
  return sections
}

/** Every page in reading order. */
export function readingOrder(
  sections: ReadonlyArray<NavSection>
): Array<NavPage> {
  return sections.flatMap((section) => section.pages)
}

/** The pages before and after `url` in reading order. */
export function neighbours(
  sections: ReadonlyArray<NavSection>,
  url: string
): { previous: NavPage | undefined; next: NavPage | undefined } {
  const pages = readingOrder(sections)
  const index = pages.findIndex((page) => page.url === url)
  if (index === -1) return { previous: undefined, next: undefined }
  return { previous: pages[index - 1], next: pages[index + 1] }
}
