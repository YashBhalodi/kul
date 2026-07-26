# Kul developer workflow. Run `just` for the default check.

default: check

# Format-check + clippy + tests. Local-green should imply CI-green.
check: fmt-check lint test check-ts

# Run the TypeScript workspace gates (Vitest in packages/preview and
# editor/vscode) — the same suites .github/workflows/vscode-extension.yml
# runs per PR. Requires Node 22 and a prior `npm ci` at the repo root.
#
# The extension build runs too, and is not redundant with the tests: Vitest
# resolves `packages/preview/src`, while the extension bundles `dist`. Only
# the bundle catches a module that never reached `dist` — the failure mode a
# `src/` subdirectory introduces, invisible to every test.
check-ts:
    npm test --workspaces --if-present
    npm run build:preview --workspace kul
    npm run bundle --workspace kul

# Run the full test suite via cargo-nextest.
test:
    cargo nextest run --workspace --no-tests=pass

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

# Reinstall the VSCode extension end-to-end: build the LSP, package the
# `.vsix`, and install via `code --install-extension --force`. Idempotent
# — re-run after each code change. Reload the VSCode window afterwards
# (Cmd+Shift+P -> "Developer: Reload Window"). Pass `release` for an
# optimized LSP build; defaults to debug.
vscode mode="debug":
    editor/vscode/scripts/dev-install.sh {{mode}}
