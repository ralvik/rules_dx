#!/usr/bin/env bash
# Release packaging qualification harness for artifact-into-releases.
#
# Qualifies packaging and publishing of release artifacts as part of the
# release flow with fixture evidence (issue #813):
# - packaged unit: `//deploy/release:release_artifacts` aggregates the
#   validated audit curator bytes (`//:audit_curator` over `licenses.toml`)
#   with the seed binary plus man page plus NOTICE bundle plus
#   SBOM/provenance pair, so one build proves the publishing set travels
#   together;
# - SBOM/provenance linkage: SPDX 2.3 plus SLSA v1 in-toto Statement v1
#   with subject digest equal to artifact sha256, hermetic Rust toolchain
#   only, verified by `dx_release_tools_test` plus the dry-run linkage
#   (provenance carries the SPDX subject digest);
# - NOTICE packaging: hermetic `notice_bundle` over the audited inventory
#   with deterministic bytes plus byte-identical rebuilds,
#   `missing-notice-text` fails, verified by `notice_verify_files` plus
#   `dx_verify --notice`, signed alongside the SBOM pair via
#   `//deploy/release:signing_demo`;
# - pipeline wiring: `publish-dry-run.yml` builds plus stages plus verifies
#   the unit (SPDX plus SLSA subject linkage, NOTICE header, signing log
#   covers the SBOM pair plus NOTICE) with `published: False` and a clean
#   checkout, never publishing;
# - runbook update: `docs/deploy/release-runbook.md#packaging` documents
#   the artifact-into-releases packaging with the curator plus SBOM plus
#   NOTICE plus signing linkage.
# Seed only: no Supported claim; tag cut stays owned gap
# under process #808 (wider release matrix qualified per-host under issue #815).
#
# Versioned here, run by CI via `bazel run //tools/ci:release_packaging_qualification`,
# following //tools/ci:release_windows_qualification.
set -euo pipefail

# Shared workspace + runfiles helpers.
# Bootstrap via tools/sh/bootstrap.sh (issue #654): runfiles forest first, then source tree.
source "${RUNFILES_DIR:-/dev/null}/_main/tools/sh/bootstrap.sh" 2>/dev/null || source "${TEST_SRCDIR:-/dev/null}/_main/tools/sh/bootstrap.sh" 2>/dev/null || source "$0.runfiles/_main/tools/sh/bootstrap.sh" 2>/dev/null || source "${BASH_SOURCE[0]}.runfiles/_main/tools/sh/bootstrap.sh" 2>/dev/null || source "$(git rev-parse --show-toplevel 2>/dev/null)/tools/sh/bootstrap.sh"
dx_bootstrap "tools/sh/lib.sh"

dx_cd_workspace

dx_test_init

release_build="deploy/release/BUILD.bazel"
dryrun=".github/workflows/publish-dry-run.yml"
driver="deploy/release/src/lib.rs"
runbook="docs/deploy/release-runbook.md"
pins="tools/ci/tests/fixtures/release_packaging/pins.bzl"
expected="tools/ci/tests/fixtures/release_packaging/release_packaging.expected"
fixture_build="tools/ci/tests/fixtures/release_packaging/BUILD.bazel"
targets_b="tools/ci/ci_targets_b.bzl"
freshness="tools/ci/dogfood_freshness.sh"

# Packaged unit aggregates curator plus binary plus man page plus NOTICE plus SBOM.
if grep -q -F -e 'name = "release_artifacts"' "$release_build" &&
  grep -q -F -e '"//:audit_curator"' "$release_build" &&
  grep -q -F -e '"//cli/cli:dx"' "$release_build" &&
  grep -q -F -e '"//cli/cli:man_pages"' "$release_build" &&
  grep -q -F -e '":notice_demo"' "$release_build" &&
  grep -q -F -e '":sbom_demo"' "$release_build"; then
  ok
else
  bad "deploy/release/BUILD.bazel lost its release_artifacts curator plus binary plus man plus NOTICE plus SBOM unit (#813)"
fi

# Signing still binds the SBOM pair plus the NOTICE bundle together.
if grep -q -F -e 'name = "signing_demo"' "$release_build" &&
  grep -q -F -e '":sbom_demo_spdx"' "$release_build" &&
  grep -q -F -e '":sbom_demo_provenance"' "$release_build" &&
  grep -q -F -e '":notice_demo_notice"' "$release_build"; then
  ok
else
  bad "deploy/release/BUILD.bazel lost its signing_demo SBOM pair plus NOTICE binding (#813)"
fi

# Dry-run builds the packaged unit on the seed host.
if grep -q -F -e '//deploy/release:release_artifacts' "$dryrun" &&
  grep -q -F -e 'bazel build //cli/cli:dx //cli/cli:dx_standalone //deploy/release:all //deploy/release:release_artifacts' "$dryrun"; then
  ok
else
  bad "publish-dry-run.yml lost its release_artifacts seed build (#813)"
fi

# Dry-run stages plus verifies the SBOM/provenance linkage.
if grep -q -F -e 'sbom_demo.spdx.json' "$dryrun" &&
  grep -q -F -e 'sbom_demo.provenance.json' "$dryrun" &&
  grep -q -F -e 'SPDX-2.3' "$dryrun" &&
  grep -q -F -e 'https://slsa.dev/provenance/v1' "$dryrun" &&
  grep -q -F -e 'sbom_digest' "$dryrun"; then
  ok
else
  bad "publish-dry-run.yml lost its SBOM SPDX plus SLSA subject-linkage staging (#813)"
fi

# Dry-run stages plus verifies the NOTICE bundle.
if grep -q -F -e 'notice_demo.NOTICE' "$dryrun" &&
  grep -q -F -e 'NOTICE for ' "$dryrun"; then
  ok
else
  bad "publish-dry-run.yml lost its NOTICE bundle staging plus header check (#813)"
fi

# Dry-run signing log covers the SBOM pair plus NOTICE packaging linkage.
if grep -q -F -e 'grep -q "sbom_demo.spdx.json" "$stage/signing-dry-run.log"' "$dryrun" &&
  grep -q -F -e 'grep -q "sbom_demo.provenance.json" "$stage/signing-dry-run.log"' "$dryrun" &&
  grep -q -F -e 'grep -q "notice_demo.NOTICE" "$stage/signing-dry-run.log"' "$dryrun"; then
  ok
else
  bad "publish-dry-run.yml lost its signing-log SBOM pair plus NOTICE linkage proof (#813)"
fi

# Dry-run report records packaging plus SBOM plus NOTICE with published False.
if grep -q -F -e '"packaging"' "$dryrun" &&
  grep -q -F -e '"notice"' "$dryrun" &&
  grep -q -F -e 'release_artifacts' "$dryrun" &&
  grep -q -F -e '"published": False' "$dryrun"; then
  ok
else
  bad "publish-dry-run.yml lost its packaging plus notice report record with published False (#813)"
fi

# Dry-run summary records packaging plus SBOM linkage plus NOTICE, publishes nothing.
if grep -q -F -e '//deploy/release:release_artifacts' "$dryrun" &&
  grep -q -F -e 'subject digest equals artifact sha256' "$dryrun" &&
  grep -q -F -e 'signed alongside SBOM pair' "$dryrun"; then
  ok
else
  bad "publish-dry-run.yml lost its packaging plus SBOM-linkage plus NOTICE summary (#813)"
fi

# Dry-run keeps its dispatch-only plus clean-checkout plus read-only shape.
if grep -q -F -e 'workflow_dispatch' "$dryrun" &&
  grep -q -F -e 'RUNNER_TEMP/publish-dry-run' "$dryrun" &&
  grep -q -F -e 'test -z "$(git status --porcelain)"' "$dryrun" &&
  grep -q -F -e 'persist-credentials: false' "$dryrun"; then
  ok
else
  bad "publish-dry-run.yml lost its dispatch-only plus staging plus clean-checkout shape (#813)"
fi

# Driver names the packaging order before signing plus draft.
if grep -q -F -e 'release_artifacts' "$driver" &&
  grep -q -F -e 'subject digest equal to artifact sha256' "$driver" &&
  grep -q -F -e 'signed alongside the SBOM pair' "$driver"; then
  ok
else
  bad "release driver lost its release_artifacts plus subject-digest plus signed-alongside plan (#813)"
fi

# Runbook documents the artifact-into-releases packaging with linkage.
if grep -q -F -e '## Packaging' "$runbook" &&
  grep -q -F -e '//:audit_curator' "$runbook" &&
  grep -q -F -e '//deploy/release:release_artifacts' "$runbook" &&
  grep -q -F -e 'subject digest equals artifact sha256' "$runbook" &&
  grep -q -F -e 'signed alongside the SBOM pair' "$runbook" &&
  grep -q -F -e 'release_packaging_qualification' "$runbook"; then
  ok
else
  bad "release-runbook.md lost its Packaging curator plus SBOM-linkage plus signing record (#813)"
fi

# SBOM wire profile stays pinned: SPDX-2.3 plus SLSA v1 hermetic.
if grep -q -F -e 'SPDX-2.3' "deploy/release/sbom.bzl" &&
  grep -q -F -e 'https://slsa.dev/provenance/v1' "deploy/release/sbom.bzl" &&
  grep -q -F -e 'via Rust' "deploy/release/sbom.bzl"; then
  ok
else
  bad "sbom.bzl lost its SPDX-2.3 plus SLSA-v1 plus hermetic pins (#813)"
fi

# NOTICE stays hermetic with byte-identical rebuilds plus missing-notice-text.
if grep -q -F -e 'def notice_bundle' "deploy/release/notice.bzl" &&
  grep -q -F -e 'byte-identical' "deploy/release/notice.bzl" &&
  grep -q -F -e 'missing-notice-text' "deploy/release/notice.bzl"; then
  ok
else
  bad "notice.bzl lost its hermetic plus byte-identical plus missing-notice-text pins (#813)"
fi

# Fixture files stay present with corpus coverage.
if [[ -f "$pins" && -f "$expected" && -f "$fixture_build" ]] &&
  grep -q -F -e 'pins.bzl' "$fixture_build" &&
  grep -q -F -e 'release_packaging.expected' "$fixture_build" &&
  grep -q -F -e 'corpus_starlark' "$fixture_build"; then
  ok
else
  bad "release-packaging fixture missing (want pins.bzl plus release_packaging.expected plus corpus BUILD)"
fi

# Pins record packaging plus linkage plus pipeline plus honesty.
if grep -q -F -e 'PACKAGING_TARGET = "//deploy/release:release_artifacts' "$pins" &&
  grep -q -F -e 'PACKAGING_CURATOR = "//:audit_curator' "$pins" &&
  grep -q -F -e 'SBOM_LINKAGE' "$pins" &&
  grep -q -F -e 'NOTICE_SIGNED' "$pins" &&
  grep -q -F -e 'PIPELINE_BUILD' "$pins" &&
  grep -q -F -e 'DRIVER_PACKAGING' "$pins" &&
  grep -q -F -e 'qualified seed-only under issue #813' "$pins" &&
  grep -q -F -e 'no Supported claim' "$pins"; then
  ok
else
  bad "pins.bzl lost its packaging plus linkage plus pipeline plus honesty pins under issue #813"
fi

# Expected fixture pins the packaging plus linkage plus honesty lines.
if grep -q -F -e 'Release packaging for artifact-into-releases (issue #813)' "$expected" &&
  grep -q -F -e '//deploy/release:release_artifacts builds curator plus binary' "$expected" &&
  grep -q -F -e 'subject digest equal to artifact' "$expected" &&
  grep -q -F -e 'signed alongside the SBOM pair' "$expected" &&
  grep -q -F -e 'publish-dry-run.yml builds' "$expected" &&
  grep -q -F -e 'Qualified' "$expected" &&
  grep -q -F -e 'no Supported claim' "$expected"; then
  ok
else
  bad "release_packaging.expected lost its packaging plus linkage plus honesty lines under #813"
fi

# BUILD owns the harness target plus dogfood-freshness wires it.
if grep -q -F -e 'name = "release_packaging_qualification"' "$targets_b" &&
  grep -q -F -e 'release_packaging_qualification.sh' "$targets_b" &&
  grep -q -F -e 'bazel run --noshow_progress //tools/ci:release_packaging_qualification' "$freshness"; then
  ok
else
  bad "tools/ci/ci_targets_b.bzl or dogfood_freshness.sh lost the release_packaging_qualification wiring (want target plus freshness)"
fi

# Live proof: the packaged unit builds green on the seed host.
if bazel build //deploy/release:release_artifacts --noshow_progress >/dev/null 2>&1; then
  ok
else
  bad "release_artifacts failed to build (want green packaging unit, issue #813)"
fi

# Live proof: the fixture package builds green on the seed host.
if bazel build //tools/ci/tests/fixtures/release_packaging/... --noshow_progress >/dev/null 2>&1; then
  ok
else
  bad "release-packaging fixture failed to build (want green on the seed host, issue #813)"
fi

dx_test_summary "release packaging harness"
