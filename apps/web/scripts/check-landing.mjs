// Checks the prerendered landing page and the files that describe the site
// to the outside (Open Graph image, favicons, robots.txt, sitemap.xml).
// check-dist runs these on every build; the landing page's expected
// content comes from the engine (src/landing/landing-content.ts), so the
// page is checked against what Kul says today.

/** Words the landing page must not pitch with. */
export const BANNED_WORDS = ["offline", "privacy", "private", "local-first"]

const ENTITIES = {
  amp: "&",
  lt: "<",
  gt: ">",
  quot: '"',
  apos: "'",
  nbsp: " ",
}

/** The text of an HTML document: tags dropped, entities decoded. */
export function textOf(html) {
  return html
    .replace(/<script\b[\s\S]*?<\/script>/gi, "")
    .replace(/<style\b[\s\S]*?<\/style>/gi, "")
    .replace(/<[^>]+>/g, "")
    .replace(/&(#x[0-9a-f]+|#\d+|[a-z]+);/gi, (entity, name) => {
      if (name.startsWith("#x"))
        return String.fromCodePoint(parseInt(name.slice(2), 16))
      if (name.startsWith("#"))
        return String.fromCodePoint(Number(name.slice(1)))
      return ENTITIES[name.toLowerCase()] ?? entity
    })
}

const lineText = (line) => line.map((run) => run.text).join("")
const tokenRuns = (lines) =>
  lines.flat().filter((run) => run.kind !== undefined).length

/**
 * Every problem with the landing page's HTML, given the content the build
 * made from the engine: the sample's lines and token spans, its tree, each
 * feature example and the diagnostic message, and no banned pitch.
 */
export function landingProblems(html, content) {
  const problems = []
  const text = textOf(html)
  for (const word of BANNED_WORDS) {
    if (new RegExp(`\\b${word}\\b`, "i").test(html)) {
      problems.push(`the landing page says "${word}"`)
    }
  }

  const { sample, features } = content
  for (const line of sample.lines) {
    const shown = lineText(line)
    if (shown.trim() !== "" && !text.includes(shown)) {
      problems.push(`the landing sample lacks the line \`${shown}\``)
    }
  }
  const spans = html.match(/<span class="tok-[a-z]+[^"]*"/g)?.length ?? 0
  const expected =
    tokenRuns(sample.lines) +
    features.reduce((total, feature) => total + tokenRuns(feature.lines), 0)
  if (spans < expected) {
    problems.push(
      `the landing page has ${String(spans)} highlighted token spans, not the ${String(expected)} the engine made`
    )
  }
  if (!html.includes(sample.svg)) {
    problems.push("the landing page lacks the sample's tree")
  }
  if (!/<g class="kul-card"/.test(html)) {
    problems.push("the landing page's tree has no kul-card")
  }

  for (const feature of features) {
    for (const line of feature.lines) {
      const shown = lineText(line)
      if (!text.includes(shown)) {
        problems.push(
          `the landing example "${feature.id}" lacks the line \`${shown}\``
        )
      }
    }
    for (const diagnostic of feature.diagnostics) {
      if (!text.includes(diagnostic.message.replaceAll("`", ""))) {
        problems.push(
          `the landing page lacks the engine's ${diagnostic.code} message`
        )
      }
    }
  }
  if (!features.some((feature) => feature.diagnostics.length > 0)) {
    problems.push("the landing page shows no diagnostic message")
  }
  return problems
}

/** A PNG's width and height, from its IHDR chunk; undefined if not a PNG. */
export function pngSize(bytes) {
  const signature = [0x89, 0x50, 0x4e, 0x47, 0x0d, 0x0a, 0x1a, 0x0a]
  if (!signature.every((byte, index) => bytes[index] === byte)) return undefined
  const view = new DataView(bytes.buffer, bytes.byteOffset, bytes.byteLength)
  return { width: view.getUint32(16), height: view.getUint32(20) }
}

/** The pages a sitemap lists, as paths on `origin`. */
export function sitemapPaths(xml, origin) {
  return [...xml.matchAll(/<loc>([^<]+)<\/loc>/g)].map(([, url]) =>
    url.startsWith(origin) ? url.slice(origin.length) : url
  )
}

/**
 * Every problem with the files that describe the site: `files` is the
 * build's file list, `readBytes` reads one.
 */
export function siteFileProblems(files, readBytes, origin) {
  const problems = []
  const has = (file) => files.includes(file)
  const text = (file) => new TextDecoder().decode(readBytes(file))

  if (!has("og.png")) {
    problems.push("missing og.png (the Open Graph image)")
  } else {
    const size = pngSize(readBytes("og.png"))
    if (size?.width !== 1200 || size.height !== 630)
      problems.push("og.png is not a 1200×630 PNG")
  }
  if (!has("apple-touch-icon.png")) {
    problems.push("missing apple-touch-icon.png")
  } else {
    const size = pngSize(readBytes("apple-touch-icon.png"))
    if (size?.width !== 180 || size.height !== 180)
      problems.push("apple-touch-icon.png is not a 180×180 PNG")
  }
  if (!has("favicon.ico")) {
    problems.push("missing favicon.ico")
  } else {
    const ico = readBytes("favicon.ico")
    if (ico[0] !== 0 || ico[1] !== 0 || ico[2] !== 1 || ico[3] !== 0)
      problems.push("favicon.ico is not an icon file")
  }
  if (!has("robots.txt")) {
    problems.push("missing robots.txt")
  } else if (!text("robots.txt").includes(`Sitemap: ${origin}/sitemap.xml`)) {
    problems.push("robots.txt does not point at the sitemap")
  }

  if (!has("sitemap.xml")) {
    problems.push("missing sitemap.xml")
  } else {
    const listed = new Set(sitemapPaths(text("sitemap.xml"), origin))
    const pages = files
      .filter(
        (file) =>
          (file === "index.html" || file.endsWith("/index.html")) &&
          !file.startsWith("app/")
      )
      .map((file) => `/${file.slice(0, -"index.html".length)}`)
    for (const page of pages) {
      if (!listed.has(page)) problems.push(`sitemap.xml does not list ${page}`)
    }
    for (const path of listed) {
      if (!pages.includes(path))
        problems.push(`sitemap.xml lists ${path}, which is not a built page`)
    }
  }
  return problems
}
