#!/usr/bin/env bash
# Runner plus SDK rotation qualification harness.
#
# Owns the review cadence plus retirement handling left without an owner:
# - runners: ubuntu-latest plus ubuntu-24.04-arm in ci.yml plus macos-14
#   plus windows-latest only via the reusable consumer matrix, hygiene via
#   setup-bazel plus the shared BuildBuddy remote cache (no per-profile
#   cache scopes); macos x86_64 is Not
#   planned per #976 with no runner (macos-13 retired December 2025,
#   macos-15-intel sunset history stays docs-only, ubuntu-latest plus
#   windows-latest float and age out);
# - cadence: quarterly review plus on retirement notice plus on
#   hermetic-llvm release, sole maintainer owns every row until delegation;
# - SDK scope: glibc 2.28 plus MacOSX26.5 via
#   hermetic-llvm v0.8.19 plus MSVC/redist/SDK identities (exact values
#   owned by issues #410-#414 plus #500, not pinned here); retirement
#   handling updates ci.yml plus docs plus pins in one reviewed PR;
# - qualification: locally/on-demand with customer flows only (bazel
#   build plus bazel test plus dx coverage seed gate plus on-demand
#   per-host, no new non-customer CI jobs);
# - fixtures: `tools/ci/tests/fixtures/runner_rotation/` (`pins.bzl`
#   plus `runner_rotation.expected`) pins runners plus retirement plus
#   cadence plus SDK scope plus rejected plus honesty;
# - scope: infra only. Seed only: no Supported claim.
#
# Versioned here, run on demand via `bazel run //tools/ci:runner_rotation_qualification`,
# following //tools/ci:ci_matrix_qualification.
set -euo pipefail

# Shared workspace + runfiles helpers.
# Bootstrap via tools/sh/bootstrap.sh (issue #654): runfiles forest first, then source tree.
source "${RUNFILES_DIR:-/dev/null}/_main/tools/sh/bootstrap.sh" 2>/dev/null || source "${TEST_SRCDIR:-/dev/null}/_main/tools/sh/bootstrap.sh" 2>/dev/null || source "$0.runfiles/_main/tools/sh/bootstrap.sh" 2>/dev/null || source "${BASH_SOURCE[0]}.runfiles/_main/tools/sh/bootstrap.sh" 2>/dev/null || source "$(git rev-parse --show-toplevel 2>/dev/null)/tools/sh/bootstrap.sh"
dx_bootstrap "tools/sh/lib.sh"

dx_cd_workspace

dx_test_init

dx_bash_pin

pins="tools/ci/tests/fixtures/runner_rotation/pins.bzl"
expected="tools/ci/tests/fixtures/runner_rotation/runner_rotation.expected"
fixture_build="tools/ci/tests/fixtures/runner_rotation/BUILD.bazel"
contract="docs/github-ci.md"
native="docs/native-toolchains.md"
test_matrix="docs/testing/github-ci.md"
build="tools/ci/BUILD.bazel"
ci=".github/workflows/ci.yml"
consumer=".github/workflows/reusable-consumer.yml"
floors="cc/tests/fixtures/deployment_floors/pins.bzl"

# Fixture files stay present.
if [[ -f "$pins" && -f "$expected" && -f "$fixture_build" ]]; then
  ok
else
  bad "runner rotation fixture missing (want $pins plus runner_rotation.expected plus BUILD.bazel)"
fi

# Pins record the qualified runner set (floating labels pinned here).
if grep -q -F -e 'RUNNER_SEED = "ubuntu-latest"' "$pins" &&
  grep -q -F -e 'RUNNER_ARM64 = "ubuntu-24.04-arm"' "$pins" &&
  grep -q -F -e 'RUNNER_MACOS_ARM64 = "macos-14"' "$pins" &&
  ! grep -q -F -e 'RUNNER_MACOS_X86_64' "$pins" &&
  grep -q -F -e 'RUNNER_WINDOWS = "windows-latest"' "$pins"; then
  ok
else
  bad "pins.bzl lost its qualified runner set (ubuntu-latest plus ubuntu-24.04-arm plus macos-14 plus windows-latest, no x86_64 per #976)"
fi

# Pins record retirement handling (never silent).
if grep -q -F -e 'macos-13 retired December 2025' "$pins" &&
  grep -q -F -e 'macos x86_64 Not planned' "$pins" &&
  grep -q -F -e 'float and age out' "$pins"; then
  ok
else
  bad "pins.bzl lost its retirement record (macos-13 retired plus x86_64 Not-planned plus floating age-out, issue #642 plus #976)"
fi

# Pins record review cadence plus owner.
if grep -q -F -e 'quarterly review plus on retirement notice plus on hermetic-llvm release' "$pins" &&
  grep -q -F -e 'sole maintainer owns every row until delegation' "$pins"; then
  ok
else
  bad "pins.bzl lost its quarterly cadence plus sole-maintainer owner under issue #642"
fi

# Pins record the SDK plus floor review scope (exact values owned elsewhere).
if grep -q -F -e 'glibc 2.28 symbol floor' "$pins" &&
  grep -q -F -e 'MacOSX26.5 via hermetic-llvm v0.8.19' "$pins" &&
  grep -q -F -e 'MSVC 14.50.35717' "$pins" &&
  ! grep -q -F -e 'musl' "$pins"; then
  ok
else
  bad "pins.bzl lost its SDK/floor review scope (glibc plus MacOSX26.5 plus MSVC, issue #642)"
fi

# Pins record customer-flows-only qualification (no new CI job).
if grep -q -F -e 'bazel build //...' "$pins" &&
  grep -q -F -e 'bazel test //...' "$pins" &&
  grep -q -F -e 'dx coverage --min-coverage 97 //...' "$pins" &&
  grep -q -F -e 'qualified locally/on-demand with customer flows only' "$pins"; then
  ok
else
  bad "pins.bzl lost its customer-flows-only qualification (build plus test plus coverage, issue #642)"
fi

# Pins record the rejected permanent rotation job.
if grep -q -F -e 'permanent rotation job in CI rejected' "$pins" &&
  grep -q -F -e 'keep CI customer-only' "$pins" &&
  grep -q -F -e 'no new non-customer CI jobs' "$pins"; then
  ok
else
  bad "pins.bzl lost its permanent-rotation-job rejection plus customer-only plus no-new-job pins under #642"
fi

# Pins record honesty (seed-only, infra only, no Supported).
if grep -q -F -e 'qualified seed-only under issue #642' "$pins" &&
  grep -q -F -e 'no Supported claim' "$pins" &&
  grep -q -F -e 'infra only' "$pins"; then
  ok
else
  bad "pins.bzl lost its seed-only plus infra-only plus no-Supported honesty under #642"
fi

# Expected fixture pins runners plus retirement plus cadence plus SDK plus flows plus rejected.
if grep -q -F -e 'ubuntu-latest plus ubuntu-24.04-arm plus macos-14' "$expected" &&
  grep -q -F -e 'macos-13 retired December 2025' "$expected" &&
  grep -q -F -e 'macos x86_64 Not planned' "$expected" &&
  grep -q -F -e 'quarterly plus on retirement notice plus on hermetic-llvm' "$expected" &&
  grep -q -F -e 'glibc 2.28 plus MacOSX26.5' "$expected" &&
  grep -q -F -e 'bazel build //...' "$expected" &&
  grep -q -F -e 'Permanent rotation job in CI rejected' "$expected" &&
  grep -q -F -e 'Qualified seed-only under issue #642' "$expected"; then
  ok
else
  bad "runner_rotation.expected lost its runners plus retirement plus cadence plus SDK plus flows plus rejected lines under #642 plus #976"
fi

# Fixture BUILD exports pins plus expected with corpus coverage.
if grep -q -F -e 'pins.bzl' "$fixture_build" &&
  grep -q -F -e 'runner_rotation.expected' "$fixture_build" &&
  grep -q -F -e 'corpus_starlark' "$fixture_build"; then
  ok
else
  bad "runner_rotation BUILD.bazel lost its pins plus expected exports with corpus under #642"
fi

# Contract owns the rotation cadence plus retirement handling with fixture proof.
if grep -q -F -e 'Runner Plus SDK Rotation' "$contract" &&
  grep -q -F -e 'quarterly review plus on retirement notice plus on hermetic-llvm release' "$contract" &&
  grep -q -F -e 'macos-13 retired December 2025' "$contract" &&
  grep -q -F -e 'macos x86_64 Not planned' "$contract" &&
  grep -q -F -e 'Permanent rotation job in CI rejected' "$contract" &&
  grep -q -F -e 'tools/ci/tests/fixtures/runner_rotation/pins.bzl' "$contract" &&
  grep -q -F -e 'bazel run //tools/ci:runner_rotation_qualification' "$contract" &&
  grep -q -F -e 'issue #642' "$contract"; then
  ok
else
  bad "github-ci.md lost its runner-plus-SDK rotation cadence plus retirement plus rejected record with fixture proof under #642 plus #976"
fi

# Native plan links SDK/floor review to the rotation contract (no copy).
if grep -q -F -e 'Runner Plus SDK Rotation' "$native" &&
  grep -q -F -e 'github-ci.md#runner-plus-sdk-rotation' "$native" &&
  grep -q -F -e 'issue #642' "$native" &&
  grep -q -F -e 'cc/tests/fixtures/deployment_floors/pins.bzl' "$native"; then
  ok
else
  bad "docs/native-toolchains.md lost its SDK/floor rotation link to the contract under #642"
fi

# Test matrix records the on-demand customer-flows-only qualification.
if grep -q -F -e 'Runner Plus SDK Rotation' "$test_matrix" &&
  grep -q -F -e 'issue #642' "$test_matrix" &&
  grep -q -F -e 'bazel run //tools/ci:runner_rotation_qualification' "$test_matrix"; then
  ok
else
  bad "testing/github-ci.md lost its #642 rotation on-demand qualification record"
fi

# BUILD owns the harness target plus CI wires it records seed-only evidence.
if grep -q -F -e 'name = "runner_rotation_qualification"' "tools/ci/ci_targets_b.bzl" &&
  grep -q -F -e 'runner_rotation_qualification.sh' "tools/ci/ci_targets_b.bzl" &&
  grep -q -F -e 'bazel run --noshow_progress //tools/ci:runner_rotation_qualification' tools/ci/dogfood_freshness.sh; then
  ok
else
  bad "tools/ci/ci_targets_b.bzl or dogfood_freshness.sh lost the runner_rotation_qualification wiring (want target plus dogfood-freshness)"
fi

# As-built runners plus cache record: ci.yml keeps the seed Linux
# x86_64 runner on the shared BuildBuddy remote cache (per-host cache
# prefixes are gone), the Linux arm64, macOS plus Windows runners live
# only in the consumer matrix, no paid runner exists, and floors keep
# their SDK identities.
if grep -q -F -e 'runs-on: ubuntu-latest' "$ci" &&
  grep -q -F -e "'ubuntu-24.04-arm'" "$consumer" &&
  grep -q -F -e "'macos-14'" "$consumer" &&
  grep -q -F -e "'windows-latest'" "$consumer" &&
  ! grep -q -F -e 'runs-on: macos-14' "$ci" &&
  ! grep -q -F -e 'runs-on: windows-latest' "$ci" &&
  ! grep -q -F -e 'bazel-macos-arm64-' "$ci" &&
  ! grep -q -F -e 'bazel-windows-x86_64-' "$ci" &&
  grep -q -F -e 'BAZEL_CONFIG:' "$ci" &&
  grep -q -F -e 'BB_ARGS:' "$ci" &&
  ! grep -E -q 'runs-on:.*(self-hosted|larger|macos-latest)' "$ci" &&
  grep -q -F -e 'APPLE_SDK_IDENTITY = "MacOSX26.5"' "$floors" &&
  grep -q -F -e 'GLIBC_FLOOR = "2.28"' "$floors"; then
  ok
else
  bad "ci.yml lost its as-built runner plus cache record or floors lost SDK identities (want four runners plus no paid plus MacOSX26.5 plus glibc 2.28, issue #642 plus #976)"
fi

# Live proof: customer build flow only, no new CI job.
if bazel build //tools/ci/tests/fixtures/runner_rotation/... --noshow_progress >/dev/null 2>&1; then
  ok
else
  bad "runner-rotation fixture failed to build (want green via customer build flow, issue #642)"
fi

dx_test_summary "runner rotation qualification harness"
