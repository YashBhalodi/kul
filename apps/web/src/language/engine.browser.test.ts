import { describe, expect, it } from "vitest"

import { APP_VERSION } from "@/site"
import { createLanguageClient } from "./client"
import type { ClientStatus } from "./client"
import { spawnLanguageWorker } from "./engine"
import type { AnalysisReport } from "./protocol"

const PERSON = 'person ann name:"Ann" gender:female\n'

function startClient() {
  const reports: Array<AnalysisReport> = []
  const statuses: Array<ClientStatus> = []
  let restarted = 0
  const client = createLanguageClient({
    spawn: spawnLanguageWorker,
    onAnalysis: (report) => reports.push(report),
    onStatus: (status) => statuses.push(status),
    onRestart: () => restarted++,
  })
  return { client, reports, statuses, restarts: () => restarted }
}

async function until<T>(read: () => T | undefined, ms = 5000): Promise<T> {
  const deadline = performance.now() + ms
  for (;;) {
    const value = read()
    if (value !== undefined) return value
    if (performance.now() > deadline) throw new Error("timed out")
    await new Promise((resolve) => setTimeout(resolve, 20))
  }
}

describe("the language worker", () => {
  it("loads kul-wasm and reports its versions", async () => {
    const { client } = startClient()
    try {
      const info = await client.ready
      // The web app and the engine release in lockstep.
      expect(info.core).toBe(APP_VERSION)
      expect(info.language).toMatch(/^\d+\.\d+$/)
    } finally {
      client.dispose()
    }
  })

  it("answers edits with lexical tokens and pushes one analysis per pause", async () => {
    const { client, reports } = startClient()
    try {
      await client.ready
      const first = client.setFile("a.kul", "person")
      const last = client.setFile("a.kul", PERSON)
      expect((await first.tokens)?.version).toBe(first.version)
      const tokens = await last.tokens
      expect(tokens?.tokens.map((token) => token.kind)).toEqual([
        "keyword",
        "declaration",
        "fieldName",
        "string",
        "fieldName",
        "enumValue",
      ])
      const report = await until(() => reports.at(-1))
      // Both edits arrived within the debounce: one analysis, of the last.
      expect(reports).toHaveLength(1)
      expect(report.files).toEqual([{ name: "a.kul", version: last.version }])
      expect(report.diagnostics).toEqual([])
      expect(report.svg).toContain("kul-card")
      expect(report.semantic["a.kul"]?.length).toBeGreaterThan(0)
    } finally {
      client.dispose()
    }
  })

  it("answers features across files, in UTF-16 offsets", async () => {
    const { client } = startClient()
    try {
      client.replaceFiles([
        { name: "a.kul", text: `# ગુજરાતી\n${PERSON}` },
        {
          name: "b.kul",
          text: 'person bo name:"Bo" gender:male\nmarriage m ann bo\n',
        },
      ])
      await client.ready
      const text = client.file("b.kul")?.text ?? ""
      const definition = await client.query(
        "definition",
        "b.kul",
        text.indexOf("ann")
      )
      const a = client.file("a.kul")?.text ?? ""
      expect(definition).toEqual({
        file: "a.kul",
        span: { start: a.indexOf("ann"), end: a.indexOf("ann") + 3 },
      })
      const hover = await client.query("hover", "a.kul", a.indexOf("ann"))
      expect(hover?.markdown).toContain("Ann")
    } finally {
      client.dispose()
    }
  })

  it("restarts after a crash with the file set intact", async () => {
    const { client, statuses, restarts } = startClient()
    try {
      client.replaceFiles([{ name: "a.kul", text: PERSON }])
      await client.ready
      client.crashWorker()
      await until(() => (restarts() > 0 ? true : undefined))
      expect(statuses.at(-1)).toMatchObject({ state: "ready", restarts: 1 })
      const hover = await client.query("hover", "a.kul", 7)
      expect(hover?.markdown).toContain("Ann")
    } finally {
      client.dispose()
    }
  })
})
