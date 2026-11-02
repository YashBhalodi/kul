#!/usr/bin/env bash
# Smoke-checks a deployed site over HTTP with curl:
#   - /, /docs, /llms.txt and /app/anything answer 200 (following redirects
#     within the site: /docs moves to /docs/);
#   - the pages carry the Content-Security-Policy and Cache-Control headers;
#   - the WASM (found in the service worker's precache list) is served as
#     application/wasm and cached as immutable;
#   - /sw.js is served with Cache-Control: no-cache.
#
#   scripts/smoke-deployed.sh <url>
#
# web.yml runs it right after every Netlify deploy, preview or production.
# Locally, point it at serve-dist (`node apps/web/scripts/serve-dist.mjs`).
# Prints one line per check and exits 1 when any fails.

set -euo pipefail

if [ "$#" -ne 1 ]; then
  echo "usage: scripts/smoke-deployed.sh <url>" >&2
  exit 2
fi
base="${1%/}"
failures=0

# The final response for a path, as five lines: status, URL, Content-Type,
# Cache-Control and Content-Security-Policy.
fetch() {
  curl --silent --show-error --location --max-redirs 3 \
    --retry 3 --retry-delay 2 --retry-all-errors \
    --output /dev/null \
    --write-out '%{http_code}\n%{url_effective}\n%{content_type}\n%header{cache-control}\n%header{content-security-policy}\n' \
    "$base$1" || true
}

# check <description> <test…>: runs the test and reports it.
check() {
  local description="$1"
  shift
  if "$@"; then
    echo "ok    $description"
  else
    echo "FAIL  $description"
    failures=$((failures + 1))
  fi
}

has() { [[ "$1" == *"$2"* ]]; }
is() { [[ "$1" == "$2" ]]; }

load() {
  { read -r status; read -r final; read -r type; read -r cache; read -r csp; } < <(fetch "$1")
}

for path in / /docs /llms.txt /app/anything; do
  status="" final="" type="" cache="" csp=""
  load "$path"
  check "$path answers 200 (got ${status:-no answer})" is "$status" 200
  check "$path stays on $base (ended at ${final:-nothing})" has "$final" "$base/"
  check "$path has Cache-Control (${cache:-none})" has "$cache" "no-cache"
  if [ "$path" != /llms.txt ]; then
    check "$path has a Content-Security-Policy" has "$csp" "default-src 'self'"
    check "$path keeps requests same-origin" has "$csp" "connect-src 'self'"
  fi
done

wasm="$(curl --silent --show-error --retry 3 --retry-all-errors "$base/sw.js" |
  grep -oE '/assets/kul_wasm_bg-[A-Za-z0-9_-]+\.wasm' | head -n 1 || true)"
check "the service worker precaches the WASM (${wasm:-not found})" test -n "$wasm"
if [ -n "$wasm" ]; then
  status="" final="" type="" cache="" csp=""
  load "$wasm"
  check "$wasm answers 200 (got ${status:-no answer})" is "$status" 200
  check "$wasm is application/wasm (${type:-none})" is "$type" "application/wasm"
  check "$wasm is immutable (${cache:-none})" has "$cache" "immutable"
  check "$wasm has a Content-Security-Policy" has "$csp" "default-src 'self'"
fi

status="" final="" type="" cache="" csp=""
load /sw.js
check "/sw.js answers 200 (got ${status:-no answer})" is "$status" 200
check "/sw.js is no-cache (${cache:-none})" is "$cache" "no-cache"

if [ "$failures" -gt 0 ]; then
  echo "smoke-deployed: $failures check(s) failed against $base" >&2
  exit 1
fi
echo "smoke-deployed: every check passed against $base"
