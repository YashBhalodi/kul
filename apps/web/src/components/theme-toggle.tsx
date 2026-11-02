import { useSelector } from "@tanstack/react-store"
import { MonitorIcon, MoonIcon, SunIcon } from "lucide-react"

import { setThemePreference, themeStore } from "@/prefs/theme"

const OPTIONS = [
  { value: "system", label: "System theme", Icon: MonitorIcon },
  { value: "light", label: "Light theme", Icon: SunIcon },
  { value: "dark", label: "Dark theme", Icon: MoonIcon },
] as const

/**
 * The three-way theme switch: system, light, dark. The highlighted option
 * follows `data-theme-preference` on <html> through CSS (theme-init.js sets
 * it before the first paint), so prerendered pages show the right choice
 * before they hydrate.
 */
export function ThemeToggle() {
  const preference = useSelector(themeStore, (state) => state.preference)
  return (
    <div
      role="group"
      aria-label="Theme"
      data-testid="theme-toggle"
      className="theme-switch inline-flex items-center gap-0.5 rounded-lg bg-muted p-0.5"
    >
      {OPTIONS.map(({ value, label, Icon }) => (
        <button
          key={value}
          type="button"
          title={label}
          aria-label={label}
          aria-pressed={preference === value}
          data-theme-option={value}
          onClick={() => setThemePreference(value)}
          className="flex size-7 items-center justify-center rounded-md text-muted-foreground transition-colors hover:text-foreground focus-visible:ring-3 focus-visible:ring-ring/50 focus-visible:outline-none [&_svg]:size-3.5"
        >
          <Icon aria-hidden="true" />
        </button>
      ))}
    </div>
  )
}
