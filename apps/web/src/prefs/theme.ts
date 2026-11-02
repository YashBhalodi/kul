// The theme preference: system, light or dark, remembered in localStorage.
// src/theme-init.js applies it before the first paint; this module keeps it
// applied afterwards, follows the system setting while the preference is
// "system", and syncs across tabs.

import { createStore } from "@tanstack/react-store"

export type ThemePreference = "system" | "light" | "dark"
export type Theme = "light" | "dark"

export const THEME_PREFERENCES: ReadonlyArray<ThemePreference> = [
  "system",
  "light",
  "dark",
]

export const THEME_STORAGE_KEY = "kul-theme"

const DARK_QUERY = "(prefers-color-scheme: dark)"

export function isThemePreference(value: unknown): value is ThemePreference {
  return value === "system" || value === "light" || value === "dark"
}

/** The theme a preference shows, given whether the system prefers dark. */
export function resolveTheme(
  preference: ThemePreference,
  systemPrefersDark: boolean
): Theme {
  if (preference === "system") return systemPrefersDark ? "dark" : "light"
  return preference
}

/** The preference after "toggle theme": the opposite of what shows now. */
export function toggledPreference(shown: Theme): ThemePreference {
  return shown === "dark" ? "light" : "dark"
}

export function readThemePreference(): ThemePreference {
  try {
    const saved = window.localStorage.getItem(THEME_STORAGE_KEY)
    return isThemePreference(saved) ? saved : "system"
  } catch {
    return "system"
  }
}

function saveThemePreference(preference: ThemePreference): void {
  try {
    if (preference === "system") {
      window.localStorage.removeItem(THEME_STORAGE_KEY)
    } else {
      window.localStorage.setItem(THEME_STORAGE_KEY, preference)
    }
  } catch {
    // Storage can be unavailable; the choice then lasts for this page.
  }
}

export interface ThemeState {
  preference: ThemePreference
  systemPrefersDark: boolean
}

/**
 * The theme state. Prerendered HTML has no preference to read, so the store
 * starts at "system" and `connectTheme` loads the saved one on the client.
 */
export const themeStore = createStore<ThemeState>({
  preference: "system",
  systemPrefersDark: false,
})

export function shownTheme(state: ThemeState): Theme {
  return resolveTheme(state.preference, state.systemPrefersDark)
}

export function setThemePreference(preference: ThemePreference): void {
  saveThemePreference(preference)
  themeStore.setState((state) => ({ ...state, preference }))
}

export function toggleTheme(): void {
  setThemePreference(toggledPreference(shownTheme(themeStore.state)))
}

function applyTheme(state: ThemeState): void {
  const root = document.documentElement
  root.classList.toggle("dark", shownTheme(state) === "dark")
  root.dataset.themePreference = state.preference
}

/**
 * Load the saved preference, apply the theme on every change, follow the
 * system setting and other tabs' choices. Returns the disconnect function.
 */
export function connectTheme(): () => void {
  const media = window.matchMedia(DARK_QUERY)
  themeStore.setState(() => ({
    preference: readThemePreference(),
    systemPrefersDark: media.matches,
  }))
  const subscription = themeStore.subscribe(applyTheme)
  applyTheme(themeStore.state)

  const onSystemChange = (event: MediaQueryListEvent) =>
    themeStore.setState((state) => ({
      ...state,
      systemPrefersDark: event.matches,
    }))
  const onStorage = (event: StorageEvent) => {
    if (event.key !== THEME_STORAGE_KEY && event.key !== null) return
    themeStore.setState((state) => ({
      ...state,
      preference: readThemePreference(),
    }))
  }
  media.addEventListener("change", onSystemChange)
  window.addEventListener("storage", onStorage)
  return () => {
    subscription.unsubscribe()
    media.removeEventListener("change", onSystemChange)
    window.removeEventListener("storage", onStorage)
  }
}
