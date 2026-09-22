#!/usr/bin/env bash
# C/C++ sha256-integrity plus no-system-package qualification harness.
#
# Qualifies the owned gap from closed: provisional C/C++ hash authority,
# closed owner only. pinned the GoogleTest runner, not the hash wiring.
# - pinned: no ecosystem lockfile; every `http_archive` carries `sha256` or
#   `integrity`, recorded in `cc/tests/fixtures/hermetic/pins.bzl`; the
#   committed `MODULE.bazel.lock` carries the BCR `integrity` for the pinned
#   `googletest` 1.18.0 module (registryFileHashes plus source.json hash).
#   The depcheck `cc_deps.toml` plus `cc_lock.json` pair proves per-archive
#   sha256 authority offline (every manifest entry carries sha256 and matches
#   the lock; missing or stale sha256 fails consistency).
# - rejected: system packages (host apt/brew, `/usr` paths, local config) as
#   non-hermetic; hash-less `http_archive` (no sha256 and no integrity).
# - fixtures: hermetic pins plus the `cc/tests/fixtures/hello` seed and the
#   `cc/tests/fixtures/googletest` mapping over the pinned `@googletest`
#   hub; depcheck `tools/depcheck/testdata/cc` truth table (ok_used passes,
#   missing sha256 fails).
# - open owned gaps: platform plus consumer plus release evidence, MSVC
#   interop plus SDK licensing, no `Supported` claim. Compatibility is hash
#   wiring only.
#
# Versioned here, run by CI via `bazel run //tools/ci:cc_hermetic_qualification`,
# following //tools/ci:paket_qualification.
set -euo pipefail

# Shared workspace + runfiles helpers.
# Bootstrap via tools/sh/bootstrap.sh (issue #654): runfiles forest first, then source tree.
source "${RUNFILES_DIR:-/dev/null}/_main/tools/sh/bootstrap.sh" 2>/dev/null || source "${TEST_SRCDIR:-/dev/null}/_main/tools/sh/bootstrap.sh" 2>/dev/null || source "$0.runfiles/_main/tools/sh/bootstrap.sh" 2>/dev/null || source "${BASH_SOURCE[0]}.runfiles/_main/tools/sh/bootstrap.sh" 2>/dev/null || source "$(git rev-parse --show-toplevel 2>/dev/null)/tools/sh/bootstrap.sh"
dx_bootstrap "tools/sh/lib.sh"

dx_cd_workspace

dx_test_init

pins="cc/tests/fixtures/hermetic/pins.bzl"
pins_build="cc/tests/fixtures/hermetic/BUILD.bazel"
hello_build="cc/tests/fixtures/hello/BUILD.bazel"
gtest_build="cc/tests/fixtures/googletest/BUILD.bazel"
module="MODULE.bazel"
lock="MODULE.bazel.lock"
checker="tools/depcheck/src/lib.rs"
cc_ok_man="tools/depcheck/testdata/cc/ok_used/cc_deps.toml"
cc_ok_lock="tools/depcheck/testdata/cc/ok_used/cc_lock.json"
if bazel build --noshow_progress //tools/depcheck:depcheck >/dev/null 2>&1; then
  depcheck_bin="bazel-bin/tools/depcheck/depcheck"
else
  depcheck_bin="bazel run --noshow_progress //tools/depcheck:depcheck --"
fi
matrix="docs/product/support-matrix.md"
gen_readme="docs/generation/foundation-qualification.md"
build="tools/ci/BUILD.bazel"
ci=".github/workflows/ci.yml"

# Pins fixture stays present as the single hash-authority owner.
if [[ -f "$pins" && -f "$pins_build" ]]; then
  ok
else
  bad "cc hermetic fixture missing (want $pins plus $pins_build)"
fi

# Pins record the hash attrs plus lock authority plus rejected system wiring.
if grep -q -F -e 'CC_HASH_ATTR = "sha256"' "$pins" &&
  grep -q -F -e 'CC_HASH_ALT = "integrity"' "$pins" &&
  grep -q -F -e 'CC_LOCK_FILE = "//:MODULE.bazel.lock"' "$pins" &&
  grep -q -F -e 'CC_LOCK_INTEGRITY = "registryFileHashes"' "$pins" &&
  grep -q -F -e 'system packages rejected' "$pins" &&
  grep -q -F -e 'every http_archive carries sha256/integrity' "$pins"; then
  ok
else
  bad "pins.bzl lost its sha256/integrity plus lock plus rejected-system identities under issue #484"
fi

# Pins record the depcheck authority plus live proof labels.
if grep -q -F -e 'CC_DEPCHECK_MANIFEST' "$pins" &&
  grep -q -F -e 'cc_deps.toml' "$pins" &&
  grep -q -F -e 'CC_DEPCHECK_LOCK' "$pins" &&
  grep -q -F -e 'cc_lock.json' "$pins" &&
  grep -q -F -e '//cc/tests/fixtures/hello:hello_test' "$pins" &&
  grep -q -F -e '//cc/tests/fixtures/googletest:greeter_test' "$pins"; then
  ok
else
  bad "pins.bzl lost its depcheck plus live-label wiring under issue #484"
fi

# Committed BCR lock carries integrity for the pinned googletest module.
if [[ -f "$lock" ]] &&
  grep -q -F -e 'registryFileHashes' "$lock" &&
  grep -q -F -e 'modules/googletest/1.18.0/source.json' "$lock" &&
  grep -q -F -e 'modules/googletest/1.18.0/MODULE.bazel' "$lock"; then
  ok
else
  bad "MODULE.bazel.lock lost its committed googletest 1.18.0 integrity (want registryFileHashes plus source.json plus MODULE.bazel hashes)"
fi

# Every http_archive in source carries sha256 or integrity (no hash-less fetch).
# Scans Starlark plus MODULE sources; the BCR lock is integrity-owned elsewhere.
hashless=""
while IFS= read -r f; do
  if grep -q -F -e 'http_archive' "$f"; then
    if ! grep -q -F -e 'sha256' "$f" && ! grep -q -F -e 'integrity' "$f"; then
      hashless="$hashless $f"
    fi
  fi
done < <(find . -path ./bazel-bin -prune -o -path ./bazel-out -prune -o -path ./.git -prune -o \( -name '*.bzl' -print -o -name 'BUILD.bazel' -print -o -name 'MODULE.bazel' -print \) 2>/dev/null)
if [[ -z "$hashless" ]]; then
  ok
else
  bad "hash-less http_archive detected (want sha256 or integrity on every archive):$hashless"
fi

# Committed lock stays fail-closed: lockfile present and tracked.
if [[ -f "$lock" ]] && git ls-files --error-unmatch "$lock" >/dev/null 2>&1; then
  ok
else
  bad "MODULE.bazel.lock lost its committed fail-closed record (want tracked $lock)"
fi

# System packages stay rejected: no host package or local-config wiring in cc sources.
if ! grep -R --include='*.bzl' --include='BUILD.bazel' --include='*.go' -E -e 'apt-get|apt_install|system_package|/usr/include|/usr/lib|local_repository|cc_configure|brew install' -- cc gazelle/cc 2>/dev/null | grep -q .; then
  ok
else
  bad "system package wiring detected in cc sources (want hermetic only, no host apt/brew or /usr or local config)"
fi

# Fixtures stay hermetic: no absolute system includes in cc fixture sources.
if ! grep -R --include='*.cc' --include='*.h' -E -e '#\s*include\s+[<"]/usr' -- cc/tests/fixtures 2>/dev/null | grep -q .; then
  ok
else
  bad "absolute system include detected in cc fixtures (want hermetic sources only)"
fi

# Depcheck proves offline hash authority with cc fixtures plus parser.
if [[ -f "$cc_ok_man" && -f "$cc_ok_lock" && -f "$checker" ]] &&
  grep -q -F -e 'cc_deps.toml' "$checker" &&
  grep -q -F -e 'cc_lock.json' "$checker" &&
  grep -q -F -e 'every http_archive carries sha256/integrity' "$checker"; then
  ok
else
  bad "depcheck lost its cc sha256 authority fixtures plus parser under issue #484"
fi

# Depcheck consistency passes on the hash-pinned ok_used pair.
if $depcheck_bin consistency --ecosystem cc --manifest "$cc_ok_man" --lock "$cc_ok_lock" >/dev/null 2>&1; then
  ok
else
  bad "depcheck cc consistency failed on ok_used (want hash-pinned pair green)"
fi

# Depcheck rejects a hash-less manifest entry (negative hash proof).
dx_mkscratch scratch "${TEST_TMPDIR:-/tmp}/cc-hermetic.XXXXXX"
cp "$cc_ok_man" "$scratch/cc_deps.toml"
cp "$cc_ok_lock" "$scratch/cc_lock.json"
python3 - "$scratch/cc_deps.toml" <<'PY'
import sys
p = sys.argv[1]
t = open(p).read()
t = t.replace('sha256 = "fixture-sha256-greet-1.0.0"', 'sha256 = ""', 1)
open(p, "w").write(t)
PY
if $depcheck_bin consistency --ecosystem cc --manifest "$scratch/cc_deps.toml" --lock "$scratch/cc_lock.json" >/dev/null 2>&1; then
  rm -rf "$scratch"
  bad "depcheck cc accepted a hash-less entry (want missing sha256 to fail)"
else
  code=$?
  rm -rf "$scratch"
  if [[ "$code" == "1" ]]; then ok; else bad "depcheck cc hash-less exit=$code want 1"; fi
fi

# Generation README owns the qualified hash record alongside the other gaps.
if grep -q -F -e 'qualified seed-only under issue #484' "$gen_readme" &&
  grep -q -F -e 'cc_hermetic_qualification' "$gen_readme" &&
  grep -q -F -e 'cc/tests/fixtures/hermetic/' "$gen_readme"; then
  ok
else
  bad "generation README lost its #484 qualified C/C++ hash record"
fi

# BUILD owns the harness target plus CI wires it in dogfood-freshness.
if grep -q -F -e 'name = "cc_hermetic_qualification"' "$build" &&
  grep -q -F -e 'bazel run --noshow_progress //tools/ci:cc_hermetic_qualification' "$ci"; then
  ok
else
  bad "tools/ci/BUILD.bazel or ci.yml lost the cc_hermetic_qualification wiring (want target plus dogfood-freshness)"
fi

# Live proof: the hermetic cc fixtures execute green on the seed host.
if bazel test //cc/tests/fixtures/hello:hello_test //cc/tests/fixtures/googletest:greeter_test --noshow_progress >/dev/null 2>&1; then
  ok
else
  bad "cc hermetic live proof failed (want hello plus greeter_test green)"
fi

dx_test_summary "cc hermetic qualification harness"
