import { describe, expect, it } from "vitest"

import {
  landingProblems,
  pngSize,
  siteFileProblems,
  textOf,
} from "./check-landing.mjs"

const ORIGIN = "https://kullang.dev"
const SVG = '<svg viewBox="0 0 1 1"><g class="kul-card"></g></svg>'
const CONTENT = {
  sample: {
    lines: [
      [
        { text: "person", kind: "keyword" },
        { text: " " },
        { text: "asha", kind: "declaration" },
      ],
      [],
    ],
    svg: SVG,
  },
  features: [
    {
      id: "mistakes",
      lines: [[{ text: "birth", kind: "keyword" }, { text: " m_x" }]],
      diagnostics: [
        { code: "KUL-R02", message: "no marriage `m_x` & no luck", line: 1 },
      ],
    },
  ],
}
const PAGE = `<main><pre><span class="tok-keyword kul-noted">person</span> <span class="tok-declaration">asha</span></pre>
<div>${SVG}</div>
<pre><span class="tok-keyword">birth</span> m_x</pre>
<p>no marriage <code>m_x</code> &amp; no luck</p></main>`

describe("landingProblems", () => {
  it("passes a page that shows the engine's content", () => {
    expect(landingProblems(PAGE, CONTENT)).toEqual([])
  })

  it("reports a banned pitch word", () => {
    expect(landingProblems(`${PAGE}<p>Works Offline.</p>`, CONTENT)).toEqual([
      'the landing page says "offline"',
    ])
    expect(landingProblems(`${PAGE}<p>local-first</p>`, CONTENT)).toEqual([
      'the landing page says "local-first"',
    ])
  })

  it("reports a missing line, token span, tree or message", () => {
    const plain = PAGE.replace(
      '<span class="tok-declaration">asha</span>',
      "asha"
    )
    expect(landingProblems(plain, CONTENT)).toEqual([
      "the landing page has 2 highlighted token spans, not the 3 the engine made",
    ])
    expect(landingProblems(PAGE.replace(SVG, ""), CONTENT)).toEqual([
      "the landing page lacks the sample's tree",
      "the landing page's tree has no kul-card",
    ])
    expect(landingProblems(PAGE.replace("&amp; no luck", ""), CONTENT)).toEqual(
      ["the landing page lacks the engine's KUL-R02 message"]
    )
    expect(
      landingProblems(PAGE.replace("</span> m_x", "</span> m_y"), CONTENT)
    ).toEqual(['the landing example "mistakes" lacks the line `birth m_x`'])
  })
})

describe("textOf", () => {
  it("drops tags and scripts and decodes entities", () => {
    expect(
      textOf('<p title="x">a &quot;b&quot; &#x27;c&#39;</p><script>no</script>')
    ).toBe(`a "b" 'c'`)
  })
})

const PNG_HEADER = (width, height) => {
  const bytes = new Uint8Array(24)
  bytes.set([0x89, 0x50, 0x4e, 0x47, 0x0d, 0x0a, 0x1a, 0x0a])
  const view = new DataView(bytes.buffer)
  view.setUint32(16, width)
  view.setUint32(20, height)
  return bytes
}

describe("siteFileProblems", () => {
  const good = {
    "og.png": PNG_HEADER(1200, 630),
    "apple-touch-icon.png": PNG_HEADER(180, 180),
    "favicon.ico": new Uint8Array([0, 0, 1, 0, 1, 0]),
    "robots.txt": new TextEncoder().encode(
      `User-agent: *\nSitemap: ${ORIGIN}/sitemap.xml\n`
    ),
    "sitemap.xml": new TextEncoder().encode(
      `<urlset><url><loc>${ORIGIN}/</loc></url><url><loc>${ORIGIN}/docs/</loc></url></urlset>`
    ),
    "index.html": new Uint8Array(),
    "docs/index.html": new Uint8Array(),
    "_shell.html": new Uint8Array(),
  }
  const check = (files) =>
    siteFileProblems(Object.keys(files), (file) => files[file], ORIGIN)

  it("passes a complete set", () => {
    expect(check(good)).toEqual([])
    expect(pngSize(good["og.png"])).toEqual({ width: 1200, height: 630 })
  })

  it("reports a wrong image size, a missing file and a sitemap mismatch", () => {
    expect(check({ ...good, "og.png": PNG_HEADER(600, 315) })).toEqual([
      "og.png is not a 1200×630 PNG",
    ])
    const noIcon = { ...good }
    delete noIcon["favicon.ico"]
    expect(check(noIcon)).toEqual(["missing favicon.ico"])
    expect(
      check({ ...good, "docs/guide/index.html": new Uint8Array() })
    ).toEqual(["sitemap.xml does not list /docs/guide/"])
    expect(check({ ...good, "robots.txt": new Uint8Array() })).toEqual([
      "robots.txt does not point at the sitemap",
    ])
  })
})
