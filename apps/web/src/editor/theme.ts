// The editor's look, from the token sheet: it reads the app's CSS
// variables, so the light and dark themes follow the app's without a
// second CodeMirror theme. Token colours are `.tok-*` rules in styles.css,
// shared with every other place Kul code is shown.

import { EditorView } from "@codemirror/view"

export const kulEditorTheme = EditorView.theme({
  "&": {
    height: "100%",
    fontSize: "13px",
    backgroundColor: "var(--card)",
    color: "var(--card-foreground)",
  },
  "&.cm-focused": { outline: "none" },
  ".cm-scroller": {
    fontFamily: "var(--font-mono)",
    lineHeight: "1.6",
  },
  ".cm-content": { caretColor: "var(--foreground)", padding: "8px 0" },
  ".cm-cursor, .cm-dropCursor": { borderLeftColor: "var(--foreground)" },
  ".cm-line": { padding: "0 12px 0 6px" },
  ".cm-gutters": {
    backgroundColor: "var(--card)",
    color: "var(--muted-foreground)",
    border: "none",
  },
  ".cm-lineNumbers .cm-gutterElement": { padding: "0 6px 0 12px" },
  ".cm-activeLine": { backgroundColor: "var(--editor-active-line)" },
  ".cm-activeLineGutter": {
    backgroundColor: "var(--editor-active-line)",
    color: "var(--foreground)",
  },
  "&.cm-focused > .cm-scroller > .cm-selectionLayer .cm-selectionBackground, .cm-selectionBackground, .cm-content ::selection":
    { backgroundColor: "var(--editor-selection)" },
  ".cm-selectionMatch": { backgroundColor: "var(--editor-match)" },
  ".cm-searchMatch": {
    backgroundColor: "var(--editor-match)",
    outline: "1px solid var(--border)",
  },
  ".cm-searchMatch.cm-searchMatch-selected": {
    backgroundColor: "var(--editor-selection)",
  },
  "&.cm-focused .cm-matchingBracket, .cm-matchingBracket": {
    backgroundColor: "var(--editor-match)",
    outline: "1px solid var(--ring)",
    borderRadius: "2px",
  },

  // Diagnostics: a wavy underline in the severity's colour.
  ".cm-lintRange": {
    backgroundImage: "none",
    paddingBottom: "0",
    textDecorationLine: "underline",
    textDecorationStyle: "wavy",
    textDecorationSkipInk: "none",
    textUnderlineOffset: "3px",
    textDecorationThickness: "1px",
  },
  ".cm-lintRange-error": { textDecorationColor: "var(--destructive)" },
  ".cm-lintRange-warning": { textDecorationColor: "var(--warning)" },
  ".cm-lintRange-info, .cm-lintRange-hint": {
    textDecorationColor: "var(--link)",
  },
  ".cm-gutter-lint": { width: "1.1em" },
  ".cm-gutter-lint .cm-gutterElement": { padding: "0 0 0 4px" },
  ".cm-lint-marker": { width: "0.7em", height: "0.7em", content: "none" },
  ".cm-lint-marker-error": {
    content: "none",
    backgroundColor: "var(--destructive)",
    borderRadius: "50%",
  },
  ".cm-lint-marker-warning": {
    content: "none",
    backgroundColor: "var(--warning)",
    borderRadius: "50%",
  },
  ".cm-lint-marker-info": {
    content: "none",
    backgroundColor: "var(--link)",
    borderRadius: "50%",
  },
  ".cm-diagnostic": {
    padding: "6px 10px",
    borderLeft: "3px solid var(--border)",
    fontFamily: "var(--font-sans)",
    fontSize: "13px",
  },
  ".cm-diagnostic-error": { borderLeftColor: "var(--destructive)" },
  ".cm-diagnostic-warning": { borderLeftColor: "var(--warning)" },
  ".cm-diagnostic-info": { borderLeftColor: "var(--link)" },

  // Tooltips: hover, diagnostics, completion.
  ".cm-tooltip": {
    backgroundColor: "var(--popover)",
    color: "var(--popover-foreground)",
    border: "1px solid var(--border)",
    borderRadius: "calc(var(--radius) * 0.8)",
    boxShadow: "0 8px 24px -8px var(--shadow)",
    overflow: "hidden",
  },
  ".cm-tooltip-hover": { maxWidth: "28rem" },
  ".cm-tooltip.cm-tooltip-autocomplete > ul": {
    fontFamily: "var(--font-mono)",
    fontSize: "12.5px",
    maxHeight: "16em",
    padding: "4px",
  },
  ".cm-tooltip.cm-tooltip-autocomplete > ul > li": {
    padding: "3px 8px",
    borderRadius: "calc(var(--radius) * 0.6)",
    lineHeight: "1.5",
  },
  ".cm-tooltip.cm-tooltip-autocomplete > ul > li[aria-selected]": {
    backgroundColor: "var(--accent)",
    color: "var(--accent-foreground)",
  },
  ".cm-completionDetail": {
    marginLeft: "1em",
    fontFamily: "var(--font-sans)",
    fontStyle: "normal",
    color: "var(--muted-foreground)",
  },
  ".cm-completionMatchedText": {
    textDecoration: "none",
    fontWeight: "700",
    color: "var(--primary)",
  },
  ".cm-completionIcon": { opacity: "0.8", color: "var(--muted-foreground)" },

  // The search panel.
  ".cm-panels": {
    backgroundColor: "var(--muted)",
    color: "var(--foreground)",
  },
  ".cm-panels.cm-panels-top": { borderBottom: "1px solid var(--border)" },
  ".cm-panel.cm-search": {
    fontFamily: "var(--font-sans)",
    fontSize: "12px",
    padding: "6px 28px 6px 8px",
  },
  ".cm-panel.cm-search input, .cm-panel.cm-search button": {
    fontSize: "12px",
    borderRadius: "calc(var(--radius) * 0.6)",
  },
  ".cm-textfield": {
    backgroundColor: "var(--background)",
    border: "1px solid var(--input)",
    color: "var(--foreground)",
    padding: "2px 6px",
  },
  ".cm-button": {
    backgroundImage: "none",
    backgroundColor: "var(--background)",
    border: "1px solid var(--input)",
    color: "var(--foreground)",
    padding: "2px 8px",
  },
  ".cm-panel.cm-search label": { color: "var(--muted-foreground)" },
  ".cm-panel button[name=close]": {
    color: "var(--muted-foreground)",
    fontSize: "16px",
  },
})
