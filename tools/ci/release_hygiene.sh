#!/usr/bin/env bash
# Release-hygiene harness.
#
# No release has been cut. Tags, GitHub releases, registry submissions, and
# publication outputs require explicit owner approval (see CONTRIBUTING.md):
# `dist/` and `release/` are git-ignored build outputs and must never be
# committed, `MODULE.bazel` keeps `version = "0.0.0"` as a module version
# string only, and the publish dry-run stays `workflow_dispatch`-only so CI
# stays green by construction and nothing runs uninvited.
#
# This harness machine-checks the no-publication-inputs half that is
# verifiable on a clean tree today (table-driven guard rows via
# `tools/sh/guards.sh` plus three custom git/filesystem gates):
# dist/release git-ignored and uncommitted, module at 0.0.0, no version
# tags, SECURITY.md reporting link + enabled record, publish dry-run
# dispatch-only with a default-closed approve gate, no-secrets minimal
# permissions plus no secrets usage, RUNNER_TEMP staging plus a
# clean-checkout proof, explicit release matrix (all four qualified with
# per-host evidence under issue #815; macOS x86_64 Not planned per #976),
# SBOM/BCR deferrals to tooling, signing-first +
# GHCR-separate notes, checkout SHA pin, typed approve plus
# non-cancelling concurrency, least-privilege no-packages-write,
# no-secrets usage, and sole-tracker deletion plus reporting-enabled
# record plus consumer/docs-caller SHA pins plus both-callers policy plus
# no-tag/no-release/no-submission record plus never-rebuild policy plus
# byte-identity fail-closed record plus seed exercised path (standalone
# archive, draft dry-run, BCR shape check, verifier refusal). Platform,
# provenance (SPDX/SLSA), registry submission, and public-install smoke
# runs stay unqualified per and are recorded as gaps, not claimed here.
#
# Versioned here, run by CI via `bazel run //tools/ci:release_hygiene`,
# following //tools/ci:corpus_audit.
set -euo pipefail

# Shared workspace + runfiles helpers.
# Bootstrap via tools/sh/bootstrap.sh (issue #654): runfiles forest first, then source tree.
source "${RUNFILES_DIR:-/dev/null}/_main/tools/sh/bootstrap.sh" 2>/dev/null || source "${TEST_SRCDIR:-/dev/null}/_main/tools/sh/bootstrap.sh" 2>/dev/null || source "$0.runfiles/_main/tools/sh/bootstrap.sh" 2>/dev/null || source "${BASH_SOURCE[0]}.runfiles/_main/tools/sh/bootstrap.sh" 2>/dev/null || source "$(git rev-parse --show-toplevel 2>/dev/null)/tools/sh/bootstrap.sh"
dx_bootstrap "tools/sh/lib.sh"
dx_bootstrap "tools/sh/guards.sh"

dx_cd_workspace

dx_test_init

dryrun=".github/workflows/publish-dry-run.yml"

# dist/ and release/ are git-ignored build outputs (never commit).
dx_guards_contains .gitignore ".gitignore lost the /dist/ + /release/ entries" \
  '/dist/' \
  '/release/'

# No dist/ or release/ bytes are committed (rebuilt locally on demand).
if [[ -z "$(git ls-files | grep -E '^(dist|release)/' || true)" ]]; then
  ok
else
  bad "dist/ or release/ paths are committed: $(git ls-files | grep -E '^(dist|release)/' | head -n 5)"
fi

# Atomic version (issue #931, CHANGELOG deleted per #983): at 0.0.0 consumers pin
# reviewed commits, never tags; CHANGELOG.md stays deleted with status in
# docs/product/support-matrix.md and planned work in issues. A recreated
# CHANGELOG at a bumped version must name the new version without the
# no-release marker. Either half alone fails closed (see release_policy.sh
# for the mirrored gate).
dx_guard_re_contains MODULE.bazel '^module\(' "MODULE.bazel lost its module() header"
hygiene_module_version="$(grep -o -E -e '^    version = "[^"]+"' MODULE.bazel | head -1 | cut -d'"' -f2 || true)"
if [[ -z "$hygiene_module_version" ]]; then
  bad "MODULE.bazel lost its version pin (want single-version atomic, issue #931)"
elif [[ "$hygiene_module_version" == "0.0.0" ]]; then
  if [[ ! -e "CHANGELOG.md" ]]; then
    ok
  else
    bad "CHANGELOG.md reappeared (deleted per #983; status lives in docs/product/support-matrix.md)"
  fi
else
  if [[ ! -e "CHANGELOG.md" ]]; then
    ok
  elif grep -q -F -e 'No release has been cut' CHANGELOG.md; then
    bad "CHANGELOG.md still carries the no-release marker with MODULE.bazel at $hygiene_module_version (recreate only at the release gate per #983)"
  elif grep -q -F -e "$hygiene_module_version" CHANGELOG.md && grep -q -E -e '^## ' CHANGELOG.md; then
    ok
  else
    bad "CHANGELOG.md lost its SemVer entry for MODULE.bazel $hygiene_module_version (recreate only at the release gate per #983)"
  fi
fi

# No version tags without explicit owner approval (the unapproved v0.1.0
# candidate was removed; remote stays tag-free, verified via ls-remote).
if [[ -z "$(git tag --list 'v*' || true)" ]]; then
  ok
else
  bad "unapproved version tag present: $(git tag --list 'v*' | head -n 5)"
fi

# SECURITY.md private-vulnerability-report link points at this repo, not
# the upstream-copy leftover (small verification slice,).
dx_guard_contains SECURITY.md 'github.com/ralvik/rules_dx/security/advisories/new' "SECURITY.md reporting link drifted from ralvik/rules_dx"
dx_guard_absent SECURITY.md 'TrapsterDK/rules_dx/security/advisories' "SECURITY.md reporting link drifted from ralvik/rules_dx"

# The publish dry-run never publishes uninvited: workflow_dispatch only,
# no push/pull_request/tag/schedule trigger.
dx_guard_contains "$dryrun" 'workflow_dispatch:' "publish-dry-run.yml gained a non-dispatch trigger"
dx_guard_re_absent "$dryrun" '^  (push|pull_request|schedule):' "publish-dry-run.yml gained a non-dispatch trigger"

# The dry-run approval gate stays explicit and default-closed (issue
# an `approve` input defaulting to false, with nothing publishing
# either way.
dx_guards_contains "$dryrun" "publish-dry-run.yml lost the default-closed approve gate" \
  'approve:' \
  'default: false'

# The dry-run stores no secrets and keeps minimal permissions (issue
# checkout with persist-credentials false, contents read-only.
dx_guards_contains "$dryrun" "publish-dry-run.yml drifted from no-secrets minimal permissions" \
  'persist-credentials: false' \
  'contents: read'

# The dry-run never dirties the checkout: staging under
# RUNNER_TEMP plus a clean-checkout proof step.
dx_guards_contains "$dryrun" "publish-dry-run.yml lost RUNNER_TEMP staging or the clean-checkout proof" \
  'RUNNER_TEMP/publish-dry-run' \
  'test -z "$(git status --porcelain)"'

# The release-hygiene policy itself stays documented (prevents silent
# deletion of the gate this harness enforces).
dx_guard_contains CONTRIBUTING.md 'No tags, GitHub releases' "CONTRIBUTING.md lost the no-tags/no-releases policy"

# The dry-run report names the full release matrix explicitly (issue
# #815): seed linux-x86_64 qualified-built-here, the other three
# (linux-arm64, macos-arm64, windows-x86_64) qualified with per-host
# evidence (frozen in deploy/release/matrix.bzl; macOS x86_64 Not planned
# per #976, removal tracked under #815).
dx_guards_contains "$dryrun" "publish-dry-run.yml lost the explicit release-matrix qualification table" \
  'dx-linux-arm64' \
  'dx-macos-arm64' \
  'dx-windows-x86_64' \
  'qualified-host-evidence'

# SBOM/provenance and BCR submission run owner-gated dry-run-first per
# the dry run exercises //deploy/release:sbom_demo and
# //deploy/release:bcr_demo in dry-run mode, publishing nothing.
dx_guards_contains "$dryrun" "publish-dry-run.yml lost the SBOM/BCR owner-gated exercise (issue #311)" \
  '//deploy/release:sbom_demo' \
  'BCR_DRY_RUN=1 bazel run //deploy/release:bcr_demo'

# Signing-first order stays explicit: Sigstore keyless +
# GitHub attestations on the trust root, GHCR signs separately via
# cosign <digest> under — dry-run here, never publishing.
dx_guard_contains "$dryrun" 'Signing/attestation publishing (Sigstore keyless + GitHub attestations on the trust root' "publish-dry-run.yml lost the signing-first owner-gated record"

# GHCR stays a separate workflow (owner decision,): the dry
# run must name the separate ghcr.yml route, never fold images here.
dx_guard_contains "$dryrun" 'GHCR prebuilt images (separate workflow .github/workflows/ghcr.yml' "publish-dry-run.yml lost the GHCR-separate note (issue #460)"

# Third-party actions stay SHA-pinned: the sole third-party
# action (checkout) must pin to a commit SHA, never float a tag.
dx_guard_re_contains "$dryrun" 'uses: actions/checkout@[0-9a-f]{40}' "publish-dry-run.yml lost the checkout SHA pin or gained a docker/* action"
dx_guard_absent "$dryrun" 'uses: docker/' "publish-dry-run.yml lost the checkout SHA pin or gained a docker/* action"

# The approve gate stays typed and non-cancelling: boolean
# input type plus concurrency cancel-in-progress false so overlapping
# dispatches queue instead of cancelling the qualification run.
dx_guards_contains "$dryrun" "publish-dry-run.yml lost the typed approve gate or non-cancelling concurrency" \
  'type: boolean' \
  'cancel-in-progress: false'

# The dry-run stays least-privilege: no packages:write
# (only ghcr.yml needs packages:write for image push; the dry run
# never publishes, so contents:read is sufficient).
dx_guard_absent "$dryrun" 'packages: write' "publish-dry-run.yml gained packages:write (dry run must stay read-only; push lives in ghcr.yml)"

# The dry-run uses no secrets at all: no `secrets.`
# reference (the gated GHCR push alone uses GITHUB_TOKEN under;
# the dry run only builds locally and reports, so any secret reference
# would be an unreviewed publication input).
dx_guard_absent "$dryrun" 'secrets.' "publish-dry-run.yml gained a secrets reference (dry run must use no secrets)"

# Sole tracker stays sole: the local `issues/` mirror and
# `archive/delivery-v0.1/` were deleted; delivery history lives in git
# history. Neither may reappear on disk nor committed.
if [[ ! -e "issues" ]] && [[ ! -e "archive/delivery-v0.1" ]] && [[ -z "$(git ls-files | grep -E '^(issues|archive/delivery-v0\.1)/' || true)" ]]; then
  ok
else
  bad "issues/ mirror or archive/delivery-v0.1/ reappeared (sole tracker is issue #5)"
fi

# SECURITY.md reporting stays enabled
# public release): the static half records that private vulnerability
# reporting is enabled; live API verification remains a manual gap
# recorded in the issue, never claimed here.
dx_guard_contains SECURITY.md 'Private vulnerability reporting is enabled' "SECURITY.md lost the private-reporting-enabled record (precondition per issue #5)"

# Consumer-CI caller template pins the reusable workflow at a reviewed
# commit SHA (context): never a tag or floating ref, so
# consumers track qualified commits while MODULE stays at 0.0.0.
dx_guard_re_contains examples/consumer-ci/caller.yml 'reusable-consumer\.yml@[0-9a-f]{40}' "examples/consumer-ci/caller.yml lost its reviewed-commit SHA pin (issue #5)"

# Docs-CI caller template pins the reusable docs workflow at a reviewed
# commit SHA (context, CONTRIBUTING.md both-callers policy):
# never a tag or floating ref, so docs consumers track the same
# qualified commit as consumer-ci (sync enforced by
# //tools/ci:examples_pins_test; this guard keeps the release-hygiene
# track self-contained).
dx_guard_re_contains examples/docs-ci/caller.yml 'reusable-docs\.yml@[0-9a-f]{40}' "examples/docs-ci/caller.yml lost its reviewed-commit SHA pin (issue #5)"

# CONTRIBUTING both-callers pin policy stays documented
# context): caller pins stay frozen at the last qualified commit, stay
# in sync across both example callers, and move only together — so the
# SHA-pin guards above cannot be silently redefined as floating tags.
dx_guards_contains CONTRIBUTING.md "CONTRIBUTING.md lost the both-callers pin policy (issue #5)" \
  'pin reviewed commits, never release tags' \
  'stay in sync across both example callers'

# The dry-run report stays explicit that it creates nothing
# policy: no tags, GitHub releases, or registry submissions without
# explicit owner approval): the not_attempted list must name that any
# tag, registry submission, or release creation is out of scope, so the
# nothing-publishes ceiling cannot be silently narrowed to only the
# tooling deferrals above.
dx_guard_contains "$dryrun" 'Any tag, registry submission, or release creation' "publish-dry-run.yml lost the no-tag/no-release/no-submission record (issue #5)"

# Published bytes are never rebuilt or substituted silently
# next-steps): the policy stays recorded in CONTRIBUTING.md (CHANGELOG.md
# deleted per #983), so the approval gate cannot be read as allowing a quiet
# byte swap after approval.
dx_guards_contains CONTRIBUTING.md "CONTRIBUTING.md lost the never-rebuild-or-substitute record (issue #5)" \
  'never' \
  'rebuilt or substituted silently'

# Byte identity stays fail-closed (never-rebuild mechanism):
# the artifact generator rejects changed upstream bytes instead of
# silently recording new content, and the dry run records the seed
# binary sha256 digest, so a substituted byte cannot pass as the same
# release.
dx_guards_contains quality/artifacts/update.py "byte-identity fail-closed record lost (update.py, issue #5)" \
  'instead of silently recording new content' \
  'does not match published'
dx_guard_contains "$dryrun" 'sha256' "byte-identity fail-closed record lost (dry-run sha256, issue #5)"

# Seed standalone packaging stays exercised (seed-qualified):
# the dry run builds //cli/cli:dx_standalone and stages the tarball plus
# checksum under RUNNER_TEMP, never committed; the wider matrix is frozen
# in deploy/release/matrix.bzl, qualified per-host under issue #815.
dx_guards_contains "$dryrun" "publish-dry-run.yml lost the seed standalone exercise (dx_standalone + tarball + checksum, issue #78)" \
  '//cli/cli:dx_standalone' \
  'dx-standalone.tar.gz' \
  'dx-standalone.tar.gz.sha256'

# Draft creation stays exercised without publishing (draft-only):
# the dry run runs //cli/cli:github_draft with GH_RELEASE_DRY_RUN=1 and
# proves the placeholder plus draft-only flags, publishing nothing.
dx_guards_contains "$dryrun" "publish-dry-run.yml lost the draft dry-run exercise (github_draft + GH_RELEASE_DRY_RUN=1 + draft-only flags, issue #78)" \
  'GH_RELEASE_DRY_RUN=1 bazel run //cli/cli:github_draft' \
  'v0.0.0-dryrun' \
  '--draft --verify-tag'

# BCR shape stays checked-not-submitted (owner-gated):
# the dry run runs //deploy/release:bcr_demo in BCR_DRY_RUN=1 mode and
# records checked-not-submitted without submitting.
dx_guards_contains "$dryrun" "publish-dry-run.yml lost the BCR owner-gated record (bcr_demo + BCR_DRY_RUN=1 + submitted False, issue #311)" \
  'bcr-shape.txt' \
  'BCR_DRY_RUN=1 bazel run //deploy/release:bcr_demo' \
  '"submitted": False'

# Install-verifier refusal stays proved (signing-first):
# the dry run proves //deploy/install:dx_verify refuses checksum-only
# inputs on the TUF trust root and installs nothing, without network.
dx_guards_contains "$dryrun" "publish-dry-run.yml lost the verifier-refusal exercise (dx_verify checksum-only refused, issue #78)" \
  'bazel run //deploy/install:dx_verify' \
  'verify-refusal.log' \
  'checksum-only verification is not publisher-identity proof'

# Release tests stay exercised: the dry run runs
# //deploy/release:all green, proving matrix + SBOM + signing + BCR +
# human-run gates without publishing.
dx_guard_contains "$dryrun" 'bazel test //deploy/release:all' "publish-dry-run.yml lost the release-tests exercise (//deploy/release:all, issue #311)"

# Signing dry-run stays exercised
# successor to closed for the human-run path): the dry run
# runs //deploy/release:signing_demo with RELEASE_SIGN_DRY_RUN=1 and
# proves the trust root plus would-sign, publishing nothing.
dx_guards_contains "$dryrun" "publish-dry-run.yml lost the signing dry-run exercise (signing_demo + RELEASE_SIGN_DRY_RUN=1, issue #311)" \
  'RELEASE_SIGN_DRY_RUN=1 bazel run //deploy/release:signing_demo' \
  'tuf-repo-cdn.sigstore.dev'

# Human-run driver stays exercised (live successor to closed
# for the human-run path): the dry run runs
# //deploy/release:release_driver in dry-run mode, proving the tag ceiling
# plus owner-approval gate with nothing published.
dx_guards_contains "$dryrun" "publish-dry-run.yml lost the human-run driver exercise (release_driver dry run, issue #458)" \
  'bazel run //deploy/release:release_driver' \
  'human-run-dry-run.log'

# Release runbook stays owned (live successor to closed
# for the human-run path): the human-run path is
# documented, not just workflow steps.
dx_guard_file docs/deploy/release-runbook.md "release runbook missing (docs/deploy/release-runbook.md, issue #458)"
dx_guard_contains docs/deploy/release-runbook.md 'never creates or pushes tags' "release runbook missing tag ceiling (docs/deploy/release-runbook.md, issue #458)"

# Review routing stays owned: CODEOWNERS exists with the sole
# maintainer owning every row per the support-matrix core section, and the
# consumer-CI contract records that this repo's own routing is owned while
# the integration prescribes no consumer CODEOWNERS policy.
dx_guard_file CODEOWNERS "CODEOWNERS missing (want CODEOWNERS with @ralvik, issue #424)"
dx_guards_contains CODEOWNERS "CODEOWNERS lost the sole-maintainer record (want @ralvik plus sole maintainer, issue #424)" \
  '@ralvik' \
  'sole maintainer'
dx_guards_contains docs/github-ci.md "docs/github-ci.md lost the sole-maintainer record (want CODEOWNERS plus sole maintainer, issue #424)" \
  'CODEOWNERS' \
  'sole maintainer'

dx_test_summary "release hygiene harness"
