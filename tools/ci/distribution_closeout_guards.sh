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
# today (12 checks): hygiene policy, distribution doc ownership,
# workflow separation + triggers + gates, scaffold state, security
# precondition, gitignored outputs, matrix close-out page, and
# no-publish invariants. Matrix/SBOM/BCR/install verification and the
# full green battery stay open under their issues.
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

# #54 close-out page owns the battery + matrix.
if grep -q -F -e 'issue #54' docs/testing/verification-matrix.md \
  && grep -q -F -e '## Battery' docs/testing/verification-matrix.md; then
  ok
else
  bad "verification-matrix lost its #54 close-out ownership or Battery section"
fi

# #54 battery commands recorded as built behavior.
if grep -q -F -e 'bazel build //...' docs/testing/verification-matrix.md \
  && grep -q -F -e 'bazel test //...' docs/testing/verification-matrix.md; then
  ok
else
  bad "verification-matrix lost its build/test battery record"
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
