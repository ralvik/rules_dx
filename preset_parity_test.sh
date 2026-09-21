#!/usr/bin/env bash
# Preset parity fixture (snapshot workflow): the
# repository's own `tools/bazelrc/preset.bazelrc` is a snapshot of both the
# Rust inventory (`tools/bazelrc/src/lib.rs`) and the Rust renderer
# (`cli/adopt/src/preset_fragment.rs` via `dx update`), so the fragment
# consumers regenerate is the one dogfooded here. A drifting hand copy is
# worse than none.
#
# Snapshot testing, not brittle equality: `snapshot_diff` fails with a
# unified diff, while `UPDATE_EXPECT=1` refreshes the golden instead of
# failing (bazel test --test_env=UPDATE_EXPECT). Schema validation below
# pins the contract fields via the table-driven guard rows so inventory
# drift fails at the source even when the snapshot is refreshed.
#
# Shell sources have no corpus class. Process-spawning tests stay out of
# the coverage denominator per the repo coverage preset.
set -euo pipefail

# Shared snapshot helper.
# Bootstrap via tools/sh/bootstrap.sh (issue #654): runfiles forest first, then source tree.
source "${RUNFILES_DIR:-/dev/null}/_main/tools/sh/bootstrap.sh" 2>/dev/null || source "${TEST_SRCDIR:-/dev/null}/_main/tools/sh/bootstrap.sh" 2>/dev/null || source "$0.runfiles/_main/tools/sh/bootstrap.sh" 2>/dev/null || source "${BASH_SOURCE[0]}.runfiles/_main/tools/sh/bootstrap.sh" 2>/dev/null || source "$(git rev-parse --show-toplevel 2>/dev/null)/tools/sh/bootstrap.sh"
dx_bootstrap "tools/sh/snapshot.sh"

# Shared CI helpers.
# Bootstrap via tools/sh/bootstrap.sh (issue #654): runfiles forest first, then source tree.
dx_bootstrap "tools/sh/lib.sh"
dx_bootstrap "tools/sh/guards.sh"

# Portable helpers via tools/sh/lib.sh dx_realpath/dx_mkscratch.

expected="$(dx_realpath "$1")"
dx_bin="$(dx_realpath "$2")"
preset_rs="$(dx_realpath "$3")"

dx_test_init

# Schema validation: pin the contract fields on the checked-in fragment,
# mirroring `tools/bazelrc/preset_tests.bzl` and
# `cli/adopt/src/preset_fragment.rs::fragment_matches_preset_inventory`.
# Exact bytes stay in the snapshot below; this fails first on shape drift.
# Fixed-string guard table (fail-closed, no refresh): snapshot owns the
# byte-identity golden, this table owns the contract sentences.
dx_guards_contains "$expected" "preset.bazelrc missing contract lines" \
  'GENERATED, do not edit' \
  'Regenerate: `bazel run //tools/bazelrc:preset.update`' \
  'common --enable_bzlmod' \
  'build --verbose_failures' \
  'test --test_output=errors' \
  '# Owned extra_presets group: coverage.' \
  'coverage --test_env=GENERATE_LLVM_LCOV=1' \
  'coverage --combined_report=lcov' \
  'coverage --test_tag_filters=-no-coverage' \
  'coverage --test_env=COVERAGE_GCOV_PATH=/usr/bin/gcov' \
  'coverage --instrumentation_filter=^//' \
  '# Owned build profiles (issue #177; See: docs/decisions/0021-build-profiles.md).' \
  'build:dx_debug --compilation_mode=dbg' \
  'build:dx_dev --compilation_mode=fastbuild' \
  'build:dx_release --compilation_mode=opt'
echo "preset schema: checked-in fragment carries the pinned contract"

# Rust inventory pins (mirrors //tools/ci:pin_consistency_test for the
# Bazel pin, plus the per-release dx stamp).
dx_guards_contains "$preset_rs" "preset src lost its Bazel/dx version pins" \
  'PRESET_BAZEL_VERSION' \
  'PRESET_DX_VERSION' \
  '"9.2.0"' \
  '"0.0.0"'

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
# snapshot below proves it matches the Rust-generated checked-in file.
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
# hermetically under `bazel test //...`,; no nested Bazel).
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
