#!/usr/bin/env bash
# Release evidence qualification harness for Linux static-musl profiles.
#
# Owns per-profile release evidence for linux_x86_64_musl plus
# linux_arm64_musl with fixture evidence pinned in
# `tools/ci/tests/fixtures/release_musl/pins.bzl` (plus
# `release_musl.expected`), without claiming unqualified support:
# - delivered as-built: SPDX-2.3 plus SLSA v1 via //deploy/release:sbom_demo
#   with subject digest equal to artifact sha256, hermetic Rust toolchain
#   only, //deploy/rules:release_demo_archive as subject fixture, verified
#   via bazel test //deploy/release:dx_release_tools_test;
# - CI surface: ci.yml carries no sbom-musl-x86_64 and no sbom-musl-arm64
#   jobs, no upload-artifact, and no per-host cache prefixes (musl
#   sbom-provenance stays an owner-gated local release record in
#   docs/deploy/release-runbook.md; CI never signs PR code and publishes
#   nothing, fork-safe); static native closure only, dynamic musl
#   explicitly out of scope;
# - promotion-checklist cells per profile: Platform-qualified static musl
#   (#411), dogfood consumer all nine checks plus musl jobs (#408), per-cell
#   musl coverage with no union, tag hygiene at 0.0.0 with no v* tags,
#   --verify-tag versioning, per-profile sbom-provenance (#804);
# - attestation stays owner-gated human-run via //deploy/release:signing_demo
#   (Sigstore keyless cosign sign-blob --bundle plus gh attestation create
#   on the TUF trust root); CI never signs PR code;
# - dry-run only forever rejected, checksum-only rejected; Release only.
# Platform-qualified, no Supported claim; remaining hosts plus tag cut stay
# owned gap under #805-#807 plus process #808.
#
# Versioned here, run by CI via `bazel run //tools/ci:release_musl_qualification`,
# following //tools/ci:release_arm64_qualification.
set -euo pipefail

# Shared workspace + runfiles helpers.
# Bootstrap via tools/sh/bootstrap.sh (issue #654): runfiles forest first, then source tree.
source "${RUNFILES_DIR:-/dev/null}/_main/tools/sh/bootstrap.sh" 2>/dev/null || source "${TEST_SRCDIR:-/dev/null}/_main/tools/sh/bootstrap.sh" 2>/dev/null || source "$0.runfiles/_main/tools/sh/bootstrap.sh" 2>/dev/null || source "${BASH_SOURCE[0]}.runfiles/_main/tools/sh/bootstrap.sh" 2>/dev/null || source "$(git rev-parse --show-toplevel 2>/dev/null)/tools/sh/bootstrap.sh"
dx_bootstrap "tools/sh/lib.sh"

dx_cd_workspace

dx_test_init

dx_bash_pin

sbom="deploy/release/sbom.bzl"
verify="deploy/release/src/lib.rs"
release_build="deploy/release/BUILD.bazel"
ci=".github/workflows/ci.yml"
runbook="docs/deploy/release-runbook.md"
support="docs/product/support-matrix.md"
platform_rs="cli/cli/src/platform.rs"
cells="tools/coverage/cells.txt"
pins="tools/ci/tests/fixtures/release_musl/pins.bzl"
expected="tools/ci/tests/fixtures/release_musl/release_musl.expected"
fixture_build="tools/ci/tests/fixtures/release_musl/BUILD.bazel"
build="tools/ci/BUILD.bazel"
targets_b="tools/ci/ci_targets_b.bzl"
freshness="tools/ci/dogfood_freshness.sh"

# SBOM wire profile stays pinned: SPDX-2.3 plus SLSA v1 via the hermetic Rust toolchain.
if grep -q -F -e 'SPDX-2.3' "$sbom" &&
  grep -q -F -e 'https://slsa.dev/provenance/v1' "$sbom" &&
  grep -q -F -e 'via Rust' "$sbom"; then
  ok
else
  bad "sbom.bzl lost its SPDX-2.3 plus SLSA-v1 plus hermetic-toolchain pins (#804)"
fi

# Provenance binds exact bytes: SPDX plus in-toto v1 plus SLSA subject digest.
if grep -q -F -e 'spdxVersion' "$verify" &&
  grep -q -F -e 'https://in-toto.io/Statement/v1' "$verify" &&
  grep -q -F -e 'https://slsa.dev/provenance/v1' "$verify"; then
  ok
else
  bad "Rust launch lost its SPDX plus in-toto plus SLSA subject-binding checks (#804)"
fi

# Release BUILD keeps the demo plus its portable Rust verifier over the seed fixture.
if grep -q -F -e 'name = "sbom_demo"' "$release_build" &&
  grep -q -F -e 'name = "dx_release_tools_test"' "$release_build" &&
  grep -q -F -e '//deploy/rules:release_demo_archive' "$release_build"; then
  ok
else
  bad "deploy/release/BUILD.bazel lost its sbom_demo plus dx_release_tools_test over release_demo_archive (#804)"
fi

# CI runs no sbom-musl-x86_64 build or verify: no release-tool targets in
# ci.yml (x86_64 musl SBOM stays an owner-gated local release record).
if ! grep -q -F -e 'sbom-musl-x86_64' "$ci" &&
  ! grep -q -F -e 'sbom_demo' "$ci"; then
  ok
else
  bad "ci.yml still carries an sbom-musl-x86_64 build plus verify job (want no sbom-musl-x86_64 job and no sbom_demo in CI; SBOM stays owner-gated local release, #804)"
fi

# CI runs no sbom-musl-arm64 build or verify: no release-tool test in ci.yml.
if ! grep -q -F -e 'sbom-musl-arm64' "$ci" &&
  ! grep -q -F -e 'dx_release_tools_test' "$ci"; then
  ok
else
  bad "ci.yml still carries an sbom-musl-arm64 build plus verify job (want no sbom-musl-arm64 job and no dx_release_tools_test in CI; SBOM stays owner-gated local release, #804)"
fi

# CI stages no x86_64 musl sbom directory and uploads no provenance artifact.
if ! grep -q -F -e 'RUNNER_TEMP/sbom-musl-x86_64' "$ci" &&
  ! grep -q -F -e 'sbom-provenance-linux_x86_64_musl' "$ci" &&
  ! grep -q -F -e 'actions/upload-artifact' "$ci"; then
  ok
else
  bad "ci.yml still stages or uploads sbom-provenance-linux_x86_64_musl (want no RUNNER_TEMP/sbom-musl-x86_64 stage and no upload-artifact; CI publishes nothing, #804)"
fi

# CI stages no arm64 musl sbom directory and uploads no provenance artifact.
if ! grep -q -F -e 'RUNNER_TEMP/sbom-musl-arm64' "$ci" &&
  ! grep -q -F -e 'sbom-provenance-linux_arm64_musl' "$ci" &&
  ! grep -q -F -e 'sbom-provenance' "$ci"; then
  ok
else
  bad "ci.yml still stages or uploads sbom-provenance-linux_arm64_musl (want no RUNNER_TEMP/sbom-musl-arm64 stage and no sbom-provenance in CI, #804)"
fi

# No upload-artifact pin remains; checkout stays least-privilege plus
# setup-bazel, and the runbook records publishes-nothing plus CI-never-signs.
if ! grep -q -F -e 'actions/upload-artifact' "$ci" &&
  ! grep -q -F -e 'if-no-files-found: error' "$ci" &&
  grep -q -F -e 'persist-credentials: false' "$ci" &&
  grep -q -F -e 'bazel-contrib/setup-bazel' "$ci" &&
  grep -q -F -e 'publishes nothing' "$runbook" &&
  grep -q -F -e 'CI never signs PR code' "$runbook"; then
  ok
else
  bad "ci.yml lost its no-upload plus least-privilege setup-bazel record or runbook lost publishes-nothing plus CI-never-signs (want no upload-artifact for the sbom-musl jobs, #804)"
fi

# No sbom jobs plus no per-host cache prefixes remain in ci.yml (caching is
# setup-bazel bazelisk cache plus BuildBuddy only).
if ! grep -q -F -e 'name: sbom' "$ci" &&
  ! grep -q -F -e 'prefix: bazel-' "$ci" &&
  ! grep -q -F -e 'sbom-provenance' "$ci"; then
  ok
else
  bad "ci.yml still carries sbom jobs or per-host cache prefixes (want none: all sbom jobs deleted, caching is setup-bazel plus BuildBuddy, #804)"
fi

# Attestation stays owner-gated human-run (fork-safe, no CI signing): the
# signing_demo target plus runbook own the record, ci.yml never signs.
if grep -q -F -e 'name = "signing_demo"' "$release_build" &&
  grep -q -F -e 'CI never signs PR code' "$runbook" &&
  ! grep -q -F -e 'signing_demo' "$ci" &&
  ! grep -q -F -e 'id-token' "$ci"; then
  ok
else
  bad "release BUILD or runbook lost the owner-gated signing_demo attestation record with ci.yml never signing (#804)"
fi

# Runbook records the musl CI uploads plus owner-gated attestation under #804.
if grep -q -F -e 'issue #804' "$runbook" &&
  grep -q -F -e 'sbom-musl-x86_64' "$runbook" &&
  grep -q -F -e 'sbom-musl-arm64' "$runbook" &&
  grep -q -F -e 'sbom-provenance-linux_x86_64_musl' "$runbook" &&
  grep -q -F -e 'sbom-provenance-linux_arm64_musl' "$runbook" &&
  grep -q -F -e 'owner-gated human-run' "$runbook" &&
  grep -q -F -e '//deploy/release:signing_demo' "$runbook"; then
  ok
else
  bad "release-runbook.md lost its #804 sbom-musl per-profile uploads plus owner-gated attestation record"
fi

# Per-cell consumer evidence: musl build jobs stay plus the dogfood self-call
# runs every check incl test on all four hosts (min coverage 97, no
# disabled_checks, #408).
if grep -q -F -e 'build-musl-x86_64' "$ci" &&
  grep -q -F -e 'build-musl-arm64' "$ci" &&
  grep -q -F -e "platforms: '[\"linux_x86_64\", \"linux_arm64\", \"macos_arm64\", \"windows_x86_64\"]'" "$ci" &&
  grep -q -F -e 'min_coverage: "97"' "$ci" &&
  ! grep -q -F -e 'disabled_checks' "$ci"; then
  ok
else
  bad "ci.yml lost its dogfood all-nine-checks plus musl-jobs per-cell evidence (want four-platform self-call plus min_coverage 97 plus no disabled_checks, #804)"
fi

# Per-cell coverage stays two musl cells with no union.
if grep -q -F -e 'qualified linux_x86_64_musl' "$cells" &&
  grep -q -F -e 'qualified linux_arm64_musl' "$cells" &&
  grep -q -F -e 'coverage-musl-x86_64 (dx coverage gate, musl x86_64 cell)' "$ci" &&
  grep -q -F -e 'coverage-musl-arm64 (dx coverage gate, musl arm64 cell)' "$ci"; then
  ok
else
  bad "coverage cells or ci.yml lost the two musl per-cell gates with no union (#804)"
fi

# Static native closure only; dynamic musl stays explicitly out of scope.
if grep -q -F -e '"linux_x86_64_static_musl"' "$platform_rs" &&
  grep -q -F -e '"linux_arm64_static_musl"' "$platform_rs" &&
  grep -q -F -e 'dynamic musl' "$platform_rs"; then
  ok
else
  bad "platform.rs lost the static-only closure plus dynamic-out-of-scope record (#804)"
fi

# BUILD owns the harness target plus dogfood-freshness wires it.
if grep -q -F -e 'name = "release_musl_qualification"' "$targets_b" &&
  grep -q -F -e 'release_musl_qualification.sh' "$targets_b" &&
  grep -q -F -e 'bazel run --noshow_progress //tools/ci:release_musl_qualification' "$freshness"; then
  ok
else
  bad "tools/ci/ci_targets_b.bzl or dogfood_freshness.sh lost the release_musl_qualification wiring (want target plus freshness)"
fi

# Fixture files stay present with corpus coverage.
if [[ -f "$pins" && -f "$expected" && -f "$fixture_build" ]] &&
  grep -q -F -e 'pins.bzl' "$fixture_build" &&
  grep -q -F -e 'release_musl.expected' "$fixture_build" &&
  grep -q -F -e 'corpus_starlark' "$fixture_build"; then
  ok
else
  bad "release-musl fixture missing (want pins.bzl plus release_musl.expected plus corpus BUILD)"
fi

# Pins record wire plus no-CI-upload plus cells plus attestation plus rejected plus honesty.
if grep -q -F -e 'SPDX-2.3 via //deploy/release:sbom_demo' "$pins" &&
  grep -q -F -e 'https://slsa.dev/provenance/v1 via //deploy/release:sbom_demo' "$pins" &&
  grep -q -F -e 'ci.yml carries no sbom-musl-x86_64 job; SBOM stays a local target under #804' "$pins" &&
  grep -q -F -e 'ci.yml carries no sbom-musl-arm64 job; SBOM stays a local target under #804' "$pins" &&
  grep -q -F -e 'no upload-artifact, no RUNNER_TEMP stage; CI publishes nothing under #804' "$pins" &&
  grep -q -F -e 'no per-host cache scope; disk cache deleted, BuildBuddy remote cache only' "$pins" &&
  grep -q -F -e 'Platform-qualified static musl under issue #411' "$pins" &&
  grep -q -F -e 'self-call dx test plus dx coverage plus musl jobs' "$pins" &&
  grep -q -F -e 'via //deploy/release:signing_demo' "$pins" &&
  grep -q -F -e 'Dry-run only forever is rejected' "$pins" &&
  grep -q -F -e 'Compatibility: Release only' "$pins" &&
  grep -q -F -e 'qualified under issue #804' "$pins" &&
  grep -q -F -e 'no Supported claim' "$pins"; then
  ok
else
  bad "pins.bzl lost its wire plus no-CI-upload plus cells plus attestation plus rejected plus honesty pins under issue #804"
fi

# Expected fixture pins the no-job plus no-upload plus cells plus rejected plus honesty lines.
if grep -q -F -e 'Release evidence for Linux static-musl profiles (issue #804)' "$expected" &&
  grep -q -F -e 'SPDX-2.3 via //deploy/release:sbom_demo' "$expected" &&
  grep -q -F -e 'ci.yml carries no sbom-musl-x86_64 job' "$expected" &&
  grep -q -F -e 'ci.yml carries no sbom-musl-arm64 job' "$expected" &&
  grep -q -F -e 'no upload-artifact, no RUNNER_TEMP' "$expected" &&
  grep -q -F -e 'no per-host cache scope' "$expected" &&
  grep -q -F -e 'Seed sbom job removed with the sbom job deletion' "$expected" &&
  grep -q -F -e 'arm64 sbom-arm64 job removed with the sbom job deletion' "$expected" &&
  grep -q -F -e 'sbom-provenance delivered under issue' "$expected" &&
  grep -q -F -e 'CI never signs PR' "$expected" &&
  grep -q -F -e 'Dry-run only forever is rejected' "$expected" &&
  grep -q -F -e 'Compatibility: Release only' "$expected" &&
  grep -q -F -e 'Qualified under issue #804' "$expected" &&
  grep -q -F -e 'no Supported claim' "$expected"; then
  ok
else
  bad "release_musl.expected lost its no-CI-upload plus cells plus rejected plus honesty lines under #804"
fi

# Live proof: the fixture package builds green on the seed host.
if bazel build //tools/ci/tests/fixtures/release_musl/... --noshow_progress >/dev/null 2>&1; then
  ok
else
  bad "release-musl fixture failed to build (want green on the seed host, issue #804)"
fi

dx_test_summary "release evidence musl harness"
