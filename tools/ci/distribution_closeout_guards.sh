#!/usr/bin/env bash
# Distribution/closeout guards.
#
# No release has been cut: no tags, GitHub releases, registry
# submissions, or publication outputs without explicit owner approval.
# Standalone dx binaries + BCR publication + matrix/SBOM/signing,
# the human-run signing-first dry run
# for the human-run path), and prebuilt devcontainer
# images on GHCR (live successor to closed, separate workflow) stay owner-gated with SECURITY
# reporting as the release precondition. Close-out battery runs the
# full battery on a clean tree.
#
# This harness machine-checks the frozen half verifiable on a clean tree
# today (39 checks): hygiene policy, exact 0.0.0 module pin + consumer
# pin + reviewed-commit workflow pin + qualified-matrix record,
# workflow separation + triggers + default-closed approve gates +
# never-publishes + dry-run report + clean-checkout record + seed
# exercised path (standalone + draft dry-run + BCR shape + verifier
# refusal),
# signing-first trust-root + attestation + SBOM detail, digest-pinned
# prebuilt base + scaffold state + quota record + scaffold-update +
# Bazelisk delegation + cosign deferral + admissibility gate +
# never-latest gate, scaffold state, self-call consumer smoke, security
# precondition, gitignored outputs, hermetic CLI-contract pins (issue
# replaces E2E driver/format slices + E2E-case convention),
# install-time publisher-identity verification
# (implemented via //deploy/install:dx_verify + //cli/cli:dx_standalone),
# and no-publish invariants. Full matrix/SBOM/signing/BCR/human-run are
# implemented owner-gated per (//deploy/release:all + runbook;
# human-run driver owned under, live successor to closed for
# the human-run path); the
# full green battery stays open under its issue.
#
# Versioned here, run by CI via `bazel run //tools/ci:distribution_closeout_guards`,
# following //tools/ci:ghcr_publish_guards.
set -euo pipefail

# Shared workspace + runfiles helpers.
# Bootstrap via tools/sh/bootstrap.sh (issue #654): runfiles forest first, then source tree.
source "${RUNFILES_DIR:-/dev/null}/_main/tools/sh/bootstrap.sh" 2>/dev/null || source "${TEST_SRCDIR:-/dev/null}/_main/tools/sh/bootstrap.sh" 2>/dev/null || source "$0.runfiles/_main/tools/sh/bootstrap.sh" 2>/dev/null || source "${BASH_SOURCE[0]}.runfiles/_main/tools/sh/bootstrap.sh" 2>/dev/null || source "$(git rev-parse --show-toplevel 2>/dev/null)/tools/sh/bootstrap.sh"
dx_bootstrap "tools/sh/lib.sh"

dx_cd_workspace

dx_test_init

# release hygiene policy: approval gate + gitignored outputs.
if grep -q -F -e 'approval' CONTRIBUTING.md &&
  grep -q -F -e 'dist/' .gitignore &&
  grep -q -F -e 'release/' .gitignore; then
  ok
else
  bad "release hygiene lost its approval policy or dist/release gitignore"
fi

# module stays unpublishable version string only.
if grep -q -F -e 'version' MODULE.bazel; then
  ok
else
  bad "MODULE.bazel lost its version string record"
fi

# exact 0.0.0 pin: consumers pin reviewed commits, never tags.
if grep -q -F -e 'version = "0.0.0"' MODULE.bazel; then
  ok
else
  bad "MODULE.bazel lost its exact 0.0.0 unpublishable pin"
fi

# SECURITY reporting precondition present.
if grep -q -F -e 'report' SECURITY.md; then
  ok
else
  bad "SECURITY.md lost its reporting record"
fi

# dry run stays dispatch-only, publishes nothing either way.
if grep -q -F -e 'workflow_dispatch' .github/workflows/publish-dry-run.yml &&
  grep -q -F -e 'published' .github/workflows/publish-dry-run.yml; then
  ok
else
  bad "publish dry run lost its dispatch-only / nothing-publishes shape"
fi

# GHCR stays a separate workflow from releases, gated push.
if [[ -f ".github/workflows/ghcr.yml" ]] &&
  grep -q -F -e 'workflow_dispatch' .github/workflows/ghcr.yml &&
  grep -q -F -e 'approve' .github/workflows/ghcr.yml; then
  ok
else
  bad "GHCR workflow lost its separate-file / dispatch + approve gate shape"
fi

# signing-first trust root named in the dry-run report order.
if grep -q -F -e 'Sigstore keyless' .github/workflows/publish-dry-run.yml &&
  grep -q -F -e 'trust root' .github/workflows/publish-dry-run.yml; then
  ok
else
  bad "publish dry run lost its signing-first trust-root record"
fi

# prebuilt base stays digest-pinned, never floating.
if grep -q -E -e '^FROM [^ ]+@sha256:[0-9a-f]{64}' .devcontainer/Dockerfile.prebuilt; then
  ok
else
  bad "Dockerfile.prebuilt lost its digest-pinned FROM"
fi

# scaffold image field stays present.
if grep -q -F -e 'image' .devcontainer/devcontainer.json; then
  ok
else
  bad "devcontainer scaffold lost its image field"
fi

# Prior slices stay green.
if [[ -f "tools/ci/release_hygiene.sh" ]] &&
  [[ -f "tools/ci/publish_trust.sh" ]] &&
  [[ -f "tools/ci/ghcr_hygiene.sh" ]] &&
  [[ -f "tools/ci/ghcr_publish_guards.sh" ]]; then
  ok
else
  bad "prior publication harnesses missing (release_hygiene/publish_trust/ghcr*)"
fi

# / approve gates stay default-closed (explicit owner approval
# until standing approval exists).
if grep -q -F -e 'default: false' .github/workflows/publish-dry-run.yml &&
  grep -q -F -e 'default: false' .github/workflows/ghcr.yml; then
  ok
else
  bad "publish/ghcr workflows lost their default-closed approve gates (#78/#460)"
fi

# Bazelisk delegation stays pinned (launcher sha + 9.2.0 via
# USE_BAZEL_VERSION, no ambient toolchains).
if grep -q -F -e 'Bazelisk' .devcontainer/Dockerfile.prebuilt &&
  grep -q -F -e 'USE_BAZEL_VERSION=9.2.0' .devcontainer/Dockerfile.prebuilt; then
  ok
else
  bad "Dockerfile.prebuilt lost its Bazelisk delegation pin (#460)"
fi

# consumer pin stays on the unpublishable 0.0.0 (reviewed commit
# SHAs, never tags).
if grep -q -F -e 'rules_dx_version: "0.0.0"' examples/consumer-ci/caller.yml; then
  ok
else
  bad "consumer-ci caller lost its 0.0.0 unpublishable pin (#5)"
fi

# (replaces E2E-case convention): nested E2E deleted, so
# no e2e_cases harness and no integration/ cases remain; CLI-contract
# coverage is hermetic under `bazel test //...`.
if [[ -f "tools/ci/e2e_cases.sh" ]] || [[ -d "integration" ]]; then
  bad "nested E2E remnants still present (e2e_cases.sh/integration/, issue #407)"
else
  ok
fi

# SBOM + signing-first detail stays recorded in the dry-run
# report order (owner-gated tooling, trust root shared with GHCR).
if grep -q -F -e 'SBOM' .github/workflows/publish-dry-run.yml &&
  grep -q -F -e 'Signing' .github/workflows/publish-dry-run.yml; then
  ok
else
  bad "publish dry run lost its SBOM/signing-first detail (#311)"
fi

# / cosign record stays explicit (signs <digest> on the 
# trust root owner-gated after push, dry-run would-sign otherwise).
if grep -q -F -e 'cosign' .github/workflows/ghcr.yml; then
  ok
else
  bad "GHCR workflow lost its cosign record (#460/#311)"
fi

# self-call smoke stays wired: dogfood in ci.yml proves the versioned
# consumer workflow on this repo, and the docs workflow stays
# self-called by its pinned example caller (the docs-ci job left ci.yml;
# reusable-docs.yml is the gate).
if grep -q -F -e 'reusable-consumer' .github/workflows/ci.yml &&
  grep -q -F -e 'reusable-docs' .github/workflows/reusable-docs.yml &&
  grep -q -F -e 'reusable-docs.yml@' examples/docs-ci/caller.yml; then
  ok
else
  bad "ci.yml lost its dogfood self-call plus the reusable-docs caller smoke (#5)"
fi

# dry-run report record stays explicit (summary + logs report
# what would publish; the release itself stays owner-gated).
if grep -q -F -e 'dry-run report' .github/workflows/publish-dry-run.yml; then
  ok
else
  bad "publish dry run lost its dry-run report record (#78)"
fi

# (replaces E2E driver/format slices): nested drivers
# deleted; hermetic CLI-contract pins live under `bazel test //...`.
if [[ -f "tools/ci/e2e.sh" ]] || [[ -f "tools/ci/e2e_format.sh" ]] || [[ -f "tools/ci/e2e_preset.sh" ]]; then
  bad "nested E2E drivers still present (e2e.sh/e2e_format.sh/e2e_preset.sh, issue #407)"
else
  ok
fi

# GHCR gate messages stay explicit: digest-pinned FROM, never
# latest, build-only without approve (push/signing still gated).
if grep -q -F -e 'never latest/bare tag' .github/workflows/ghcr.yml &&
  grep -q -F -e 'build-only, nothing pushes' .github/workflows/ghcr.yml; then
  ok
else
  bad "GHCR workflow lost its never-latest/build-only gate record (#460)"
fi

# consumer caller pins the reusable workflow at a reviewed commit
# SHA, never a tag or floating ref (bumps are reviewed pins only).
if grep -q -F -e 'reusable-consumer.yml@' examples/consumer-ci/caller.yml; then
  ok
else
  bad "consumer-ci caller lost its reviewed-commit workflow pin (#5)"
fi

# attestation record stays owned: Sigstore keyless plus GitHub
# attestations on the trust root (owner-gated dry-run-first).
if grep -q -F -e 'attest' .github/workflows/publish-dry-run.yml; then
  ok
else
  bad "publish dry run lost its attestation record (#311)"
fi

# admissibility gate stays versioned: the scaffold image must
# pass devcontainer_is_admissible (pinned bootstrap, Bazel
# delegation, no ambient tools).
if grep -q -F -e 'devcontainer_is_admissible' cli/adopt/src/lib.rs; then
  ok
else
  bad "adopt crate lost its devcontainer admissibility gate (#460)"
fi

# never-publishes stays machine-checked: the dry-run report stages
# a binary with published False and approves nothing by default
# (release itself stays owner-gated).
if grep -q -F -e '"published": False' .github/workflows/publish-dry-run.yml; then
  ok
else
  bad "publish dry run lost its never-publishes record (#78)"
fi

# scaffold + quota record stays explicit: the scaffold still floats
# off the prebuilt digest until the first push, and GHCR quotas are
# qualified on first push (this slice pushes nothing).
if grep -q -F -e 'scaffold still references' .devcontainer/Dockerfile.prebuilt &&
  grep -q -F -e 'GHCR quotas/retention are qualified on first push' .devcontainer/Dockerfile.prebuilt; then
  ok
else
  bad "Dockerfile.prebuilt lost its scaffold/quota record (#460)"
fi

# qualified-matrix record stays explicit: all release matrix
# entries are qualified with per-host evidence in the frozen matrix (no
# platform claimed qualified without host evidence).
if grep -q -F -e 'qualified-host-evidence' .github/workflows/publish-dry-run.yml; then
  ok
else
  bad "publish dry run lost its qualified-matrix record (#815)"
fi

# release-matrix record stays owned: the dry-run report carries the
# all-qualified matrix shape with per-host evidence (frozen in
# deploy/release/matrix.bzl).
if grep -q -F -e 'release_matrix' .github/workflows/publish-dry-run.yml; then
  ok
else
  bad "publish dry run lost its release-matrix record (#311)"
fi

# clean-checkout record stays explicit: the dry run stages under
# RUNNER_TEMP and proves the checkout is left clean (release gated).
if grep -q -F -e 'RUNNER_TEMP' .github/workflows/publish-dry-run.yml; then
  ok
else
  bad "publish dry run lost its clean-checkout record (#78)"
fi

# / scaffold-update record stays explicit: scaffold digest updates
# plus cosign signing follow on the trust root (push still gated).
if grep -q -F -e 'scaffold update' .github/workflows/ghcr.yml; then
  ok
else
  bad "GHCR workflow lost its scaffold-update record (#460)"
fi

# install-time publisher-identity verification stays owned: the
# verifier requires a Sigstore bundle plus identity/issuer, refuses
# checksum-only, reports the TUF trust root, and fails before
# install/exec; seed-host standalone packaging stays wired.
if [[ -f "deploy/install/src/lib.rs" ]] &&
  grep -q -F -e 'checksum-only verification is not publisher-identity proof' deploy/install/src/lib.rs &&
  grep -q -F -e 'tuf-repo-cdn.sigstore.dev' deploy/install/src/lib.rs &&
  grep -q -F -e 'cosign verify-blob' deploy/install/src/lib.rs; then
  ok
else
  bad "install verifier lost its bundle-required / no-checksum-fallback / trust-root record (#26)"
fi

if grep -q -F -e 'never executed' deploy/install/src/lib.rs &&
  grep -q -F -e 'before install' deploy/install/src/lib.rs; then
  ok
else
  bad "install verifier lost its fail-before-install/exec record (#26)"
fi

if grep -q -F -e 'dx_standalone' cli/cli/BUILD.bazel &&
  grep -q -F -e 'archive_deploy(' cli/cli/BUILD.bazel; then
  ok
else
  bad "seed-host standalone archive missing (//cli/cli:dx_standalone, #26)"
fi

if grep -q -F -e '//deploy/install:dx_verify' docs/deploy/authoring.md &&
  grep -q -F -e '//deploy/install:dx_verify' docs/environments/environment.md; then
  ok
else
  bad "docs lost the install-verification owner record (#26)"
fi

# No-publish invariant: no tags claimed, no release outputs committed.
if [[ -z "$(git tag --list 'v*' | head -1)" ]] &&
  [[ -z "$(git ls-files 'dist/*' 'release/*' 2>/dev/null | head -1)" ]]; then
  ok
else
  bad "a version tag or committed dist/release output appeared without owner approval"
fi

# No auto-publish workflow trigger smuggled in.
if ! grep -rn -F -e 'tags:' .github/workflows/publish-dry-run.yml 2>/dev/null | grep -q . &&
  ! grep -rn -F -e 'tags:' .github/workflows/ghcr.yml 2>/dev/null | grep -q .; then
  ok
else
  bad "a tag trigger appeared in publish/ghcr workflows without owner approval"
fi

# / seed exercised path stays explicit: standalone archive plus
# draft dry-run plus SBOM plus signing dry-run plus BCR shape plus
# verifier refusal staged under RUNNER_TEMP, publishing nothing either way.
if grep -q -F -e 'dx-standalone.tar.gz' .github/workflows/publish-dry-run.yml &&
  grep -q -F -e 'GH_RELEASE_DRY_RUN=1 bazel run //cli/cli:github_draft' .github/workflows/publish-dry-run.yml &&
  grep -q -F -e 'bcr-shape.txt' .github/workflows/publish-dry-run.yml &&
  grep -q -F -e 'verify-refusal.log' .github/workflows/publish-dry-run.yml &&
  grep -q -F -e '//deploy/release:sbom_demo' .github/workflows/publish-dry-run.yml &&
  grep -q -F -e 'RELEASE_SIGN_DRY_RUN=1 bazel run //deploy/release:signing_demo' .github/workflows/publish-dry-run.yml; then
  ok
else
  bad "publish dry run lost its seed exercised path record (#78/#311 standalone + draft + SBOM + signing + BCR shape + verifier refusal)"
fi

# / exercised report shape stays explicit: the dry-run report
# carries the exercised seed steps plus draft/SBOM/signing/BCR/verify
# detail with published False everywhere, so the nothing-publishes
# ceiling cannot be narrowed.
if grep -q -F -e '"exercised"' .github/workflows/publish-dry-run.yml &&
  grep -q -F -e '"draft_dry_run"' .github/workflows/publish-dry-run.yml &&
  grep -q -F -e '"bcr_shape"' .github/workflows/publish-dry-run.yml &&
  grep -q -F -e '"verify_refusal"' .github/workflows/publish-dry-run.yml &&
  grep -q -F -e '"sbom"' .github/workflows/publish-dry-run.yml &&
  grep -q -F -e '"signing_dry_run"' .github/workflows/publish-dry-run.yml; then
  ok
else
  bad "publish dry run lost its exercised report shape (#78/#311 exercised + draft/SBOM/signing/BCR/verify detail)"
fi

# full release path stays owned: matrix + SBOM + signing + BCR +
# human-run driver wired with policy tests.
if [[ -f "deploy/release/matrix.bzl" ]] &&
  [[ -f "deploy/release/sbom.bzl" ]] &&
  [[ -f "deploy/release/signing.bzl" ]] &&
  [[ -f "deploy/release/bcr.bzl" ]] &&
  [[ -f "deploy/release/src/bin_release_driver.rs" ]] &&
  [[ -f "docs/deploy/release-runbook.md" ]]; then
  ok
else
  bad "full release path missing (deploy/release matrix/sbom/signing/bcr/release.sh + runbook, #311 with human-run under #458)"
fi

# release policy tests stay wired.
if grep -q -F -e '//deploy/release:all' .github/workflows/publish-dry-run.yml; then
  ok
else
  bad "publish dry run lost the release policy tests record (//deploy/release:all, #311)"
fi

dx_test_summary "distribution closeout guards harness"
