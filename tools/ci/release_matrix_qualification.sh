#!/usr/bin/env bash
# Release-matrix follow-up qualification harness for issue #815.
#
# Owns the flip of the three follow-up matrix cells from unqualified to
# qualified with per-host evidence, without claiming Supported and without
# weakening the draft-only ceiling plus owner-approval gate:
# - matrix shape: four cells frozen in `deploy/release/matrix.bzl`, seed
#   `dx-linux-x86_64` qualified-seed-built-here, Linux arm64 plus macOS
#   arm64 plus Windows x86_64 qualified-host-evidence (macOS x86_64 Not
#   planned per #976 with no cell), no `unqualified-per-issue-311` remains;
#   `release_matrix_unqualified()` returns empty via the `qualified-` prefix;
# - per-host evidence: Platform-qualified per the support matrix
#   (Linux arm64 #410, static-musl profiles #411, macOS arm64 #412,
#   Windows x86_64 #414) plus per-host `sbom-provenance` release evidence
#   (linux_arm64 #803, musl profiles #804, macos_arm64 #805,
#   windows_x86_64 #807, process #808);
# - workflow parity: `.github/workflows/publish-dry-run.yml` carries the
#   same four-cell qualified matrix, stays seed-only build with non-seed
#   cells human-run only, keeps `published: False` plus `submitted: False`;
# - docs parity: `docs/deploy/authoring.md` plus
#   `docs/deploy/release-runbook.md` plus `cli/cli/BUILD.bazel` record the
#   qualified matrix with per-host evidence pointers;
# - ceiling unchanged: draft-only (`draft` defaults True, no
#   `draft = False`, `--draft --verify-tag`, `v0.0.0-dryrun` placeholder),
#   owner approval (`approve` default false, `RELEASE_APPROVE=1` plus
#   pre-pushed tag, no tag creation), module at `0.0.0` with no `v*` tags,
#   no Supported claim.
# Qualified with per-host evidence; no Supported claim; remaining tag cut
# stays owned gap under process #808.
#
# Versioned here, run by CI via `bazel run //tools/ci:release_matrix_qualification`,
# following //tools/ci:release_packaging_qualification.
set -euo pipefail

# Shared workspace + runfiles helpers.
# Bootstrap via tools/sh/bootstrap.sh (issue #654): runfiles forest first, then source tree.
source "${RUNFILES_DIR:-/dev/null}/_main/tools/sh/bootstrap.sh" 2>/dev/null || source "${TEST_SRCDIR:-/dev/null}/_main/tools/sh/bootstrap.sh" 2>/dev/null || source "$0.runfiles/_main/tools/sh/bootstrap.sh" 2>/dev/null || source "${BASH_SOURCE[0]}.runfiles/_main/tools/sh/bootstrap.sh" 2>/dev/null || source "$(git rev-parse --show-toplevel 2>/dev/null)/tools/sh/bootstrap.sh"
dx_bootstrap "tools/sh/lib.sh"

dx_cd_workspace

dx_test_init

matrix="deploy/release/matrix.bzl"
tests="deploy/release/matrix_tests.bzl"
ci=".github/workflows/ci.yml"
dryrun=".github/workflows/publish-dry-run.yml"
authoring="docs/deploy/authoring.md"
runbook="docs/deploy/release-runbook.md"
support="docs/product/support-matrix.md"
cli_build="cli/cli/BUILD.bazel"
pins="tools/ci/tests/fixtures/release_matrix/pins.bzl"
expected="tools/ci/tests/fixtures/release_matrix/release_matrix.expected"
fixture_build="tools/ci/tests/fixtures/release_matrix/BUILD.bazel"
targets_b="tools/ci/ci_targets_b.bzl"
freshness="tools/ci/dogfood_freshness.sh"

# Matrix keeps the frozen four-cell order with the seed first.
if grep -q -F -e 'dx-linux-x86_64' "$matrix" &&
  grep -q -F -e 'dx-linux-arm64' "$matrix" &&
  grep -q -F -e 'dx-macos-arm64' "$matrix" &&
  grep -q -F -e 'dx-windows-x86_64' "$matrix" &&
  ! grep -q -F -e 'dx-macos-x86_64' "$matrix"; then
  ok
else
  bad "matrix.bzl lost its frozen four-cell order (want dx-linux-x86_64 plus dx-linux-arm64 plus dx-macos-arm64 plus dx-windows-x86_64 with no dx-macos-x86_64, #815 plus #976)"
fi

# Seed cell stays qualified-built-here.
if grep -q -F -e 'qualified-seed-built-here' "$matrix"; then
  ok
else
  bad "matrix.bzl lost its seed qualified-seed-built-here cell (#815)"
fi

# All three follow-ups are qualified with per-host evidence; no unqualified remains.
if grep -q -F -e 'qualified-host-evidence' "$matrix" &&
  ! grep -q -F -e 'unqualified-per-issue-311' "$matrix" &&
  ! grep -q -F -e 'dx-macos-x86_64' "$matrix"; then
  ok
else
  bad "matrix.bzl lost its three qualified-host-evidence cells or still carries unqualified-per-issue-311 or dx-macos-x86_64 (#815 plus #976)"
fi

# Unqualified helper keys on the qualified- prefix, so the flipped matrix reports empty.
if grep -q -F -e 'startswith("qualified-")' "$matrix"; then
  ok
else
  bad "matrix.bzl lost its qualified-prefix unqualified helper (want startswith qualified-, #815)"
fi

# Unit tests pin the flipped statuses plus the empty unqualified list.
if grep -q -F -e 'qualified-host-evidence' "$tests" &&
  ! grep -q -F -e 'unqualified-per-issue-311' "$tests"; then
  ok
else
  bad "matrix_tests.bzl lost its qualified-host-evidence pins or still carries unqualified-per-issue-311 (#815)"
fi

# Workflow matrix mirrors matrix.bzl: four qualified cells, none unqualified, no macOS x86_64.
if grep -q -F -e '"status": "qualified-host-evidence"' "$dryrun" &&
  ! grep -q -F -e 'unqualified-per-issue-311' "$dryrun" &&
  ! grep -q -F -e 'dx-macos-x86_64' "$dryrun"; then
  ok
else
  bad "publish-dry-run.yml lost its qualified-host-evidence matrix or still carries unqualified-per-issue-311 or dx-macos-x86_64 (#815 plus #976)"
fi

# Workflow stays seed-only build with non-seed cells human-run only, publishing nothing.
if grep -q -F -e 'human-run only, not built' "$dryrun" &&
  grep -q -F -e '"published": False' "$dryrun" &&
  grep -q -F -e '"submitted": False' "$dryrun"; then
  ok
else
  bad "publish-dry-run.yml lost its seed-only plus human-run-only plus never-publishes record (#815)"
fi

# Authoring records the qualified matrix with per-host evidence pointers.
if grep -q -F -e 'qualified with per-host evidence' "$authoring" &&
  grep -q -F -e 'under issue #815' "$authoring" &&
  grep -q -F -e 'Draft-only ceiling enforced' "$authoring"; then
  ok
else
  bad "authoring.md lost its qualified-matrix plus #815 plus draft-only-ceiling record"
fi

# Runbook records the qualified matrix with per-host evidence and no Supported claim.
if grep -q -F -e 'qualified with per-host evidence under issue' "$runbook" &&
  grep -q -F -e 'No cell is `Supported` yet' "$runbook"; then
  ok
else
  bad "release-runbook.md lost its qualified-matrix plus #815 plus no-Supported record"
fi

# CLI standalone comment records the wider matrix as qualified.
if grep -q -F -e 'is qualified with per-host evidence' "$cli_build"; then
  ok
else
  bad "cli/cli/BUILD.bazel lost its wider-matrix qualified record (#815)"
fi

# Draft-only ceiling unchanged: no draft opt-out, draft-only flags, placeholder tag.
if ! grep -rn -F -e 'draft = False' --include='BUILD.bazel' . | head -n 5 | grep -q . &&
  grep -q -F -e '"--draft", "--verify-tag"' deploy/rules/github_deploy.py &&
  grep -q -F -e 'v0.0.0-dryrun' "$dryrun"; then
  ok
else
  bad "draft-only ceiling drifted (want no draft=False plus --draft --verify-tag plus v0.0.0-dryrun, #815)"
fi

# Owner approval unchanged: default-closed approve gate, module at 0.0.0, no version tags.
if grep -q -F -e 'default: false' "$dryrun" &&
  grep -q -F -e 'version = "0.0.0"' MODULE.bazel &&
  [[ -z "$(git tag --list 'v*' || true)" ]]; then
  ok
else
  bad "owner-approval ceiling drifted (want default-closed approve plus 0.0.0 plus no v* tags, #815)"
fi

# Existing matrix guards track the qualified shape.
if grep -q -F -e 'qualified-host-evidence' tools/ci/signing_distribution_qualification.sh &&
  grep -q -F -e 'qualified-host-evidence' tools/ci/publish_trust.sh &&
  grep -q -F -e 'qualified-host-evidence' tools/ci/release_hygiene.sh &&
  grep -q -F -e 'qualified-host-evidence' tools/ci/distribution_closeout_guards.sh; then
  ok
else
  bad "existing harnesses lost their qualified-host-evidence matrix pins (signing_distribution plus publish_trust plus release_hygiene plus closeout, #815)"
fi

# BUILD owns the harness target plus dogfood-freshness wires it.
if grep -q -F -e 'name = "release_matrix_qualification"' "$targets_b" &&
  grep -q -F -e 'release_matrix_qualification.sh' "$targets_b" &&
  grep -q -F -e 'bazel run --noshow_progress //tools/ci:release_matrix_qualification' "$freshness"; then
  ok
else
  bad "tools/ci/ci_targets_b.bzl or dogfood_freshness.sh lost the release_matrix_qualification wiring (want target plus freshness)"
fi

# Fixture files stay present with corpus coverage.
if [[ -f "$pins" && -f "$expected" && -f "$fixture_build" ]] &&
  grep -q -F -e 'pins.bzl' "$fixture_build" &&
  grep -q -F -e 'release_matrix.expected' "$fixture_build" &&
  grep -q -F -e 'corpus_starlark' "$fixture_build"; then
  ok
else
  bad "release-matrix fixture missing (want pins.bzl plus release_matrix.expected plus corpus BUILD)"
fi

# Pins record matrix plus per-host evidence plus ceiling plus honesty.
if grep -q -F -e 'qualified-seed-built-here' "$pins" &&
  grep -q -F -e 'qualified-host-evidence' "$pins" &&
  grep -q -F -e 'Platform-qualified' "$pins" &&
  grep -q -F -e 'sbom-provenance' "$pins" &&
  grep -q -F -e 'Draft-only ceiling enforced' "$pins" &&
  grep -q -F -e 'Compatibility: Release only' "$pins" &&
  grep -q -F -e 'qualified under issue #815' "$pins" &&
  grep -q -F -e 'no Supported claim' "$pins"; then
  ok
else
  bad "pins.bzl lost its matrix plus per-host-evidence plus ceiling plus honesty pins under issue #815"
fi

# Expected fixture pins the qualified matrix plus per-host evidence plus ceiling lines.
if grep -q -F -e 'Release matrix follow-ups qualified (issue #815)' "$expected" &&
  grep -q -F -e 'qualified-host-evidence' "$expected" &&
  grep -q -F -e 'sbom-provenance' "$expected" &&
  grep -q -F -e 'Draft-only ceiling enforced' "$expected" &&
  grep -q -F -e 'Compatibility: Release only' "$expected" &&
  grep -q -F -e 'Qualified under issue #815' "$expected" &&
  grep -q -F -e 'no Supported claim' "$expected"; then
  ok
else
  bad "release_matrix.expected lost its qualified-matrix plus per-host-evidence plus ceiling lines under #815"
fi

dx_test_summary "release matrix follow-ups harness"
