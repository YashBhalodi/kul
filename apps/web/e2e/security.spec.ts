import { expect, test } from "@playwright/test"

import { expectAppReady, recordCspViolations } from "./support"

const PAGES = ["/", "/docs/", "/docs/cheat-sheet/", "/app/", "/app/a-project"]

test("every page is served with the content security policy", async ({
  request,
}) => {
  for (const path of PAGES) {
    const response = await request.get(path)
    const csp = response.headers()["content-security-policy"] ?? ""
    expect(csp, path).toContain("default-src 'self'")
    expect(csp, path).toContain("connect-src 'self'")
    expect(csp, path).toContain("script-src 'self' 'wasm-unsafe-eval'")
    const scriptSrc = csp
      .split(";")
      .map((directive) => directive.trim())
      .find((directive) => directive.startsWith("script-src"))
    expect(scriptSrc, path).not.toContain("unsafe-inline")
    expect(response.headers()["x-content-type-options"]).toBe("nosniff")
    expect(response.headers()["referrer-policy"]).toBe("no-referrer")
  }
})

test("pages and the language engine load without a CSP violation", async ({
  page,
}) => {
  const violations = await recordCspViolations(page)
  await page.goto("/")
  await expect(page.getByRole("heading", { level: 1 })).toBeVisible()
  expect(await violations()).toEqual([])

  await page.goto("/docs/")
  await expect(page.getByRole("heading", { level: 1 })).toBeVisible()
  expect(await violations()).toEqual([])

  await page.goto("/app/")
  await expectAppReady(page)
  expect(await violations()).toEqual([])
})

test("hashed assets are immutable and the WASM is application/wasm", async ({
  page,
  request,
}) => {
  const wasm = page.waitForResponse((response) =>
    response.url().endsWith(".wasm")
  )
  await page.goto("/app/")
  const response = await wasm
  expect(response.headers()["content-type"]).toBe("application/wasm")
  expect(response.headers()["cache-control"]).toBe(
    "public, max-age=31536000, immutable"
  )
  const shell = await request.get("/app/")
  expect(shell.headers()["cache-control"]).toBe("no-cache")
})
