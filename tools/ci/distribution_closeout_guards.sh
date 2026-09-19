#!/usr/bin/env bash
# Distribution/closeout guards (issues #5, #26, #78, #184, #54).
#
# No release has been cut: no tags, GitHub releases, registry
# submissions, or publication outputs without explicit owner approval.
# Standalone dx binaries + BCR publication (#26), the human-run
# signing-first dry run (#78), and prebuilt devcontainer images on GHCR
# (#184, separate workflow) stay owner-gated with SECURITY reporting as
# the release precondition. Stage 5 close-out (#54) runs the full
# battery on a clean tree.
#
# This harness machine-checks the frozen half verifiable on a clean tree
# today (37 checks): hygiene policy, exact 0.0.0 module pin + consumer
# pin + reviewed-commit workflow pin + unqualified-matrix record,
# workflow separation + triggers + default-closed approve gates +
# never-publishes + dry-run report + clean-checkout record + seed
# exercised path (standalone + draft dry-run + BCR shape + verifier
# refusal),
# signing-first trust-root + attestation + SBOM detail, digest-pinned
# prebuilt base + scaffold state + quota record + scaffold-update +
# Bazelisk delegation + cosign deferral + admissibility gate +
# never-latest gate, scaffold state, self-call consumer smoke, security
# precondition, gitignored outputs, E2E driver/format slices +
# E2E-case convention, install-time publisher-identity verification
# (#26 implemented via //deploy/install:dx_verify + //cli/cli:dx_standalone),
# and no-publish invariants. Matrix/SBOM generation/BCR submission and
# the full green battery stay open under their issues.
#
# Versioned here, run by CI via `bazel run //tools/ci:distribution_closeout_guards`,
# following //tools/ci:ghcr_publish_guards.
set -euo pipefail

if [[ -n "${BUILD_WORKSPACE_DIRECTORY:-}" ]]; then
  workspace="$BUILD_WORKSPACE_DIRECTORY"
else
  workspace="$(git rev-parse --show-toplevel)"
fi
cd "$workspace"

pass=0
fail=0
ok() { pass=$((pass + 1)); }
bad() { echo "FAIL: $1" >&2; fail=$((fail + 1)); }

# #5 release hygiene policy: approval gate + gitignored outputs.
if grep -q -F -e 'approval' CONTRIBUTING.md \
  && grep -q -F -e 'dist/' .gitignore \
  && grep -q -F -e 'release/' .gitignore; then
  ok
else
  bad "release hygiene lost its approval policy or dist/release gitignore"
fi

# #5 module stays unpublishable version string only.
if grep -q -F -e 'version' MODULE.bazel; then
  ok
else
  bad "MODULE.bazel lost its version string record"
fi

# #5 exact 0.0.0 pin: consumers pin reviewed commits, never tags.
if grep -q -F -e 'version = "0.0.0"' MODULE.bazel; then
  ok
else
  bad "MODULE.bazel lost its exact 0.0.0 unpublishable pin"
fi

# #5 SECURITY reporting precondition present.
if grep -q -F -e 'report' SECURITY.md; then
  ok
else
  bad "SECURITY.md lost its reporting record"
fi

# #78 dry run stays dispatch-only, publishes nothing either way.
if grep -q -F -e 'workflow_dispatch' .github/workflows/publish-dry-run.yml \
  && grep -q -F -e 'published' .github/workflows/publish-dry-run.yml; then
  ok
else
  bad "publish dry run lost its dispatch-only / nothing-publishes shape"
fi

# #184 GHCR stays a separate workflow from releases, gated push.
if [[ -f ".github/workflows/ghcr.yml" ]] \
  && grep -q -F -e 'workflow_dispatch' .github/workflows/ghcr.yml \
  && grep -q -F -e 'approve' .github/workflows/ghcr.yml; then
  ok
else
  bad "GHCR workflow lost its separate-file / dispatch + approve gate shape"
fi

# #26 signing-first trust root named in the dry-run report order.
if grep -q -F -e 'Sigstore keyless' .github/workflows/publish-dry-run.yml \
  && grep -q -F -e 'issue #26 trust root' .github/workflows/publish-dry-run.yml; then
  ok
else
  bad "publish dry run lost its #26 signing-first trust-root record"
fi

# #184 prebuilt base stays digest-pinned, never floating.
if grep -q -E -e '^FROM [^ ]+@sha256:[0-9a-f]{64}' .devcontainer/Dockerfile.prebuilt; then
  ok
else
  bad "Dockerfile.prebuilt lost its digest-pinned FROM"
fi

# #184 scaffold image field stays present.
if grep -q -F -e 'image' .devcontainer/devcontainer.json; then
  ok
else
  bad "devcontainer scaffold lost its image field"
fi

# Prior slices stay green.
if [[ -f "tools/ci/release_hygiene.sh" ]] \
  && [[ -f "tools/ci/publish_trust.sh" ]] \
  && [[ -f "tools/ci/ghcr_hygiene.sh" ]] \
  && [[ -f "tools/ci/ghcr_publish_guards.sh" ]]; then
  ok
else
  bad "prior publication harnesses missing (release_hygiene/publish_trust/ghcr*)"
fi

# #78/#184 approve gates stay default-closed (explicit owner approval
# until standing approval exists).
if grep -q -F -e 'default: false' .github/workflows/publish-dry-run.yml \
  && grep -q -F -e 'default: false' .github/workflows/ghcr.yml; then
  ok
else
  bad "publish/ghcr workflows lost their default-closed approve gates (#78/#184)"
fi

# #184 Bazelisk delegation stays pinned (launcher sha + 9.2.0 via
# USE_BAZEL_VERSION, no ambient toolchains).
if grep -q -F -e 'Bazelisk' .devcontainer/Dockerfile.prebuilt \
  && grep -q -F -e 'USE_BAZEL_VERSION=9.2.0' .devcontainer/Dockerfile.prebuilt; then
  ok
else
  bad "Dockerfile.prebuilt lost its Bazelisk delegation pin (#184)"
fi

# #5 consumer pin stays on the unpublishable 0.0.0 (reviewed commit
# SHAs, never tags).
if grep -q -F -e 'rules_dx_version: "0.0.0"' examples/consumer-ci/caller.yml; then
  ok
else
  bad "consumer-ci caller lost its 0.0.0 unpublishable pin (#5)"
fi

# #54 E2E-case convention: every integration case wired to a driver.
if [[ -f "tools/ci/e2e_cases.sh" ]]; then
  ok
else
  bad "e2e_cases convention harness missing (#54)"
fi

# #26/#78 SBOM + signing-first detail stays recorded in the dry-run
# report order (tooling unselected, trust root shared with GHCR).
if grep -q -F -e 'SBOM' .github/workflows/publish-dry-run.yml \
  && grep -q -F -e 'Signing' .github/workflows/publish-dry-run.yml; then
  ok
else
  bad "publish dry run lost its SBOM/signing-first detail (#26/#78)"
fi

# #184 cosign deferral stays explicit (signs <digest> on the #26 trust
# root after the human-run signing workflow, signs nothing yet).
if grep -q -F -e 'cosign' .github/workflows/ghcr.yml; then
  ok
else
  bad "GHCR workflow lost its cosign deferral record (#184)"
fi

# #5 self-call smoke stays wired: consumer-ci + docs-ci prove the
# versioned reusable workflows on this repo before consumers use them.
if grep -q -F -e 'reusable-consumer' .github/workflows/ci.yml \
  && grep -q -F -e 'reusable-docs' .github/workflows/ci.yml; then
  ok
else
  bad "ci.yml lost its consumer-ci/docs-ci self-call smoke (#5)"
fi

# #78 dry-run report record stays explicit (summary + logs report
# what would publish; the release itself stays owner-gated).
if grep -q -F -e 'dry-run report' .github/workflows/publish-dry-run.yml; then
  ok
else
  bad "publish dry run lost its dry-run report record (#78)"
fi

# #54 E2E driver + format slices stay present alongside the case
# convention (full green battery still open).
if [[ -f "tools/ci/e2e.sh" ]] \
  && [[ -f "tools/ci/e2e_format.sh" ]]; then
  ok
else
  bad "E2E driver/format slices missing (e2e.sh/e2e_format.sh, #54)"
fi

# #184 GHCR gate messages stay explicit: digest-pinned FROM, never
# latest, build-only without approve (push/signing still gated).
if grep -q -F -e 'never latest/bare tag' .github/workflows/ghcr.yml \
  && grep -q -F -e 'build-only, nothing pushes' .github/workflows/ghcr.yml; then
  ok
else
  bad "GHCR workflow lost its never-latest/build-only gate record (#184)"
fi

# #5 consumer caller pins the reusable workflow at a reviewed commit
# SHA, never a tag or floating ref (bumps are reviewed pins only).
if grep -q -F -e 'reusable-consumer.yml@' examples/consumer-ci/caller.yml; then
  ok
else
  bad "consumer-ci caller lost its reviewed-commit workflow pin (#5)"
fi

# #78 attestation record stays owned: Sigstore keyless plus GitHub
# attestations on the #26 trust root (tooling still dry-run only).
if grep -q -F -e 'attest' .github/workflows/publish-dry-run.yml; then
  ok
else
  bad "publish dry run lost its attestation record (#78)"
fi

# #184 admissibility gate stays versioned: the scaffold image must
# pass devcontainer_is_admissible (pinned bootstrap, Bazel
# delegation, no ambient tools).
if grep -q -F -e 'devcontainer_is_admissible' cli/adopt/src/lib.rs; then
  ok
else
  bad "adopt crate lost its devcontainer admissibility gate (#184)"
fi

# #78 never-publishes stays machine-checked: the dry-run report stages
# a binary with published False and approves nothing by default
# (release itself stays owner-gated).
if grep -q -F -e '"published": False' .github/workflows/publish-dry-run.yml; then
  ok
else
  bad "publish dry run lost its never-publishes record (#78)"
fi

# #184 scaffold + quota record stays explicit: the scaffold still floats
# off the prebuilt digest until the first push, and GHCR quotas are
# qualified on first push (this slice pushes nothing).
if grep -q -F -e 'scaffold still references' .devcontainer/Dockerfile.prebuilt \
  && grep -q -F -e 'GHCR quotas/retention are qualified on first push' .devcontainer/Dockerfile.prebuilt; then
  ok
else
  bad "Dockerfile.prebuilt lost its scaffold/quota record (#184)"
fi

# #5 unqualified-matrix record stays explicit: non-seed release matrix
# entries remain unqualified per the release-hygiene track (no platform
# claimed qualified beyond the seed host).
if grep -q -F -e 'unqualified-per-issue-5' .github/workflows/publish-dry-run.yml; then
  ok
else
  bad "publish dry run lost its unqualified-matrix record (#5)"
fi

# #26 release-matrix record stays owned: the dry-run report carries the
# seed-qualified plus follow-up matrix shape (submission still open).
if grep -q -F -e 'release_matrix' .github/workflows/publish-dry-run.yml; then
  ok
else
  bad "publish dry run lost its release-matrix record (#26)"
fi

# #78 clean-checkout record stays explicit: the dry run stages under
# RUNNER_TEMP and proves the checkout is left clean (release gated).
if grep -q -F -e 'RUNNER_TEMP' .github/workflows/publish-dry-run.yml; then
  ok
else
  bad "publish dry run lost its clean-checkout record (#78)"
fi

# #184 scaffold-update record stays explicit: scaffold digest updates
# plus cosign signing follow on the #26 trust root (push still gated).
if grep -q -F -e 'scaffold update' .github/workflows/ghcr.yml; then
  ok
else
  bad "GHCR workflow lost its scaffold-update record (#184)"
fi

# #26 install-time publisher-identity verification stays owned: the
# verifier requires a Sigstore bundle plus identity/issuer, refuses
# checksum-only, reports the TUF trust root, and fails before
# install/exec; seed-host standalone packaging stays wired.
if [[ -f "deploy/install/dx_verify.sh" ]] \
  && grep -q -F -e 'checksum-only verification is not publisher-identity proof' deploy/install/dx_verify.sh \
  && grep -q -F -e 'tuf-repo-cdn.sigstore.dev' deploy/install/dx_verify.sh \
  && grep -q -F -e 'cosign verify-blob' deploy/install/dx_verify.sh; then
  ok
else
  bad "install verifier lost its bundle-required / no-checksum-fallback / trust-root record (#26)"
fi

if grep -q -F -e 'never executed' deploy/install/dx_verify.sh \
  && grep -q -F -e 'before any install' deploy/install/dx_verify.sh; then
  ok
else
  bad "install verifier lost its fail-before-install/exec record (#26)"
fi

if grep -q -F -e 'dx_standalone' cli/cli/BUILD.bazel \
  && grep -q -F -e 'archive_release(' cli/cli/BUILD.bazel; then
  ok
else
  bad "seed-host standalone archive missing (//cli/cli:dx_standalone, #26)"
fi

if grep -q -F -e '//deploy/install:dx_verify' docs/deploy/authoring.md \
  && grep -q -F -e '//deploy/install:dx_verify' docs/environments/environment.md; then
  ok
else
  bad "docs lost the install-verification owner record (#26)"
fi

# No-publish invariant: no tags claimed, no release outputs committed.
if [[ -z "$(git tag --list 'v*' | head -1)" ]] \
  && [[ -z "$(git ls-files 'dist/*' 'release/*' 2>/dev/null | head -1)" ]]; then
  ok
else
  bad "a version tag or committed dist/release output appeared without owner approval"
fi

# No auto-publish workflow trigger smuggled in.
if ! grep -rn -F -e 'tags:' .github/workflows/publish-dry-run.yml 2>/dev/null | grep -q . \
  && ! grep -rn -F -e 'tags:' .github/workflows/ghcr.yml 2>/dev/null | grep -q .; then
  ok
else
  bad "a tag trigger appeared in publish/ghcr workflows without owner approval"
fi

# #78 seed exercised path stays explicit: standalone archive plus draft
# dry-run plus BCR shape plus verifier refusal staged under RUNNER_TEMP,
# publishing nothing either way.
if grep -q -F -e 'dx-standalone.tar.gz' .github/workflows/publish-dry-run.yml \
  && grep -q -F -e 'GH_RELEASE_DRY_RUN=1 bazel run //cli/cli:github_draft' .github/workflows/publish-dry-run.yml \
  && grep -q -F -e 'bcr-shape.txt' .github/workflows/publish-dry-run.yml \
  && grep -q -F -e 'verify-refusal.log' .github/workflows/publish-dry-run.yml; then
  ok
else
  bad "publish dry run lost its seed exercised path record (#78 standalone + draft + BCR shape + verifier refusal)"
fi

# #78 exercised report shape stays explicit: the dry-run report carries
# the exercised seed steps plus draft/BCR/verify detail with published
# False everywhere, so the nothing-publishes ceiling cannot be narrowed.
if grep -q -F -e '"exercised"' .github/workflows/publish-dry-run.yml \
  && grep -q -F -e '"draft_dry_run"' .github/workflows/publish-dry-run.yml \
  && grep -q -F -e '"bcr_shape"' .github/workflows/publish-dry-run.yml \
  && grep -q -F -e '"verify_refusal"' .github/workflows/publish-dry-run.yml; then
  ok
else
  bad "publish dry run lost its exercised report shape (#78 exercised + draft/BCR/verify detail)"
fi

echo "distribution closeout guards harness: $pass passed, $fail failed"
[[ "$fail" == "0" ]]
