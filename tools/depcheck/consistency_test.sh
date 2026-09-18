#!/usr/bin/env bash
# Lockfile-consistency test driver (issue #22).
# Usage: consistency_test.sh <ecosystem> <depcheck.py> <testdata-root>
# Verifies the stale-vs-consistent truth table, transitive/shared,
# non-mutating, offline, no-registry-query halves for one language.
set -euo pipefail

eco="$1"
checker_in="$2"
root_in="$3"

# Resolve runfiles vs workspace (Bazel sh_test vs local bash).
resolve() {
  local p="$1"
  if [[ "$p" = /* ]] && [[ -e "$p" ]]; then echo "$p"; return; fi
  if [[ -n "${TEST_SRCDIR:-}" ]]; then
    for cand in "$TEST_SRCDIR/rules_dx/$p" "$TEST_SRCDIR/_main/$p" "$TEST_SRCDIR/$p"; do
      if [[ -e "$cand" ]]; then echo "$cand"; return; fi
    done
    # Fall back to find (filegroup layouts vary).
    found="$(find "${TEST_SRCDIR:-/nonexistent}" -path "*$p" -print -quit 2>/dev/null || true)"
    if [[ -n "$found" ]]; then echo "$found"; return; fi
  fi
  if [[ -n "${BUILD_WORKSPACE_DIRECTORY:-}" && -e "$BUILD_WORKSPACE_DIRECTORY/$p" ]]; then echo "$BUILD_WORKSPACE_DIRECTORY/$p"; return; fi
  if [[ -e "$p" ]]; then echo "$p"; return; fi
  ws="$(git rev-parse --show-toplevel 2>/dev/null || echo .)"
  echo "$ws/$p"
}
checker="$(resolve "$checker_in")"
root="$(resolve "$root_in")"

pass=0
fail=0
ok() { pass=$((pass+1)); echo "ok: $1"; }
bad() { echo "FAIL: $1" >&2; fail=$((fail+1)); }

run_chk() {
  python3 "$checker" consistency --ecosystem "$eco" --manifest "$1" --lock "$2"
}

case "$eco" in
  rust) man="Cargo.toml"; lock="Cargo.lock" ;;
  python) man="pyproject.toml"; lock="uv.lock" ;;
  js|ts) man="package.json"; lock="pnpm-lock.yaml" ;;
  *) echo "unknown ecosystem $eco" >&2; exit 2 ;;
esac

# ok_used passes (even though newer compatible releases exist; the
# checker never queries a registry).
if run_chk "$root/ok_used/$man" "$root/ok_used/$lock" >/dev/null; then ok "$eco consistent+used passes"; else bad "$eco ok_used should pass"; fi

# stale fails even when every declaration is used.
if run_chk "$root/stale/$man" "$root/stale/$lock" >/dev/null; then bad "$eco stale should fail"; else
  code=$?
  if [[ "$code" == "1" ]]; then ok "$eco stale fails even when used"; else bad "$eco stale exit=$code want 1"; fi
fi

# consistent+unused passes consistency (usage fails separately).
if run_chk "$root/unused/$man" "$root/unused/$lock" >/dev/null; then ok "$eco consistent+unused passes consistency"; else bad "$eco unused should pass consistency"; fi

# transitive + shared-workspace passes (transitive in lock ignored,
# cross-package use counts).
if [[ "$eco" == "rust" ]]; then
  tman="$root/transitive_shared/Cargo.toml"; tlock="$root/transitive_shared/Cargo.lock"
else
  tman="$root/transitive_shared/$man"; tlock="$root/transitive_shared/$lock"
  if [[ "$eco" == "python" ]]; then tman="$root/transitive_shared/pyproject.toml"; tlock="$root/transitive_shared/uv.lock"; fi
fi
if run_chk "$tman" "$tlock" >/dev/null; then ok "$eco transitive+shared passes"; else bad "$eco transitive_shared should pass"; fi

# exception fixture passes consistency (exception never waives it).
if run_chk "$root/exception/$man" "$root/exception/$lock" >/dev/null; then ok "$eco exception passes consistency"; else bad "$eco exception should pass consistency"; fi

# category fixtures pass consistency (category is a usage concern).
if run_chk "$root/category/$man" "$root/category/$lock" >/dev/null; then ok "$eco category passes consistency"; else bad "$eco category should pass consistency"; fi
if run_chk "$root/category_ok/$man" "$root/category_ok/$lock" >/dev/null; then ok "$eco category_ok passes consistency"; else bad "$eco category_ok should pass consistency"; fi

# platform fixtures pass consistency (all direct deps locked).
if run_chk "$root/platform_optional/$man" "$root/platform_optional/$lock" >/dev/null; then ok "$eco platform passes consistency"; else bad "$eco platform_optional should pass consistency"; fi

# missing lock fails actionably (exit 2), never passes or skips.
if run_chk "$root/ok_used/$man" "$root/ok_used/MISSING.lock" >/dev/null 2>&1; then bad "$eco missing lock should fail"; else
  code=$?
  if [[ "$code" == "2" ]]; then ok "$eco missing lock fails actionably"; else bad "$eco missing lock exit=$code want 2"; fi
fi

# non-mutating: hashes unchanged across both outcomes.
for case in ok_used stale; do
  before_m="$(sha256sum "$root/$case/$man" | cut -d' ' -f1)"
  before_l="$(sha256sum "$root/$case/$lock" | cut -d' ' -f1)"
  run_chk "$root/$case/$man" "$root/$case/$lock" >/dev/null 2>&1 || true
  after_m="$(sha256sum "$root/$case/$man" | cut -d' ' -f1)"
  after_l="$(sha256sum "$root/$case/$lock" | cut -d' ' -f1)"
  if [[ "$before_m" == "$after_m" && "$before_l" == "$after_l" ]]; then ok "$eco $case non-mutating"; else bad "$eco $case mutated"; fi
done

# offline + no registry query: checker has no network imports and
# passes without env network variables. Bazel sandbox already denies
# network (no requires-network tag on these tests).
if grep -rn -E -e 'import urllib|import socket|import http|import requests|from urllib|from socket' "$checker" >/dev/null 2>&1; then bad "$eco checker contains network imports"; else ok "$eco no network imports"; fi
if env -u http_proxy -u https_proxy -u HTTP_PROXY -u HTTPS_PROXY python3 "$checker" consistency --ecosystem "$eco" --manifest "$root/ok_used/$man" --lock "$root/ok_used/$lock" >/dev/null; then ok "$eco offline pass"; else bad "$eco offline should pass"; fi

echo "consistency $eco: $pass passed, $fail failed"
[[ "$fail" == "0" ]]
