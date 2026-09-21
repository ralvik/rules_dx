#!/usr/bin/env bash
# Deploy program for `github_deploy`.
#
# Invoked via `bazel run :<name>` or `dx deploy :<name>`. The generated
# launcher resolves the tag's asset files from its runfiles forest and
# execs this script with the tag as `$1` and the asset paths after it.
# Draft-only by construction: the real path always passes
# `--draft --verify-tag`, so the program never creates or pushes tags
# itself — the tag must already exist in the remote, pushed beforehand
# with explicit owner approval. With `GH_RELEASE_DRY_RUN=1` the script
# prints the command it would run and publishes nothing; this is what
# CI exercises, so the macro stays green without network access.
set -euo pipefail

tag="$1"
shift

if [[ "${GH_RELEASE_DRY_RUN:-}" == "1" ]]; then
  echo "github_deploy: dry run (GH_RELEASE_DRY_RUN=1); would create a draft release, publishing nothing:"
  echo "  tag: ${tag}"
  for asset in "$@"; do
    echo "  asset: $(basename "${asset}") (${asset})"
  done
  printf '  command: gh release create %s' "${tag}"
  for asset in "$@"; do
    printf ' %s' "${asset}"
  done
  printf ' --draft --verify-tag\n'
  exit 0
fi

if ! command -v gh >/dev/null 2>&1; then
  echo "github_deploy: 'gh' CLI not found on PATH; install it to publish draft releases" >&2
  exit 1
fi

exec gh release create "${tag}" "$@" --draft --verify-tag
