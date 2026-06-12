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
# Each project is the directory containing `kul.yml`; export is CWD-rooted, so
# we render from inside each project dir. A multi-file project still yields a
# single `tree.svg` (one envelope per project).
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

while IFS= read -r -d '' manifest; do
  project_dir="$(dirname "$manifest")"
  out="$project_dir/tree.svg"
  # Render to a temp file first so a failing export never leaves a partial or
  # empty tree.svg behind — examples validate cleanly, so non-zero is a real
  # regression we want to surface loudly, not paper over.
  tmp="$(mktemp)"
  if ( cd "$project_dir" && "$kul_bin" export --format=svg ) >"$tmp"; then
    mv "$tmp" "$out"
    echo "rendered ${out#"$repo_root"/}"
  else
    rm -f "$tmp"
    echo "error: export failed for ${project_dir#"$repo_root"/}" >&2
    exit 1
  fi
done < <(find "$examples_dir" -name kul.yml -print0 | sort -z)
