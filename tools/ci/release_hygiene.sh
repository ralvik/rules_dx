#!/usr/bin/env bash
# Release-hygiene harness (issue #5).
#
# No release has been cut. Tags, GitHub releases, registry submissions, and
# publication outputs require explicit owner approval (see CONTRIBUTING.md):
# `dist/` and `release/` are git-ignored build outputs and must never be
# committed, `MODULE.bazel` keeps `version = "0.0.0"` as a module version
# string only, and the publish dry-run stays `workflow_dispatch`-only so CI
# stays green by construction and nothing runs uninvited.
#
# This harness machine-checks the no-publication-inputs half that is
# verifiable on a clean tree today. Platform, packaging, provenance
# (SPDX/SLSA), registry submission, and public-install smoke runs stay
# unqualified per #5 and are recorded as gaps, not claimed here.
#
# Versioned here, run by CI via `bazel run //tools/ci:release_hygiene`,
# following //tools/ci:corpus_audit.
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

# dist/ and release/ are git-ignored build outputs (never commit).
if grep -q -F -e '/dist/' .gitignore && grep -q -F -e '/release/' .gitignore; then
  ok
else
  bad ".gitignore lost the /dist/ + /release/ entries"
fi

# No dist/ or release/ bytes are committed (rebuilt locally on demand).
if [[ -z "$(git ls-files | grep -E '^(dist|release)/' || true)" ]]; then
  ok
else
  bad "dist/ or release/ paths are committed: $(git ls-files | grep -E '^(dist|release)/' | head -n 5)"
fi

# The module stays at 0.0.0; consumers pin reviewed commits, never tags.
if grep -q -E -e '^module\(|version = "0\.0\.0"' MODULE.bazel && ! grep -q -E -e 'version = "0\.1\.0"' MODULE.bazel; then
  ok
else
  bad "MODULE.bazel drifted from version 0.0.0"
fi

# No version tags without explicit owner approval (the unapproved v0.1.0
# candidate was removed; remote stays tag-free, verified via ls-remote).
if [[ -z "$(git tag --list 'v*' || true)" ]]; then
  ok
else
  bad "unapproved version tag present: $(git tag --list 'v*' | head -n 5)"
fi

# SECURITY.md private-vulnerability-report link points at this repo, not
# the upstream-copy leftover (small verification slice, issue #5).
if grep -q -F -e 'github.com/ralvik/rules_dx/security/advisories/new' SECURITY.md && ! grep -q -F -e 'TrapsterDK/rules_dx/security/advisories' SECURITY.md; then
  ok
else
  bad "SECURITY.md reporting link drifted from ralvik/rules_dx"
fi

# The publish dry-run never publishes uninvited: workflow_dispatch only,
# no push/pull_request/tag/schedule trigger.
if grep -q -F -e 'workflow_dispatch:' .github/workflows/publish-dry-run.yml && ! grep -q -E -e '^  (push|pull_request|schedule):' .github/workflows/publish-dry-run.yml; then
  ok
else
  bad "publish-dry-run.yml gained a non-dispatch trigger"
fi

# The release-hygiene policy itself stays documented (prevents silent
# deletion of the gate this harness enforces).
if grep -q -F -e 'No tags, GitHub releases' CONTRIBUTING.md; then
  ok
else
  bad "CONTRIBUTING.md lost the no-tags/no-releases policy"
fi

echo "release hygiene harness: $pass passed, $fail failed"
[[ "$fail" == "0" ]]
