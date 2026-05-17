#!/usr/bin/env bash
# Stop hook: block end-of-turn if any *.snap.new files exist anywhere in
# the workspace. Per docs/testing.md, snapshot acceptance is a deliberate
# review step — `cargo insta review` (interactive) or `cargo insta accept`
# (after careful inspection). Never commit `.snap.new`; never leave them
# lying around at the end of a turn either, since the agent should know
# whether the diff was intentional.

set -euo pipefail

project_dir="${CLAUDE_PROJECT_DIR:-$(pwd)}"
cd "$project_dir"

pending=$(find . -name '*.snap.new' -not -path './target/*' 2>/dev/null)

if [[ -z "$pending" ]]; then
  exit 0
fi

{
  echo "Pending snapshot files exist — review them before claiming done:"
  printf '  %s\n' $pending
  echo ""
  echo "Run \`cargo insta review\` (interactive) to walk each diff and accept/reject."
  echo "If you're sure every diff is intentional, \`cargo insta accept\` will accept them all in one shot."
  echo "Per docs/testing.md, every \`.snap.new\` represents a deliberate decision — don't skip the review."
} >&2

exit 2
