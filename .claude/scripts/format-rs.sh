#!/usr/bin/env bash
# PostToolUse hook: auto-format Rust files touched by Edit/Write/MultiEdit.
# Reads tool-call JSON from stdin, formats the file if it is a *.rs under
# this project, and exits silently. A formatting failure is reported as a
# non-blocking warning (exit 1) so syntax errors surface at the next
# clippy/just-check pass rather than wedging the agent.

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

cd "$project_dir"
if ! cargo fmt -- "$file_path" 2>&1; then
  echo "warning: cargo fmt failed on $file_path (syntax error?). Continuing without formatting." >&2
  exit 1
fi

exit 0
