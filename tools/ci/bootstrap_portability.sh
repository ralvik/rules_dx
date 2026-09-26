#!/usr/bin/env bash
# CI bootstrap portability harness.
#
# Machine-checks the as-built portable Bazel bootstrap with fixture
# evidence and owned gaps, without claiming Supported or Windows arm64:
# - delivered: every workflow boots Bazel through the single SHA-pinned
#   `bazel-contrib/setup-bazel` action (commit SHA plus trailing tag
#   comment) driven by the single-sourced `BAZELISK_VERSION` env with
#   `bazelisk-cache` on, and no workflow embeds a Bazelisk download;
#   job configuration stays runner-temp scoped (RUNNER_TEMP rc plus
#   GITHUB_ENV export, never a system install); curl hardening extends
#   to the Dockerfile plus ghcr cosign fetches with --retry everywhere
#   (issue #932);
# - pins: version plus linux-amd64 sha256 stay canonical in
#   `.devcontainer/Dockerfile.prebuilt` (checked by
#   //tools/ci:pin_consistency_test); Dockerfile tracks the
#   linux-amd64 pair; docs bootstrap records all five per-OS sha256s;
# - docs in place: `docs/contributing/local-workflows.md` owns the
#   copy-paste bootstrap (five-host asset map with fail-closed refusal,
#   bounded retry, portable checksum plus no-sudo install);
# - CI only: no product runtime change; Windows arm64 stays the
#   unqualified gap with clean refusal.
#
# Versioned here, run by CI via `bazel run //tools/ci:bootstrap_portability`,
# following //tools/ci:ci_matrix_qualification.
set -euo pipefail

# Shared workspace + runfiles helpers.
# Bootstrap via tools/sh/bootstrap.sh (issue #654): runfiles forest first, then source tree.
source "${RUNFILES_DIR:-/dev/null}/_main/tools/sh/bootstrap.sh" 2>/dev/null || source "${TEST_SRCDIR:-/dev/null}/_main/tools/sh/bootstrap.sh" 2>/dev/null || source "$0.runfiles/_main/tools/sh/bootstrap.sh" 2>/dev/null || source "${BASH_SOURCE[0]}.runfiles/_main/tools/sh/bootstrap.sh" 2>/dev/null || source "$(git rev-parse --show-toplevel 2>/dev/null)/tools/sh/bootstrap.sh"
dx_bootstrap "tools/sh/lib.sh"

dx_cd_workspace

dx_test_init

docs="docs/contributing/local-workflows.md"
dockerfile=".devcontainer/Dockerfile.prebuilt"
ghcr=".github/workflows/ghcr.yml"

# Single Bazel bootstrap path: every setup-bazel step is the full-SHA
# pin with its trailing release-tag comment, takes the single-sourced
# Bazelisk version, and keeps its cache on; a bare tag pin, a missing
# version argument, or a dropped setup step fails closed. Eighteen such
# steps exist today (ci 6 plus reusable-consumer 9 plus bump plus
# publish-dry-run plus reusable-docs).
setup_ok=1
setup_total=0
for wf in .github/workflows/*.yml; do
  raw="$(grep -c -F -e 'uses: bazel-contrib/setup-bazel@' "$wf" || true)"
  pinned="$(grep -c -E -e 'uses: bazel-contrib/setup-bazel@[0-9a-f]{40} # v[0-9]+\.[0-9]+\.[0-9]+' "$wf" || true)"
  if [[ "$raw" != "$pinned" ]]; then
    setup_ok=0
  fi
  if [[ "$raw" != "0" ]]; then
    versioned="$(grep -c -F -e 'bazelisk-version: ${{ env.BAZELISK_VERSION }}' "$wf" || true)"
    cached="$(grep -c -F -e 'bazelisk-cache: true' "$wf" || true)"
    if [[ "$versioned" -lt "$raw" || "$cached" -lt "$raw" ]]; then
      setup_ok=0
    fi
  fi
  setup_total=$((setup_total + raw))
done
if [[ "$setup_ok" == "1" && "$setup_total" -ge "18" ]]; then
  ok
else
  bad "workflows lost the SHA-pinned setup-bazel bootstrap (want bazel-contrib/setup-bazel@<40-hex> plus # vX.Y.Z tag plus BAZELISK_VERSION plus bazelisk-cache, issue #617)"
fi

# Curl hardening extends beyond the bootstrap action (issue #932): the
# Dockerfile Bazelisk fetch plus the ghcr cosign fetch carry the same
# fail-closed retry flags, not just presence.
if grep -q -F -e 'curl -fsSL' "$dockerfile" &&
  grep -q -F -e '--retry 3 --retry-delay 2' "$dockerfile" &&
  grep -q -F -e 'curl -fsSL' "$ghcr" &&
  grep -q -F -e '--retry 3 --retry-delay 2' "$ghcr"; then
  ok
else
  bad "Dockerfile.prebuilt or ghcr.yml lost hardened curl flags (want -fsSL plus --retry 3 --retry-delay 2, issue #932)"
fi

# No bare curl without retry survives in the fetch sites: every curl
# invocation (curl with flags) in the Dockerfile plus ghcr cosign fetch
# carries --retry (fail-closed on flag drift, not presence).
if grep -h -o -E -e 'curl -[^|;&]*' "$dockerfile" "$ghcr" 2>/dev/null | grep -q -F -e 'curl -' &&
  ! grep -h -o -E -e 'curl -[^|;&]*' "$dockerfile" "$ghcr" 2>/dev/null | grep -v -F -e '--retry' | grep -v -F -e 'curl --version' | grep -q .; then
  ok
else
  bad "a bare curl without --retry survives in Dockerfile.prebuilt or ghcr.yml (want --retry everywhere, issue #932)"
fi

# Job configuration stays runner-temp scoped: the BuildBuddy rc lands
# under RUNNER_TEMP and joins the environment through GITHUB_ENV, so no
# workflow mutates a system path or needs a privileged install.
if grep -q -F -e 'rc="${RUNNER_TEMP}/buildbuddy.bazelrc"' .github/workflows/ci.yml &&
  grep -q -F -e 'echo "BAZELRC=${rc}" >> "${GITHUB_ENV}"' .github/workflows/ci.yml &&
  grep -q -F -e 'rc="${RUNNER_TEMP}/buildbuddy.bazelrc"' .github/workflows/reusable-consumer.yml &&
  grep -q -F -e 'echo "BAZELRC=${rc}" >> "${GITHUB_ENV}"' .github/workflows/reusable-consumer.yml; then
  ok
else
  bad "workflows lost the runner-temp rc plus GITHUB_ENV export (want RUNNER_TEMP buildbuddy.bazelrc plus BAZELRC env, no system install, issue #617)"
fi

# No per-OS copy-paste: workflows never embed a Bazelisk download URL
# or a `curl ... bazelisk` install; the pinned setup-bazel action owns
# installation (Dockerfile plus docs are the only tracked copies; ghcr
# admissibility greps the Dockerfile, not a second installer).
if ! grep -rn -F -e 'bazelisk/releases/download' --include='*.yml' .github/workflows/ 2>/dev/null | grep -q . &&
  ! grep -rn -i -E -e 'curl.*bazelisk' --include='*.yml' .github/workflows/ 2>/dev/null | grep -q .; then
  ok
else
  bad "a workflow embedded a Bazelisk download outside the SHA-pinned setup-bazel action (issue #617)"
fi

# Docs in place: the copy-paste bootstrap is portable over all five
# assets plus shas with a fail-closed host refusal (Windows arm64 plus
# unknown hosts exit 1), bounded retry, portable hash comparison with a
# checksum-mismatch refusal, and no sudo install.
if grep -q -F -e 'bazelisk-linux-amd64' "$docs" &&
  grep -q -F -e 'bazelisk-linux-arm64' "$docs" &&
  grep -q -F -e 'bazelisk-darwin-amd64' "$docs" &&
  grep -q -F -e 'bazelisk-darwin-arm64' "$docs" &&
  grep -q -F -e 'bazelisk-windows-amd64.exe' "$docs" &&
  grep -q -F -e 'Pinned Bazelisk launcher (portable' "$docs" &&
  grep -q -F -e 'issue #617' "$docs" &&
  grep -q -F -e 'unsupported host' "$docs" &&
  grep -q -F -e 'download attempt $i/3 failed' "$docs" &&
  grep -q -F -e 'checksum mismatch' "$docs" &&
  grep -q -F -e '--retry 3' "$docs" &&
  grep -q -F -e 'shasum -a 256' "$docs" &&
  grep -q -F -e '$HOME/.local/bin' "$docs" &&
  ! grep -q -F -e 'sudo install' "$docs"; then
  ok
else
  bad "local-workflows.md lost the portable bootstrap record (five assets plus refusal plus retry plus portable hash plus no-sudo, issue #617)"
fi

# Bootstrap git probe stays bounded (issue #932): `timeout` guards the
# rev-parse with a fail-open fallback to the source-tree walk, and the
# budget is documented in the bootstrap header.
bootstrap="tools/sh/bootstrap.sh"
if grep -q -F -e 'DX_BOOTSTRAP_TIMEOUT' "$bootstrap" &&
  grep -q -F -e 'command -v timeout' "$bootstrap" &&
  grep -q -F -e 'git rev-parse --show-toplevel' "$bootstrap" &&
  grep -q -F -e 'Budget (issue #932)' "$bootstrap"; then
  ok
else
  bad "tools/sh/bootstrap.sh lost its bounded git probe (want DX_BOOTSTRAP_TIMEOUT plus timeout guard with fail-open walk, issue #932)"
fi

dx_test_summary "bootstrap portability harness"
