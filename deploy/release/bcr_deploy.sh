#!/usr/bin/env bash
# BCR publisher for `bcr_check`.
#
# Invoked via `bazel run :<name>` with module + version + pinned input
# paths. Owner-gated, dry-run-first: with `BCR_DRY_RUN=1` prints the
# would-submit PR (what CI exercises, submits nothing). A real submission
# needs an owner-approved SemVer version (never 0.0.0) plus explicit
# approval per docs/deploy/release-runbook.md; without approval it fails
# closed and submits nothing.
set -euo pipefail

module="$1"
version="$2"
shift 2

if [[ "$module" != "rules_dx" ]]; then
  echo "bcr: invalid module '$module': want 'rules_dx'" >&2
  exit 1
fi
if [[ "$version" == "0.0.0" && "${BCR_DRY_RUN:-}" != "1" ]]; then
  echo "bcr: version 0.0.0 is unpublishable (shape check only); a real submission needs an owner-approved SemVer release version" >&2
  exit 1
fi

if [[ "${BCR_DRY_RUN:-}" == "1" ]]; then
  echo "bcr: dry run (BCR_DRY_RUN=1); would submit, submitting nothing:"
  echo "  module: ${module}"
  echo "  version: ${version}"
  for input in "$@"; do
    echo "  input: $(basename "${input}") (${input})"
  done
  printf '  command: gh pr create --repo bazelbuild/bazel-central-registry --title "Add %s@%s" --body "Owner-approved BCR submission for %s@%s"\n' "${module}" "${version}" "${module}" "${version}"
  echo "  presubmit: bazel test @rules_dx//... (seed host) + BCR presubmit.yml"
  exit 0
fi

if [[ "${BCR_APPROVE:-}" != "1" ]]; then
  echo "bcr: submission needs explicit owner approval per issue #5 (BCR_APPROVE=1); run with BCR_DRY_RUN=1 to print the would-submit PR" >&2
  exit 1
fi
if [[ "$version" == "0.0.0" ]]; then
  echo "bcr: version 0.0.0 is unpublishable even with approval" >&2
  exit 1
fi

echo "bcr: owner-approved submission for ${module}@${version} (human-run path only; see docs/deploy/release-runbook.md)"
for input in "$@"; do
  echo "  input: $(basename "${input}") (${input})"
done
echo "bcr: open the BCR PR manually with the inputs above (this program never pushes itself)"
