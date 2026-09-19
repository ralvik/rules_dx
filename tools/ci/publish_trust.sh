#!/usr/bin/env bash
# Publication-trust harness (issues #78, #26, #5): machine-checks the
# draft-only + nothing-published invariants that the release-hygiene
# harness does not own.
#
# The publish dry-run stays `workflow_dispatch`-only with a
# default-closed approve gate (proven by //tools/ci:release_hygiene),
# and every `github_release` site stays draft-only by construction:
# `draft` defaults to True and any `draft = False` fails analysis in
# `deploy/rules/github.bzl`, tags validate against the launcher-safe
# charset with the `v0.0.0-dryrun` placeholder default, and the deploy
# program always passes `--draft --verify-tag` so it never creates or
# pushes tags. BCR submission tooling stays unselected per #26: the
# registry is named only in dry-run report strings and docs, never in
# workflow steps. The module stays `rules_dx` at `0.0.0`, an
# unpublishable shape, and SECURITY.md still records that no release
# exists.
#
# This harness machine-checks the static half verifiable on a clean
# tree today (15 checks): module name + unpublishable version, no BCR
# submission tooling in workflows, no `draft = False` site, both
# validators wired to `fail()` in the macro, default placeholder tag
# at every site, draft-only flags on the real `gh release create`
# path, no unflagged executable release-create lines, the tag charset
# gate, the SECURITY.md no-release record, plus install-time
# publisher-identity verification (#26 implemented via
# //deploy/install:dx_verify: bundle-required, no checksum-only
# fallback, TUF trust root, fail-before-install) plus seed exercised
# path (standalone wired, draft dry-run with GH_RELEASE_DRY_RUN=1 still
# publishing nothing, BCR shape checked-not-submitted, verifier refusal
# proved in the workflow). Signing/attestation generation (Sigstore
# keyless + GitHub attestations on the #26 trust root), SBOM/provenance
# generation, BCR submission, and the release matrix beyond the seed host
# stay unimplemented per #26/#78/#5 and are recorded as gaps, not claimed here.
#
# Versioned here, run by CI via `bazel run //tools/ci:publish_trust`,
# following //tools/ci:release_hygiene.
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

# The module keeps its public name at the unpublishable 0.0.0 version:
# consumers pin reviewed commits, never tags or releases.
if grep -q -F -e 'name = "rules_dx"' MODULE.bazel; then
  ok
else
  bad "MODULE.bazel lost the rules_dx module name"
fi
if grep -q -F -e 'version = "0.0.0"' MODULE.bazel && ! grep -q -F -e 'version = "0.1.0"' MODULE.bazel; then
  ok
else
  bad "MODULE.bazel drifted from the unpublishable version 0.0.0"
fi

# BCR submission stays unselected per #26: no registry tooling runs in
# any workflow. The registry is named only in dry-run report strings
# and docs (the pending destination), never as an executed step.
if grep -rn -E -e 'publish-to-bcr|bcr publish|bazel-central-registry' .github/ | grep -v -E -e '^\s*#' | head -n 5 | grep -q .; then
  bad "a workflow executes BCR submission tooling (stays unselected per #26)"
else
  ok
fi

# No github_release site opts out of the draft gate: draft defaults to
# True and any explicit False fails analysis, so the tree must carry
# no `draft = False` at all.
if grep -rn -F -e 'draft = False' --include='BUILD.bazel' . | head -n 5 | grep -q .; then
  bad "a github_release site sets draft = False (draft-only per #5)"
else
  ok
fi

# The macro wires both validators to analysis failure: a bad tag or a
# non-draft request fails the build instead of publishing.
if grep -q -F -e 'fail(tag_error' deploy/rules/github.bzl && grep -q -F -e 'fail(draft_error' deploy/rules/github.bzl; then
  ok
else
  bad "deploy/rules/github.bzl no longer fails analysis on tag/draft violations"
fi

# Every github_release site uses the default placeholder tag: no
# explicit `tag =` override may point a checked-in target at a real
# release tag.
if grep -rn -A8 -F -e 'github_release(' --include='BUILD.bazel' . | grep -F -e 'tag =' | head -n 5 | grep -q .; then
  bad "a github_release site overrides the v0.0.0-dryrun placeholder tag"
else
  ok
fi

# The real publisher path always passes --draft --verify-tag, so the
# program never creates or pushes tags itself.
if grep -q -F -e 'exec gh release create "${tag}" "$@" --draft --verify-tag' deploy/rules/github_deploy.sh; then
  ok
else
  bad "deploy/rules/github_deploy.sh real path lost the --draft --verify-tag flags"
fi

# No other executable `gh release create` line exists without the
# draft-only flags (printf dry-run text plus echo/grep verify contexts
# are the only other mentions).
if grep -rn -F -e 'gh release create' --include='*.sh' deploy/ cli/ | grep -v -F -e 'printf' | grep -v -F -e 'echo' | grep -v -F -e 'grep -q' | grep -v -F -e '--draft --verify-tag' | head -n 5 | grep -q .; then
  bad "an executable gh release create line lacks the draft-only flags"
else
  ok
fi

# Release tags embed in the generated launcher, so the charset gate
# keeps them shell-safe.
if grep -q -F -e '_VALID_TAG_CHARS' deploy/rules/github.bzl; then
  ok
else
  bad "deploy/rules/github.bzl lost the launcher-safe tag charset gate"
fi

# SECURITY.md still records that no release exists, so the supported-
# versions section cannot drift into implying support.
if grep -q -F -e 'No release exists yet' SECURITY.md; then
  ok
else
  bad "SECURITY.md lost the no-release-exists record"
fi

# Install-time publisher-identity verification is implemented per #26:
# the verifier requires a bundle plus identity/issuer, refuses
# checksum-only, and fails before install/exec on the #26 trust root.
if [[ -f "deploy/install/dx_verify.sh" ]] \
  && grep -q -F -e 'checksum-only verification is not publisher-identity proof' deploy/install/dx_verify.sh \
  && grep -q -F -e 'tuf-repo-cdn.sigstore.dev' deploy/install/dx_verify.sh; then
  ok
else
  bad "install verifier missing bundle-required / trust-root record (#26)"
fi

# Seed-host standalone packaging stays wired; the wider matrix stays
# unqualified per #5 (no platform claimed qualified beyond the seed).
if grep -q -F -e 'dx_standalone' cli/cli/BUILD.bazel; then
  ok
else
  bad "seed-host standalone archive missing (//cli/cli:dx_standalone, #26)"
fi

# Seed exercised path stays draft-only (issue #78): the workflow runs
# //cli/cli:github_draft with GH_RELEASE_DRY_RUN=1 on the placeholder
# tag with draft-only flags, and the report records published False.
if grep -q -F -e 'GH_RELEASE_DRY_RUN=1 bazel run //cli/cli:github_draft' .github/workflows/publish-dry-run.yml && grep -q -F -e '"published": False' .github/workflows/publish-dry-run.yml && grep -q -F -e '--draft --verify-tag' .github/workflows/publish-dry-run.yml; then
  ok
else
  bad "publish dry run lost the draft-only exercised record (GH_RELEASE_DRY_RUN=1 + published False, issue #78)"
fi

# BCR shape stays checked-not-submitted (issue #78): the workflow checks
# the module shape and records submitted False without running registry
# tooling (still unselected per #26, proven above).
if grep -q -F -e '"submitted": False' .github/workflows/publish-dry-run.yml && grep -q -F -e 'checked, not submitted' .github/workflows/publish-dry-run.yml; then
  ok
else
  bad "publish dry run lost the BCR checked-not-submitted record (issue #78)"
fi

# Verifier refusal stays exercised in the workflow (issue #78
# signing-first): checksum-only refused on the TUF trust root with
# nothing installed, without network.
if grep -q -F -e 'verify-refusal.log' .github/workflows/publish-dry-run.yml && grep -q -F -e 'checksum-only verification is not publisher-identity proof' .github/workflows/publish-dry-run.yml; then
  ok
else
  bad "publish dry run lost the verifier-refusal exercised record (issue #78)"
fi

echo "publish trust audit: $pass passed, $fail failed"
[[ "$fail" == "0" ]]
