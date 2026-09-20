#!/usr/bin/env bash
# CI flakiness plus timeout tuning harness (issue #619).
#
# Machine-checks the as-built flaky-retry plus timeout plus sharding tuning
# with docs in place, without claiming Supported or platform evidence:
# - delivered: every direct `bazel test` invocation in ci.yml carries
#   `--flaky_test_attempts=3 --test_timeout=300` (bounded retries for
#   transient flakes, per-test 300s cap); retry-until-green stays rejected;
# - timeouts tuned: seed test/coverage at 45 minutes, per-host test/coverage
#   at 60 minutes, no blanket 90-minute timeouts remain; builds stay 30/60;
#   long-timeouts-only stays rejected;
# - sharding proof: per-host/per-stage job sharding stays pinned (seed plus
#   arm64 plus musl pair plus macos pair plus windows, issue #415) with
#   fast-fail needs chains plus per-job summaries, no `strategy.matrix`;
#   Bazel intra-job test sharding follows ordinary semantics
#   (docs/testing/starlark.md);
# - docs in place: `docs/testing/github-ci.md` workflow hygiene plus
#   `docs/testing/README.md` battery timeout record carry issue #619;
# - CI only: no product runtime change.
#
# Versioned here, run by CI via `bazel run //tools/ci:flakiness_qualification`,
# following //tools/ci:bootstrap_portability.
set -euo pipefail

# Shared workspace + runfiles helpers (issues #319, #323).
# Bootstrap: Bazel runfiles forest first (`data = ["//tools/sh:lib"]`), then source tree.
source "${RUNFILES_DIR:-/dev/null}/_main/tools/sh/lib.sh" 2>/dev/null || source "${TEST_SRCDIR:-/dev/null}/_main/tools/sh/lib.sh" 2>/dev/null || source "$0.runfiles/_main/tools/sh/lib.sh" 2>/dev/null || source "${BASH_SOURCE[0]}.runfiles/_main/tools/sh/lib.sh" 2>/dev/null || source "$(dirname "${BASH_SOURCE[0]}")/../sh/lib.sh"

dx_cd_workspace

dx_test_init

ci=".github/workflows/ci.yml"
test_matrix="docs/testing/github-ci.md"
testing_readme="docs/testing/README.md"
verify="docs/testing/verification-matrix.md"
build="tools/ci/BUILD.bazel"
starlark="docs/testing/starlark.md"

# Header records the flakiness plus timeout tuning under issue #619 with the
# rejected long-timeouts-only alternative.
if grep -q -F -e 'Flakiness plus timeout tuning (issue #619' "$ci" &&
  grep -q -F -e '--flaky_test_attempts=3 --test_timeout=300' "$ci" &&
  grep -q -F -e 'long-timeouts-only stays rejected' "$ci"; then
  ok
else
  bad "ci.yml header lost the issue #619 flakiness plus timeout tuning record with rejected long-timeouts-only"
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
  ! grep -q -F -e 'run: bazel test --noshow_progress //deploy/release:sbom_demo_verify' "$ci"; then
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
# remains anywhere in the workflow.
if grep -A3 -e '^  test-arm64:' "$ci" | grep -q -F -e 'timeout-minutes: 60' &&
  grep -A3 -e '^  coverage-arm64:' "$ci" | grep -q -F -e 'timeout-minutes: 60' &&
  grep -A3 -e '^  coverage-musl-x86_64:' "$ci" | grep -q -F -e 'timeout-minutes: 60' &&
  grep -A3 -e '^  coverage-musl-arm64:' "$ci" | grep -q -F -e 'timeout-minutes: 60' &&
  grep -A3 -e '^  test-macos-arm64:' "$ci" | grep -q -F -e 'timeout-minutes: 60' &&
  grep -A3 -e '^  coverage-macos-arm64:' "$ci" | grep -q -F -e 'timeout-minutes: 60' &&
  grep -A3 -e '^  test-macos-x86_64:' "$ci" | grep -q -F -e 'timeout-minutes: 60' &&
  grep -A3 -e '^  coverage-macos-x86_64:' "$ci" | grep -q -F -e 'timeout-minutes: 60' &&
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

# Sharding proof reuses the per-host matrix (issue #415): seed plus arm64
# plus musl pair plus macos pair plus windows test/coverage jobs stay queued.
if grep -q -F -e 'test-arm64 (bazel test' "$ci" &&
  grep -q -F -e 'coverage-arm64 (dx coverage gate, arm64 cell)' "$ci" &&
  grep -q -F -e 'coverage-musl-x86_64' "$ci" &&
  grep -q -F -e 'coverage-musl-arm64' "$ci" &&
  grep -q -F -e 'test-macos-arm64 (bazel test' "$ci" &&
  grep -q -F -e 'coverage-macos-arm64' "$ci" &&
  grep -q -F -e 'test-macos-x86_64 (bazel test' "$ci" &&
  grep -q -F -e 'coverage-macos-x86_64' "$ci" &&
  grep -q -F -e 'test-windows-x86_64 (bazel test' "$ci" &&
  grep -q -F -e 'coverage-windows-x86_64' "$ci"; then
  ok
else
  bad "ci.yml lost per-host test/coverage sharding (seed plus arm64 plus musl pair plus macos pair plus windows, issues #415/#619)"
fi

# No strategy.matrix: per-host/per-stage jobs stay the sharding shape
# (issue #415 policy, reused under #619).
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
# timeouts plus per-host sharding under issue #619.
if grep -q -F -e 'issue #619' "$test_matrix" &&
  grep -q -F -e '--flaky_test_attempts=3' "$test_matrix" &&
  grep -q -F -e '--test_timeout=300' "$test_matrix" &&
  grep -q -F -e 'timeout-minutes' "$test_matrix" &&
  grep -q -F -e 'Long timeouts only' "$test_matrix"; then
  ok
else
  bad "docs/testing/github-ci.md lost the issue #619 flakiness plus timeout plus sharding record"
fi

# Testing README keeps the tuned-timeout battery pointer under #619.
if grep -q -F -e 'issue #619' "$testing_readme" &&
  grep -q -F -e 'seed test/coverage 45' "$testing_readme"; then
  ok
else
  bad "docs/testing/README.md lost its issue #619 tuned-timeout record"
fi

# Verification matrix owns the qualified seed-only record under #619.
if grep -q -F -e 'flakiness_qualification' "$verify" &&
  grep -q -F -e 'qualified seed-only under #619' "$verify" &&
  grep -q -F -e 'bazel run //tools/ci:flakiness_qualification' "$verify" &&
  grep -q -F -e '`flakiness_qualification` 16/16' "$verify"; then
  ok
else
  bad "verification-matrix lost its #619 flakiness qualified record"
fi

# BUILD owns the harness target plus CI wires it in prove plus dogfood-freshness.
if grep -q -F -e 'name = "flakiness_qualification"' "$build" &&
  grep -q -F -e 'bazel run --noshow_progress //tools/ci:flakiness_qualification' "$ci"; then
  ok
else
  bad "tools/ci/BUILD.bazel or ci.yml lost the flakiness_qualification wiring (want target plus prove plus dogfood-freshness)"
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
  bad "test matrix lost its CI-only plus no-Supported honesty for issue #619"
fi

dx_test_summary "ci flakiness plus timeout tuning harness"
