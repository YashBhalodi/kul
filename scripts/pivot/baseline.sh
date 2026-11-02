#!/usr/bin/env bash
# Behaviour baseline for epic #343 (the web-app pivot).
#
#   scripts/pivot/baseline.sh capture   capture the pinned pre-pivot behaviour
#   scripts/pivot/baseline.sh diff      capture the current tree and compare
#
# capture
#   Builds the tree at the pinned commit (PINNED, below) in a temporary git
#   worktree and records, over every project directory in its `examples/`:
#     - the CLI: validate, format --check, export json|svg and the query
#       outputs (scripts/pivot/cli-capture.mjs lists the cases);
#     - the language features: hover, completion, definition, references,
#       prepare-rename and rename, symbols, code actions, semantic tokens,
#       formatting, entity-at and locate, at every token start and end offset
#       of every file (scripts/pivot/lsp-dump, a standalone crate built
#       against the worktree's language server; it is never committed into
#       it);
#     - a copy of its `examples/`, which compare.mjs reads to turn the
#       language server's line/character positions into byte offsets.
#   The worktree is removed afterwards. Because the baseline always comes from
#   PINNED, it can be re-captured at any point in the epic, whatever the
#   current tree has deleted.
#
# diff
#   Captures the same inputs from the current tree into `current/` (the CLI,
#   and the language features through scripts/pivot/service-dump, built
#   against `kul-language-service`) and runs scripts/pivot/compare.mjs. It
#   maps the baseline's LSP shapes onto the service's Kul-native shapes and
#   exits non-zero on any difference outside its allowlist (removed KUL-M*
#   diagnostics, the missing-manifest error, FileId renumbering in debug
#   output). Captures the baseline first when it is missing or was taken with
#   a different version of the capture tools.
#
# Output (git-ignored), under target/pivot-baseline/ (KUL_PIVOT_STATE overrides):
#   baseline/{cli,service}/<project>.json   the pinned capture
#   baseline/examples/                      the pinned examples
#   baseline/stamp                          commit + capture-tool hash
#   current/{cli,service}/<project>.json    the latest diff run's capture
#   diff-report.txt                         the latest diff report
#   build/                                  worktree and cargo target dir
#
# The current tree's side:
#   current_cli       builds the current tree's `kul` and prints its path. The
#                     CLI cases run with the project as the working directory,
#                     which the CLI reads as its default [DIR]. An explicit
#                     [DIR] variant belongs in `invoke` in cli-capture.mjs.
#   current_service   writes the current tree's language-feature dump with
#                     service-dump.
#
# Requires git (with PINNED reachable: CI checks out with fetch-depth 0),
# cargo and Node 22.

set -euo pipefail

PINNED=db0dc68

ROOT=$(git rev-parse --show-toplevel)
HERE="$ROOT/scripts/pivot"
STATE="${KUL_PIVOT_STATE:-$ROOT/target/pivot-baseline}"
BUILD="$STATE/build"
WORKTREE="$BUILD/worktree"
# Builds of the pinned tree and the dumpers share one target dir. RUSTFLAGS is
# cleared for them so a newer toolchain's warnings can't break a capture of
# old code under CI's `-D warnings`.
PIVOT_TARGET="$BUILD/cargo-target"

log() { printf 'baseline: %s\n' "$*" >&2; }

# Hash of the tools that decide what a capture contains. A baseline taken
# with different tools is stale.
tools_hash() {
  cat "$HERE/cli-capture.mjs" "$HERE/lsp-dump/src/main.rs" | git hash-object --stdin
}

# build_dump <name> <tree> <deps>: build scripts/pivot/<name> as a standalone
# crate against <tree>'s crates (dependency lines <deps>, with the tree's
# Cargo.lock) and print the binary path.
build_dump() {
  local name=$1 tree=$2 deps=$3
  local dir="$BUILD/$name"
  rm -rf "$dir"
  mkdir -p "$dir/src"
  cp "$HERE/$name/src/main.rs" "$dir/src/main.rs"
  cp "$tree/Cargo.lock" "$dir/Cargo.lock"
  cat >"$dir/Cargo.toml" <<EOF
[package]
name = "$name"
version = "0.0.0"
edition = "2024"
publish = false

[workspace]

[dependencies]
$deps
EOF
  env -u RUSTFLAGS CARGO_TARGET_DIR="$PIVOT_TARGET" \
    cargo build --quiet --manifest-path "$dir/Cargo.toml"
  printf '%s\n' "$PIVOT_TARGET/debug/$name"
}

# The pinned tree's language-server dumper.
build_lsp_dump() {
  local tree=$1
  build_dump lsp-dump "$tree" "kul-core = { path = \"$tree/crates/kul-core\" }
kul-loader = { path = \"$tree/crates/kul-loader\" }
kul-lsp = { path = \"$tree/crates/kul-lsp\" }
serde = \"1\"
serde_json = \"1\"
tower-lsp = \"0.20\""
}

# The current tree's language-service dumper.
build_service_dump() {
  build_dump service-dump "$ROOT" "kul-core = { path = \"$ROOT/crates/kul-core\" }
kul-language-service = { path = \"$ROOT/crates/kul-language-service\" }
serde = \"1\"
serde_json = \"1\""
}

# ---------------------------------------------------------------- current tree

current_cli() {
  (cd "$ROOT" && cargo build --quiet -p kul-cli)
  printf '%s\n' "$ROOT/target/debug/kul"
}

# current_service <examples-root> <out-dir>
current_service() {
  local dump
  dump=$(build_service_dump)
  "$dump" "$1" "$2"
}

# ---------------------------------------------------------------- commands

remove_worktree() {
  git -C "$ROOT" worktree remove --force "$WORKTREE" 2>/dev/null || true
  git -C "$ROOT" worktree prune
}

cmd_capture() {
  git -C "$ROOT" cat-file -e "$PINNED^{commit}" 2>/dev/null || {
    echo "baseline: commit $PINNED is not in this clone; fetch full history (git fetch --unshallow)" >&2
    exit 1
  }
  mkdir -p "$BUILD"
  remove_worktree
  rm -rf "$WORKTREE"
  trap remove_worktree EXIT
  log "worktree at $PINNED"
  git -C "$ROOT" worktree add --quiet --detach "$WORKTREE" "$PINNED"

  local out="$STATE/baseline"
  rm -rf "$out"
  log "building the pinned CLI"
  (cd "$WORKTREE" && env -u RUSTFLAGS CARGO_TARGET_DIR="$PIVOT_TARGET" cargo build --quiet -p kul-cli)
  node "$HERE/cli-capture.mjs" "$PIVOT_TARGET/debug/kul" "$WORKTREE/examples" "$out/cli"

  log "building the pinned language-feature dumper"
  local dump
  dump=$(build_lsp_dump "$WORKTREE")
  "$dump" "$WORKTREE/examples" "$out/service"
  cp -R "$WORKTREE/examples" "$out/examples"

  printf 'commit %s\ntools %s\n' "$(git -C "$ROOT" rev-parse "$PINNED")" "$(tools_hash)" >"$out/stamp"
  remove_worktree
  trap - EXIT
  log "captured $(find "$out/cli" -name '*.json' | wc -l | tr -d ' ') projects into $out"
}

baseline_is_fresh() {
  local stamp="$STATE/baseline/stamp"
  [[ -f $stamp && -d $STATE/baseline/examples ]] && grep -qx "tools $(tools_hash)" "$stamp"
}

cmd_diff() {
  if ! baseline_is_fresh; then
    log "no baseline for the current capture tools; capturing first"
    cmd_capture
  fi
  local out="$STATE/current"
  rm -rf "$out"
  log "capturing the current tree"
  node "$HERE/cli-capture.mjs" "$(current_cli)" "$ROOT/examples" "$out/cli"
  current_service "$ROOT/examples" "$out/service"

  local status=0
  node "$HERE/compare.mjs" "$STATE/baseline" "$out" >"$STATE/diff-report.txt" || status=$?
  cat "$STATE/diff-report.txt"
  return "$status"
}

case "${1:-}" in
  capture) cmd_capture ;;
  diff) cmd_diff ;;
  *)
    echo "usage: scripts/pivot/baseline.sh capture|diff" >&2
    exit 2
    ;;
esac
