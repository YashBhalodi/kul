#!/usr/bin/env bash
# PostToolUse hook: run clippy on the crate that owns the just-edited
# *.rs file. Blocking — exit 2 if clippy fails so the agent has to fix
# warnings as they appear. Matches the workspace `clippy::all = deny`
# policy and CI's `-D warnings` posture (see `.github/workflows/rust.yml`).

set -euo pipefail

payload=$(cat)
file_path=$(printf '%s' "$payload" | /usr/bin/env python3 -c '
import json, sys
data = json.load(sys.stdin)
print(data.get("tool_input", {}).get("file_path", ""))
')

[[ -z "$file_path" ]] && exit 0
[[ "$file_path" != *.rs ]] && exit 0

project_dir="${CLAUDE_PROJECT_DIR:-$(pwd)}"
case "$file_path" in
  "$project_dir"/*) ;;
  *) exit 0 ;;
esac

# Derive crate name from path: /…/crates/<name>/… → <name>
rel="${file_path#"$project_dir"/}"
if [[ "$rel" != crates/* ]]; then
  exit 0
fi
crate=$(printf '%s' "$rel" | awk -F/ '{print $2}')
if [[ -z "$crate" ]]; then
  exit 0
fi

cd "$project_dir"
if ! output=$(cargo clippy -p "$crate" --all-targets -- -D warnings 2>&1); then
  printf 'clippy failed on crate `%s` after editing %s.\n' "$crate" "$rel" >&2
  printf '%s\n' "$output" >&2
  printf '\nFix the warnings before continuing — the workspace denies `clippy::all` and CI fails on `-D warnings`. If a lint is wrong for this specific case, add `#[allow(...)]` with a one-line justifying comment.\n' >&2
  exit 2
fi

exit 0
