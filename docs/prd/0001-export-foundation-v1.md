# PRD-0001: Export Foundation v1 — kula export CLI + canonical JSON schema + VSCode export command

## Problem Statement

Kula authors today have one and only one way to consume their `.kula` file: read it as text. There is no machine-consumable representation a downstream tool can load to render a family tree, generate a report, or feed into a visualization library. Anyone who wants to build a tool over Kula has to re-implement the entire pipeline (lex, parse, resolve, validate) themselves — or, more realistically, simply not build the tool.

This blocks the entire downstream consumer surface that Kula is designed to enable: web visualizers, VSCode webview live previews, static-site generators that bake a family tree into HTML, scripts that compute family statistics, and so on. Each would need parser-grade fidelity; without an export, none get it cheaply.

For the author working in VSCode specifically, even producing the JSON for ad-hoc use means leaving the editor, dropping to a terminal, and running a CLI invocation that doesn't reflect unsaved edits. There is no in-editor path from a `.kula` file to its JSON projection.

## Solution

Ship a canonical JSON export that becomes the foundation for every downstream consumer of Kula — both consumer apps the project may build later (web visualizer, VSCode webview live preview, etc.) and any third-party tool.

The export is exposed two ways in v1:

- A new CLI subcommand `kula export <file>` that writes the JSON envelope to stdout (or a chosen file).
- A VSCode command **Kula: Export to JSON** that produces the same JSON for the currently focused `.kula` document — including unsaved edits — and prompts the user for a save location. The command goes through the existing language server via a custom LSP request, so no second binary needs bundling and the in-memory buffer is what gets exported.

The export is **strict**: a clean validate is the precondition. If the document has any errors, the export refuses and emits the diagnostics. This keeps the foundation telling consumers a single simple thing — "here is the data, verbatim" — and leaves UX-y concerns like "render a partial view while the user is mid-edit" to the consumer (which can debounce, cache the last successful export, or render a banner).

The shape is **kinship-native**: three top-level collections — `persons`, `marriages`, `parenthood_links` — that mirror the language's primitives. Cross-references are by id; dates carry precision and circa; the schema is independently versioned from the language so additive language changes do not churn the schema.

A secondary `--format cytoscape` mode emits the same data in the Cytoscape JSON shape (`nodes` + `edges`), giving anyone with an existing Cytoscape, Sigma.js, vis-network, or Gephi workflow a one-step path from a `.kula` file to a graph picture.

## User Stories

1. As a Kula author, I want to run `kula export family.kula` and get a JSON document representing my family, so that I can pipe it into my own scripts without writing a parser.
2. As a Kula author, I want the export to fail loudly when my file has errors, so that I never accidentally feed a broken document into a downstream tool.
3. As a Kula author with a large file, I want the export to complete quickly (well under 100ms for hundreds of statements), so that it fits inside any pipeline without becoming a bottleneck.
4. As a Kula author, I want batch-export across multiple files (`kula export *.kula`), so that I can process my whole collection in one command.
5. As a pipeline integrator, I want `cat family.kula | kula export -` to read from stdin, so that I can compose the export with other tools.
6. As a CI operator, I want the exit code of `kula export` to be zero on success and non-zero on failure, so that my pipeline halts on a broken document.
7. As a downstream-app developer, I want a stable, documented JSON schema for the export, so that I can build a consumer app without worrying it will silently break on the next Kula release.
8. As a downstream-app developer, I want unknown fields and unknown enum values to be safe to ignore, so that additive Kula changes do not force me to immediately update my consumer.
9. As a downstream-app developer, I want an explicit schema version number, so that I can refuse to render data shaped by a schema I do not know about.
10. As a downstream-app developer, I want dates returned with their original precision (year vs month vs day) and circa flag preserved, so that I can render "1980" as "1980" and "~1980" as "c. 1980" without inventing a fake month or losing the fuzz.
11. As a downstream-app developer, I want cross-references kept as ids (not embedded objects), so that there are no cycles in the JSON and I can index entities however suits my framework.
12. As a downstream-app developer, I want the parenthood links promoted to a first-class collection, so that filtering "all children of marriage M" is a one-liner.
13. As a downstream-app developer, I want polygamous marriages, multiple parenthood links per child, and partial dates all faithfully represented, so that I do not have to special-case the kinship complexity Kula already supports.
14. As an integration developer with an existing Cytoscape or Gephi pipeline, I want a `--format cytoscape` mode that emits a `nodes` + `edges` graph I can drop into my tool, so that I can visualize a Kula file without writing a custom loader.
15. As an editor-integration developer, I want an opt-in `--with-positions` mode that includes byte-span information for each entity, so that I can build a "click on Alice → highlight her declaration" feature in any visualization.
16. As a default-CLI user, I want positions excluded by default, so that my output is small and clean unless I explicitly ask for them.
17. As a downstream tool author, I want the JSON envelope to carry the kula language version that produced the export, so that I can warn the user if the source predates a feature I rely on.
18. As an author editing a `.kula` file in VSCode, I want a single command to export the current document to JSON, so that I do not have to leave the editor.
19. As a VSCode author with unsaved edits, I want the export to reflect what is in my editor right now, so that I can preview a JSON shape without committing edits to disk.
20. As a VSCode author, I want a save dialog prompting me where to write the JSON, so that I control the destination.
21. As a VSCode author whose document has errors, I want a clear notification telling me to fix the problems first, so that I am not surprised by an empty file or stale JSON.
22. As a VSCode author, I want the command discoverable through the command palette under the name **Kula: Export to JSON**, so that I can find it without memorizing keybindings.
23. As a VSCode author, I want the resulting JSON file to default to a sensible name (e.g., `family.json` for `family.kula`), so that I do not have to type one.
24. As a VSCode author who triggers the command on a non-Kula file, I want a graceful "this only works for `.kula` files" message, so that nothing surprising happens.
25. As a contributor reading the spec, I want the export schema to be a normative spec section, so that any future implementer of an alternative exporter has a complete reference.
26. As a contributor maintaining the codebase, I want the schema-versioning policy and shape decisions captured as ADRs, so that future agents understand why the schema looks the way it does and can extend it without breaking consumers.

## Implementation Decisions

### Modules

- **A new `export` module inside `kula-core`.** The deep module of this PRD: a small public surface (one `export` function plus a few serializable types) hiding the projection logic, format dispatch, and strict-mode policy. Same crate as the validator and formatter — export is logically a sibling pipeline pass over `ResolvedDocument`. Per the architecture doc's deletion test, splitting it into its own crate would buy nothing.
- **Stable wire-format types** (`ExportEnvelope`, `ExportedGraph`, `ExportedPerson`, `ExportedMarriage`, `ExportedParenthoodLink`, `ExportedDate`, `ExportOptions`, `ExportFormat`) live alongside the export module. All `serde::Serialize`. These types ARE the schema; touching them touches the schema.
- **A Cytoscape transformer** as a sub-module of export, taking the canonical kinship-native graph and emitting the bipartite-node form (marriages promoted to nodes, spouse and parenthood links as typed edges with `p:` / `m:` id prefixes). Mechanical projection; cannot disagree with the canonical shape because it is derived from it.
- **A new `export` subcommand in `kula-cli`**, modeled on the existing `validate` subcommand: same stdin/-/multiple-file ergonomics, same exit-code discipline. Flags: `--format json|cytoscape`, `--with-positions`.
- **A new `export` LSP feature module in `kula-lsp`.** Custom request handler `kula/export` that takes a document URI plus options, reads the cached `CheckResult` from the document store, calls the kula-core export, and returns the envelope as the LSP response. Mirrors the existing feature-module pattern (one file per feature, all consuming the cached `ResolvedDocument` per ADR-0001). Thin adapter — the deep work is in `kula-core::export`.
- **A new VSCode command** `kulalang.export.json` registered in the extension's `package.json` with `when: editorLangId == kula`. The command sends the `kula/export` LSP request, opens a save-file dialog seeded with `<basename>.json`, writes the success payload, and surfaces a clear notification on failure.
- **No new crate** for v1. The `kula-wasm` crate, the wasm-pack pipeline, and the npm packaging surface are explicitly deferred to a follow-up backlog PRD.

### Contract

- **Strict on errors.** If the document has any error-severity diagnostics, the export refuses and returns a diagnostics envelope. Warnings do not block — a clean validate is the precondition; warnings are non-fatal observations.
- **Envelope shape.** Success: `{ ok: true, schema: <int>, kula: "<lang version>", graph: { ... } }`. Failure: `{ ok: false, diagnostics: [...] }`. Diagnostic shape reuses the existing `kula validate --format json` representation; single source of truth.
- **Kinship-native graph.** Three top-level collections: `persons`, `marriages`, `parenthood_links`. Cross-references by id only. No derived projections (no `person.children`, no `person.siblings`) — those are consumer-defined.
- **Dates as tagged structures.** Every date carries `value`, `precision` (`year` | `month` | `day`), `circa` (bool). No flat ISO strings.
- **Positions opt-in.** `--with-positions` adds `span: [start, end]` to each entity; default omits.
- **Schema version is independent of language version.** Schema bumps only when consumers might silently mis-represent data by ignoring a new construct (e.g., a brand-new top-level collection). Adding optional fields, new enum values, or new `parenthood_links.kind` values does NOT bump the schema — consumers handle these as forward-compatible additions.
- **VSCode command routes through LSP, not CLI subprocess.** The extension already manages exactly one bundled binary (the language server); shelling out for export would double the bundling/distribution surface for no gain. The LSP also has the in-memory buffer and cached parse, making the export essentially free latency-wise.

### Documentation

- **Spec section** for the export schema (will live alongside the existing 14 spec sections), normative, with a worked example matching `examples/03-three-generations.kula` and the forward-compatibility rules stated explicitly.
- **Three ADRs** documenting (a) the kinship-native shape choice and rejected alternatives, (b) the strict-on-diagnostics posture and the consumer-owns-UX principle behind it, (c) the schema-versioning policy.
- **Vocabulary update** in `CONTEXT.md`: add `ExportedGraph`, `ExportEnvelope`, `Schema number` to the glossary.
- **Architecture map update**: extend the "Where to add X" recipes with "a new exported field" (a one-row change in the field-meta table is the load-bearing step, since the exporter walks `field_meta` rather than hard-coding the field list).
- **Extension README** entry documenting the new VSCode command alongside other features.

### Sequencing

Suggested PR series, each independently reviewable:

1. **Contract:** spec section + three ADRs + glossary updates. Documentation only; surfaces all design decisions before any code lands.
2. **Default JSON export:** kula-core types + exporter + `kula export --format json` CLI subcommand + snapshot tests + CLI integration tests + perf test. After this PR, `kula export family.kula` works on the corpus.
3. **`--with-positions`:** additive flag, one boolean threaded through, snapshot variant per example.
4. **Cytoscape format:** additive `--format cytoscape`, snapshots, transformer tests.
5. **VSCode command + LSP custom request:** the `kula/export` handler in kula-lsp plus the extension command and `package.json` contribution. Integration test on the LSP side; manual smoke for the extension.
6. **Coordinated release:** version bump in `Cargo.toml` and `editor/vscode/package.json`, tag, ship — same lockstep flow documented in `docs/release.md`.

## Testing Decisions

What makes a good test in this codebase: snapshot-based, exercising external behavior (the JSON output a consumer sees), covering the example corpus end-to-end, running fast enough to be part of `just check`. Implementation details (how the projection walks the AST) are not tested directly — they are tested through the JSON they produce.

Modules getting tests:

- **The exporter** (`kula-core::export`): snapshot tests over every `examples/*.kula`, in JSON mode and Cytoscape mode, with positions on and off. Per ADR-0003, snapshot tests are the default for structured output; per the testing convention, the corpus auto-grows by dropping new files into `examples/`. Prior art: validator and formatter snapshot tests.
- **The strict-mode envelope:** a small set of hand-crafted bad inputs (duplicate id, unresolved reference, missing required field) snapshotted in their failure-envelope form. Three or four cases are sufficient — rule-by-rule diagnostic correctness is already covered by validator tests.
- **The CLI subcommand:** end-to-end tests using `assert_cmd`, mirroring the existing `kula validate` integration tests. Cover success path, failure path with non-zero exit, each `--format` and `--with-positions` flag combination, stdin and file inputs.
- **Performance:** a perf-as-test asserting the export pass completes within budget on a 1000-statement document. Target: <30ms; assertion at 5× CI ceiling (<150ms). Pattern mirrors `crates/kula-lsp/tests/perf.rs`.
- **The LSP custom request:** integration test using the existing minimal LSP client harness in `crates/kula-lsp/tests/`. Cover success on a clean document, failure on a document with errors, the with-positions flag round-trip, the cytoscape format round-trip. Prior art: existing per-feature integration tests.
- **The VSCode extension command:** not unit-tested for v1. The extension currently has no test suite; inventing one for a single command is out of proportion. Manual smoke-test acceptance is sufficient.

## Out of Scope

- **WASM packaging.** Compiling kula-core to WebAssembly, the `kula-wasm` adapter crate, npm/GitHub-release packaging, hand-written TypeScript types — captured in a separate backlog PRD that must ship before any browser-based consumer app is built.
- **A standalone `check()` public API.** Validation is currently embedded inside `export` as the precondition. A separate, dedicated `check()` surface (notably as a WASM call for editor-grade tooling) is captured in its own backlog PRD.
- **A query API** (`descendants`, `ancestors`, `siblings`, `alive_at_year`). Consumers derive these in JS over the exported graph; the foundation does not freeze any kinship-derivation semantics. Will be revisited only if a future consumer hits a wall pure JS-side derivation cannot solve cleanly.
- **The first consumer app itself** — web visualizer, VSCode webview live preview, theming, layout, filtering UX. The foundational call from the grilling session was "build the foundation, then grill the consumer app." A dedicated PRD will land after that grilling session.
- **Live-preview webview in VSCode.** A `kula/export`-on-every-keystroke webview is the natural follow-up once WASM ships, but is its own product epic (and depends on this PRD plus the WASM PRD).
- **Multiple-file export from VSCode.** Single-document is sufficient for the in-editor UX; multi-file users have the CLI's `kula export *.kula`.
- **Persistent export targets** ("export this file to the same location as last time") and **format selection in the editor.** The save dialog is shown every time; the VSCode command exports the default JSON shape only. Both are future polish items.
- **GEDCOM export.** A one-way GEDCOM bridge for genealogy-tool interop is a future ADR; entirely out of scope here.
- **Importing JSON back into Kula.** The export is one-way. The `.kula` file is the canonical source of truth.

## Further Notes

The deep module here is the exporter itself. Its interface — give it a `CheckResult` and options, get back an `ExportEnvelope` — is small enough to memorize and stable enough to outlast every future evolution of the language. The internal projection logic, by contrast, is the place where the kinship complexity (polygamous marriages, multiple parent links, partial/circa dates, optional fields) gets encoded into a wire format. Test the interface thoroughly; trust the snapshots to keep the projection honest.

The schema-versioning policy is the load-bearing reliability commitment of this PRD. A consumer that implements against schema 1 should still work against schema 1 generated a year later. The single-row-change `field_meta` discipline (per ADR-0005) is what makes that hold operationally — adding fields is one row, and the exporter picks it up automatically without any new code, so the temptation to break compatibility for convenience never arises.

The strict-on-diagnostics posture is the load-bearing simplicity commitment. The export does one thing; the consumer owns the rest. This is what keeps the foundation reliable for any downstream surface — including ones we have not yet imagined.

Routing the VSCode command through LSP rather than shelling out to the bundled CLI is the load-bearing distribution simplicity. The extension already manages exactly one bundled binary; adding a second binary just for export would double the bundling/distribution surface for no gain. Once the WASM PRD ships, the same custom request can become a webview-side call without rearchitecting the extension.

This PRD is the prerequisite for every consumer-app PRD that follows. Sequencing the WASM packaging, standalone `check()` API, and the first consumer app behind it is deliberate — each unblocks the next without forcing speculative work.
