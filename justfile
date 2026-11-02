# Kul developer workflow. Run `just` for the default check.

default: check

# Every gate that runs on each change: Rust format, clippy and tests, the
# WASM build and its gates, the web app's gates and the release script's
# tests. Local-green implies CI-green: `.github/workflows/rust.yml` and
# `web.yml` run these same recipes.
check: fmt-check lint test wasm-check web-check web-build release-test

# Everything `check` runs, plus the browser suites, Lighthouse, the epic #343
# baseline diff and old-name search, and actionlint over the workflows.
verify: check e2e shots lighthouse baseline-diff vestige-check lint-workflows

# Run the full test suite via cargo-nextest.
test:
    cargo nextest run --workspace --no-tests=pass

# The per-edit perf gates under a release build — the check and diagnostics,
# the workspace's analyze() and the tree render — because the release
# wall-clock is the cost a user feels. `test` covers the debug ceilings.
perf-release:
    cargo nextest run -p kul-language-service -p kul-wasm -p kul-visual --release -E 'binary(perf)'

# Auto-format the workspace.
fmt:
    cargo fmt --all

# Verify formatting without rewriting.
fmt-check:
    cargo fmt --all -- --check

# Lint with clippy, deny warnings.
lint:
    cargo clippy --workspace --all-targets -- -D warnings

# Passthrough to `cargo run -p kul-cli --`. Example: `just run -- --help`.
run *ARGS:
    cargo run -p kul-cli -- {{ARGS}}

# Install this checkout's `kul` binary onto PATH via `cargo install`
# (`~/.cargo/bin/kul`). Idempotent — re-run after each code change to
# refresh the installed CLI, then use `kul` from any Kul project directory
# on disk. `--force` overwrites a previously-installed build.
install:
    cargo install --path crates/kul-cli --force

# Render a committed `tree.svg` next to every example project's `.kul`
# source, using this checkout's CLI. Output is byte-for-byte deterministic
# and host-independent, so CI auto-regenerates and commits any diff (see
# `.github/workflows/render-examples.yml`). Idempotent on a clean tree.
render-examples:
    scripts/render-examples.sh

# Build the WebAssembly module into `crates/kul-wasm/pkg/` (package
# `kul-wasm`, which the web app consumes as a workspace package) and copy its
# generated `.d.ts` to the committed snapshot. Requires `wasm-pack`.
#
# `--target web` emits an ES module whose default export fetches and
# instantiates the `.wasm`; `initSync` takes the bytes instead, which is how
# the Node smoke test loads it.
wasm:
    wasm-pack build crates/kul-wasm --target web --out-dir pkg --out-name kul_wasm
    cp crates/kul-wasm/pkg/kul_wasm.d.ts crates/kul-wasm/types/kul_wasm.d.ts

# Build the WASM and run its gates: the committed `.d.ts` matches the build
# (ADR-0012), the gzipped size budget (1 MB), the Node smoke test and the
# TypeScript compile-test.
wasm-check: wasm
    #!/usr/bin/env bash
    set -euo pipefail
    git diff --exit-code -- crates/kul-wasm/types/ || {
      echo "error: generated kul_wasm.d.ts differs from the committed snapshot; commit crates/kul-wasm/types/" >&2
      exit 1
    }
    size=$(gzip -c crates/kul-wasm/pkg/kul_wasm_bg.wasm | wc -c | tr -d ' ')
    budget=$((1024 * 1024))
    echo "gzipped .wasm: $size bytes (budget: $budget)"
    [[ $size -le $budget ]] || { echo "error: wasm gzip size $size exceeds budget $budget" >&2; exit 1; }
    node crates/kul-wasm/tests/node/smoke.mjs
    npx --yes -p typescript@5.6 tsc --project crates/kul-wasm/tests/typescript/tsconfig.json

# The web app's fast gates: the serve-dist unit tests, then the app's
# typecheck, lint and unit tests (Vitest, node and browser modes).
web-check: (_web "typecheck" "lint" "test")
    node --test --test-reporter=spec apps/web/scripts/serve-dist.test.mjs

# The web app's dev server, on a fresh WASM build.
web: wasm (_web "dev")

# The web app's production build with prerender, into apps/web/dist/client,
# then its _headers (CSP hashes) and the layout check (scripts/check-dist.mjs).
web-build: wasm (_web "build")

# Playwright end-to-end suites against the built site, served by
# apps/web/scripts/serve-dist.mjs, on Chromium, WebKit and Firefox.
e2e: web-build (_web "e2e")

# The screenshot tour (apps/web/e2e/tour.spec.ts), with axe on every state.
# Writes apps/web/test-results/shots/<state>-<theme>-<width>.png.
shots: web-build (_web "shots")

# Lighthouse CI (@lhci/cli) against serve-dist, asserting the score floors.
lighthouse: web-build (_web "lighthouse")

# Run npm scripts of apps/web in order.
_web +SCRIPTS:
    #!/usr/bin/env bash
    set -euo pipefail
    for script in {{SCRIPTS}}; do
      npm --prefix apps/web run "$script"
    done

# actionlint (with shellcheck, when installed) over .github/workflows.
lint-workflows:
    actionlint

# The release workflow's version lockstep check (scripts/release-versions.mjs),
# over fixture trees and this repository.
release-test:
    node --test --test-reporter=spec scripts/release-versions.test.mjs

# Epic #343 harness. scripts/pivot/ and these two recipes are deleted when
# the epic completes.

# Compare the current tree's behaviour with the pinned pre-pivot baseline.
baseline-diff:
    scripts/pivot/baseline.sh diff

# Search for the old surfaces' names. Warn-only until the greenfield sweep.
vestige-check:
    scripts/pivot/vestige-check.sh
