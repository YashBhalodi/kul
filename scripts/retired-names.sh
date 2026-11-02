#!/usr/bin/env bash
# The names of the surfaces Kul retired when the browser app became its only
# interactive surface (ADR-0047): the editor extension and its marketplace
# listings, the language server and its transport, the preview package and
# its host seam, the project file and its diagnostics, the YAML parser and the
# directory loader. Any of them showing up again in a tracked or untracked,
# non-ignored file fails this check.
#
#   scripts/retired-names.sh   report hits; exit 1 if there are any
#
# Each hit prints as `file:line:text` under the term it matched, followed by
# a per-term count. Not counted:
#   - CHANGELOG.md (release history);
#   - ADR status lines (`**Status:** ...` in docs/adr/);
#   - superseded ADRs: the whole of every docs/adr/ file whose status line
#     begins `**Status:** Superseded`, and that file's name where another
#     file links to it. They are records of decisions no longer in force
#     (ADR-0047, "The records of the retired surfaces");
#   - names owned by third-party packages, which this repo cannot rename:
#     workbox-build's two precache-manifest functions and Shiki's TextMate
#     engine package (a transitive dependency in package-lock.json).
#
# Every pattern and exempt name holds one bracketed character or a split
# literal, so this file never spells the names it looks for.

set -euo pipefail

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
)

STATUS_LINE='^docs/adr/[^:]+:[0-9]+:\*\*Status:\*\*'

# Third-party names, removed from a hit's text before it is counted.
EXEMPT=(
  'injectManifes''t'
  'getManifes''t'
  'vs''code-textmate'
)

while IFS= read -r adr; do
  IGNORE_PATHS+=(":(exclude)$adr")
  EXEMPT+=("${adr##*/}")
done < <(git grep -l -E '^\*\*Status:\*\* Superseded' -- docs/adr)

# Drop hits that match only inside an exempt name.
keep_real_hits() {
  local pattern=$1 mode=$2 hit text name
  local grep_flags=(-q -E)
  [[ $mode == i ]] && grep_flags+=(-i)
  while IFS= read -r hit; do
    text=${hit#*:}
    text=${text#*:}
    for name in "${EXEMPT[@]}"; do
      text=${text//"$name"/}
    done
    if grep "${grep_flags[@]}" -e "$pattern" <<<"$text"; then
      printf '%s\n' "$hit"
    fi
  done
}

total=0
summary=()
for entry in "${TERMS[@]}"; do
  mode=${entry%%|*}
  pattern=${entry#*|}
  flags=(-n -I --untracked -E)
  [[ $mode == i ]] && flags+=(-i)
  hits=$(git grep "${flags[@]}" -e "$pattern" -- . "${IGNORE_PATHS[@]}" |
    grep -Ev "$STATUS_LINE" | keep_real_hits "$pattern" "$mode" || true)
  count=0
  if [[ -n $hits ]]; then
    count=$(printf '%s\n' "$hits" | wc -l | tr -d ' ')
    printf '== %s (%s)\n%s\n\n' "$pattern" "$count" "$hits"
  fi
  summary+=("$(printf '%6s  %s' "$count" "$pattern")")
  total=$((total + count))
done

printf 'retired-names: hits per term\n'
printf '%s\n' "${summary[@]}"
printf 'retired-names: %s hit(s) in total\n' "$total"

[[ $total -eq 0 ]]
