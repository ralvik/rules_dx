#!/usr/bin/env bash
# BUILD comment hygiene guard.
#
# BUILD.bazel files carry only what the target is plus non-obvious attrs;
# essays live in docs/contributing/build-conventions.md with one-line refs.
# GENERATED headers are minimal (GENERATED plus one regenerate command).
#
# This harness machine-checks the trimmed state on a clean tree
# (table-driven guard rows via `tools/sh/guards.sh` plus two custom
# numeric gates): the canonical doc, banned boilerplate absent, comment
# density, GENERATED header shape, preset header shape, Gazelle keep
# boundary, and the non-obvious attrs still present.
#
# Versioned here, run by CI via `bazel run //tools/ci:build_hygiene`,
# following //tools/ci:widen_update_loop.
set -euo pipefail

# Shared workspace + runfiles helpers.
# Bootstrap via tools/sh/bootstrap.sh (issue #654): runfiles forest first, then source tree.
source "${RUNFILES_DIR:-/dev/null}/_main/tools/sh/bootstrap.sh" 2>/dev/null || source "${TEST_SRCDIR:-/dev/null}/_main/tools/sh/bootstrap.sh" 2>/dev/null || source "$0.runfiles/_main/tools/sh/bootstrap.sh" 2>/dev/null || source "${BASH_SOURCE[0]}.runfiles/_main/tools/sh/bootstrap.sh" 2>/dev/null || source "$(git rev-parse --show-toplevel 2>/dev/null)/tools/sh/bootstrap.sh"
dx_bootstrap "tools/sh/lib.sh"
dx_bootstrap "tools/sh/guards.sh"

dx_cd_workspace

dx_test_init

# Canonical doc owns the four repeated patterns with links to owners.
dx_guards_contains docs/contributing/build-conventions.md "canonical BUILD conventions doc missing or lost sections (want Lane A/Corpus/No-Coverage/Shell plus owner links, issue #427)" \
  '# BUILD Conventions' \
  '## Lane A' \
  '## Corpus' \
  '## No-Coverage' \
  '## Shell' \
  'native-configuration' \
  'shell-and-host-tool-contract'

# Banned boilerplate stays out of BUILD comments: the essays
# moved to the canonical doc, leaving one-line refs only.
dx_guards_tree_absent_re 'BUILD.bazel' "banned BUILD boilerplate reappeared (want no forwarder-plumbing/coverage-denominator/no-corpus-entry/dogfood-policy/real_fixture_policy/expected-greeting-tagged/keep-regeneration essays in BUILD.bazel, issue #427)" \
  '^[[:space:]]*#[^!]*Proves the forwarder plumbing' \
  '^[[:space:]]*#[^!]*out of the coverage denominator per the repo coverage preset' \
  '^[[:space:]]*#[^!]*No corpus entry: shell sources' \
  '^[[:space:]]*#[^!]*direct-Bazel dogfood under the canonical' \
  '^[[:space:]]*#[^!]*real_fixture_policy' \
  'expected greeting\. Tagged' \
  '# `# keep` preserves them across regeneration'

# Comment density stays trimmed: total comment-only lines
# under 8% of all BUILD.bazel lines (was ~11% before the sweep).
if python3 -c "
import pathlib
files=list(pathlib.Path('.').rglob('BUILD.bazel'))
total=sum(len(f.read_text(errors='ignore').splitlines()) for f in files)
comment=sum(1 for f in files for l in f.read_text(errors='ignore').splitlines() if l.strip().startswith('#'))
pct=comment/total*100 if total else 100
print(f'BUILD comment density {comment}/{total} {pct:.1f}%')
assert pct < 8, pct
"; then
  ok
else
  bad "BUILD comment density >= 8% (want trimmed one-line refs per docs/contributing/build-conventions.md, issue #427)"
fi

# GENERATED artifact headers stay minimal: GENERATED plus one
# regenerate command, no Release provenance (provenance lives in
# update.py inputs plus ARTIFACT url/sha256 fields).
dx_guards_contains quality/artifacts/ruff.linux_x86_64.bzl "quality/artifacts GENERATED header lost minimal shape (want GENERATED plus regenerate only, issue #427)" \
  'GENERATED, do not edit' \
  'Regenerate with: bazel run //quality/artifacts:update'
dx_guards_tree_absent '*.linux_x86_64.bzl' "quality/artifacts GENERATED header lost minimal shape (want no Release line in files, issue #427)" \
  'Release: https://'
dx_guard_contains quality/artifacts/update.py 'Regenerate with: bazel run //quality/artifacts:update' "quality/artifacts generator lost minimal shape (want regenerate only, issue #427)"
dx_guard_absent quality/artifacts/update.py 'Release: %s' "quality/artifacts generator lost minimal shape (want no Release line, issue #427)"

# Vendored preset header stays minimal: GENERATED plus one
# regenerate command (version and consumer provenance lives in src/lib.rs
# pins plus preset_tests.bzl file_checks).
dx_guards_contains tools/bazelrc/preset.bazelrc "preset.bazelrc lost minimal GENERATED header (want GENERATED plus regenerate only, issue #427)" \
  'GENERATED, do not edit' \
  'Regenerate:' \
  'preset_update'
dx_guards_absent tools/bazelrc/preset.bazelrc "preset.bazelrc lost minimal GENERATED header (want no Version-matched/Consumer-refresh/Upstream-derived prose, issue #427)" \
  'Version-matched to Bazel' \
  'Consumer refresh:' \
  'Upstream-derived flags'
dx_guard_contains tools/bazelrc/src/lib.rs 'PRESET_BAZEL_VERSION' "preset src lost its version pin (want PRESET_BAZEL_VERSION, issue #427)"

# Gazelle boundary stays hand-free: no hand prose mixed into
# regen stanzas; bare # keep only where the generator requires it.
dx_guard_tree_absent_re 'BUILD.bazel' "Gazelle keep boundary regressed (want no keep-regeneration essay, issue #427)" \
  '# `# keep` preserves them across regeneration'
dx_guards_contains examples/adopt-python/app/BUILD.bazel "Gazelle keep boundary regressed (want bare # keep with one-line User-owned ref, issue #427)" \
  '# keep' \
  'User-owned deps (# keep)'

# Non-obvious attrs survive the trim (is comment-only): the
# proof bindings still carry aspect_hints, bash harnesses still carry
# Linux-only labels, and process-spawning tests still carry no-coverage.
dx_guard_contains python/tests/fixtures/hello/BUILD.bazel 'aspect_hints' "trim dropped non-obvious attrs (want aspect_hints proof bindings, issue #427)"
dx_guard_contains rust/tests/fixtures/hello/BUILD.bazel 'aspect_hints' "trim dropped non-obvious attrs (want aspect_hints proof bindings, issue #427)"
dx_guard_contains python/tests/fixtures/hello/BUILD.bazel 'no-coverage' "trim dropped non-obvious attrs (want no-coverage tags, issue #427)"
labels="$(grep -r -F -e 'target_compatible_with' --include='BUILD.bazel' --include='*.bzl' --exclude-dir='bazel-*' --exclude-dir='.git' . | wc -l)"
if [[ "$labels" -ge 69 ]]; then
  ok
else
  bad "trim dropped non-obvious attrs (want >= 69 Linux-only labels, found $labels, issue #427)"
fi

# New doc stays in the docs corpus (audit shape).
dx_guard_contains docs/BUILD.bazel 'contributing/build-conventions.md' "docs/BUILD.bazel lost the build-conventions corpus entry (want contributing/build-conventions.md, issue #427)"

dx_test_summary "build hygiene harness"
