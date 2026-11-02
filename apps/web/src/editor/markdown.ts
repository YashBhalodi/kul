// The Markdown the language service writes for hovers: paragraphs, bullet
// lists, fenced code, and inline bold, italic, code and links. Parsed into
// a small tree and built as DOM nodes, never as HTML, so no text from a
// project can become markup.

export type Inline =
  | { type: "text"; text: string }
  | { type: "code"; text: string }
  | { type: "strong"; children: Array<Inline> }
  | { type: "em"; children: Array<Inline> }
  | { type: "link"; href: string; children: Array<Inline> }

export type Block =
  | { type: "paragraph"; children: Array<Inline> }
  | { type: "list"; items: Array<Array<Inline>> }
  | { type: "code"; text: string }

export function parseMarkdown(markdown: string): Array<Block> {
  const blocks: Array<Block> = []
  const lines = markdown.replace(/\r\n?/g, "\n").split("\n")
  let index = 0
  while (index < lines.length) {
    const line = lines[index] ?? ""
    if (line.trim() === "") {
      index++
    } else if (line.startsWith("```")) {
      const body: Array<string> = []
      index++
      while (index < lines.length && !(lines[index] ?? "").startsWith("```")) {
        body.push(lines[index] ?? "")
        index++
      }
      index++
      blocks.push({ type: "code", text: body.join("\n") })
    } else if (/^[-*] /.test(line)) {
      const items: Array<Array<Inline>> = []
      while (index < lines.length && /^[-*] /.test(lines[index] ?? "")) {
        items.push(parseInline((lines[index] ?? "").slice(2)))
        index++
      }
      blocks.push({ type: "list", items })
    } else {
      const body: Array<string> = []
      while (
        index < lines.length &&
        (lines[index] ?? "").trim() !== "" &&
        !/^([-*] |```)/.test(lines[index] ?? "")
      ) {
        body.push(lines[index] ?? "")
        index++
      }
      blocks.push({ type: "paragraph", children: parseInline(body.join(" ")) })
    }
  }
  return blocks
}

export function parseInline(text: string): Array<Inline> {
  const out: Array<Inline> = []
  let plain = ""
  const flush = () => {
    if (plain) out.push({ type: "text", text: plain })
    plain = ""
  }
  let index = 0
  while (index < text.length) {
    const rest = text.slice(index)
    const char = text.charAt(index)
    if (char === "\\" && index + 1 < text.length) {
      plain += text.charAt(index + 1)
      index += 2
      continue
    }
    if (char === "`") {
      const end = text.indexOf("`", index + 1)
      if (end > index) {
        flush()
        out.push({ type: "code", text: text.slice(index + 1, end) })
        index = end + 1
        continue
      }
    }
    if (rest.startsWith("**")) {
      const end = text.indexOf("**", index + 2)
      if (end > index + 2) {
        flush()
        out.push({
          type: "strong",
          children: parseInline(text.slice(index + 2, end)),
        })
        index = end + 2
        continue
      }
    }
    if (char === "*") {
      const end = text.indexOf("*", index + 1)
      if (end > index + 1) {
        flush()
        out.push({
          type: "em",
          children: parseInline(text.slice(index + 1, end)),
        })
        index = end + 1
        continue
      }
    }
    if (char === "[") {
      const link = /^\[([^\]]+)\]\(([^)\s]+)\)/.exec(rest)
      if (link?.[1] && link[2]) {
        flush()
        out.push({
          type: "link",
          href: link[2],
          children: parseInline(link[1]),
        })
        index += link[0].length
        continue
      }
    }
    plain += char
    index++
  }
  flush()
  return out
}

/** Only web links open; anything else is shown as text. */
function safeHref(href: string): string | undefined {
  return /^https:\/\//.test(href) ? href : undefined
}

function inlineNodes(document: Document, inlines: Array<Inline>): Array<Node> {
  return inlines.map((inline): Node => {
    switch (inline.type) {
      case "text":
        return document.createTextNode(inline.text)
      case "code": {
        const code = document.createElement("code")
        code.textContent = inline.text
        return code
      }
      case "strong":
      case "em": {
        const element = document.createElement(inline.type)
        element.append(...inlineNodes(document, inline.children))
        return element
      }
      case "link": {
        const href = safeHref(inline.href)
        if (!href) {
          const span = document.createElement("span")
          span.append(...inlineNodes(document, inline.children))
          return span
        }
        const anchor = document.createElement("a")
        anchor.href = href
        anchor.target = "_blank"
        anchor.rel = "noopener noreferrer"
        anchor.append(...inlineNodes(document, inline.children))
        return anchor
      }
    }
  })
}

/** Build hover Markdown as DOM nodes inside `container`. */
export function renderMarkdown(container: HTMLElement, markdown: string): void {
  const document = container.ownerDocument
  for (const block of parseMarkdown(markdown)) {
    if (block.type === "paragraph") {
      const p = document.createElement("p")
      p.append(...inlineNodes(document, block.children))
      container.append(p)
    } else if (block.type === "list") {
      const ul = document.createElement("ul")
      for (const item of block.items) {
        const li = document.createElement("li")
        li.append(...inlineNodes(document, item))
        ul.append(li)
      }
      container.append(ul)
    } else {
      const pre = document.createElement("pre")
      pre.textContent = block.text
      container.append(pre)
    }
  }
}
