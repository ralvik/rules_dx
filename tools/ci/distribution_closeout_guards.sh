#!/usr/bin/env bash
# Distribution/closeout guards (issues #5, #26, #78, #184, #54).
#
# No release has been cut: no tags, GitHub releases, registry
# submissions, or publication outputs without explicit owner approval.
# Standalone dx binaries + BCR publication (#26), the human-run
# signing-first dry run (#78), and prebuilt devcontainer images on GHCR
# (#184, separate workflow) stay owner-gated with SECURITY reporting as
# the release precondition. Stage 5 close-out (#54) runs the full
# battery on a clean tree with docs matching as-built behavior.
#
# This harness machine-checks the frozen half verifiable on a clean tree
# today (32 checks): hygiene policy, exact 0.0.0 module pin + consumer
# pin, distribution doc ownership + Bazel-first + standalone-install
# records + BCR destination + draft-only ceiling, workflow separation
# + triggers + default-closed approve gates + dry-run report record,
# signing-first trust-root + SBOM detail, digest-pinned prebuilt base
# + Bazelisk delegation + cosign deferral + prebuilt doc section +
# never-latest gate, scaffold state, self-call consumer/docs smoke,
# security precondition, gitignored outputs, matrix close-out page +
# Layer-4 E2E + battery-audit record, E2E driver/format slices +
# E2E-case convention, and no-publish invariants. Matrix/SBOM/BCR/
# install verification and the full green battery stay open under
# their issues.
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

# #26 distribution ownership in environments doc.
if grep -q -F -e '## Distribution' docs/environments/environment.md; then
  ok
else
  bad "environment.md lost its Distribution section (#26 ownership)"
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

# #184 scaffold + devcontainer route documented.
if grep -q -F -e 'image' .devcontainer/devcontainer.json \
  && grep -q -F -e 'GHCR' docs/contributing/devcontainer.md; then
  ok
else
  bad "devcontainer scaffold lost its image field or GHCR route doc"
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

# #26 BCR destination stays recorded as the approved v1 module route
# (destinations only, not credentials/sequence).
if grep -q -F -e 'Bazel Central Registry' docs/environments/environment.md; then
  ok
else
  bad "environment.md lost its BCR v1-destination record (#26)"
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

# #54 close-out page owns the battery + matrix.
if grep -q -F -e 'issue #54' docs/testing/verification-matrix.md \
  && grep -q -F -e '## Battery' docs/testing/verification-matrix.md; then
  ok
else
  bad "verification-matrix lost its #54 close-out ownership or Battery section"
fi

# #54 E2E-case convention: every integration case wired to a driver.
if [[ -f "tools/ci/e2e_cases.sh" ]]; then
  ok
else
  bad "e2e_cases convention harness missing (#54)"
fi

# #54 battery commands recorded as built behavior.
if grep -q -F -e 'bazel build //...' docs/testing/verification-matrix.md \
  && grep -q -F -e 'bazel test //...' docs/testing/verification-matrix.md; then
  ok
else
  bad "verification-matrix lost its build/test battery record"
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

# #54 Layer-4 E2E suite stays recorded as the thin CLI-contract gate
# (clean/dirty/format-roundtrip, explicit-only, carve-out automatic).
if grep -q -F -e 'Layer-4 E2E' docs/testing/verification-matrix.md; then
  ok
else
  bad "verification-matrix lost its Layer-4 E2E suite record (#54)"
fi

# #26 standalone-install record stays owned (no local Rust toolchain
# or Bazel required; install verification still open).
if grep -q -F -e 'Standalone installation' docs/environments/environment.md; then
  ok
else
  bad "environment.md lost its standalone-install record (#26)"
fi

# #78 dry-run report record stays explicit (summary + logs report
# what would publish; the release itself stays owner-gated).
if grep -q -F -e 'dry-run report' .github/workflows/publish-dry-run.yml; then
  ok
else
  bad "publish dry run lost its dry-run report record (#78)"
fi

# #184 prebuilt-image doc section stays owned (GHCR route, scaffold
# files, separate workflow; image build still owner-gated).
if grep -q -F -e '## Prebuilt images (GHCR)' docs/contributing/devcontainer.md; then
  ok
else
  bad "devcontainer.md lost its Prebuilt images (GHCR) section (#184)"
fi

# #54 E2E driver + format slices stay present alongside the case
# convention (full green battery still open).
if [[ -f "tools/ci/e2e.sh" ]] \
  && [[ -f "tools/ci/e2e_format.sh" ]]; then
  ok
else
  bad "E2E driver/format slices missing (e2e.sh/e2e_format.sh, #54)"
fi

# #26 Bazel-first path stays explicit: `bazel run //dx:env` is the
# supported install with no checksum-only fallback (standalone
# verification still open).
if grep -q -F -e 'Bazel-first installation path is `bazel run //dx:env`' docs/environments/environment.md \
  && grep -q -F -e 'checksum-only fallback' docs/environments/environment.md; then
  ok
else
  bad "environment.md lost its Bazel-first/no-checksum-fallback record (#26)"
fi

# #78 draft-only publisher ceiling stays pinned: draft default with
# the dry-run placeholder tag, never creating tags itself.
if grep -q -F -e 'Draft-only publisher ceiling' docs/environments/environment.md \
  && grep -q -F -e 'v0.0.0-dryrun' docs/environments/environment.md; then
  ok
else
  bad "environment.md lost its draft-only publisher ceiling (#78)"
fi

# #184 GHCR gate messages stay explicit: digest-pinned FROM, never
# latest, build-only without approve (push/signing still gated).
if grep -q -F -e 'never latest/bare tag' .github/workflows/ghcr.yml \
  && grep -q -F -e 'build-only, nothing pushes' .github/workflows/ghcr.yml; then
  ok
else
  bad "GHCR workflow lost its never-latest/build-only gate record (#184)"
fi

# #54 battery-audit record stays explicit: corpus + code-ownership
# audits and the explicit E2E suite inside the close-out battery.
if grep -q -F -e 'corpus and code ownership audits' docs/testing/verification-matrix.md \
  && grep -q -F -e 'explicit E2E suite' docs/testing/verification-matrix.md; then
  ok
else
  bad "verification-matrix lost its battery-audit record (#54)"
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

echo "distribution closeout guards harness: $pass passed, $fail failed"
[[ "$fail" == "0" ]]
