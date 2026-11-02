# Architecture

The implementation map for KulLang. Read this when you need to make a change and don't yet know where to put it.

For domain vocabulary (Person, Marriage, Birth, ResolvedDocument, etc.), see [`CONTEXT.md`](../CONTEXT.md). For the test layout, see [`testing.md`](./testing.md). For ADRs that explain *why* a particular shape was chosen, see [`adr/`](./adr/).

## Vocabulary

This document uses the architecture words consistently:

- **Module** — anything with an interface and an implementation: a function, a type, a Rust module, a crate. The unit of "this thing has a public face and an internal one."
- **Interface** — everything a caller must know: types, invariants, error modes, ordering, configuration. Not just the function signature.
- **Implementation** — the code inside.
- **Seam** — where an interface lives. A place behaviour can be altered without editing in place. `ResolvedDocument` is a seam; the language-service feature modules are adapters at the editor seam.
- **Depth** — leverage at the interface. A small interface that hides a lot of behaviour is **deep**; an interface nearly as complex as its implementation is **shallow**. The validator's `Diagnostic` type is deep; the workspace's UTF-16 offset conversion is appropriately shallow.
- **Deletion test** — imagine deleting a module. If complexity vanishes, it was a pass-through. If complexity reappears across N callers, it was earning its keep.

## Pipeline

A project's manifest YAML plus its [`InputFile`](../CONTEXT.md#kulfile)s flow through the toolchain like this:

```
(manifest_name, manifest_yaml, &[InputFile])
  │
  ▼  manifest::validate    produces typed Manifest + KUL-Mxx diagnostics
  │                        (anchored at FileId::MANIFEST in kul.yml)
  │
  ▼  lexer.rs              produces tokens per .kul file
  │
  ▼  parser.rs             produces statements per .kul file (KulFile)
  │
  ▼  ast::Document         multi-file container the rest of the pipeline
  │                        operates on (manifest at FileId(0); .kul files
  │                        at FileId(1..))
  │
  ▼  semantic.rs           produces ResolvedDocument (project-wide id + children indexes)
  │                        emits rule 01 diagnostics inline (duplicate ids,
  │                        scoped project-wide per ADR-0015)
  │
  ▼  validator.rs          runs rules 02–12 per file, then rule 13 once
  │                        across the project; rule 13 delegates to cycles.rs
  │                        which walks the project-wide parent graph
  │
  ▼  CheckResult { resolved, diagnostics, manifest }
```

`kul_core::check(manifest_name, manifest_yaml, &[InputFile])` is the single entry point that runs the whole pipeline. The CLI calls it once per project: each subcommand is CWD-rooted (no positional `<file>`), discovers the manifest and every sibling `.kul` via the shared `kul-loader` crate, and hands the bytes off in one call. The browser `Workspace` (`kul-wasm`) calls it once per edit, lazily: it holds the project's files, and the first request after a change re-checks the whole set and caches the result for every request until the next change. The pipeline operates on `Document.kul_files` with `N>=1` consumers (per [ADR-0015](./adr/0015-global-project-namespace.md)): the resolver builds one project-wide id index, the validator threads file iteration through it, and R01/R02/R13 honour the global namespace.

The shape is deliberately linear. Each pass produces a strictly richer artifact; nothing earlier in the pipeline ever consults something later. This is why:

- The lexer doesn't know about IDs.
- The parser doesn't know which IDs are declared.
- The validator never reaches into raw `KulFile.statements` — it queries through `ResolvedDocument` (per [ADR-0001](./adr/0001-resolved-document-as-query-seam.md)). All thirteen spec rules (R01 lives inline in `semantic::resolve` because it's a property of insertion order; R02–R13 live in `validator.rs`).

## Crate map

```
kul-core   ── library (no_std-friendly intent, but uses std for now)
              the entire pipeline lives here. Public surface is the
              CheckResult API plus the AST types, ResolvedDocument
              query methods, the formatter, the export module
              (kinship-native + cytoscape projections), and the query
              module (the kinship-query seam: id → detail lookups and
              lineal kin-set queries today, collateral kin and
              relationship resolution later — one evaluation path over a
              declarative Query value, layered over ResolvedDocument,
              never over the export; ADR-0024/0025/0026).

kul-cli    ── thin binary `kul`
              Four subcommands over a project directory (`[DIR]`,
              default the working directory, read flat for `*.kul`):
              `validate` (renders diagnostics with miette), `format`
              (canonicalize per ADR-0004), `export` (JSON via
              kul_core::export, or SVG via kul-visual), and `query` (id
              → detail lookups, kin-set queries and relationship
              resolution via kul_core::query). Owns argument parsing and
              human/JSON output formatting; `query --format json` prints
              the QueryEnvelope (ADR-0024).

kul-language-service ── library
              The language features as pure functions over a checked
              project: diagnostics, hover, complete, definition,
              references, prepare_rename and rename, symbols,
              quick_fixes, semantic_tokens, lexical_tokens, format,
              entity_at, locate and details. No IO, no editor protocol,
              no async runtime. Files are addressed by name, offsets are
              UTF-8 bytes, results are Kul-native types. Cross-file
              features are project-wide per ADR-0015. Never re-implements
              pipeline logic — it asks ResolvedDocument and node_at.

kul-layout ── library
              Positioning pass for the canonical UI pattern. Public
              surface is one function: `layout(&RenderShape,
              &LayoutConfig) -> PositionedShape`. Two deep modules:
              `walker` (Buchheim O(n) Reingold–Tilford–Walker port,
              with sibling-subtree collision avoidance) and `adapter`
              (canonical-pattern wrapper — thick marriage edges
              between spouses, ghost slots per current-intimacy placement, generation rows,
              orthogonal right-angle edge routing). `PositionedShape`
              is an internal Rust seam — not Serialize, not
              schema-versioned (ADR-0016). Depends on kul-render.

kul-svg    ── library
              Theme-agnostic SVG emitter. Public surface is one
              function: `render(&PositionedShape, &ThemeConfig) ->
              String`. Emits SVG whose classes name the entity type
              (`kul-card`, `kul-edge`) and whose every property is a
              `data-*` attribute (ADR-0016, ADR-0021), with no inline
              colours; theming is a per-surface stylesheet concern.
              SVG-only forever —
              downstream consumers convert to raster via standard
              tools. Depends on kul-layout and kul-render.

kul-wasm   ── library (cdylib + rlib), built with wasm-pack --target web
              The browser's engine: one stateful `Workspace` class. It
              holds a project's files, caches one check per edit, and
              serves every kul-language-service feature plus the render
              (`analyze`, `renderSvg`) through kul-visual, converting
              every offset to UTF-16 code units at the boundary. Surface
              shape is settled in ADR-0011; TypeScript types are derived
              via Tsify, committed, and diffed per ADR-0012.
```

The dependency graph is unidirectional: `kul-cli → kul-visual → kul-core`, `kul-wasm → kul-language-service → kul-core`, `kul-wasm → kul-visual`, and `kul-visual → kul-svg → kul-layout → kul-render → kul-core` for the visual-rendering pipeline. Nothing depends on the CLI or the WASM crate; nothing in core depends on the language service or the visual-rendering crates. The visual-rendering crates sit beside the language service, not under it, so the editor features never drag the canonical-pattern projection in, and the workspace composes the two. New crates should preserve this unidirectional shape.

### Why a separate `kul-language-service` crate at all?

The editor features (hover, completion and its token-stream classifier, rename, highlighting, …) are language-tooling concern, not language concern: `kul-core` answers what a project *means*, the service answers what an editor should *show*. Keeping them out of `kul-core` keeps the CLI and the core free of them; keeping them out of `kul-wasm` keeps them testable as plain Rust with no JavaScript boundary and no offset conversion in sight. The deletion test passes: removing the crate would either move the features into the WASM adapter, mixing them with ABI and UTF-16 concerns, or eliminate editor support.

### Why a separate `kul-wasm` crate at all?

The browser cannot shell out to the `kul` binary; it needs a JS-callable surface, and compiling to WebAssembly with `wasm-bindgen` is the only viable path. The bridge has its own concerns — `cdylib` crate type, `wasm-bindgen` derives, `console_error_panic_hook` registration, the workspace's cached state, UTF-16 offsets, the `--target web` build — that have no place in `kul-core`, the language service or the CLI. The deletion test passes: removing `kul-wasm` would either reproduce the JS adapter elsewhere, or eliminate the browser app. The `tsify` features of `kul-core` and `kul-language-service` exist exactly so the WASM crate can derive accurate `.d.ts` from the Rust source of truth without forcing tsify onto the CLI's build graph (per [ADR-0012](./adr/0012-tsify-derived-types-committed-and-diffed.md)).

## Language-feature request flow

A request from the editor lands here:

```
editor (CodeMirror, on the main thread)
  │  the language client (apps/web/src/language/client.ts): every edit sends
  │  the file's whole text with a new version; answers that an edit has
  │  overtaken are dropped
  ▼  Comlink
language worker (apps/web/src/language/worker.ts)
  │  method call with a file name and UTF-16 offsets
  ▼
kul_wasm::Workspace::<method>
  │  offsets → UTF-8 bytes through the file's Utf16Index
  │  the cached CheckResult (re-checked only if a file changed since)
  ▼
kul_language_service::<feature>(&check, file, byte_offset)
  │  file name → FileId; resolved.node_at(file, offset) → Option<Node<'_>>
  │  pattern-matches the Node variant; cross-file features resolve target
  │  FileIds through resolved.entity(name).file and name them by file name
  ▼
Kul-native result (byte spans keyed by file name)
  │
  ▼  the workspace converts every span back to UTF-16 units
JS value (Tsify-derived type)
```

In the worker, each edit answers with the file's lexical tokens at once, and analysis waits for edits to pause (200ms), a newer edit replacing the waiting one; a feature that needs the check (hover, completion, rename, …) runs the waiting analysis first. A crashed worker is replaced and given the file set again.

Lifecycle: `setFile(name, text, version)` replaces one file's text and drops the cached check; `renameFile` and `deleteFile` do the same for the file set. The next request re-checks the whole project once (per [ADR-0007](./adr/0007-resolved-document-owns-document.md) and [ADR-0015](./adr/0015-global-project-namespace.md)) and every request after it reads the cache. `analyze()` returns the project's diagnostics, each anchored at a named file, plus the rendered tree; the caller filters diagnostics per file. `lexicalTokens` reads only the stored text, so it never waits for a check.

Four things make this work:

1. The workspace holds one `CheckResult` — including the cached `ResolvedDocument` — for the whole project. A dozen requests after an edit trigger one resolve, not a dozen.
2. `node_at` is the shared foundation. Hover, definition, completion, references and rename all start with "what's at the cursor?" — implementing it once means they can't disagree about it.
3. `Node` is a typed enum, not a tag. Each feature pattern-matches the variants relevant to it; the compiler enforces exhaustiveness when a new variant lands.
4. Offsets are converted in exactly one place. The service never sees UTF-16 and JavaScript never sees bytes.

### The canvas

Each analysis that can draw the project carries the theme-agnostic SVG (`analyze().svg`); the canvas (`apps/web/src/app/canvas/`) swaps it into the page under a CSS-transform camera and paints it from the token sheet, reading only the `kul-svg` contract (the `kul-*` classes and `data-*` attributes, [ADR-0016](./adr/0016-visualization-pipeline-crate-boundaries.md), [ADR-0021](./adr/0021-language-properties-plumb-to-svg.md)). An analysis with errors has no SVG, so the canvas keeps the last one, dimmed. The canvas's own decisions — the camera's maths, the selection and linked marks re-applied by id after each swap, revealing an entity clear of the editor window and the details panel, the details panel's model and the legend's rows — are pure modules in `apps/web/src/canvas/`. The details panel asks the worker for `details(id)`, the link from the editor asks `entityAt(file, offset)` once the cursor rests, a selection asks `locate(id)` to move the editor's cursor, and the download asks `renderSvg("fileExport")`, the self-contained variant the CLI's `kul export --format=svg` writes ([ADR-0022](./adr/0022-svg-legend.md)).

### The docs

The docs ("Learn Kul", `/docs/`) are MDX pages under `apps/web/content/docs/`, compiled at build time by Fumadocs MDX into a collection (`apps/web/src/docs/collection.ts`). The routes load it only from their loaders (`src/docs/pages.ts`): the collection awaits its entries at the top level, and a static import would split every page's shared code into many more files. The layout, navigation and code blocks are the site's own components (`src/docs/`); the navigation follows each folder's `meta.json`. `src/docs/source.ts` builds a Fumadocs source on the same collection for what is generated at prerender: the Markdown variants (`/docs/<slug>.md`), `llms.txt` and `llms-full.txt` (`src/docs/llms.ts`), and the search index (`/docs/search.json`, Fumadocs' built-in search exported statically). Search runs in the browser in Fumadocs UI's dialog, which loads the first time it opens. Everything is prerendered, and React with the router ships as one `framework` chunk (`vite.config.ts`), so a page's first load fetches a handful of files.

Kul code on the site is coloured and checked by the engine itself, never by a grammar: `apps/web/src/highlight/` runs `kul-wasm` in Node at build time (`analyzeKul`: the lexical tokens with the semantic tokens over them, the diagnostics and the tree), and `rehype-kul` turns each ```kul block into a `<KulCode>` element. A block that doesn't check clean, or that is fenced `expect-error=KUL-…` and reports anything other than those codes, fails the build. A clean block's **Try it** leaves its files in `sessionStorage` (`src/store/try-it.ts`) and opens `/app/`, which makes them a new project. The diagnostics reference is held to `kul_core::diagnostic::DIAGNOSTIC_CODES` (exported as `diagnosticCodes()`) by a check in `vite.config.ts` and a unit test.

### The landing page

The landing page (`/`, `apps/web/src/landing/`) shows Kul with the engine's own output, made at build time: `landing-content.ts` runs the same Node highlighter on an example from `examples/` (its comments left out) and on each "What the language handles" example, checked with the lines it hides, and the Vite plugin in `vite-plugin.ts` serves the result as `virtual:kul-landing`. The sample must draw the example's tree and keep the tokens its callouts point at, or the build fails. The same plugin writes `og.png` (the sample's tree as the downloaded SVG colours it, rasterised by resvg with Inter), `favicon.ico`, `apple-touch-icon.png` and `sitemap.xml`; `robots.txt` is static. `scripts/check-landing.mjs`, run by `check-dist`, holds the prerendered page to that content.

### Offline

The site works offline after one visit. `npm run build` ends by writing `/sw.js` (`apps/web/scripts/build-sw.mjs`): Vite bundles `apps/web/src/sw/sw.ts` into one classic script, and Workbox's `injectManifest` adds the precache manifest, which lists every file the build wrote, found by glob: the pages, the SPA shell, the Markdown and text files, the docs search index, scripts, styles, the `.wasm`, fonts and icons. Files under `/assets/` are content-hashed, so the precache answers them first. Everything else on the origin goes to the network first and falls back to the precached copy when the network fails or takes longer than 3 seconds; every `/app` path falls back to `/_shell.html`. The worker is served `no-cache` with the site's CSP, which governs its fetches as it does the pages'.

Every page registers the worker (`apps/web/src/offline/updates.ts`, with `workbox-window`) and looks for a new build when it becomes visible and every hour. A new build installs beside the running one and waits; the page then shows "A new version of Kul is ready" with **Reload**, in the app's toaster or, on the landing page and the docs, one that loads only then. Reload writes the open project's pending edits, then lets the waiting worker take over and reloads the page into the new build. Nothing reloads on its own.

The site's version (`APP_VERSION`, shown in the footer) is `apps/web/package.json`'s, unless the build sets `KUL_SITE_VERSION` (`apps/web/scripts/site-version.mjs`); the end-to-end suite uses it to build a second site to update to.

## The seams

The most load-bearing interfaces in the codebase. Don't bypass these.

| Seam                                  | File                                          | What's behind it                                                                                                |
| ------------------------------------- | --------------------------------------------- | --------------------------------------------------------------------------------------------------------------- |
| `kul_core::check`                    | `crates/kul-core/src/lib.rs`                 | The whole pipeline. The CLI and the browser `Workspace` both enter here. Takes `&[InputFile]` — the project's files, which it puts in [file order](../CONTEXT.md#file-order) itself (per [ADR-0014](./adr/0014-file-identity-and-per-file-namespaces.md)); returns a `CheckResult` whose `resolved: ResolvedDocument` field is the cached query view (per [ADR-0007](./adr/0007-resolved-document-owns-document.md)). |
| `FileId` / `FileSpan`                 | `crates/kul-core/src/span.rs`                | Project-wide locators. `FileId` indexes into `Document.kul_files` (with `FileId::MANIFEST` = `FileId(0)`); `FileSpan` is the `(file, byte-range)` pair every diagnostic anchors on. AST nodes keep bare `ByteSpan` (per ADR-0014) — their owning `KulFile` provides file context implicitly. |
| `ResolvedDocument` query methods      | `crates/kul-core/src/semantic.rs`            | All kinship questions. ADR-0001 mandates queries go through this; raw AST iteration is the seam's job. Per-id lookups (`person`, `marriage`, `entity`) take a bare id (project-wide per [ADR-0015](./adr/0015-global-project-namespace.md)); per-file iteration helpers (`persons_in(file)`, etc.) keep the file parameter for per-file language features. One-hop parenthood: `parents_of`, `children_of`, `children_of_marriage` — inverse children indexes are built during `semantic::resolve` (owned statement-idx style, ADR-0007) so the query engine and detail lookup do not rebuild them. Owns its `Arc<Document>` so the resolved view can be cached alongside other artifacts (no self-referential lifetime). |
| `ResolvedDocument::node_at`           | `crates/kul-core/src/node_at.rs`             | "What's at byte offset X?" Returns a typed `Node`. Foundation for hover, definition, completion.                |
| `ResolvedDocument::statement_at`      | `crates/kul-core/src/semantic.rs`            | "What top-level statement encloses byte offset X?" Returns `&Statement`. Coarser than `node_at` — used by completion to know whether a cursor sitting on a fresh line is "still under" the previous statement. |
| `Node::entity_reference`              | `crates/kul-core/src/node_at.rs`             | "What entity (person / marriage) is the cursor pointing at?" Returns an `EntityNode` summary (id, kind, decl span, target). Used by goto-definition, find-references, rename. |
| `Diagnostic` + `Severity` + code + `detail` | `crates/kul-core/src/diagnostic.rs`    | The error currency. Carries spans, codes (KUL-Rxx), related info, and (per ADR-0006) an optional sub-case tag. |
| `field_meta::FieldMeta`               | `crates/kul-core/src/field_meta.rs`          | Per-field taxonomy: value shape, completion description, hover Markdown. Hover, completion, and semantic-tokens consume it (ADR-0005). |
| `export::export`                      | `crates/kul-core/src/export.rs`              | Canonical JSON projection of a `CheckResult` into an `ExportEnvelope`. Strict on errors; format-dispatched (kinship-native or cytoscape). The deep module the CLI's `kul export` calls, and the render pipeline's input. Schema documented in [`spec/15-export-schema.md`](../spec/15-export-schema.md); shape, posture, and versioning settled in ADRs 0008–0010. Person/marriage/parenthood-link wire shapes are single-sourced with the `query` lookups via `projection::build_one_*` (`crates/kul-core/src/projection.rs`). |
| `query::{person, marriage, details, evaluate, kin_query}` + `QueryEnvelope` | `crates/kul-core/src/query.rs` (+ `query/`) | The kinship-query seam. Id → detail lookups (`person(id)` / `marriage(id)`) return the same export shapes, `None` when absent (absence is the answer). `details(resolved, &[DetailTarget])` is the batched member of that family — one check, many targets, each answered with its own fields plus its relational neighbourhood, flat in the number of targets (ADR-0035, ADR-0037). `evaluate(resolved, &Query)` is the single evaluation path for kin-set queries over the declarative `Query` value; named sugar (`parents_of`, `ancestors_of(depth)`, …) desugars to it, and `kin_query` wraps it in the envelope (ADR-0025). Results carry the terminology-neutral `RelationshipDescriptor` (ADR-0026). `QueryEnvelope<T>` (gated on passing checks, strict per ADR-0009) single-sources the contract serialization the CLI `kul query` and the WASM query surface both emit. Layered over `ResolvedDocument`, never over the export; collateral kin, relationship resolution, and attribute filtering land here later (ADR-0024, PRD 0005). |
| `kul_layout::layout`                  | `crates/kul-layout/src/lib.rs`               | `(RenderShape, LayoutConfig) -> PositionedShape`. Positioning pass for the canonical UI pattern. Wraps Walker's algorithm with the marriage-edge / ghost-slot / generation-row adapter. `PositionedShape` is an internal Rust seam — not Serialize, not schema-versioned (ADR-0016). |
| `kul_svg::render`                     | `crates/kul-svg/src/lib.rs`                  | `(PositionedShape, ThemeConfig) -> String`. Theme-agnostic SVG emitter. Uses semantic CSS classes; consuming surfaces own theming via stylesheet (ADR-0016). |
| `kul_language_service::*`             | `crates/kul-language-service/src/lib.rs`     | The language features, one pure function each over a `CheckResult`, a file name and a byte offset. Results are Kul-native: `Location { file, span }`, `TextEdit`, `FileEdit`, and enums for completion, symbol, token and diagnostic kinds. Hover, completion and highlighting read `field_meta` (ADR-0005); completion classifies tokens first (ADR-0002). |
| `Utf16Index`                          | `crates/kul-wasm/src/utf16.rs`               | UTF-8 byte offset ↔ UTF-16 code-unit offset for one file's text, one per workspace file. The only place the two units meet. |
| `kul_wasm::Workspace`                 | `crates/kul-wasm/src/lib.rs`                 | The WASM/JS surface: a stateful class holding a project's files and one cached check (per [ADR-0011](./adr/0011-wasm-surface-three-shapes-no-wrappers.md)). One method per language feature plus `analyze` and `renderSvg`, each a thin call that converts offsets at the boundary. No convenience layer and no kinship queries. |

If you find yourself reaching around one of these (e.g. iterating `document.statements` from a feature module), stop and consider extending the seam instead.

## Where to add X

### A new validator rule

1. Add a function `fn rule_NN_<short_name>(resolved: &ResolvedDocument) -> Vec<Diagnostic>` in `crates/kul-core/src/validator.rs`. (R02–R13 all live here. R01 — duplicate ids — lives inside `semantic::resolve` because it's a property of insertion order.)
2. Call it from the rule jump-list at the top of the file.
3. Allocate a code `KUL-RNN`. Update [`spec/07-validation-rules.md`](../spec/07-validation-rules.md) with the rule definition.
4. Add a test `rule_NN_<short_name>` in `crates/kul-core/tests/validator.rs` covering the positive case (rule fires) and negative case (rule doesn't fire). Snapshot the diagnostic output.
5. If the rule needs a new kinship query ("does this person have any siblings?"), add it as a method on `ResolvedDocument` rather than walking the AST inline. ADR-0001.

### A new language feature

1. Read [ADR-0001](./adr/0001-resolved-document-as-query-seam.md) — your feature should phrase itself as a question against `ResolvedDocument`.
2. Add a module under `crates/kul-language-service/src/` with one `pub fn` taking `(&CheckResult, file: &str, …)` and returning a Kul-native type (byte spans, files by name). Re-export it from `lib.rs`. Give its result types the `cfg_attr(feature = "tsify", …)` derive.
3. Add inline tests, `insta` snapshots for structured output, in the same module; cover a cross-file case if the feature can reach another file.
4. Add a `Workspace` method in `crates/kul-wasm/src/lib.rs` that converts input offsets to bytes, calls the feature, and converts every span in the result back to UTF-16 (destructure the result exhaustively, so a new span field cannot slip through unconverted).
5. Cover the method in `crates/kul-wasm/tests/workspace.rs`, its offsets in `tests/utf16.rs`, and the built module in `tests/node/smoke.mjs`; run `just wasm` and commit the regenerated `.d.ts`.
6. If the feature needs new "what's at the cursor?" information, extend the `Node` enum in `node_at.rs` (additively — don't rename existing variants). If it keys on "what entity is the cursor pointing at?", call `node.entity_reference()` instead of pattern-matching the four id-bearing `Node` variants by hand.

### A new AST variant

This is the highest-risk change because the AST is a stable surface. Read the additivity principle (in agent memory) before starting.

1. Extend the relevant enum / struct in `crates/kul-core/src/ast.rs` — *additively*. New variant or new optional field; never reorder, rename, or remove.
2. Update the parser to produce the new shape.
3. Update `node_at.rs` so the cursor-resolver covers it (else hover/definition/completion will silently miss it).
4. Update the validator if any rule should care.
5. Update language-service feature modules whose `match` arms might now be non-exhaustive (the compiler will tell you).
6. Update the spec.

### A new field on a statement

Per ADR-0005 the field taxonomy lives in `field_meta`. Adding a field is mostly a one-table change.

1. Extend `FieldName` (in `lexer.rs`) and the relevant `*FieldKind` enum in `ast.rs` — *additively*.
2. Add a row to `META` in `crates/kul-core/src/field_meta.rs` and add the `FieldName` to the right `*_FIELDS` slice (canonical formatter order).
3. Update the parser to emit the new variant.
4. If the field is required, R03 needs a new arm; otherwise the validator picks it up automatically through field accessors.
5. Hover, completion, and semantic-tokens all read the new row at runtime — no editing needed in those features.

### A new sub-case on an existing rule

Per ADR-0006 a single rule can carry multiple sub-cases on the same primary span, distinguished by a `detail` tag. Add one when the quick fixes (or any other tooling consumer) need to behave differently per sub-case.

1. Add a `pub const` tag in `kul_core::diagnostic::detail` (naming: `<rule>-<short>`).
2. Set it on the producing diagnostic via `.with_detail(detail::TAG)`.
3. Match on it at the consumer (e.g. `crates/kul-language-service/src/quick_fixes.rs`).

### A new CLI subcommand

1. Add a variant to `Command` in `crates/kul-cli/src/main.rs`.
2. Add the implementation under `crates/kul-cli/src/commands/`.
3. Add an end-to-end test in `crates/kul-cli/tests/` using `assert_cmd`.

### A new kinship-query capability (kin-set variant, projection, or filter)

The query engine lives under `crates/kul-core/src/query/` and has exactly one evaluation path (ADR-0025). Extend it additively:

1. Add the new shape to the **`Query` value** (`query/pattern.rs`) — a new `QuerySource` / `PatternClassification` variant, a new `Projection`, or a filter field. Enums, not reshapes.
2. Teach the **evaluator** (`query/engine.rs`) to handle the new variant, and derive any new descriptor dimension in `query/descriptor.rs`. The descriptor's serialization is a committed contract (ADR-0026) — new dimensions were pinned at their `notApplicable` defaults up front, so populate them rather than adding fields.
3. Add **named sugar** in `query/sugar.rs` *defined as* its Query-value expansion — never a second evaluation path.
4. The adapter is thin wiring: a CLI verb (`crates/kul-cli/src/commands/query.rs`) that goes through the one core path and prints the query envelope for `--format json`.
5. Prove correctness **once** at the core seam with `insta` snapshots (`crates/kul-core/tests/kin.rs`) plus targeted fixtures. The CLI gets wiring/serialization tests only.

A **detail lookup** is not this. Lookups take an id (or a `DetailTarget`), not a `Query` value, and live directly in `query.rs` / `query/detail.rs` — see ADR-0024 for why absence rather than a typed error is their answer, and ADR-0037 for the batched one's shape. Step 4's CLI verb is also not automatic: the batched detail lookup deliberately has none, because an adoption's `(childId, marriageId)` pair and a mixed-kind batch have no CLI spelling that isn't CLI-only semantics. Its consumer is the language service's `details`, which feeds the canvas details panel.

### A new exported field

The projection module (`crates/kul-core/src/projection.rs`) projects every Person, Marriage, and parenthood-link field through a single per-entity builder (`build_one_person`, `build_one_marriage`, `build_one_birth_link`, `build_one_adoption_link`) that both the whole-graph export loop and the `query` lookups call. When you add a new field on a Person/Marriage/Adoption (per "A new field on a statement" above), the export side picks it up in three steps:

1. Add an optional field to the matching `Exported*` struct in `export.rs` (mark it `#[serde(skip_serializing_if = "Option::is_none")]` if not always present).
2. Read the field via the existing `*Stmt::<field>()` accessor in the matching `projection::build_one_*` builder and assign it.
3. Document the new field in [`spec/15-export-schema.md`](../spec/15-export-schema.md) — additively, since per [ADR-0010](./adr/0010-export-schema-versioning.md) new optional fields do NOT bump the `schema` integer.

The export snapshot suite (`crates/kul-core/tests/export.rs`) auto-grows as each example file changes shape, so the new field's representation gets snapshot-locked the moment you `cargo insta accept`.

If the new construct is structurally large enough that consumers might silently drop it (a new top-level collection, an existing field's semantics changing incompatibly), bump `SCHEMA_VERSION` in `export.rs` in the same change and document the bump in [ADR-0010](./adr/0010-export-schema-versioning.md).

### A new WASM-exposed method or type

The bridge is intentionally thin (per [ADR-0011](./adr/0011-wasm-surface-three-shapes-no-wrappers.md)) and adding to it should stay thin. Re-read that ADR before adding any new surface — the rule of three says wait until a third independent consumer asks for the same helper. A new language feature follows the recipe above; anything else:

1. Add a method to `Workspace` in `crates/kul-wasm/src/lib.rs` with `#[wasm_bindgen(js_name = "camelCaseName")]`. Its body converts offsets in, makes one call, and converts offsets out.
2. If the return type is new, derive `serde::Serialize` + `tsify::Tsify` with `#[tsify(into_wasm_abi)]` and `#[serde(rename_all = "camelCase")]`, on the type where it is defined (a `kul-core` or `kul-language-service` type behind their `tsify` feature, or a `kul-wasm` type). Its rustdoc becomes the JSDoc on the generated `.d.ts` — write it for a JS/TS consumer.
3. Prefer an existing type over inventing a parallel WASM-only one. Single source of truth.
4. Add a test in `crates/kul-wasm/tests/workspace.rs`, and in `tests/utf16.rs` if the method reads or returns offsets.
5. Extend `crates/kul-wasm/tests/typescript/usage.ts` to exercise the new surface from a real consumer perspective (`just wasm-check` runs `tsc --noEmit` on it) and `tests/node/smoke.mjs` to call it on the built module.
6. Run `just wasm` to regenerate `crates/kul-wasm/types/kul_wasm.d.ts`. Commit the diff. `just wasm-check` (per [ADR-0012](./adr/0012-tsify-derived-types-committed-and-diffed.md)) fails if the committed snapshot drifts from the regenerated output.

An `Option<T>` field projects to `T | undefined` and, over the WASM boundary, serializes to an *absent* key (tsify's default serializer omits `None`); a method returning `Option<T>` returns `undefined` for `None`.

### A new public type or function

If it's part of the kul-core surface, add `//!`/`///` rustdoc explaining the contract. This is the *interface*, not the implementation. Internal helpers can be terse; public surface earns documentation.

If the new type crosses the WASM boundary (i.e. it's part of `ExportEnvelope` or one of its descendants, or `kul-wasm` will reuse it), the rustdoc on the type *is* the JSDoc that lands in the generated `.d.ts`. Write it for a JS/TS consumer, not just a Rust reader. Mark optional fields `#[serde(skip_serializing_if = "Option::is_none")]` so they are omitted from the JSON when absent rather than serialized as `null`; `Option<T>` automatically projects to `T | undefined` (a `?`-marked TS field) via the existing `derive(Tsify)`.

## What not to add

Things to push back on if you find yourself reaching for them:

- A `Visitor` trait over the AST. Pattern matches on a 2-variant enum are clearer; add this only if the AST grows past ~6 statement variants. ADR-0001 anti-suggestion.
- A "framework" for parser error recovery. The grammar is small; ad-hoc recovery is fine. Reconsider if the grammar doubles.
- A shared language-feature query helper. `completion` and `hover` overlap a little; that's fine. Extract only when a third feature wants the same code (rule of three) — `entity_under` in `kul-language-service` is that extraction, for definition, references, rename and entity-at.
- Re-exposing `Document.statements` to external callers. ADR-0001 closes this off.
- A trait abstraction over "things that can validate." There's one validator; abstractions enabled by future-supposed alternatives are speculative.

## Performance budget

The current targets are **<100ms** for check plus the diagnostics projection on a 1000-statement project, and **<2ms** (release) for one `Workspace::analyze()` on `examples/09-family-across-a-century`. The budgets live as tests (not benchmarks), in each crate's `tests/perf.rs` ([`kul-language-service`](../crates/kul-language-service/tests/perf.rs), [`kul-wasm`](../crates/kul-wasm/tests/perf.rs), [`kul-visual`](../crates/kul-visual/tests/perf.rs) for the 5k-card render). Each asserts a ~5× ceiling so it doesn't flake on slow runners; the comment in the test records the actual target. `just perf-release` runs them under a release build.

If a change pushes a test over budget, *fix the regression* — don't loosen the assertion. If a new pass legitimately needs more headroom, raise the budget in the same change with a comment justifying why.

See [`testing.md`](./testing.md) for the rationale on perf-as-tests-rather-than-benches.
