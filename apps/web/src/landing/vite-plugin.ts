// The landing page's build step. It serves `virtual:kul-landing` (the
// sample, its tree and the feature examples, from the engine) and writes
// the files that describe the site to the outside: the Open Graph image,
// favicon.ico and apple-touch-icon.png, and sitemap.xml listing every
// prerendered page.

import { readFileSync } from "node:fs"

import type { Plugin } from "vite"

import {
  buildLandingContent,
  exampleFiles,
  SAMPLE_EXAMPLE,
} from "./landing-content.ts"
import { faviconImages, ogImage } from "./site-images.ts"

const VIRTUAL_ID = "virtual:kul-landing"
const RESOLVED_ID = `\0${VIRTUAL_ID}`

/** The sitemap of the pages at these paths on `origin`. */
export function sitemapXml(
  origin: string,
  paths: ReadonlyArray<string>
): string {
  const urls = paths.map((path) => `  <url><loc>${origin}${path}</loc></url>`)
  return `<?xml version="1.0" encoding="UTF-8"?>\n<urlset xmlns="http://www.sitemaps.org/schemas/sitemap/0.9">\n${urls.join("\n")}\n</urlset>\n`
}

export function landingPage({
  origin,
  pages,
}: {
  /** The canonical origin, which the sitemap's URLs start with. */
  origin: string
  /** The prerendered pages' paths, for the sitemap. */
  pages: () => ReadonlyArray<string>
}): Plugin {
  const favicon = new URL("../../public/favicon.svg", import.meta.url)
  return {
    name: "kul-landing",
    resolveId(id) {
      return id === VIRTUAL_ID ? RESOLVED_ID : undefined
    },
    load(id) {
      if (id !== RESOLVED_ID) return undefined
      return `export default ${JSON.stringify(buildLandingContent())}`
    },
    async generateBundle() {
      if (this.environment.name !== "client") return
      const files = exampleFiles(SAMPLE_EXAMPLE)
      const icon = readFileSync(favicon, "utf8")
      const { ico, touchIcon } = await faviconImages(icon, files)
      const emit = (fileName: string, source: string | Uint8Array) =>
        this.emitFile({ type: "asset", fileName, source })
      emit("og.png", await ogImage(files, icon))
      emit("favicon.ico", ico)
      emit("apple-touch-icon.png", touchIcon)
      emit("sitemap.xml", sitemapXml(origin, pages()))
    },
  }
}
