# ADR 0013 — Project manifest (`kul.yml`)

**Status:** Accepted
**Date:** 2026-05-09
**Deciders:** owner

## Context

The Kul language version was originally declared inside the `.kul` source as `kul 0.1` on the first non-blank line. That tied a piece of metadata *about* the source to the grammar of the source itself, which is awkward for three reasons:

1. The version literal is metadata — every conforming tool needs to read it before anything else, but it isn't kinship and shouldn't be authored as kinship.
2. Multi-file Kul projects (issue #63) need a project-level seat for configuration that goes beyond the version: enumeration of source files, exporter options, custom rules. There's no natural place to put any of that inside a `.kul` document, and inventing one stretches the grammar.
3. Every Kul-language consumer (this toolchain today, third-party tools tomorrow) needs the same discovery rule — "what version is this written for?" — and reinventing it per implementation invites drift.

Three positions on how to lift the version metadata out:

1. **Conventional-but-not-normative.** The toolchain ships its own manifest format; another implementation could disagree. Maximum freedom; minimum interoperability.
2. **Normative + optional.** The spec defines a manifest format. A `.kul` file may carry one alongside it. If absent, the parser falls back to "latest version known."
3. **Normative + required.** The spec defines a manifest format. A `.kul` file without a sibling manifest is not a valid Kul project. Tools MUST report the absence as an error.

## Decision

**Position 3.** A Kul project is "a `kul.yml` plus one or more `.kul` files in the same directory." The manifest is normative and required.

Concrete shape (today; manifest schema evolves alongside the language version per the additivity principle):

```yaml
kul: "0.1"
```

**Filename**: `kul.yml`. Lowercase, single dot, `.yml` extension (matches the seven existing `.yml` files in the repo; zero `.yaml`).

**Discovery**: directory-scoped. The manifest for `<dir>/<file>.kul` is `<dir>/kul.yml`, and only that. No walk-up to ancestor directories. (The multi-file refactor that follows this issue may revisit; today the rule is purely directory-scoped.)

**Conformance**: `kul-core::check` requires a `&Manifest` argument; the in-grammar `kul X.Y` line is removed. CLI subcommands load `kul.yml` from the input file's parent directory before calling `check`. The LSP loads it once per URI at `did_open`. The WASM bridge takes the manifest as a JS object alongside the source.

**Diagnostics infrastructure is deferred.** Manifest errors in this issue are reported by adapters as ad-hoc strings (CLI: stderr; LSP: synthetic LSP `Diagnostic` at byte 0..1; WASM: `tsify` deserialization failure). Promoting these to a typed `ManifestDiagnostic` with normative `KUL-Mxx` codes lands with the multi-file type-system refactor that follows this issue, when the unified diagnostic infrastructure (a `FileSpan` over both `.kul` source and `kul.yml` as different `FileId`s) needs to exist anyway.

## Consequences

**Positive.** The `.kul` grammar simplifies — `kul` is no longer a reserved keyword; the lexer drops `KulKw`; the parser drops `parse_version_decl`; the AST drops `Document.version`. The two source files that previously read `kul 0.1\n\nperson alice …` now read just `person alice …`, and the version lives in a sibling `kul.yml`.

The manifest is the seat #63 needs. When multi-file projects land, the manifest gains fields like `files: [...]` or globs without needing a fresh round of "where do project-level settings live?" debate.

Every Kul-language consumer agrees on one discovery rule. A future third-party Kul tool reads the same `kul.yml` to learn the version; nothing about the rule is implementation-private to this toolchain.

**Negative.** The "open a single `.kul` file" workflow now requires a sibling `kul.yml`. Users hitting the missing-manifest error need to know what to write — the error message points them at the manual fix. A future `kul init` subcommand (out of scope for this issue) is the obvious ergonomic improvement.

The WASM bridge takes one extra argument. JS callers must construct `{ kul: "0.1" }` themselves; there is no implicit default. This is the right tradeoff — a JS host doesn't have the on-disk path to discover the manifest from, and pretending otherwise would bury the question of "which version is this source written for?" in the bridge.

The change is breaking. There are no real Kul users yet (per project policy), so the breaking change is intentional and shipped without a migration path.

## Alternatives considered

**Conventional-but-not-normative manifest.** Rejected because it gives up on Kul as a language with a single conformance story. If two third-party implementations disagree about the manifest, every consumer between them ends up implementing both rules, and the language fragments.

**Normative + optional with a "latest version known" fallback.** Rejected because the failure mode is silent: a `.kul` written for 0.2 but missing a manifest gets parsed as 0.3 by a 0.3-aware tool with no warning. The required posture turns the failure mode loud — a missing manifest is an error every implementation reports the same way.

**`kul.yaml` extension.** Rejected because the repo already standardizes on `.yml` (seven existing `.yml` files; zero `.yaml`). One extension, one rule.

**Inline a `manifest_version:` field.** Rejected because the manifest schema already evolves alongside the Kul language version. Adding a separate version axis for the manifest itself doubles the surface for no win — additive new fields don't need a version bump, and breaking changes already gate on a major language version increment.
