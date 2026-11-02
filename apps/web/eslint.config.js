// @ts-check
import js from "@eslint/js"
import reactHooks from "eslint-plugin-react-hooks"
import globals from "globals"
import tseslint from "typescript-eslint"

export default tseslint.config(
  {
    ignores: [
      "dist/",
      "test-results/",
      "playwright-report/",
      ".lighthouseci/",
      "src/routeTree.gen.ts",
    ],
  },
  js.configs.recommended,
  ...tseslint.configs.strictTypeChecked,
  {
    languageOptions: {
      parserOptions: {
        projectService: { allowDefaultProject: ["*.js", "scripts/*.mjs"] },
        tsconfigRootDir: import.meta.dirname,
      },
      globals: { ...globals.browser },
    },
  },
  {
    files: ["**/*.{ts,tsx}"],
    plugins: { "react-hooks": reactHooks },
    rules: {
      ...reactHooks.configs.recommended.rules,
      "@typescript-eslint/no-confusing-void-expression": [
        "error",
        { ignoreArrowShorthand: true },
      ],
      // The router's notFound() and redirect() are thrown by design.
      "@typescript-eslint/only-throw-error": [
        "error",
        {
          allow: [
            {
              from: "package",
              package: "@tanstack/router-core",
              name: ["NotFoundError", "Redirect"],
            },
          ],
        },
      ],
      "@typescript-eslint/restrict-template-expressions": [
        "error",
        { allowNumber: true },
      ],
    },
  },
  {
    // Storage belongs to two folders: src/store/ (projects, and the last one
    // opened) and src/prefs/ (UI preferences such as the theme).
    files: ["src/**/*.{ts,tsx,js}"],
    ignores: ["src/store/**", "src/prefs/**"],
    rules: {
      "no-restricted-globals": [
        "error",
        ...["localStorage", "sessionStorage", "indexedDB"].map((name) => ({
          name,
          message: "Storage access lives in src/store/ or src/prefs/.",
        })),
      ],
      "no-restricted-properties": [
        "error",
        ...["localStorage", "sessionStorage", "indexedDB", "getDirectory"].map(
          (property) => ({
            property,
            message: "Storage access lives in src/store/ or src/prefs/.",
          })
        ),
      ],
    },
  },
  {
    // Node scripts and configs: plain JavaScript, checked without types.
    files: ["**/*.{js,mjs}"],
    extends: [tseslint.configs.disableTypeChecked],
    languageOptions: { globals: { ...globals.node } },
  }
)
