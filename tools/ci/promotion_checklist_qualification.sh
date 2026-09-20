#!/usr/bin/env bash
# Promotion-checklist qualification harness.
#
# Owns tag hygiene, versioning, and what evidence promotes a cell to
# `Supported`, with fixture evidence pinned in
# `tools/ci/tests/fixtures/promotion_checklist/pins.bzl` (plus
# `promotion_checklist.expected`), without claiming unqualified support:
# - delivered as-built: module at 0.0.0 with no v* tags, /dist/ plus
#   /release/ ignored and uncommitted, caller SHA pins in sync;
# - versioning: SemVer bump in a reviewed PR, tag pre-pushed with approval,
#   --verify-tag never creates tags, single-version dx == module, callers on
#   reviewed commits, never rebuilt with byte-identity fail-closed;
# - platform evidence per cell under //// with floors
# , routes, per-cell coverage, clean refusal;
# - consumer evidence: nine checks plus explicit platforms plus all-enabled
# self-call verbatim //... via, sole updater native-only;
# - release evidence: sbom_demo plus signing-first signing_demo plus
#   checked-not-submitted bcr_demo plus separate GHCR plus human-run
#   release.sh plus verifier refusal plus dry-run-first, pinned by
#   release_hygiene plus release_policy plus publish_trust plus
#   distribution_closeout_guards plus signing_distribution_qualification;
# - promotion only during release qualification via supported_evidence_gate;
#   ad-hoc release without this checklist rejected; Release only.
# Seed only for platform plus consumer plus release evidence; no
# Supported claim.
#
# Versioned here, run by CI via `bazel run //tools/ci:promotion_checklist_qualification`,
# following //tools/ci:consumer_ci_qualification.
set -euo pipefail

# Shared workspace + runfiles helpers.
# Bootstrap via tools/sh/bootstrap.sh (issue #654): runfiles forest first, then source tree.
source "${RUNFILES_DIR:-/dev/null}/_main/tools/sh/bootstrap.sh" 2>/dev/null || source "${TEST_SRCDIR:-/dev/null}/_main/tools/sh/bootstrap.sh" 2>/dev/null || source "$0.runfiles/_main/tools/sh/bootstrap.sh" 2>/dev/null || source "${BASH_SOURCE[0]}.runfiles/_main/tools/sh/bootstrap.sh" 2>/dev/null || source "$(git rev-parse --show-toplevel 2>/dev/null)/tools/sh/bootstrap.sh"
dx_bootstrap "tools/sh/lib.sh"

dx_cd_workspace

dx_test_init

checklist="docs/product/promotion-checklist.md"
pins="tools/ci/tests/fixtures/promotion_checklist/pins.bzl"
expected="tools/ci/tests/fixtures/promotion_checklist/promotion_checklist.expected"
fixture_build="tools/ci/tests/fixtures/promotion_checklist/BUILD.bazel"
build="tools/ci/BUILD.bazel"
ci=".github/workflows/ci.yml"
support="docs/product/support-matrix.md"
verify="docs/testing/verification-matrix.md"

# Checklist doc owns tag hygiene, versioning, and promotion evidence under.
if [[ -f "$checklist" ]] &&
  grep -q -F -e 'owned under issue #611' "$checklist" &&
  grep -q -F -e 'Tag hygiene, versioning, and what' "$checklist" &&
  grep -q -F -e 'evidence promotes a cell' "$checklist" &&
  grep -q -F -e 'tools/ci/tests/fixtures/promotion_checklist/pins.bzl' "$checklist" &&
  grep -q -F -e 'bazel run //tools/ci:promotion_checklist_qualification' "$checklist"; then
  ok
else
  bad "promotion-checklist.md lost its #611 tag plus versioning plus evidence ownership with fixture proof"
fi

# Checklist records the lifecycle plus no-Supported honesty.
if grep -q -F -e 'Planned` → `Seed-host-delivered` → `Platform-qualified` → `Supported`' "$checklist" &&
  grep -q -F -e 'No cell is currently `Supported`' "$checklist" &&
  grep -q -F -e 'support-matrix.md' "$checklist" &&
  grep -q -F -e 'verification matrix' "$checklist"; then
  ok
else
  bad "promotion-checklist.md lost its lifecycle plus no-Supported honesty"
fi

# Checklist records tag hygiene as-built.
if grep -q -F -e 'version = "0.0.0"' "$checklist" &&
  grep -q -F -e 'No `v*` tags without explicit owner approval' "$checklist" &&
  grep -q -F -e '/dist/` plus `/release/` stay git-ignored' "$checklist" &&
  grep -q -F -e 'reusable-consumer.yml@<sha>' "$checklist" &&
  grep -q -F -e 'bazel run //tools/ci:release_hygiene' "$checklist"; then
  ok
else
  bad "promotion-checklist.md lost its tag-hygiene as-built pins"
fi

# Checklist records versioning.
if grep -q -F -e 'from `0.0.0` to SemVer in a reviewed PR' "$checklist" &&
  grep -q -F -e 'pushed to the remote beforehand with owner approval' "$checklist" &&
  grep -q -F -e '--verify-tag` everywhere' "$checklist" &&
  grep -q -F -e 'never creates or pushes tags itself' "$checklist" &&
  grep -q -F -e 'Single-version `dx` == module pin' "$checklist" &&
  grep -q -F -e 'never rebuilt or substituted silently' "$checklist"; then
  ok
else
  bad "promotion-checklist.md lost its versioning pins"
fi

# Checklist records platform evidence per cell.
if grep -q -F -e 'Linux arm64 native issue #410' "$checklist" &&
  grep -q -F -e 'static-musl profiles' "$checklist" &&
  grep -q -F -e 'issue #413' "$checklist" &&
  grep -q -F -e 'bazel run //tools/ci:deployment_floors_qualification' "$checklist" &&
  grep -q -F -e 'bazel run //tools/ci:cross_routes_qualification' "$checklist" &&
  grep -q -F -e 'bazel run //tools/ci:coverage_qualification' "$checklist" &&
  grep -q -F -e 'unsupported_platform' "$checklist"; then
  ok
else
  bad "promotion-checklist.md lost its platform-evidence pins"
fi

# Checklist records consumer evidence per cell.
if grep -q -F -e 'Nine checks through the reusable workflow' "$checklist" &&
  grep -q -F -e 'All-enabled self-call in `ci.yml` (issue #408, verbatim `//...`)' "$checklist" &&
  grep -q -F -e 'build-only self-call forever' "$checklist" &&
  grep -q -F -e 'bazel run //tools/ci:consumer_ci_qualification' "$checklist" &&
  grep -q -F -e 'sole updater' "$checklist"; then
  ok
else
  bad "promotion-checklist.md lost its consumer-evidence pins"
fi

# Checklist records release evidence per cell.
if grep -q -F -e '//deploy/release:sbom_demo' "$checklist" &&
  grep -q -F -e 'subject digest equal to artifact sha256' "$checklist" &&
  grep -q -F -e '//deploy/release:signing_demo' "$checklist" &&
  grep -q -F -e 'nothing is drafted or published unsigned' "$checklist" &&
  grep -q -F -e '//deploy/release:bcr_demo' "$checklist" &&
  grep -q -F -e 'GHCR stays the separate' "$checklist" &&
  grep -q -F -e 'deploy/release/release.sh' "$checklist" &&
  grep -q -F -e 'refuses checksum-only' "$checklist" &&
  grep -q -F -e 'bazel run //tools/ci:signing_distribution_qualification' "$checklist"; then
  ok
else
  bad "promotion-checklist.md lost its release-evidence pins"
fi

# Checklist records the promotion rule plus rejected ad-hoc plus honesty.
if grep -q -F -e 'Supported` promotion occurs only during release' "$checklist" &&
  grep -q -F -e 'A missing cell remains an explicit gap and blocks that support' "$checklist" &&
  grep -q -F -e 'bazel run //tools/ci:supported_evidence_gate' "$checklist" &&
  grep -q -F -e 'Ad-hoc release without this checklist is rejected' "$checklist" &&
  grep -q -F -e 'Compatibility: Release only' "$checklist" &&
  grep -q -F -e 'platform plus consumer plus release' "$checklist" &&
  grep -q -F -e 'no Supported claim' "$checklist"; then
  ok
else
  bad "promotion-checklist.md lost its promotion rule plus rejected plus honesty"
fi

# Fixture files stay present with corpus coverage.
if [[ -f "$pins" && -f "$expected" && -f "$fixture_build" ]] &&
  grep -q -F -e 'pins.bzl' "$fixture_build" &&
  grep -q -F -e 'promotion_checklist.expected' "$fixture_build" &&
  grep -q -F -e 'corpus_starlark' "$fixture_build"; then
  ok
else
  bad "promotion-checklist fixture missing (want pins.bzl plus promotion_checklist.expected plus corpus BUILD)"
fi

# Pins record tag plus version plus rejected decisions.
if grep -q -F -e 'version = "0.0.0"' "$pins" &&
  grep -q -F -e 'no v* tags without explicit owner approval' "$pins" &&
  grep -q -F -e '/dist/ plus /release/ stay git-ignored' "$pins" &&
  grep -q -F -e 'from 0.0.0 to SemVer in a reviewed PR' "$pins" &&
  grep -q -F -e '--verify-tag everywhere, never creates or pushes tags' "$pins" &&
  grep -q -F -e 'Single-version dx == module pin' "$pins" &&
  grep -q -F -e 'never rebuilt or substituted silently' "$pins" &&
  grep -q -F -e 'Ad-hoc release without this checklist is rejected' "$pins" &&
  grep -q -F -e 'Compatibility: Release only' "$pins"; then
  ok
else
  bad "pins.bzl lost its tag plus version plus rejected pins under issue #611"
fi

# Pins record platform plus consumer plus release evidence plus honesty.
if grep -q -F -e 'Linux arm64 native issue #410' "$pins" &&
  grep -q -F -e 'bazel run //tools/ci:deployment_floors_qualification' "$pins" &&
  grep -q -F -e 'bazel run //tools/ci:cross_routes_qualification' "$pins" &&
  grep -q -F -e 'bazel run //tools/ci:coverage_qualification' "$pins" &&
  grep -q -F -e 'Nine checks through the reusable workflow' "$pins" &&
  grep -q -F -e 'All-enabled self-call in ci.yml (issue #408, verbatim //...)' "$pins" &&
  grep -q -F -e '//deploy/release:sbom_demo' "$pins" &&
  grep -q -F -e '//deploy/release:signing_demo' "$pins" &&
  grep -q -F -e 'bazel run //tools/ci:supported_evidence_gate' "$pins" &&
  grep -q -F -e 'platform plus consumer plus release evidence stays owned gap' "$pins" &&
  grep -q -F -e 'qualified seed-only under issue #611' "$pins" &&
  grep -q -F -e 'no Supported claim' "$pins"; then
  ok
else
  bad "pins.bzl lost its platform plus consumer plus release plus honesty pins under issue #611"
fi

# Expected fixture pins the checklist plus rejected plus honesty lines.
if grep -q -F -e 'Promotion checklist to Supported (issue #611)' "$expected" &&
  grep -q -F -e 'version 0.0.0 with no v* tags' "$expected" &&
  grep -q -F -e 'from 0.0.0 to SemVer' "$expected" &&
  grep -q -F -e '--verify-tag everywhere' "$expected" &&
  grep -q -F -e 'deployment_floors_qualification' "$expected" &&
  grep -q -F -e 'unsupported_platform refusal' "$expected" &&
  grep -q -F -e 'all-enabled self-call in ci.yml' "$expected" &&
  grep -q -F -e 'build-only self-call forever rejected' "$expected" &&
  grep -q -F -e 'nothing' "$expected" &&
  grep -q -F -e 'drafted unsigned' "$expected" &&
  grep -q -F -e 'Ad-hoc release without this checklist is' "$expected" &&
  grep -q -F -e 'Compatibility: Release only' "$expected" &&
  grep -q -F -e 'blocks that support claim via supported_evidence_gate' "$expected" &&
  grep -q -F -e 'Qualified seed-only' "$expected" &&
  grep -q -F -e 'no Supported claim' "$expected"; then
  ok
else
  bad "promotion_checklist.expected lost its checklist plus rejected plus honesty lines under #611"
fi

# BUILD owns the harness target plus CI wires it in dogfood-freshness.
if grep -q -F -e 'name = "promotion_checklist_qualification"' "$build" &&
  grep -q -F -e 'promotion_checklist_qualification.sh' "$build" &&
  grep -q -F -e 'bazel run --noshow_progress //tools/ci:promotion_checklist_qualification' "$ci"; then
  ok
else
  bad "tools/ci/BUILD.bazel or ci.yml lost the promotion_checklist_qualification wiring (want target plus dogfood-freshness)"
fi

# Support matrix links the checklist as the promotion owner.
if grep -q -F -e 'promotion-checklist.md' "$support" &&
  grep -q -F -e 'issue #611' "$support"; then
  ok
else
  bad "support-matrix.md lost its promotion-checklist owner link under #611"
fi

# Verification matrix owns the harness entry as seed-only fixture evidence.
if grep -q -F -e ':promotion_checklist_qualification' "$verify" &&
  grep -q -F -e 'issue #611' "$verify" &&
  grep -q -F -e '`promotion_checklist_qualification` 16/16' "$verify"; then
  ok
else
  bad "verification-matrix.md lost its promotion_checklist_qualification entry with 16/16 under #611"
fi

# Live proof: the fixture package builds green on the seed host.
if bazel build //tools/ci/tests/fixtures/promotion_checklist/... --noshow_progress >/dev/null 2>&1; then
  ok
else
  bad "promotion-checklist fixture failed to build (want green on the seed host, issue #611)"
fi

dx_test_summary "promotion checklist qualification harness"
