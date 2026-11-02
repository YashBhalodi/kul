import { afterEach, describe, expect, it } from "vitest"

import themeInitSource from "@/theme-init.js?raw"
import {
  THEME_PREFERENCES,
  THEME_STORAGE_KEY,
  connectTheme,
  readThemePreference,
  resolveTheme,
  setThemePreference,
  themeStore,
} from "./theme"

const realMatchMedia = window.matchMedia.bind(window)

/** Make `(prefers-color-scheme: dark)` report `dark`. */
function emulateSystem(dark: boolean) {
  window.matchMedia = (query: string) => {
    if (query !== "(prefers-color-scheme: dark)") return realMatchMedia(query)
    const fake: Pick<
      MediaQueryList,
      "matches" | "media" | "addEventListener" | "removeEventListener"
    > = {
      matches: dark,
      media: query,
      addEventListener: () => undefined,
      removeEventListener: () => undefined,
    }
    return fake as MediaQueryList
  }
}

/** Run theme-init.js the way a page does: as a classic script element. */
function runThemeInit() {
  const script = document.createElement("script")
  script.textContent = themeInitSource
  document.head.append(script)
  script.remove()
}

const isDark = () => document.documentElement.classList.contains("dark")

afterEach(() => {
  window.matchMedia = realMatchMedia
  localStorage.clear()
  document.documentElement.classList.remove("dark")
})

describe("theme-init.js", () => {
  for (const preference of THEME_PREFERENCES) {
    for (const systemDark of [false, true]) {
      it(`agrees with theme.ts for ${preference} on a ${systemDark ? "dark" : "light"} system`, () => {
        if (preference !== "system") {
          localStorage.setItem(THEME_STORAGE_KEY, preference)
        }
        emulateSystem(systemDark)
        runThemeInit()
        expect(isDark()).toBe(
          resolveTheme(readThemePreference(), systemDark) === "dark"
        )
        expect(document.documentElement.dataset.themePreference).toBe(
          readThemePreference()
        )
      })
    }
  }
})

describe("connectTheme", () => {
  it("applies the saved preference and every later change", () => {
    localStorage.setItem(THEME_STORAGE_KEY, "dark")
    emulateSystem(false)
    const disconnect = connectTheme()
    expect(themeStore.state.preference).toBe("dark")
    expect(isDark()).toBe(true)

    setThemePreference("light")
    expect(isDark()).toBe(false)
    expect(localStorage.getItem(THEME_STORAGE_KEY)).toBe("light")

    setThemePreference("system")
    expect(localStorage.getItem(THEME_STORAGE_KEY)).toBeNull()
    expect(isDark()).toBe(false)
    disconnect()
  })
})
