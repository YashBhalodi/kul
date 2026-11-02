#!/usr/bin/env bash
# Old-name search for epic #343 (the web-app pivot): the epic's Definition of
# Done search over every tracked and untracked, non-ignored file.
#
#   scripts/pivot/vestige-check.sh            report hits, exit 0 (warn-only)
#   scripts/pivot/vestige-check.sh --strict   report hits, exit 1 on any
#
# Each hit prints as `file:line:text` under the term it matched, followed by
# a per-term count. Ignored:
#   - CHANGELOG.md (release history);
#   - ADR status lines (`**Status:** ...` in docs/adr/);
#   - scripts/pivot/ itself (this harness; deleted at the end of the epic).
#
# Every pattern holds one bracketed character (`kul[.]yml`) so the patterns
# never spell the names they look for, and a plain search for a name finds
# no hit in this file.
#
# The search is warn-only until the greenfield sweep (#356) turns on
# --strict. That slice owns any further ignore rule (for example the bodies
# of superseded ADRs); add it to IGNORE_PATHS or the status-line filter here.

set -euo pipefail

STRICT=0
case "${1:-}" in
  --strict) STRICT=1 ;;
  "") ;;
  *)
    echo "usage: scripts/pivot/vestige-check.sh [--strict]" >&2
    exit 2
    ;;
esac

ROOT=$(git rev-parse --show-toplevel)
cd "$ROOT"

# <case>|<extended regex>. `i` matches case-insensitively, `s` exactly.
TERMS=(
  's|kul[.]yml'
  's|Manifes[t]'
  's|KUL-M[0]'
  'i|vscod[e]'
  'i|VS Cod[e]'
  'i|tower[-_]ls[p]'
  'i|toki[o]'
  'i|kul[-_]ls[p]'
  'i|kul ls[p]'
  'i|@kullang/previe[w]'
  's|HostAdapte[r]'
  'i|webvie[w]'
  'i|OVS[X]'
  'i|VSC[E]'
  'i|serde[-_]norwa[y]'
  'i|kul[-_]loade[r]'
)

IGNORE_PATHS=(
  ':(exclude)CHANGELOG.md'
  ':(exclude)scripts/pivot'
)

STATUS_LINE='^docs/adr/[^:]+:[0-9]+:\*\*Status:\*\*'

total=0
summary=()
for entry in "${TERMS[@]}"; do
  mode=${entry%%|*}
  pattern=${entry#*|}
  flags=(-n -I --untracked -E)
  [[ $mode == i ]] && flags+=(-i)
  hits=$(git grep "${flags[@]}" -e "$pattern" -- . "${IGNORE_PATHS[@]}" | grep -Ev "$STATUS_LINE" || true)
  count=0
  if [[ -n $hits ]]; then
    count=$(printf '%s\n' "$hits" | wc -l | tr -d ' ')
    printf '== %s (%s)\n%s\n\n' "$pattern" "$count" "$hits"
  fi
  summary+=("$(printf '%6s  %s' "$count" "$pattern")")
  total=$((total + count))
done

printf 'vestige-check: hits per term\n'
printf '%s\n' "${summary[@]}"
printf 'vestige-check: %s hit(s) in total\n' "$total"

if [[ $total -gt 0 ]]; then
  if [[ $STRICT -eq 1 ]]; then
    exit 1
  fi
  printf 'vestige-check: warn-only (run with --strict to fail on hits)\n'
fi
