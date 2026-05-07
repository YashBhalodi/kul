# Kul developer workflow. Run `just` for the default check.

default: check

# Format-check + clippy + tests. Local-green should imply CI-green.
check: fmt-check lint test

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

# Build the WebAssembly artifact for `@kul/wasm`. Requires `wasm-pack`.
# Patches the generated `package.json` so its `name` is `@kul/wasm`
# (wasm-pack derives the npm name from the Cargo crate name, which is
# `kul-wasm`), then refreshes the committed `.d.ts` snapshot.
wasm:
    wasm-pack build crates/kul-wasm --target bundler --out-dir pkg --out-name kul_wasm
    node -e 'const fs=require("fs"),p="crates/kul-wasm/pkg/package.json";const j=JSON.parse(fs.readFileSync(p));j.name="@kul/wasm";fs.writeFileSync(p,JSON.stringify(j,null,2)+"\n")'
    cp crates/kul-wasm/pkg/kul_wasm.d.ts crates/kul-wasm/types/kul_wasm.d.ts

# Reinstall the VSCode extension end-to-end: build the LSP, package the
# `.vsix`, and install via `code --install-extension --force`. Idempotent
# — re-run after each code change. Reload the VSCode window afterwards
# (Cmd+Shift+P -> "Developer: Reload Window"). Pass `release` for an
# optimized LSP build; defaults to debug.
vscode mode="debug":
    editor/vscode/scripts/dev-install.sh {{mode}}
