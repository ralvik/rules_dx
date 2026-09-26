#!/usr/bin/env bash
# CI flakiness plus timeout tuning harness.
#
# Machine-checks the as-built flaky-retry plus timeout plus sharding tuning
# with docs in place, without claiming Supported or platform evidence:
# - delivered: `.bazelrc` owns `test --flaky_test_attempts=3` plus
#   `test --test_timeout=300` plus `test --local_test_jobs=4` for every
#   entrypoint, and the one direct `bazel test` in ci.yml (devcontainer
#   parity) repeats the bounded retries plus per-test cap inline
#   (retry-until-green stays rejected);
# - timeouts tuned: raw per-cell test/coverage jobs are gone (each cell
#   compiles once under dx); the surviving ci.yml jobs stay bounded
#   (musl build/coverage plus dogfood-freshness 60, devcontainer 15,
#   aggregate 5), no blanket 90-minute timeouts remain;
#   long-timeouts-only stays rejected;
# - reusable parity (issue #932): reusable-consumer.yml timeouts stay pinned
#   (gate 5, Linux-once 30, per-platform 60, aggregate 10, no 90) with every
#   job bounded; per-target `size` plus `timeout` on every sh_test keeps the
#   global cap from masking slowness;
# - sharding proof: the four-host fan-out rides the dogfood consumer
#   matrix (seed plus arm64 plus macos arm64 plus windows) with the
#   static-musl build plus coverage pair kept as its own ci.yml cells
#   (macOS x86_64 removed per #976); no `strategy.matrix` in ci.yml;
#   Bazel intra-job test sharding follows ordinary semantics
#   (docs/testing/starlark.md);
# - docs in place: `docs/testing/github-ci.md` workflow hygiene plus
# `docs/testing/strategy-details.md` battery timeout record carry;
# - CI only: no product runtime change.
#
# Versioned here, run by CI via `bazel run //tools/ci:flakiness_qualification`,
# following //tools/ci:bootstrap_portability.
set -euo pipefail

# Shared workspace + runfiles helpers.
# Bootstrap via tools/sh/bootstrap.sh (issue #654): runfiles forest first, then source tree.
source "${RUNFILES_DIR:-/dev/null}/_main/tools/sh/bootstrap.sh" 2>/dev/null || source "${TEST_SRCDIR:-/dev/null}/_main/tools/sh/bootstrap.sh" 2>/dev/null || source "$0.runfiles/_main/tools/sh/bootstrap.sh" 2>/dev/null || source "${BASH_SOURCE[0]}.runfiles/_main/tools/sh/bootstrap.sh" 2>/dev/null || source "$(git rev-parse --show-toplevel 2>/dev/null)/tools/sh/bootstrap.sh"
dx_bootstrap "tools/sh/lib.sh"

dx_cd_workspace

dx_test_init

dx_bash_pin

ci=".github/workflows/ci.yml"
ci_notes="docs/testing/workflow-notes.md"
test_matrix="docs/testing/github-ci.md"
testing_readme="docs/testing/strategy-details.md"
build="tools/ci/BUILD.bazel"
targets_b="tools/ci/ci_targets_b.bzl"
prove="tools/ci/prove.sh"
starlark="docs/testing/starlark.md"
consumer=".github/workflows/reusable-consumer.yml"
bazelrc=".bazelrc"

# Header records the flakiness plus timeout tuning with the
# rejected long-timeouts-only alternative (lives in the workflow notes
# since the #915 header split; ci.yml carries no header comments).
if grep -q -F -e 'Flakiness plus timeout tuning (issue #619' "$ci_notes" &&
  grep -q -F -e '--flaky_test_attempts=3' "$ci_notes" &&
  grep -q -F -e '--test_timeout=300' "$ci_notes" &&
  grep -q -F -e 'long-timeouts-only stays rejected' "$ci_notes"; then
  ok
else
  bad "workflow notes lost the issue #619 flakiness plus timeout tuning record with rejected long-timeouts-only"
fi

# Every direct bazel test invocation carries bounded flaky retries, and
# .bazelrc owns the same bound for every other entrypoint (issue #619).
if grep -q -F -e 'test --flaky_test_attempts=3' "$bazelrc" &&
  [[ "$(grep -c -F -e 'bazel test --noshow_progress' "$ci")" -ge "1" ]] &&
  ! grep -F -e 'bazel test --noshow_progress' "$ci" | grep -v -F -e '--flaky_test_attempts=3' | grep -q .; then
  ok
else
  bad "ci.yml lost bounded flaky retries on a direct bazel test invocation (want --flaky_test_attempts=3 everywhere, issue #619)"
fi

# Every direct bazel test invocation carries the per-test timeout cap,
# with .bazelrc carrying it once for every entrypoint.
if grep -q -F -e 'test --test_timeout=300' "$bazelrc" &&
  [[ "$(grep -c -F -e '--test_timeout=300' "$ci")" -ge "1" ]] &&
  ! grep -F -e 'bazel test --noshow_progress' "$ci" | grep -v -F -e '--test_timeout=300' | grep -q .; then
  ok
else
  bad "ci.yml lost the per-test timeout cap on a direct bazel test invocation (want --test_timeout=300 everywhere, issue #619)"
fi

# No bare full-tree test without retry plus timeout flags remains: the
# long-timeouts-only substitute stays rejected.
if ! grep -q -F -e 'run: bazel test --noshow_progress //...' "$ci" &&
  ! grep -q -F -e 'run: bazel test --noshow_progress //.devcontainer' "$ci" &&
  ! grep -q -F -e 'run: bazel test --noshow_progress //deploy/release:dx_release_tools_test' "$ci"; then
  ok
else
  bad "a bare bazel test without --flaky_test_attempts plus --test_timeout survives (long-timeouts-only rejected, issue #619)"
fi

# Seed test plus coverage tuning moved to .bazelrc (issue #619): the raw
# seed test/coverage jobs are gone (each cell compiles once under dx), so
# the bounded retries plus per-test cap plus test-job bound live once for
# every entrypoint instead of per-job timeout knobs
# (hermetic context search, issue #1006).
if grep -q -F -e 'test --flaky_test_attempts=3' "$bazelrc" &&
  grep -q -F -e 'test --test_timeout=300' "$bazelrc" &&
  grep -q -F -e 'test --local_test_jobs=4' "$bazelrc" &&
  ! grep -E -q '^  (test|coverage):' "$ci"; then
  ok
else
  bad "seed test/coverage lost their .bazelrc-owned tuning record (want the three test flags plus no raw seed test/coverage jobs, issue #619)"
fi

# Every surviving ci.yml job stays capped: the musl build/coverage pair
# plus dogfood-freshness at 60, devcontainer at 15, no blanket 90
# anywhere, and no raw per-host test/coverage jobs remain (macOS x86_64
# removed per #976) (hermetic context search, issue #1006).
if DX_CONTEXT_ANCHOR_RE=1 dx_context_contains "$ci" '^  build-musl-x86_64:' -A 3 'timeout-minutes: 60' &&
  DX_CONTEXT_ANCHOR_RE=1 dx_context_contains "$ci" '^  build-musl-arm64:' -A 3 'timeout-minutes: 60' &&
  DX_CONTEXT_ANCHOR_RE=1 dx_context_contains "$ci" '^  coverage-musl-x86_64:' -A 3 'timeout-minutes: 60' &&
  DX_CONTEXT_ANCHOR_RE=1 dx_context_contains "$ci" '^  coverage-musl-arm64:' -A 3 'timeout-minutes: 60' &&
  DX_CONTEXT_ANCHOR_RE=1 dx_context_contains "$ci" '^  dogfood-freshness:' -A 3 'timeout-minutes: 60' &&
  DX_CONTEXT_ANCHOR_RE=1 dx_context_contains "$ci" '^  devcontainer-check:' -A 3 'timeout-minutes: 15' &&
  ! grep -q -F -e 'test-macos-x86_64' "$ci" &&
  ! grep -q -F -e 'coverage-macos-x86_64' "$ci" &&
  ! grep -E -q '^  (test|coverage|test-arm64|coverage-arm64|test-macos-arm64|coverage-macos-arm64|test-windows-x86_64|coverage-windows-x86_64):' "$ci" &&
  ! grep -q -F -e 'timeout-minutes: 90' "$ci"; then
  ok
else
  bad "per-host test/coverage lost their tuned 60-minute caps or a blanket 90-minute timeout survives (issue #619)"
fi

# Build timeouts: the raw seed/prove/per-host build jobs are gone (the
# dogfood self-call plus the musl build cells own the build scope), the
# aggregate stays at 5, and no blanket long timeout remains
# (hermetic context search, issue #1006).
if DX_CONTEXT_ANCHOR_RE=1 dx_context_contains "$ci" '^  ci:' -A 20 'timeout-minutes: 5' &&
  ! grep -E -q '^  (build|prove|build-arm64|build-macos-arm64|build-windows-x86_64):' "$ci" &&
  ! grep -q -F -e 'timeout-minutes: 90' "$ci"; then
  ok
else
  bad "build timeouts drifted (want aggregate 5 plus no raw seed/prove/per-host build jobs with no blanket 90, issue #619)"
fi

# Sharding proof: the four-host fan-out rides the dogfood consumer
# matrix while the static-musl build plus coverage pair keeps its own
# ci.yml cells; no raw per-host test/coverage job remains (macOS x86_64
# removed per #976).
if grep -q -F -e "platforms: '[\"linux_x86_64\", \"linux_arm64\", \"macos_arm64\", \"windows_x86_64\"]'" "$ci" &&
  grep -q -F -e 'platform: ${{ fromJSON(' "$consumer" &&
  grep -q -F -e 'build-musl-x86_64' "$ci" &&
  grep -q -F -e 'build-musl-arm64' "$ci" &&
  grep -q -F -e 'coverage-musl-x86_64' "$ci" &&
  grep -q -F -e 'coverage-musl-arm64' "$ci" &&
  ! grep -q -F -e 'test-macos-x86_64 (bazel test' "$ci" &&
  ! grep -q -F -e 'coverage-macos-x86_64' "$ci" &&
  ! grep -E -q '^  (test|coverage|test-arm64|coverage-arm64|test-macos-arm64|coverage-macos-arm64|test-windows-x86_64|coverage-windows-x86_64):' "$ci"; then
  ok
else
  bad "ci.yml lost per-host test/coverage sharding (want the dogfood four-platform consumer matrix plus the musl pair; raw per-host jobs removed, x86_64 per #976, issues #415/#619)"
fi

# No strategy.matrix in ci.yml: plain named jobs stay the sharding shape
# there while the four-platform fan-out lives in reusable-consumer.yml
# (policy, reused under).
if ! grep -q -F -e 'strategy:' "$ci" &&
  ! grep -q -F -e 'matrix:' "$ci"; then
  ok
else
  bad "ci.yml gained strategy/matrix sharding (the consumer matrix stays the platform fan-out under #415/#619)"
fi

# Bazel intra-job test sharding follows ordinary semantics (link, not copy).
if grep -q -F -e 'Caching, timeouts, and sharding follow' "$starlark" &&
  grep -q -F -e 'ordinary Bazel test semantics' "$starlark"; then
  ok
else
  bad "docs/testing/starlark.md lost the ordinary Bazel sharding semantics record (issue #619 links it, never copies)"
fi

# Docs in place: workflow hygiene records bounded retries plus tuned
# timeouts plus per-host sharding.
if grep -q -F -e 'issue #619' "$test_matrix" &&
  grep -q -F -e '--flaky_test_attempts=3' "$test_matrix" &&
  grep -q -F -e '--test_timeout=300' "$test_matrix" &&
  grep -q -F -e 'timeout-minutes' "$test_matrix" &&
  grep -q -F -e 'Long timeouts only' "$test_matrix"; then
  ok
else
  bad "docs/testing/github-ci.md lost the issue #619 flakiness plus timeout plus sharding record"
fi

# Strategy details links the tuned-timeout battery record (numbers live once in the matrix).
if grep -q -F -e 'github-ci.md' "$testing_readme" &&
  grep -q -F -e 'Flakiness' "$testing_readme"; then
  ok
else
  bad "docs/testing/strategy-details.md lost its flakiness link to github-ci.md"
fi

# Target lives in the sharded list (issue #915) plus CI wires it in
# prove plus dogfood-freshness.
if grep -q -F -e 'name = "flakiness_qualification"' "$targets_b" &&
  grep -q -F -e 'bazel run --noshow_progress //tools/ci:flakiness_qualification' "$prove"; then
  ok
else
  bad "tools/ci/ci_targets_b.bzl or prove.sh lost the flakiness_qualification wiring (want target plus prove plus dogfood-freshness)"
fi

# Retry-until-green stays rejected: bounded attempts only, reruns preserve
# selection via native controls (contract honesty, no analyzer retry loop).
if grep -q -F -e 'retry-until-green' "$test_matrix" &&
  grep -q -F -e 'rejected' "$test_matrix" &&
  grep -q -F -e 'native rerun' "$test_matrix"; then
  ok
else
  bad "test matrix lost its retry-until-green rejection with native-rerun honesty (issue #619)"
fi

# CI only: no product runtime change, no Supported claim.
if grep -q -F -e 'CI only' "$test_matrix" &&
  grep -q -F -e 'no Supported claim' "$test_matrix"; then
  ok
else
  bad "test matrix lost its CI-only plus no-Supported honesty (issue #619)"
fi

# Reusable consumer timeouts stay pinned (issue #932): the reusable template
# drifts without a guard because the checks above assert ci.yml only.
# platforms-gate 5, six Linux-once checks 30, three per-platform checks 60,
# aggregate dx-ci 10; no blanket 90.
reusable=".github/workflows/reusable-consumer.yml"
if DX_CONTEXT_ANCHOR_RE=1 dx_context_contains "$reusable" 'platforms-gate:' -A 5 'timeout-minutes: 5' &&
  DX_CONTEXT_ANCHOR_RE=1 dx_context_contains "$reusable" '^  lint:' -A 5 'timeout-minutes: 30' &&
  DX_CONTEXT_ANCHOR_RE=1 dx_context_contains "$reusable" '^  typecheck:' -A 5 'timeout-minutes: 30' &&
  DX_CONTEXT_ANCHOR_RE=1 dx_context_contains "$reusable" '^  format:' -A 5 'timeout-minutes: 30' &&
  DX_CONTEXT_ANCHOR_RE=1 dx_context_contains "$reusable" '^  generate:' -A 5 'timeout-minutes: 30' &&
  DX_CONTEXT_ANCHOR_RE=1 dx_context_contains "$reusable" '^  security-audit:' -A 5 'timeout-minutes: 30' &&
  DX_CONTEXT_ANCHOR_RE=1 dx_context_contains "$reusable" '^  license-audit:' -A 5 'timeout-minutes: 30' &&
  DX_CONTEXT_ANCHOR_RE=1 dx_context_contains "$reusable" '^  test:' -A 5 'timeout-minutes: 60' &&
  DX_CONTEXT_ANCHOR_RE=1 dx_context_contains "$reusable" '^  build:' -A 5 'timeout-minutes: 60' &&
  DX_CONTEXT_ANCHOR_RE=1 dx_context_contains "$reusable" '^  coverage:' -A 5 'timeout-minutes: 60' &&
  DX_CONTEXT_ANCHOR_RE=1 dx_context_contains "$reusable" '^  dx-ci:' -A 5 'timeout-minutes: 10' &&
  ! grep -q -F -e 'timeout-minutes: 90' "$reusable"; then
  ok
else
  bad "reusable-consumer.yml timeouts drifted (want gate 5 plus Linux-once 30 plus per-platform 60 plus aggregate 10 with no 90, issue #932)"
fi

# Every reusable job carries an explicit timeout: no job without one.
if ! python3 -c "
import re, sys
text = open('.github/workflows/reusable-consumer.yml').read()
jobs = re.findall(r'^  ([a-z-]+):\s*\n', text, re.M)
missing = []
for job in jobs:
    block = re.search(r'^  ' + job + r':\s*\n(.*?)(?=^  [a-z-]+:|\Z)', text, re.M | re.S)
    body = block.group(1) if block else ''
    if 'timeout-minutes:' not in body:
        missing.append(job)
if missing:
    print('missing timeout: ' + ','.join(missing))
    sys.exit(1)
"; then
  bad "a reusable-consumer.yml job lost its timeout-minutes (every job stays bounded, issue #932)"
else
  ok
fi

# Per-target timeouts: every sh_test carries explicit size plus timeout
# (issue #932) so the global --test_timeout=300 cap never masks slowness.
# Small grep harnesses use short; drivers running nested Bazel use moderate.
if python3 -c "
import pathlib, re, sys
roots = list(pathlib.Path('.').rglob('BUILD.bazel')) + list(pathlib.Path('tools/ci').glob('*.bzl')) + [pathlib.Path('libs/starlark/tests/negative/negative_tests.bzl')]
bad = []
for f in roots:
    if '.git/' in str(f):
        continue
    text = f.read_text(errors='ignore')
    parts = re.split(r'(\bsh_test\(\n)', text)
    for idx in range(1, len(parts), 2):
        chunk = parts[idx+1] if idx+1 < len(parts) else ''
        m = re.search(r'\n[ ]{0,8}\)\n', '\n' + chunk)
        block = chunk[:m.end()] if m else chunk[:1500]
        if not ('size =' in block and 'timeout =' in block):
            name = re.search(r'name = \"([^\"]+)\"', block)
            bad.append(str(f) + ':' + (name.group(1) if name else '?'))
if bad:
    print('\n'.join(bad))
    sys.exit(1)
"; then
  ok
else
  bad "an sh_test lost its explicit size plus timeout (want per-target timeouts everywhere, issue #932)"
fi

dx_test_summary "ci flakiness plus timeout tuning harness"
