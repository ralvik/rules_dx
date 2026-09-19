#!/usr/bin/env bash
# Preset parity fixture (issue #332, snapshot workflow issue #322): the
# repository's own `tools/bazelrc/preset.bazelrc` is a snapshot of both the
# Python inventory (`tools/bazelrc/preset.py`) and the Rust renderer
# (`cli/adopt/src/preset_fragment.rs` via `dx update`), so the fragment
# consumers regenerate is the one dogfooded here. A drifting hand copy is
# worse than none.
#
# Snapshot testing, not brittle equality: `snapshot_diff` fails with a
# unified diff, while `UPDATE_EXPECT=1` refreshes the golden instead of
# failing (bazel test --test_env=UPDATE_EXPECT). Schema validation below
# pins the contract fields so inventory drift fails at the source even when
# the snapshot is refreshed.
#
# Shell sources have no corpus class. Process-spawning tests stay out of
# the coverage denominator per the repo coverage preset.
set -euo pipefail

# Shared snapshot helper (issue #322).
# Bootstrap: Bazel runfiles forest first (`data = ["//tools/sh:snapshot"]`), then source tree.
source "${RUNFILES_DIR:-/dev/null}/_main/tools/sh/snapshot.sh" 2>/dev/null || source "${TEST_SRCDIR:-/dev/null}/_main/tools/sh/snapshot.sh" 2>/dev/null || source "$0.runfiles/_main/tools/sh/snapshot.sh" 2>/dev/null || source "${BASH_SOURCE[0]}.runfiles/_main/tools/sh/snapshot.sh" 2>/dev/null || source "$(dirname "${BASH_SOURCE[0]}")/tools/sh/snapshot.sh"

# Shared CI helpers (issues #319, #323).
# Bootstrap: Bazel runfiles forest first (`data = ["//tools/sh:lib"]`), then source tree.
source "${RUNFILES_DIR:-/dev/null}/_main/tools/sh/lib.sh" 2>/dev/null || source "${TEST_SRCDIR:-/dev/null}/_main/tools/sh/lib.sh" 2>/dev/null || source "$0.runfiles/_main/tools/sh/lib.sh" 2>/dev/null || source "${BASH_SOURCE[0]}.runfiles/_main/tools/sh/lib.sh" 2>/dev/null || source "$(dirname "${BASH_SOURCE[0]}")/tools/sh/lib.sh"

# Portable helpers via tools/sh/lib.sh dx_realpath/dx_mkscratch (issues #299, #323).

expected="$(dx_realpath "$1")"
dx_bin="$(dx_realpath "$2")"
preset_py="$(dx_realpath "$3")"

dx_test_init

# Schema validation: pin the contract fields on the checked-in fragment,
# mirroring `tools/bazelrc/preset_tests.bzl` and
# `cli/adopt/src/preset_fragment.rs::fragment_matches_python_inventory`.
# Exact bytes stay in the snapshot below; this fails first on shape drift.
for needle in \
  "GENERATED, do not edit" \
  "Version-matched to Bazel 9.2.0" \
  "and dx 0.0.0" \
  "Consumer refresh: \`dx update\`" \
  "common --enable_bzlmod" \
  "build --verbose_failures" \
  "test --test_output=errors" \
  "# Owned extra_presets group: coverage." \
  "coverage --test_env=GENERATE_LLVM_LCOV=1" \
  "coverage --combined_report=lcov" \
  "coverage --test_tag_filters=-no-coverage" \
  "coverage --test_env=COVERAGE_GCOV_PATH=/usr/bin/gcov" \
  "coverage --instrumentation_filter=^//" \
  "# Owned build profiles (issue #177)." \
  "build:dx_debug --compilation_mode=dbg" \
  "build:dx_dev --compilation_mode=fastbuild" \
  "build:dx_release --compilation_mode=opt"; do
  if grep -q -F -e "$needle" "$expected"; then
    ok
  else
    bad "preset.bazelrc missing contract line: $needle"
  fi
done
echo "preset schema: checked-in fragment carries the pinned contract"

# Python inventory pins (mirrors //tools/ci:pin_consistency_test for the
# Bazel pin, plus the per-release dx stamp).
if grep -q -F -e 'PRESET_BAZEL_VERSION = "9.2.0"' "$preset_py" &&
  grep -q -F -e 'PRESET_DX_VERSION = "0.0.0"' "$preset_py"; then
  ok
else
  bad "preset.py lost its Bazel/dx version pins"
fi

dx_mkscratch scratch

# The `dx` binary resolves its workspace through a `MODULE.bazel`
# marker (or `--workspace` override): seed a stub consumer workspace so
# `update` plans against an empty tree instead of this repository.
touch "$scratch/MODULE.bazel"
printf '%s\n' \
  "import %workspace%/tools/bazelrc/preset.bazelrc" \
  "try-import %workspace%/user.bazelrc" \
  >"$scratch/.bazelrc"

# Rust renderer parity: `dx update go` (Go is a no-op backend, no Bazel
# launch) regenerates the fragment via `dx_adopt::update_preset`; the
# snapshot below proves it matches the Python-generated checked-in file.
if "$dx_bin" --workspace "$scratch" update go --quiet >/dev/null 2>&1; then
  ok
else
  bad "dx update go failed in scratch workspace"
fi

actual="$scratch/tools/bazelrc/preset.bazelrc"
if [[ ! -f "$actual" ]]; then
  bad "dx update did not create tools/bazelrc/preset.bazelrc"
fi

snapshot_diff "$expected" "$actual" "tools/bazelrc/preset.bazelrc"
echo "preset parity: Rust renderer output matches checked-in definition (snapshot)"

# Stale gate: `dx update --check` passes on the fresh fragment, fails on a
# dirty one without writing, and `dx update` fixes it (check→update→recheck
# hermetically under `bazel test //...`, issue #407; no nested Bazel).
if "$dx_bin" --workspace "$scratch" update --check --quiet >/dev/null 2>&1; then
  ok
else
  bad "dx update --check failed on the fresh fragment"
fi

printf '%s\n' "# dirty" >"$actual"
if "$dx_bin" --workspace "$scratch" update --check --quiet >/dev/null 2>&1; then
  bad "dx update --check passed on a dirty fragment"
else
  ok
fi
if grep -q -F -e "# dirty" "$actual"; then
  ok
else
  bad "check mode mutated the dirty fragment (check must never write)"
fi

if "$dx_bin" --workspace "$scratch" update go --quiet >/dev/null 2>&1; then
  ok
else
  bad "dx update failed to fix the dirty fragment"
fi
if "$dx_bin" --workspace "$scratch" update --check --quiet >/dev/null 2>&1; then
  ok
else
  bad "dx update --check failed after the fix"
fi

dx_test_summary "preset parity"
