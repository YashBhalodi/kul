# Kul developer workflow. Run `just` for the default check.

default: check

# Every gate that runs on each change: Rust format, clippy and tests, the
# WASM build and its gates, and the web app's gates. Local-green implies
# CI-green: `.github/workflows/rust.yml` and `web.yml` run these same recipes.
check: fmt-check lint test wasm-check web-check web-build

# Everything `check` runs, plus the browser suites, Lighthouse, the epic #343
# baseline diff and old-name search, and actionlint over the workflows.
verify: check e2e shots lighthouse baseline-diff vestige-check lint-workflows

# Run the full test suite via cargo-nextest.
test:
    cargo nextest run --workspace --no-tests=pass

# The language server's perf gate under a release build: the release
# wall-clock is the per-edit cost a user feels. `test` covers the debug
# ceiling.
perf-release:
    cargo nextest run -p kul-lsp --release -E 'binary(perf)'

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

# Build both WebAssembly artifacts. Requires `wasm-pack`.
#
# `pkg/` is the `--target bundler` build published to npm as `@kullang/wasm`.
# Its generated `package.json` is patched so its `name` is `@kullang/wasm`
# (wasm-pack derives the npm name from the Cargo crate name, which is
# `kul-wasm`), and it is the source of the committed `.d.ts` snapshot.
#
# `pkg-web/` is the `--target web` build the VSCode preview loads in its
# webview (ADR-0040). It emits an ESM module with an `init(url)` entry point
# that fetches and instantiates, which is the lazy shape ADR-0034 wants; the
# extension stages it into `media/preview/wasm/` at package time. It is never
# published — the same commit's core, shipped inside one extension artifact.
wasm:
    wasm-pack build crates/kul-wasm --target bundler --out-dir pkg --out-name kul_wasm
    node -e 'const fs=require("fs"),p="crates/kul-wasm/pkg/package.json";const j=JSON.parse(fs.readFileSync(p));j.name="@kullang/wasm";fs.writeFileSync(p,JSON.stringify(j,null,2)+"\n")'
    cp crates/kul-wasm/pkg/kul_wasm.d.ts crates/kul-wasm/types/kul_wasm.d.ts
    wasm-pack build crates/kul-wasm --target web --out-dir pkg-web --out-name kul_wasm

# Build the WASM and run its gates: the committed `.d.ts` matches the build
# (ADR-0012), the package name and version, the gzipped size budget (1 MB),
# the Node smoke test and the TypeScript compile-test.
wasm-check: wasm
    #!/usr/bin/env bash
    set -euo pipefail
    git diff --exit-code -- crates/kul-wasm/types/ || {
      echo "error: generated kul_wasm.d.ts differs from the committed snapshot; commit crates/kul-wasm/types/" >&2
      exit 1
    }
    name=$(node -p 'require("./crates/kul-wasm/pkg/package.json").name')
    version=$(node -p 'require("./crates/kul-wasm/pkg/package.json").version')
    ws_version=$(grep -m1 '^version = ' Cargo.toml | sed -E 's/version = "(.*)"/\1/')
    [[ $name == "@kullang/wasm" ]] || { echo "error: package name $name != @kullang/wasm" >&2; exit 1; }
    [[ $version == "$ws_version" ]] || { echo "error: package version $version != workspace $ws_version" >&2; exit 1; }
    size=$(gzip -c crates/kul-wasm/pkg/kul_wasm_bg.wasm | wc -c | tr -d ' ')
    budget=$((1024 * 1024))
    echo "gzipped .wasm: $size bytes (budget: $budget)"
    [[ $size -le $budget ]] || { echo "error: wasm gzip size $size exceeds budget $budget" >&2; exit 1; }
    node --experimental-wasm-modules crates/kul-wasm/tests/node/smoke.mjs
    npx --yes -p typescript@5.6 tsc --project crates/kul-wasm/tests/typescript/tsconfig.json

# The web app's fast gates: the serve-dist unit tests, then the app's
# typecheck, lint and unit tests (Vitest, node and browser modes).
web-check: (_web "typecheck" "lint" "test")
    node --test --test-reporter=spec apps/web/scripts/serve-dist.test.mjs

# The web app's production build with prerender, into apps/web/dist/client.
web-build: wasm (_web "build")

# Playwright end-to-end suites against the built site, served by
# apps/web/scripts/serve-dist.mjs, on Chromium, WebKit and Firefox.
e2e: web-build (_web "e2e")

# The screenshot tour (apps/web/e2e/tour.spec.ts), with axe on every state.
# Writes apps/web/test-results/shots/<state>-<theme>-<width>.png.
shots: web-build (_web "shots")

# Lighthouse CI (@lhci/cli) against serve-dist, asserting the score floors.
lighthouse: web-build (_web "lighthouse")

# Run npm scripts of apps/web in order; skip while the app does not exist.
_web +SCRIPTS:
    #!/usr/bin/env bash
    set -euo pipefail
    if [[ ! -f apps/web/package.json ]]; then
      echo "skip: apps/web/package.json does not exist yet, so the web app's {{SCRIPTS}} gate(s) do not run"
      exit 0
    fi
    for script in {{SCRIPTS}}; do
      npm --prefix apps/web run "$script"
    done

# actionlint (with shellcheck, when installed) over .github/workflows.
lint-workflows:
    actionlint

# Epic #343 harness. scripts/pivot/ and these two recipes are deleted when
# the epic completes.

# Compare the current tree's behaviour with the pinned pre-pivot baseline.
baseline-diff:
    scripts/pivot/baseline.sh diff

# Search for the old surfaces' names. Warn-only until the greenfield sweep.
vestige-check:
    scripts/pivot/vestige-check.sh

# Reinstall the VSCode extension end-to-end: build the LSP, package the
# `.vsix`, and install via `code --install-extension --force`. Idempotent
# — re-run after each code change. Reload the VSCode window afterwards
# (Cmd+Shift+P -> "Developer: Reload Window"). Pass `release` for an
# optimized LSP build; defaults to debug.
vscode mode="debug":
    editor/vscode/scripts/dev-install.sh {{mode}}
