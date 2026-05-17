# CONTEXT

Canonical vocabulary for KulLang. When discussing this project — in issue titles, code reviews, hypothesis statements, ADRs, test names, PR descriptions — use these terms exactly as defined here. Don't drift into synonyms (no "service / handler / component / API"); when a concept is missing, extend this file in the same change.

The architecture vocabulary (**module**, **interface**, **seam**, **depth**, **adapter**, etc.) is documented in [`docs/architecture.md`](./docs/architecture.md) and used throughout the codebase. This file focuses on the project's domain and implementation nouns.

## What this project is

**KulLang** is the project: a language design (Kul) plus the reference toolchain that consumes it. **Kul** is the language itself — a DSL for describing human kinship as plain text. A **Kul document** is a `.kul` file; it contains a sequence of declarations a human writes by hand and a machine can validate, query, and render.

The project's design discipline is the **additivity principle**: adding new information to a Kul document must never require rewriting existing declarations. This shapes the AST (optional fields, stable IDs), the validator (rules tolerate omissions where the spec allows them), and the [version policy](./spec/13-versioning-policy.md) (new fields land additively).

## Kinship vocabulary (the language)

These are the user-facing nouns. They appear in `.kul` source, in spec section names, in diagnostic messages, in hover popovers, and in test names. They are also the names of the AST node types in `crates/kul-core/src/ast.rs`.

### Person

A declared individual: `person <id> name:"…" born:… died:… gender:…`. The **id** is the stable handle (lowercase + digits + underscore); the rest are **fields**. A person may carry a single sub-statement — either a **birth** or an **adoption** — declaring how they entered a family.

### Marriage

A declared union: `marriage <id> <spouse_a> <spouse_b> start:… end:… end_reason:…`. The two spouse positions reference declared persons by id. Marriages are identified, not anonymous — children link to a marriage by id.

A person may participate in multiple marriages (sequential or concurrent — concurrent marriages are valid; see `examples/04-polygamous-family/polygamous-family.kul`). The spec does not restrict marriages to particular gender combinations.

### Birth

A sub-statement under a person: `birth <marriage_id>`. It declares that this person is the biological child of the spouses of the named marriage. The person's **biological parents** are derived; they are not stored on the person directly.

### Adoption

A sub-statement under a person: `adoption <marriage_id> start:…`. Declares this person as adopted into the named marriage. A person may have both a `birth` (their biological origin) and one or more `adoption`s; all surface in the **parent set**.

### Field

Any `key:value` pair on a Person, Marriage, Birth, or Adoption. Fields are optional unless the spec marks them required (see [`spec/04-validation-rules.md`](./spec/07-validation-rules.md)). They are unordered. Repeating a field in the same declaration is an error (KUL-R05).

### Date literal

`YYYY`, `YYYY-MM`, or `YYYY-MM-DD`, optionally prefixed `~` to indicate **circa**. A **partial date** is one of the truncated forms (e.g. `1980` is the year-only form). A **circa date** is `~YYYY[-…]`. The two are independent: `~1980-03` is partial *and* circa.

### Spouse

A resolved Person on either side of a Marriage. The function `ResolvedDocument::spouses_of(&MarriageStmt)` yields them; if a spouse-id was unresolved, it's silently skipped (rule 02 has already reported it).

### Parent

A resolved Person derived from either a `birth` link (biological) or an `adoption` link (adoptive). The function `ResolvedDocument::parents_of(&PersonStmt)` yields the union; each is tagged with the link type.

### Child

The inverse of parent. There is no `child` declaration in the language — children are **derived**, not declared on parents. (This is what makes the additivity principle hold: adding a new child to a family does not require editing the parents' declarations.)

### Validator rule

One of the thirteen spec-defined checks (KUL-R01 through KUL-R13). See [`spec/04-validation-rules.md`](./spec/07-validation-rules.md). In code, each rule lives as a function in `crates/kul-core/src/validator.rs` named `rule_NN_<short_name>`; tests follow the same pattern.

### Diagnostic

An error or warning emitted by the manifest validator pass, the parser, the resolver, or the validator. Carries a **code** (`KUL-Mxx` for manifest, `KUL-Lxx`/`KUL-Pxx` for lex/parse, `KUL-Rxx` for validator rules), a **severity**, a **message**, an optional **primary** [`FileSpan`](#filespan), and optional **related** spans (each anchored to a `FileSpan`, possibly in a sibling file under project-wide resolution). The optional primary covers `KUL-M01` (manifest-not-found) — the only diagnostic with no source position to anchor at. `KUL-M06` (project has `kul.yml` but zero `.kul` files) anchors at the manifest. Rendered to the user via `miette` (CLI) or translated to LSP diagnostics (editor); the latter filters to the active URI's `FileId`.

### ExportEnvelope

The top-level value `kul export` (and the public `kul_core::export::export` function) emits. Either a **success envelope** carrying a `schema` number, the source's `kul` language version (sourced from the [`Manifest`](#manifest)), and the [`ExportedGraph`](#exportedgraph), or a **failure envelope** carrying the diagnostic list. The export is strict on errors per [ADR-0009](./docs/adr/0009-export-strict-on-diagnostics.md).

### Project manifest

The `kul.yml` file alongside one or more `.kul` files. Carries the Kul language version the source targets and (in the future) any project-level configuration. Required: a `.kul` file without a sibling `kul.yml` is not a valid Kul project. Discovery is directory-scoped — no walk-up. Defined normatively in [`spec/14-project-manifest.md`](./spec/14-project-manifest.md); decision recorded in [ADR-0013](./docs/adr/0013-project-manifest.md).

### Project (project-wide namespace)

A directory containing one `kul.yml` plus one or more `.kul` files. Every id declared in any of the project's `.kul` files is visible from every other file by bare name — there is no `import` statement, no namespace prefix, and no qualified-reference syntax. The file boundary is purely organizational; the project is one logical namespace. Subdirectories are not walked; non-`.kul` files are silently ignored. Defined normatively in [`spec/14-project-manifest.md`](./spec/14-project-manifest.md); decision recorded in [ADR-0015](./docs/adr/0015-global-project-namespace.md), which supersedes ADR-0014's Position B.

### Manifest

The typed Rust representation of the project manifest. Lives at `crates/kul-core/src/manifest.rs` as `pub struct Manifest { pub kul_version: String }`. Adapters (`kul-cli`, `kul-lsp`, `kul-wasm`) load the on-disk YAML and hand the **raw bytes** to `kul_core::check` (or hand a typed `Manifest` to `kul_core::check_with_manifest` from the WASM bridge); `kul-core` itself never reads the filesystem. The `manifest::validate(yaml, file)` pass produces a typed `Manifest` plus diagnostics with normative `KUL-M01..M05` codes anchored at the manifest's [`FileId`](#fileid) (per [ADR-0014](./docs/adr/0014-file-identity-and-per-file-namespaces.md)).

### CheckEnvelope

The top-level value `@kullang/wasm`'s `check(source)` function returns. A single-field object — `{ diagnostics: ExportedDiagnostic[] }` — carrying every diagnostic the validator produced (errors, warnings, and notes alike). An empty array means a clean document; consumers discriminate on emptiness, with no `ok` field. The diagnostic shape reuses [`ExportEnvelope`](#exportenvelope)'s failure-arm `ExportedDiagnostic` so CLI export and WASM check agree on one source of truth. Defined at `crates/kul-wasm/src/lib.rs`; surface decision recorded in [ADR-0011](./docs/adr/0011-wasm-surface-three-shapes-no-wrappers.md).

### ExportedGraph

The kinship-native graph projection inside a success [`ExportEnvelope`](#exportenvelope). Three flat collections — `persons`, `marriages`, `parenthood_links` — that mirror the language primitives one-to-one, with cross-references by id. Defined normatively in [`spec/16-export-schema.md`](./spec/16-export-schema.md); shape choice motivated in [ADR-0008](./docs/adr/0008-export-kinship-native-shape.md).

### Schema number

The `schema:` integer on a success [`ExportEnvelope`](#exportenvelope). Discriminator for the structural shape of the envelope; bumped only when consumers might silently mis-represent data by ignoring a new construct. Independent of the language version (the `kul:` field). Policy in [ADR-0010](./docs/adr/0010-export-schema-versioning.md).

## Implementation vocabulary

These names appear in code, ADRs, and architecture discussion.

### KulFile

One parsed `.kul` source file: name, raw source bytes, and a list of **statements**. Type lives at `crates/kul-core/src/ast.rs` (introduced by [ADR-0014](./docs/adr/0014-file-identity-and-per-file-namespaces.md)). AST nodes inside a `KulFile` carry bare [`ByteSpan`](#span--bytespan)s — their owning `KulFile` provides file context implicitly.

### Document

The multi-file project container: the [`Manifest`](#manifest) plus zero or more [`KulFile`](#kulfile)s, each addressable by a [`FileId`](#fileid). Type lives at `crates/kul-core/src/ast.rs`. Each `KulFile` is held behind an `Arc` so callers can keep cheap shared handles. At v1, the toolchain only ever constructs N=1 `kul_files`; the multi-file shape exists so subsequent issues (cross-`.kul`-file resolution, document merging) can build on file-aware spans without further breaking changes ([ADR-0014](./docs/adr/0014-file-identity-and-per-file-namespaces.md)).

### Statement

The two top-level AST nodes: `Statement::Person(PersonStmt)` and `Statement::Marriage(MarriageStmt)`. Sub-statements (birth, adoption) are nested inside a `PersonStmt`, not top-level.

### FileId

An opaque `Copy` newtype indexing into a [`Document`](#document)'s files (`crates/kul-core/src/span.rs`). `FileId::MANIFEST` (= `FileId(0)`) is the project manifest by convention; subsequent ids are the `.kul` files in input order. Adapters and tests reach for `FileId::MANIFEST` or read ids out of an existing `FileSpan`; `FileId::from_raw(u32)` is available for fixture construction.

### Span / ByteSpan

A `(start, end)` byte range into a single source string. Every AST node carries one. Used for hover ranges, goto-definition targets, completion contexts. Type lives at `crates/kul-core/src/span.rs`.

### FileSpan

A `(file: FileId, span: ByteSpan)` pair: the project-wide locator a [`Diagnostic`](#diagnostic) anchors on, and the shape of `EntityRef.span` and `EntityNode.ident_span`. Lives at `crates/kul-core/src/span.rs`. Decouples a span from the implicit "this file" context that AST nodes can rely on; introduced by [ADR-0014](./docs/adr/0014-file-identity-and-per-file-namespaces.md).

### Lexer / Parser

Two passes in `crates/kul-core/src/`. The lexer produces a flat token stream (`TokenKind` + span); the parser builds the AST. Both are hand-written and small (~350 + ~750 lines). Recovery is ad-hoc per production: hit an error, sync to newline, continue.

### Resolver

The function `kul_core::semantic::resolve(Arc<Document>) -> (ResolvedDocument, Vec<Diagnostic>)`. Walks every [`KulFile`](#kulfile) in the [`Document`](#document), builds the project-wide id-to-statement index, and reports duplicate ids (R01) inline as the index is populated. Lives at `crates/kul-core/src/semantic.rs`.

### ResolvedDocument

The **kinship-query seam** (per [ADR-0001](./docs/adr/0001-resolved-document-as-query-seam.md)). All cross-reference questions ("who are this person's parents?", "is this id declared?", "who are the spouses of this marriage?") are answered by methods on this type. Validator rules and LSP features query through it; raw AST traversal is reserved for the seam's implementation, not its callers.

Owns its [`Document`](#document) via `Arc<Document>` (per [ADR-0007](./docs/adr/0007-resolved-document-owns-document.md)) so the resolved view can be cached alongside other artifacts. The id index is **project-wide** (per [ADR-0015](./docs/adr/0015-global-project-namespace.md)): `resolved.person(id)`, `resolved.marriage(id)`, `resolved.entity(id)` take only the bare id and return the unique declaration regardless of which file owns it. Iteration queries (`persons()`, `marriages()`, `statements()`) walk every `.kul` file; `_in(file)` variants restrict to one file (the LSP uses them for per-URI symbol listings). `references_to(id, kind)` is project-wide too and returns `FileSpan`s; per-URI LSP consumers (find-references, rename) filter to the active file at the call site. `node_at(file, offset)` and `statement_at(file, offset)` keep their file parameter because byte offsets are inherently per-file. R01 fires across files; cross-file references resolve cleanly.

### Validator

The pass that runs spec rules R02–R13 over a `ResolvedDocument`, accumulating diagnostics. Lives at `crates/kul-core/src/validator.rs`. Each rule is a function; the validator's job is to call them and collect output. (R01 — duplicate ids — is the one rule that lives inside `semantic::resolve`, because the duplicate check is a property of insertion order as the entity table is built.) Rules R02–R12 iterate one `.kul` file at a time for deterministic source-order diagnostic grouping; R13 walks the project-wide parent graph in one pass so cross-file cycles are detected as single cycles.

### Cycle detector

A standalone algorithm at `crates/kul-core/src/cycles.rs`, called by rule 13 (parenthood cycles). Pure function over the project-wide parent graph; separated from the rule because the algorithm is independently testable and the rule is a thin shell around it. The graph spans every file in the project (per ADR-0015): cycles that cross file boundaries are detected just like within-file cycles.

### Node-at-cursor / `node_at`

The query `ResolvedDocument::node_at(byte_offset) -> Option<Node<'a>>`. Lives at `crates/kul-core/src/node_at.rs`. Returns a typed enum identifying *what's at the cursor* — keyword, identifier declaration, identifier reference (with resolved target), field name, field value. The shared foundation for hover, goto-definition, and completion. See [`docs/architecture.md`](./docs/architecture.md) for the data-flow diagram.

### Entity-reference accessor

The method `Node::entity_reference(&self) -> Option<EntityNode<'a>>` (in `crates/kul-core/src/node_at.rs`) collapses the four id-bearing `Node` variants (`PersonDeclId`, `MarriageDeclId`, `PersonRef`, `MarriageRef`) into a uniform summary: `kind`, `name`, `ident_span`, `is_decl`, and the resolved `target`. LSP features that key on "what entity is the user pointing at?" (goto-definition, find-references, rename) phrase themselves as a query for this summary instead of re-pattern-matching the four variants by hand.

The `target` is an `EntityTarget` carrying both the resolved AST statement and the `FileId` that owns it (the `Node::PersonRef`/`MarriageRef` reference variants carry the same `(FileId, &Stmt)` pair). Under project-wide resolution (ADR-0015) that file may be a sibling of the active URI's file; `EntityNode::decl_span()` returns the correct project-wide `FileSpan` directly so feature modules do not re-query `ResolvedDocument::entity(name)` just to recover the target's file.

### Server

The `tower-lsp` Backend implementation in `crates/kul-lsp/src/server.rs`. Owns the project cache, dispatches LSP requests to feature modules, advertises capabilities. `did_open` discovers the project from the opened URI (sibling `kul.yml` plus every `.kul` file in the URI's directory) and inserts one cache entry for the whole project; `did_change` mutates the URI's overlay and re-runs `kul_core::check` for the project; `did_close` flips the URI's overlay to `None` and evicts the entry when no URIs remain open. Diagnostic publishes broadcast to every project file (open or disk-only) so the Problems pane reflects project-wide health.

### Document cache

Project-keyed map from `ProjectRoot` (the URI's parent directory) to a [`ProjectEntry`](#projectentry) in `crates/kul-lsp/src/state.rs`. Every URI that belongs to one project shares the same cached `CheckResult` and `ResolvedDocument` — opening a second `.kul` file from the same directory does not trigger a second resolve. Updated on `did_open` / `did_change` / `did_close`; evicted when the last open URI of a project closes.

### ProjectEntry

The cached value in the project cache (`crates/kul-lsp/src/state.rs`). Bundles the project's [`CheckResult`](#resolveddocument), the per-file [`LineIndex`](#lineindex) slice in `FileId(1..)` order, the matching URL slice (so features can map `FileId` ↔ `Url`), and the per-URI overlay map (editor-buffer source for open URIs, `None` for files only on disk). Cross-file features (goto-definition, find-references, rename, completion) read through this single entry; the URL slice is what turns a project-wide query result into a `Vec<Location>` keyed by the right URIs.

### View / Cursor

The per-request handles `ProjectEntry::view_for_uri(uri)` and `ProjectEntry::cursor_for_uri(uri, position)` return (`crates/kul-lsp/src/state.rs`). A `View` bundles the URI's `FileId`, the cached [`ResolvedDocument`](#resolveddocument), and the [`LineIndex`](#lineindex) for file-level LSP requests (document-symbol, semantic-tokens). A `Cursor` adds the byte offset for cursor-shaped requests (hover, definition, completion, references, rename, prepare-rename). Replaces the three-line `offset / file / resolved` setup every cursor-shaped request handler used to repeat inline; the UTF-16 ↔ UTF-8 conversion lives in one place. Returns `Option<Cursor<'_>>` so a stale client request past EOF — or a URI that isn't part of this project — resolves to `None` rather than panicking.

### Feature module

One per LSP feature — `crates/kul-lsp/src/features/{hover,definition,completion,diagnostics}.rs`. Each turns a typed request into a typed response by reading the document cache and querying through `ResolvedDocument` + `node_at`. None should walk the AST directly.

### Completion classifier

The token-stream-first context detector in `features/completion.rs`. Identifies which of seven contexts the cursor is in (TopLevelStart, IndentedUnderPerson, PersonFieldList, MarriageFieldList, AdoptionFieldList, AfterGenderColon, AfterEndReasonColon). Token-stream-first because partial / mid-typed input doesn't always parse cleanly. See [ADR-0002](./docs/adr/0002-token-stream-first-completion-classifier.md).

### LineIndex

Byte-offset ↔ LSP-position converter in `crates/kul-lsp/src/convert.rs`. Handles UTF-16 code-unit positions (LSP spec) ↔ UTF-8 byte offsets (kul-core), with CRLF round-trip safety.

## When this glossary is incomplete

If you're naming a concept that isn't here:

- **Common case** — you're inventing language the project doesn't use. Find the canonical term and use it.
- **Real gap** — the concept genuinely doesn't have a name yet. Add it here in the same change, with a one-paragraph definition. If the concept is load-bearing enough that future agents will need to understand *why* it exists, also write an ADR.

Architecture vocabulary (module / interface / seam / depth / adapter / leverage / locality / deletion test) is intentionally not duplicated here — see [`docs/architecture.md`](./docs/architecture.md).
