#!/usr/bin/env bash
# Dev-time end-to-end VSCode extension install.
#
# Builds the language server, packages the extension, and installs the
# `.vsix` into the system VSCode. Idempotent — safe to re-run after every
# code change. After it exits, reload the VSCode window
# (Cmd+Shift+P → "Developer: Reload Window") to pick up the new bundle.
#
# Usage:
#   ./dev-install.sh           # debug build (fast)
#   ./dev-install.sh release   # optimized build
#
# Invoked by `just vscode [mode]` from the repo root.

set -euo pipefail

mode="${1:-debug}"
case "$mode" in
  debug) cargo_flag="" ;;
  release) cargo_flag="--release" ;;
  *) echo "usage: $(basename "$0") [debug|release]" >&2; exit 2 ;;
esac

repo_root="$(git rev-parse --show-toplevel)"
ext_dir="$repo_root/editor/vscode"

cd "$repo_root"
cargo build -p kul-lsp $cargo_flag

cd "$ext_dir"
[ -d node_modules ] || npm install
npm run package

version="$(node -p "require('./package.json').version")"
vsix="$ext_dir/kul-${version}.vsix"
code --install-extension "$vsix" --force

binary="$repo_root/target/$mode/kul-lsp"
node "$ext_dir/scripts/update-settings.mjs" "$binary"

echo
echo "Installed kul-${version}.vsix"
echo "LSP binary: $binary"
echo "Reload the VSCode window: Cmd+Shift+P -> Developer: Reload Window"
