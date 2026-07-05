# ADR 0030 — Replace unmaintained `serde_yaml` with the maintained `serde_norway` fork

**Status:** Accepted
**Date:** 2026-07-06
**Deciders:** owner

## Context

The project manifest (`kul.yml`, specced in [`spec/14-project-manifest.md`](../../spec/14-project-manifest.md)) is parsed by [`kul-core::manifest`](../../crates/kul-core/src/manifest.rs) on every `kul validate` / `format` / `export` run and on every LSP project load. That path parses input the user did not necessarily author — a project cloned from a stranger carries its own `kul.yml`. The YAML parser therefore sits on an untrusted-input surface.

The parser was [`serde_yaml`](https://crates.io/crates/serde_yaml), which its maintainer **archived** — `Cargo.lock` carried it as `0.9.34+deprecated` and RUSTSEC flags it *unmaintained* (RUSTSEC-2024-0320). Any future YAML parsing vulnerability will never be patched upstream. The manifest schema is tiny (one `kul:` field today, per [ADR-0013](./0013-project-manifest.md)), so the swap is small and the right moment is before the surface grows (issue [#228](https://github.com/YashBhalodi/kul/issues/228)).

## Decision

### Swap to `serde_norway`, a maintained rename-fork of `serde_yaml` 0.9

[`serde_norway`](https://crates.io/crates/serde_norway) (0.9.42 at time of writing) is a maintained fork that preserves the `serde_yaml` 0.9 API verbatim: `from_str`, `to_string`, `Value` (with `String` / `Mapping` variants and `as_str`), and `Error::location() -> Option<Location>` with `Location::line()` / `column()` (0-indexed). Every one of these is load-bearing here — `KUL-M02` anchors its diagnostic span at the parser's reported `(line, column)` ([`manifest.rs`](../../crates/kul-core/src/manifest.rs)), so a fork that reported locations differently would shift diagnostics and snapshots. Because the fork is a straight rename, the migration is a mechanical path substitution: `serde_yaml::` → `serde_norway::` across the workspace dep, the `kul-core` optional dep, `manifest.rs`, and the `kul-layout` snapshot-serialization dev-dependency.

### The manifest format and the `yaml` feature name are unchanged

`kul.yml` stays YAML — a format migration is out of scope (see Alternatives). The `kul-core` `yaml` feature keeps its name: it describes the *format* the seam parses, not the crate that parses it, so `kul-wasm`'s deliberate opt-out (it receives an already-typed `Manifest` and never parses YAML, per [ADR-0012](./0012-tsify-derived-types-committed-and-diffed.md)) stays meaningful. The one private helper whose name advertised the old crate, `serde_yaml_span`, is renamed `yaml_error_span`.

### Behaviour-identical is the acceptance bar

The existing 13 manifest unit tests (including the `KUL-M02..M05` diagnostic paths, whose messages embed the parser's error text) plus the `kul-layout` corpus snapshots (which serialize `PositionedShape` through this crate) are the characterization suite for the swap. **Zero snapshot churn and byte-identical diagnostic wording** is the pass condition, not a formality — a diff would mean the fork is not a faithful drop-in. Verified green: no `.snap` changed, no diagnostic wording moved.

## Alternatives considered

- **`serde_yml`** — another `serde_yaml` fork. Rejected during planning over fork-quality concerns; `serde_norway` tracks the upstream 0.9 API more conservatively.
- **Migrate the manifest to TOML** — rejected. `kul.yml` is a specced, shipping surface ([`spec/14-project-manifest.md`](../../spec/14-project-manifest.md)); changing the on-disk format is a breaking change to solve a supply-chain problem that a drop-in fork solves without it.
- **Add size / nesting-depth limits to YAML parsing** — worthwhile input hardening, but orthogonal to the maintenance swap; deferred until the schema grows beyond the single `kul:` field.

## Consequences

- `Cargo.lock` swaps `serde_yaml` (+ `unsafe-libyaml`) for `serde_norway` (+ `unsafe-libyaml-norway`); `serde_yaml` no longer appears in the lockfile.
- No API change in `kul-core::manifest`; callers are untouched.
- Snapshot output and `KUL-M02..M05` diagnostics are byte-identical to the pre-swap state.
- Dependabot's grouped cargo updates now track `serde_norway`; nothing extra to configure.
- Future watch: if the manifest schema grows, revisit input hardening (document size, nesting depth) — deliberately out of scope here.
