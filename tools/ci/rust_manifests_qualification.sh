#!/usr/bin/env bash
# Rust manifest discovery plus generation check (issue #1038).
#
# Closes the MODULE.bazel Bzlmod monolith gap: `crate.from_cargo`
# enumerates manifests by hand, so adding a Rust crate without editing
# MODULE.bazel (via `modules/rust.bzl`) breaks the build only at repin
# or analysis time. This harness fails closed with a generation message:
# - discovery: every in-scope checked-in `Cargo.toml` must be listed in
#   `modules/rust.bzl` (single source; MODULE.bazel mirrors it and
#   `//tools/ci:pin_consistency_test` fails on mirror drift);
# - generation: every wrapper manifest must resolve to a
#   `cargo-bazel-lock.json` workspace member (repin via
#   `CARGO_BAZEL_REPIN=1 bazel build //rust/tests/fixtures/hello:hello`).
#
# Scope: first-party shipped crates (`cli/`, `deploy/`, `env/`,
# `generation/`, `quality/`, `docs/`, `tools/bazelrc`, `tools/depcheck`,
# `rust/tests/fixtures/hello`). Exemptions (never hub members):
# - `examples/` (separate consumer workspaces with their own locks);
# - `gazelle/rust/testdata/` (generation fixtures);
# - `rust/tests/fixtures/` except `hello` (generation edge-case fixtures
#   with own scope; `cxx_identity` records its pin under
#   `[package.metadata]` until wired);
# - `tools/depcheck/testdata/` (depcheck unit-test inputs).
#
# Versioned here, run by CI via `bazel run //tools/ci:rust_manifests_qualification`.
set -euo pipefail

# Shared workspace + runfiles helpers.
source "${RUNFILES_DIR:-/dev/null}/_main/tools/sh/bootstrap.sh" 2>/dev/null || source "${TEST_SRCDIR:-/dev/null}/_main/tools/sh/bootstrap.sh" 2>/dev/null || source "$0.runfiles/_main/tools/sh/bootstrap.sh" 2>/dev/null || source "${BASH_SOURCE[0]}.runfiles/_main/tools/sh/bootstrap.sh" 2>/dev/null || source "$(git rev-parse --show-toplevel 2>/dev/null)/tools/sh/bootstrap.sh"
dx_bootstrap "tools/sh/lib.sh"

dx_cd_workspace

dx_test_init

dx_bash_pin

wrapper="modules/rust.bzl"
module="MODULE.bazel"
lock="cargo-bazel-lock.json"

# Wrapper plus MODULE plus lock stay present.
if [[ -f "$wrapper" && -f "$module" && -f "$lock" ]]; then
  ok
else
  bad "rust manifest inputs missing (want $wrapper plus $module plus $lock)"
fi

# Discovery: in-scope Cargo.toml via git ls-files with documented exemptions.
git ls-files '*Cargo.toml' | LC_ALL=C sort -u > /tmp/dx_manifests_all.txt
grep -v -E -e '^examples/' -e '^gazelle/rust/testdata/' -e '^tools/depcheck/testdata/' /tmp/dx_manifests_all.txt | LC_ALL=C sort -u > /tmp/dx_manifests_noexcl.txt
# rust/tests/fixtures: keep only hello (the lock owner plus hub member).
grep -v -E -e '^rust/tests/fixtures/' /tmp/dx_manifests_noexcl.txt > /tmp/dx_manifests_scope.txt || true
echo "rust/tests/fixtures/hello/Cargo.toml" >> /tmp/dx_manifests_scope.txt
LC_ALL=C sort -u /tmp/dx_manifests_scope.txt -o /tmp/dx_manifests_scope.txt
if grep -q -F -e 'cli/qualification/Cargo.toml' /tmp/dx_manifests_scope.txt; then
  ok
else
  bad "discovery scope lost cli/qualification (want first-party cli crates in scope)"
fi

# Every in-scope manifest must be listed in the wrapper (as //dir:Cargo.toml).
missing=0
while IFS= read -r manifest; do
  [[ -n "$manifest" ]] || continue
  dir="${manifest%/Cargo.toml}"
  label="//${dir}:Cargo.toml"
  if ! grep -q -F -e "\"$label\"" "$wrapper"; then
    echo "rust manifest discovery failed: $manifest not listed in $wrapper (add it to its group plus MODULE.bazel mirror, then repin)"
    missing=1
  fi
done < /tmp/dx_manifests_scope.txt
if [[ "$missing" == "0" ]]; then
  ok
else
  bad "rust manifest discovery failed (see missing manifests above; exemption policy lives in this script header)"
fi

# Every wrapper manifest must exist on disk (no stale entries).
stale=0
while IFS= read -r label; do
  [[ -n "$label" ]] || continue
  path="${label#//}"
  path="${path%:Cargo.toml}/Cargo.toml"
  if [[ ! -f "$path" ]]; then
    echo "rust manifest stale: $label listed in $wrapper but $path missing from tree"
    stale=1
  fi
done < <(grep -o -E -e '"//[^"]*:Cargo.toml"' "$wrapper" | tr -d '"' | LC_ALL=C sort -u)
if [[ "$stale" == "0" ]]; then
  ok
else
  bad "rust manifest wrapper lists stale manifests (see above)"
fi

# Generation: every wrapper manifest package must resolve to a lock workspace member.
# Lock values are parent-dir plus Cargo package name (e.g. cli/dx_adopt for
# //cli/adopt); compare package names, not paths.
if python3 -c "
import json, re, sys
wrapper = open('$wrapper').read()
labels = sorted(set(re.findall(r'\"//([^\"]*):Cargo\.toml\"', wrapper)))
lock = json.load(open('$lock'))
members = lock.get('workspace_members', {})
member_names = set(k.rsplit(' ', 1)[0] for k in members.keys())
missing = []
for label in labels:
    toml_path = label + '/Cargo.toml'
    try:
        text = open(toml_path).read()
    except FileNotFoundError:
        print('missing file: ' + toml_path)
        sys.exit(2)
    m = re.search(r'(?m)^name\s*=\s*\"([^\"]+)\"', text)
    if not m:
        print('no package name in ' + toml_path)
        sys.exit(2)
    if m.group(1) not in member_names:
        missing.append(toml_path + ' (package ' + m.group(1) + ')')
if missing:
    print('missing from lock:')
    print('\n'.join(missing))
    sys.exit(1)
print('lock covers ' + str(len(labels)) + ' wrapper manifests')
"; then
  ok
else
  bad "cargo-bazel-lock.json workspace_members misses wrapper manifests (repin via CARGO_BAZEL_REPIN=1 bazel build //rust/tests/fixtures/hello:hello)"
fi

dx_test_summary "rust manifests qualification harness"
