# Kula developer workflow. Run `just` for the default check.

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

# Passthrough to `cargo run -p kula-cli --`. Example: `just run -- --help`.
run *ARGS:
    cargo run -p kula-cli -- {{ARGS}}

# Build the WebAssembly artifact for `@kulalang/wasm`. Requires `wasm-pack`.
# Patches the generated `package.json` so its `name` is `@kulalang/wasm`
# (wasm-pack derives the npm name from the Cargo crate name, which is
# `kula-wasm`), then refreshes the committed `.d.ts` snapshot.
wasm:
    wasm-pack build crates/kula-wasm --target bundler --out-dir pkg --out-name kula_wasm
    node -e 'const fs=require("fs"),p="crates/kula-wasm/pkg/package.json";const j=JSON.parse(fs.readFileSync(p));j.name="@kulalang/wasm";fs.writeFileSync(p,JSON.stringify(j,null,2)+"\n")'
    cp crates/kula-wasm/pkg/kula_wasm.d.ts crates/kula-wasm/types/kula_wasm.d.ts
