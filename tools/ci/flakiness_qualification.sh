#!/usr/bin/env bash
# CI flakiness plus timeout tuning harness.
#
# Machine-checks the as-built flaky-retry plus timeout plus sharding tuning
# with docs in place, without claiming Supported or platform evidence:
# - delivered: every direct `bazel test` invocation in ci.yml carries
#   `--flaky_test_attempts=3 --test_timeout=300` (bounded retries for
#   transient flakes, per-test 300s cap); retry-until-green stays rejected;
# - timeouts tuned: seed test/coverage at 45 minutes, per-host test/coverage
#   at 60 minutes, no blanket 90-minute timeouts remain; builds stay 30/60;
#   long-timeouts-only stays rejected;
# - reusable parity (issue #932): reusable-consumer.yml timeouts stay pinned
#   (gate 5, Linux-once 30, per-platform 60, aggregate 10, no 90) with every
#   job bounded; per-target `size` plus `timeout` on every sh_test keeps the
#   global cap from masking slowness;
# - sharding proof: per-host/per-stage job sharding stays pinned (seed plus
# arm64 plus musl pair plus macos pair plus windows,) with
#   fast-fail needs chains plus per-job summaries, no `strategy.matrix`;
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

ci=".github/workflows/ci.yml"
ci_notes="docs/testing/workflow-notes.md"
test_matrix="docs/testing/github-ci.md"
testing_readme="docs/testing/strategy-details.md"
verify="docs/testing/verification-matrix.md"
build="tools/ci/BUILD.bazel"
targets_b="tools/ci/ci_targets_b.bzl"
prove="tools/ci/prove.sh"
starlark="docs/testing/starlark.md"

# Header records the flakiness plus timeout tuning with the
# rejected long-timeouts-only alternative (lives in the workflow notes
# since the #915 header split; ci.yml carries no header comments).
if grep -q -F -e 'Flakiness plus timeout tuning (issue #619' "$ci_notes" &&
  grep -q -F -e '--flaky_test_attempts=3 --test_timeout=300' "$ci_notes" &&
  grep -q -F -e 'long-timeouts-only stays rejected' "$ci_notes"; then
  ok
else
  bad "workflow notes lost the issue #619 flakiness plus timeout tuning record with rejected long-timeouts-only"
fi

# Every direct bazel test invocation carries bounded flaky retries.
if [[ "$(grep -c -F -e 'bazel test --noshow_progress' "$ci")" -ge "7" ]] &&
  [[ "$(grep -c -F -e '--flaky_test_attempts=3' "$ci")" -ge "7" ]] &&
  ! grep -F -e 'bazel test --noshow_progress' "$ci" | grep -v -F -e '--flaky_test_attempts=3' | grep -q .; then
  ok
else
  bad "ci.yml lost bounded flaky retries on a direct bazel test invocation (want --flaky_test_attempts=3 everywhere, issue #619)"
fi

# Every direct bazel test invocation carries the per-test timeout cap.
if [[ "$(grep -c -F -e '--test_timeout=300' "$ci")" -ge "7" ]] &&
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

# Seed test plus coverage stay tuned to 45 minutes (not blanket 60/90).
if grep -A3 -e '^  test:' "$ci" | grep -q -F -e 'timeout-minutes: 45' &&
  grep -A3 -e '^  coverage:' "$ci" | grep -q -F -e 'timeout-minutes: 45'; then
  ok
else
  bad "seed test/coverage lost their tuned 45-minute timeouts (issue #619)"
fi

# Per-host test plus coverage stay capped at 60 minutes; no blanket 90
# remains anywhere in the workflow (macOS x86_64 removed per #976).
if grep -A3 -e '^  test-arm64:' "$ci" | grep -q -F -e 'timeout-minutes: 60' &&
  grep -A3 -e '^  coverage-arm64:' "$ci" | grep -q -F -e 'timeout-minutes: 60' &&
  grep -A3 -e '^  coverage-musl-x86_64:' "$ci" | grep -q -F -e 'timeout-minutes: 60' &&
  grep -A3 -e '^  coverage-musl-arm64:' "$ci" | grep -q -F -e 'timeout-minutes: 60' &&
  grep -A3 -e '^  test-macos-arm64:' "$ci" | grep -q -F -e 'timeout-minutes: 60' &&
  grep -A3 -e '^  coverage-macos-arm64:' "$ci" | grep -q -F -e 'timeout-minutes: 60' &&
  ! grep -q -F -e 'test-macos-x86_64' "$ci" &&
  ! grep -q -F -e 'coverage-macos-x86_64' "$ci" &&
  grep -A3 -e '^  test-windows-x86_64:' "$ci" | grep -q -F -e 'timeout-minutes: 60' &&
  grep -A3 -e '^  coverage-windows-x86_64:' "$ci" | grep -q -F -e 'timeout-minutes: 60' &&
  ! grep -q -F -e 'timeout-minutes: 90' "$ci"; then
  ok
else
  bad "per-host test/coverage lost their tuned 60-minute caps or a blanket 90-minute timeout survives (issue #619)"
fi

# Builds stay 30/60: seed plus sbom plus prove plus dogfood-freshness fast,
# per-host builds bounded, no blanket long timeout.
if grep -A3 -e '^  build:' "$ci" | grep -q -F -e 'timeout-minutes: 30' &&
  grep -A3 -e '^  prove:' "$ci" | grep -q -F -e 'timeout-minutes: 30' &&
  grep -A3 -e '^  dogfood-freshness:' "$ci" | grep -q -F -e 'timeout-minutes: 30' &&
  grep -A3 -e '^  build-arm64:' "$ci" | grep -q -F -e 'timeout-minutes: 60' &&
  grep -A3 -e '^  build-macos-arm64:' "$ci" | grep -q -F -e 'timeout-minutes: 60' &&
  grep -A3 -e '^  build-windows-x86_64:' "$ci" | grep -q -F -e 'timeout-minutes: 60'; then
  ok
else
  bad "build timeouts drifted (want seed/prove/dogfood 30 plus per-host builds 60, issue #619)"
fi

# Sharding proof reuses the per-host matrix: seed plus arm64
# plus musl pair plus macos arm64 plus windows test/coverage jobs stay queued
# (macOS x86_64 removed per #976).
if grep -q -F -e 'test-arm64 (bazel test' "$ci" &&
  grep -q -F -e 'coverage-arm64 (dx coverage gate, arm64 cell)' "$ci" &&
  grep -q -F -e 'coverage-musl-x86_64' "$ci" &&
  grep -q -F -e 'coverage-musl-arm64' "$ci" &&
  grep -q -F -e 'test-macos-arm64 (bazel test' "$ci" &&
  grep -q -F -e 'coverage-macos-arm64' "$ci" &&
  ! grep -q -F -e 'test-macos-x86_64 (bazel test' "$ci" &&
  ! grep -q -F -e 'coverage-macos-x86_64' "$ci" &&
  grep -q -F -e 'test-windows-x86_64 (bazel test' "$ci" &&
  grep -q -F -e 'coverage-windows-x86_64' "$ci"; then
  ok
else
  bad "ci.yml lost per-host test/coverage sharding (seed plus arm64 plus musl pair plus macos arm64 plus windows, issues #415/#619; x86_64 removed per #976)"
fi

# No strategy.matrix: per-host/per-stage jobs stay the sharding shape
# (policy, reused under).
if ! grep -q -F -e 'strategy:' "$ci" &&
  ! grep -q -F -e 'matrix:' "$ci"; then
  ok
else
  bad "ci.yml gained strategy/matrix sharding (per-host jobs stay the sharding shape under #415/#619)"
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

# Verification matrix owns the qualified seed-only record under.
if grep -q -F -e 'flakiness_qualification' "$verify" &&
  grep -q -F -e 'qualified seed-only under #619' "$verify" &&
  grep -q -F -e 'bazel run //tools/ci:flakiness_qualification' "$verify" &&
  grep -q -F -e '`flakiness_qualification` 19/19' "$verify"; then
  ok
else
  bad "verification-matrix lost its #619 flakiness qualified record"
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
if grep -A5 -e 'platforms-gate:' "$reusable" | grep -q -F -e 'timeout-minutes: 5' &&
  grep -A5 -e '^  lint:' "$reusable" | grep -q -F -e 'timeout-minutes: 30' &&
  grep -A5 -e '^  typecheck:' "$reusable" | grep -q -F -e 'timeout-minutes: 30' &&
  grep -A5 -e '^  format:' "$reusable" | grep -q -F -e 'timeout-minutes: 30' &&
  grep -A5 -e '^  generate:' "$reusable" | grep -q -F -e 'timeout-minutes: 30' &&
  grep -A5 -e '^  security-audit:' "$reusable" | grep -q -F -e 'timeout-minutes: 30' &&
  grep -A5 -e '^  license-audit:' "$reusable" | grep -q -F -e 'timeout-minutes: 30' &&
  grep -A5 -e '^  test:' "$reusable" | grep -q -F -e 'timeout-minutes: 60' &&
  grep -A5 -e '^  build:' "$reusable" | grep -q -F -e 'timeout-minutes: 60' &&
  grep -A5 -e '^  coverage:' "$reusable" | grep -q -F -e 'timeout-minutes: 60' &&
  grep -A5 -e '^  dx-ci:' "$reusable" | grep -q -F -e 'timeout-minutes: 10' &&
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
    parts = re.split(r'(sh_test\(\n)', text)
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
