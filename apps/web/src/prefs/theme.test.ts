import { describe, expect, it } from "vitest"

import { isThemePreference, resolveTheme, toggledPreference } from "./theme"

describe("resolveTheme", () => {
  it("follows the system only for the system preference", () => {
    expect(resolveTheme("system", true)).toBe("dark")
    expect(resolveTheme("system", false)).toBe("light")
    expect(resolveTheme("light", true)).toBe("light")
    expect(resolveTheme("dark", false)).toBe("dark")
  })
})

describe("toggledPreference", () => {
  it("chooses the opposite of the theme on screen", () => {
    expect(toggledPreference("dark")).toBe("light")
    expect(toggledPreference("light")).toBe("dark")
  })
})

describe("isThemePreference", () => {
  it("accepts exactly the three preferences", () => {
    expect(["system", "light", "dark"].every(isThemePreference)).toBe(true)
    expect([null, "", "Dark", "auto"].some(isThemePreference)).toBe(false)
  })
})
