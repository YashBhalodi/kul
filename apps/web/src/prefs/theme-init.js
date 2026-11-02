// Runs in <head>, before the first paint, on every page: applies the saved
// theme so the page never flashes the wrong one. It is a blocking external
// script (the CSP allows no inline script). theme.ts, beside it, owns the same
// storage key and resolution rule at runtime; theme.browser.test.ts proves
// the two agree.
;(function () {
  var preference = null
  try {
    preference = window.localStorage.getItem("kul-theme")
  } catch {
    // Storage can be unavailable (blocked cookies); follow the system.
  }
  var dark =
    preference === "dark" ||
    (preference !== "light" &&
      window.matchMedia("(prefers-color-scheme: dark)").matches)
  var root = document.documentElement
  root.classList.toggle("dark", dark)
  root.dataset.themePreference =
    preference === "light" || preference === "dark" ? preference : "system"
})()
