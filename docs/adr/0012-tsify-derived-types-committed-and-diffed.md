# ADR 0012 — TypeScript types are derived from Rust via Tsify, committed, and diffed in CI

**Status:** Accepted
**Date:** 2026-05-06
**Deciders:** owner

## Context

`@kulalang/wasm` consumers are TypeScript users. They expect a typed API where their IDE catches mistakes — calling `exportGraph` with the wrong options shape, destructuring `parenthood_links` instead of `parenthoodLinks`, narrowing `GraphPayload` between kinship-native and Cytoscape arms. The runtime (the `.wasm` blob) and the types (the `.d.ts`) must agree.

Three positions on how to keep them in sync:

1. **Hand-write the `.d.ts` in `crates/kula-wasm/types/`.** Maximum control over the TS representation, but the runtime types live in `crates/kula-core/src/export.rs` (`ExportEnvelope`, `ExportedGraph`, `ExportedPerson`, etc.) — a Rust-side rename or a new field requires a manual TS-side edit, which a contributor will inevitably forget. Drift between Rust and TS is silent until a consumer hits it at runtime.
2. **Generate `.d.ts` at build time, never commit it.** Run `wasm-pack build` on every consumer install and trust the generated output. Eliminates the dual-source problem, but every PR that touches a Rust type that crosses the boundary lands as an opaque "WASM types changed" — reviewers can't see what the consumer-visible shape becomes without running the build locally.
3. **Generate `.d.ts` from Rust via [`tsify`](https://docs.rs/tsify), commit the generated file, and assert in CI that regenerating it produces no diff.** Rust source-of-truth + reviewable diff in PRs + impossible-to-merge drift, in exchange for one optional `kula-core` feature flag and one CI step.

The third option lines up with the project's existing snapshot-as-default discipline ([ADR-0003](./0003-snapshot-tests-as-primary-validation.md)). Generated artifacts are reviewed as diffs; CI gates the diff against the committed snapshot.

## Decision

`kula-core` carries an optional `tsify` feature (default-off). Behind the flag, every export-envelope type carries `#[cfg_attr(feature = "tsify", derive(Tsify))]`. `kula-wasm` is the only crate that enables `kula-core/tsify`; the CLI and LSP never pay the cost.

`kula-wasm` exposes its own surface types (`CheckEnvelope`) with the same `Tsify` derive. `wasm-pack build` writes the merged `.d.ts` to `crates/kula-wasm/pkg/kula_wasm.d.ts`. The `just wasm` recipe copies that file to the committed snapshot at `crates/kula-wasm/types/kula_wasm.d.ts`.

CI's `wasm-build` job ([`.github/workflows/rust.yml`](../../.github/workflows/rust.yml)) regenerates the `.d.ts` and runs `git diff --exit-code` against the committed snapshot. If a Rust type that crosses the boundary changed, the diff fails and the contributor must regenerate (`just wasm`) and commit. The CI gate is what makes the snapshot honest — the file is regenerated, not lovingly hand-edited.

The TypeScript consumer compile-test at `crates/kula-wasm/tests/typescript/usage.ts` runs `tsc --noEmit` against the same committed snapshot in CI. It exercises the consumer-visible patterns (discriminating on `ok`, narrowing `GraphPayload`, iterating `parenthoodLinks`, `@ts-expect-error` on illegal arguments) so a "compiles against itself" type that's not actually usable in real consumer code surfaces immediately.

## Consequences

- A Rust-side type change that affects consumers shows up in a PR as a reviewable `.d.ts` diff. Reviewers see the consumer surface change before merge; consumers don't get surprised on the next `npm install`.
- Drift between runtime and types is impossible to merge. The CI gate runs on every push and PR.
- The `.d.ts` lives next to its source-of-truth: re-deriving from `crates/kula-core/src/export.rs` is automatic; the snapshot in `crates/kula-wasm/types/` is one `cp` away.
- The cost of the `tsify` feature on `kula-core` is zero in production builds — default-off, only `kula-wasm` enables it. The CLI binary and the LSP server never pull `tsify` or `wasm-bindgen` into their dependency graphs.
- The TS compile-test is the second line of defense. The `.d.ts` could pass `git diff` but still emit a type that real TS code can't actually use (e.g. a recursive type that the inference engine gives up on); the compile-test catches this by exercising consumer patterns and asserting `tsc --noEmit` is clean.
- Adding a new `Exported*` field becomes one file to edit (the Rust struct) plus `just wasm && cargo insta accept`. The committed `.d.ts` updates automatically; CI confirms the consumer-visible surface is clean.

## Anti-suggestions (do not re-propose)

- **"Hand-write `kula_wasm.d.ts` so it has nicer JSDoc / cleaner types."** Drift is silent and inevitable. The whole point of deriving from Rust is that the type *is* the source of truth, not a manually-curated translation. If a derived type renders ugly, fix the Rust-side `Tsify` annotation, not the generated file.
- **"Drop the committed snapshot; just generate at install time."** Hides type changes from PR review. A Rust-side rename of `parenthood_links` → `parenthoodEdges` would land as an invisible runtime-only change in CI; consumers would see it only after `npm install`. The snapshot is what makes the change visible at review time.
- **"Skip the TS compile-test — `tsc --noEmit` against the `.d.ts` itself is enough."** It isn't. A type can be syntactically valid and still impossible to narrow/discriminate from real consumer code. The `usage.ts` fixture exercises the realistic patterns so consumers don't discover the gap on first integration.
- **"Move the `tsify` derive into `kula-wasm` instead of `kula-core` (so `kula-core` stays dependency-clean)."** Forces `kula-wasm` to redeclare every export-envelope type with the same fields, then convert at the boundary. That's exactly the dual-source problem this ADR exists to avoid. The `cfg_attr` + default-off `tsify` feature keeps `kula-core` clean for non-WASM consumers without forking the type definitions.
- **"Generate the snapshot from `wasm-pack` output as a build script (`build.rs`) so it's never out of sync."** Build scripts run during `cargo build`, which contributors run far more often than `wasm-pack`. A build script that regenerates the `.d.ts` would slow every local build for every contributor who isn't touching WASM. The dedicated `just wasm` recipe + CI diff is the right granularity.
