#!/usr/bin/env bash
#
# render-examples.sh — render a committed `tree.svg` next to every example
# project's `.kul` source, using *this checkout's* CLI.
#
# `kul export --format=svg` is byte-for-byte deterministic and
# host-font-independent (kul-layout uses a fixed card width; kul-svg emits
# `<text>` labels over an unmeasured label-column budget), so the bytes this
# produces on Linux CI and a maintainer's macOS are identical. That is what
# lets CI auto-commit the regenerated SVGs without churn (see
# `.github/workflows/render-examples.yml`).
#
# Each example project is a directory directly under `examples/` holding
# `.kul` files; `kul export` takes it as its `[DIR]` argument. A multi-file
# project still yields a single `tree.svg` (one envelope per project).
#
# Override the binary with KUL_BIN to skip the build (CI passes the binary it
# already built); otherwise we build kul-cli once and reuse it across examples.

set -euo pipefail

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
examples_dir="$repo_root/examples"

kul_bin="${KUL_BIN:-}"
if [[ -z "$kul_bin" ]]; then
  cargo build -q -p kul-cli
  kul_bin="$repo_root/target/debug/kul"
fi

for project_dir in "$examples_dir"/*/; do
  project_dir="${project_dir%/}"
  compgen -G "$project_dir/*.kul" >/dev/null || continue
  out="$project_dir/tree.svg"
  # Render to a temp file first so a failing export never leaves a partial or
  # empty tree.svg behind — examples validate cleanly, so non-zero is a real
  # regression we want to surface loudly, not paper over.
  tmp="$(mktemp)"
  if "$kul_bin" export --format=svg "$project_dir" >"$tmp"; then
    mv "$tmp" "$out"
    echo "rendered ${out#"$repo_root"/}"
  else
    rm -f "$tmp"
    echo "error: export failed for ${project_dir#"$repo_root"/}" >&2
    exit 1
  fi
done
