#!/usr/bin/env bash
# BUILD comment hygiene guard (issue #427).
#
# BUILD.bazel files carry only what the target is plus non-obvious attrs;
# essays live in docs/contributing/build-conventions.md with one-line refs.
# GENERATED headers are minimal (GENERATED plus one regenerate command).
#
# This harness machine-checks the trimmed state on a clean tree (8 checks):
# the canonical doc, banned boilerplate absent, comment density, GENERATED
# header shape, preset header shape, Gazelle keep boundary, and the
# non-obvious attrs still present.
#
# Versioned here, run by CI via `bazel run //tools/ci:build_hygiene`,
# following //tools/ci:widen_update_loop.
set -euo pipefail

# Shared workspace + runfiles helpers (issue #319).
# Bootstrap: Bazel runfiles forest first (`data = ["//tools/sh:lib"]`), then source tree.
source "${RUNFILES_DIR:-/dev/null}/_main/tools/sh/lib.sh" 2>/dev/null || source "${TEST_SRCDIR:-/dev/null}/_main/tools/sh/lib.sh" 2>/dev/null || source "$0.runfiles/_main/tools/sh/lib.sh" 2>/dev/null || source "${BASH_SOURCE[0]}.runfiles/_main/tools/sh/lib.sh" 2>/dev/null || source "$(dirname "${BASH_SOURCE[0]}")/../sh/lib.sh"

dx_cd_workspace

dx_test_init

# Canonical doc owns the four repeated patterns with links to owners.
if [[ -f "docs/contributing/build-conventions.md" ]] &&
  grep -q -F -e '# BUILD Conventions' docs/contributing/build-conventions.md &&
  grep -q -F -e '## Lane A' docs/contributing/build-conventions.md &&
  grep -q -F -e '## Corpus' docs/contributing/build-conventions.md &&
  grep -q -F -e '## No-Coverage' docs/contributing/build-conventions.md &&
  grep -q -F -e '## Shell' docs/contributing/build-conventions.md &&
  grep -q -F -e 'native-configuration' docs/contributing/build-conventions.md &&
  grep -q -F -e 'shell-and-host-tool-contract' docs/contributing/build-conventions.md; then
  ok
else
  bad "canonical BUILD conventions doc missing or lost sections (want docs/contributing/build-conventions.md with Lane A/Corpus/No-Coverage/Shell plus owner links, issue #427)"
fi

# Banned boilerplate stays out of BUILD comments (issue #427): the essays
# moved to the canonical doc, leaving one-line refs only.
if ! grep -rn --include='BUILD.bazel' -e '^[[:space:]]*#[^!]*Proves the forwarder plumbing' . 2>/dev/null | grep -q . &&
  ! grep -rn --include='BUILD.bazel' -e '^[[:space:]]*#[^!]*out of the coverage denominator per the repo coverage preset' . 2>/dev/null | grep -q . &&
  ! grep -rn --include='BUILD.bazel' -e '^[[:space:]]*#[^!]*No corpus entry: shell sources' . 2>/dev/null | grep -q . &&
  ! grep -rn --include='BUILD.bazel' -e '^[[:space:]]*#[^!]*direct-Bazel dogfood under the canonical' . 2>/dev/null | grep -q . &&
  ! grep -rn --include='BUILD.bazel' -e '^[[:space:]]*#[^!]*real_fixture_policy' . 2>/dev/null | grep -q . &&
  ! grep -rn --include='BUILD.bazel' -e 'expected greeting\. Tagged' . 2>/dev/null | grep -q . &&
  ! grep -rn --include='BUILD.bazel' -e '# `# keep` preserves them across regeneration' . 2>/dev/null | grep -q .; then
  ok
else
  bad "banned BUILD boilerplate reappeared (want no forwarder-plumbing/coverage-denominator/no-corpus-entry/dogfood-policy/real_fixture_policy/expected-greeting-tagged/keep-regeneration essays in BUILD.bazel, issue #427)"
fi

# Comment density stays trimmed (issue #427): total comment-only lines
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

# GENERATED artifact headers stay minimal (issue #427): GENERATED plus one
# regenerate command, no Release provenance (provenance lives in
# update.py inputs plus ARTIFACT url/sha256 fields).
if grep -q -F -e 'GENERATED, do not edit' quality/artifacts/ruff.linux_x86_64.bzl &&
  grep -q -F -e 'Regenerate with: bazel run //quality/artifacts:update' quality/artifacts/ruff.linux_x86_64.bzl &&
  ! grep -rn -F -e 'Release: https://' quality/artifacts/*.linux_x86_64.bzl 2>/dev/null | grep -q . &&
  grep -q -F -e 'Regenerate with: bazel run //quality/artifacts:update' quality/artifacts/update.py &&
  ! grep -q -F -e 'Release: %s' quality/artifacts/update.py; then
  ok
else
  bad "quality/artifacts GENERATED header lost minimal shape (want GENERATED plus regenerate only, no Release line in files or generator, issue #427)"
fi

# Vendored preset header stays minimal (issue #427): GENERATED plus one
# regenerate command (version and consumer provenance lives in preset.py
# pins plus preset_tests.bzl file_checks).
if grep -q -F -e 'GENERATED, do not edit' tools/bazelrc/preset.bazelrc &&
  grep -q -F -e 'Regenerate:' tools/bazelrc/preset.bazelrc &&
  grep -q -F -e 'preset.update' tools/bazelrc/preset.bazelrc &&
  ! grep -q -F -e 'Version-matched to Bazel' tools/bazelrc/preset.bazelrc &&
  ! grep -q -F -e 'Consumer refresh:' tools/bazelrc/preset.bazelrc &&
  ! grep -q -F -e 'Upstream-derived flags' tools/bazelrc/preset.bazelrc &&
  grep -q -F -e 'PRESET_BAZEL_VERSION' tools/bazelrc/preset.py; then
  ok
else
  bad "preset.bazelrc lost minimal GENERATED header (want GENERATED plus regenerate only, no Version-matched/Consumer-refresh/Upstream-derived prose, issue #427)"
fi

# Gazelle boundary stays hand-free (issue #427): no hand prose mixed into
# regen stanzas; bare # keep only where the generator requires it.
if ! grep -rn --include='BUILD.bazel' -e '# `# keep` preserves them across regeneration' . 2>/dev/null | grep -q . &&
  grep -q -F -e '# keep' examples/adopt-python/app/BUILD.bazel &&
  grep -q -F -e 'User-owned deps (# keep)' examples/adopt-python/app/BUILD.bazel; then
  ok
else
  bad "Gazelle keep boundary regressed (want no keep-regeneration essay plus bare # keep with one-line User-owned ref in examples/adopt-python/app/BUILD.bazel, issue #427)"
fi

# Non-obvious attrs survive the trim (issue #427 is comment-only): the
# proof bindings still carry aspect_hints, bash harnesses still carry
# Linux-only labels, and process-spawning tests still carry no-coverage.
labels="$(grep -r -F -e 'target_compatible_with' --include='BUILD.bazel' . | wc -l)"
if grep -q -F -e 'aspect_hints' python/tests/fixtures/hello/BUILD.bazel &&
  grep -q -F -e 'aspect_hints' rust/tests/fixtures/hello/BUILD.bazel &&
  grep -q -F -e 'no-coverage' python/tests/fixtures/hello/BUILD.bazel &&
  [[ "$labels" -ge 69 ]]; then
  ok
else
  bad "trim dropped non-obvious attrs (want aspect_hints proof bindings plus no-coverage tags plus >= 69 Linux-only labels, found $labels, issue #427)"
fi

# New doc stays in the docs corpus (issue #15 audit shape).
if grep -q -F -e 'contributing/build-conventions.md' docs/BUILD.bazel; then
  ok
else
  bad "docs/BUILD.bazel lost the build-conventions corpus entry (want contributing/build-conventions.md, issue #427)"
fi

dx_test_summary "build hygiene harness"
