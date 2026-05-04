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
