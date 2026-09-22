#!/usr/bin/env bash
# CI bootstrap portability harness.
#
# Machine-checks the as-built portable Bazelisk bootstrap with fixture
# evidence and owned gaps, without claiming Supported or Windows arm64:
# - delivered: single `.github/actions/setup-bazelisk` composite action
#   covers Linux x86_64 plus Linux arm64 plus macOS arm64 plus macOS x86_64
#   plus Windows x86_64 with retry and no sudo assumption; OS/arch resolve
#   from RUNNER_OS/RUNNER_ARCH with a uname fallback; download retries;
#   checksum verifies via sha256sum with shasum plus python3 fallbacks;
#   install lands under RUNNER_TEMP without sudo and joins GITHUB_PATH;
#   per-OS copy-paste is rejected; curl hardening extends to the
#   Dockerfile plus ghcr cosign fetches with --retry everywhere (issue #932);
# - pins: version plus per-OS sha256 inputs stay canonical in action.yml
#   (checked by //tools/ci:pin_consistency_test); Dockerfile tracks the
#   linux-amd64 pair; docs bootstrap documents all five hosts;
# - docs in place: `docs/contributing/local-workflows.md` bootstrap is
#   portable with retry plus checksum plus no-sudo install;
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

action=".github/actions/setup-bazelisk/action.yml"
docs="docs/contributing/local-workflows.md"
dockerfile=".devcontainer/Dockerfile.prebuilt"
ghcr=".github/workflows/ghcr.yml"

# Single portable action owns the Bazelisk install: per-OS sha inputs exist,
# legacy single sha256 stays absent.
if grep -q -F -e 'sha256_linux_amd64:' "$action" &&
  grep -q -F -e 'sha256_linux_arm64:' "$action" &&
  grep -q -F -e 'sha256_darwin_amd64:' "$action" &&
  grep -q -F -e 'sha256_darwin_arm64:' "$action" &&
  grep -q -F -e 'sha256_windows_amd64:' "$action" &&
  ! grep -A3 -E -e '^  sha256:' "$action" | grep -q -F -e 'default:'; then
  ok
else
  bad "setup-bazelisk lost per-OS sha256 inputs or kept legacy single sha256 (issue #617)"
fi

# OS/arch map covers all five qualified hosts with the exact upstream assets
# and fails closed on Windows ARM64.
if grep -q -F -e 'Linux/X64) asset="bazelisk-linux-amd64"' "$action" &&
  grep -q -F -e 'Linux/ARM64) asset="bazelisk-linux-arm64"' "$action" &&
  grep -q -F -e 'macOS/X64) asset="bazelisk-darwin-amd64"' "$action" &&
  grep -q -F -e 'macOS/ARM64) asset="bazelisk-darwin-arm64"' "$action" &&
  grep -q -F -e 'Windows/X64) asset="bazelisk-windows-amd64.exe"' "$action" &&
  grep -q -F -e 'Windows ARM64 is out of scope' "$action" &&
  grep -q -F -e 'RUNNER_OS' "$action" &&
  grep -q -F -e 'RUNNER_ARCH' "$action" &&
  grep -q -F -e 'uname -s' "$action"; then
  ok
else
  bad "setup-bazelisk lost the five-host OS/arch map with uname fallback plus Windows-ARM64 refusal (issue #617)"
fi

# Retry stays bounded: curl retry flags plus an outer 3-attempt loop.
if grep -q -F -e 'curl -fsSL --retry 3' "$action" &&
  grep -q -F -e 'download attempt' "$action" &&
  grep -q -F -e 'download failed after 3 attempts' "$action"; then
  ok
else
  bad "setup-bazelisk lost bounded download retry (curl --retry plus 3-attempt loop, issue #617)"
fi

# Curl hardening extends beyond the action (issue #932): the Dockerfile
# Bazelisk fetch plus the ghcr cosign fetch carry the same fail-closed
# retry flags, not just presence.
if grep -q -F -e 'curl -fsSL' "$dockerfile" &&
  grep -q -F -e '--retry 3 --retry-delay 2' "$dockerfile" &&
  grep -q -F -e 'curl -fsSL' "$ghcr" &&
  grep -q -F -e '--retry 3 --retry-delay 2' "$ghcr"; then
  ok
else
  bad "Dockerfile.prebuilt or ghcr.yml lost hardened curl flags (want -fsSL plus --retry 3 --retry-delay 2, issue #932)"
fi

# No bare curl without retry survives in the fetch sites: every curl
# invocation (curl with flags) in the action plus Dockerfile plus ghcr
# cosign fetch carries --retry (fail-closed on flag drift, not presence).
if grep -h -o -E -e 'curl -[^|;&]*' "$action" "$dockerfile" "$ghcr" 2>/dev/null | grep -q -F -e 'curl -' &&
  ! grep -h -o -E -e 'curl -[^|;&]*' "$action" "$dockerfile" "$ghcr" 2>/dev/null | grep -v -F -e '--retry' | grep -q .; then
  ok
else
  bad "a bare curl without --retry survives in setup-bazelisk, Dockerfile.prebuilt, or ghcr.yml (want --retry everywhere, issue #932)"
fi

# No sudo assumption: no sudo command in executable lines (comments may name
# it as the banned form), no /usr/local/bin install; install lands under
# RUNNER_TEMP and joins GITHUB_PATH via cp plus chmod.
if ! grep -E -e '^[^#]*\bsudo\b' "$action" | grep -q . &&
  ! grep -q -F -e '/usr/local/bin/bazel' "$action" &&
  grep -q -F -e 'RUNNER_TEMP' "$action" &&
  grep -q -F -e 'GITHUB_PATH' "$action" &&
  grep -q -F -e 'cp -f' "$action" &&
  grep -q -F -e 'chmod +x' "$action"; then
  ok
else
  bad "setup-bazelisk kept a sudo or /usr/local/bin assumption or lost RUNNER_TEMP plus GITHUB_PATH install (issue #617)"
fi

# Portable checksum: sha256sum with shasum plus python3 fallbacks, compared
# as strings (no bare sha256sum -c assumption).
if grep -q -F -e 'command -v sha256sum' "$action" &&
  grep -q -F -e 'shasum -a 256' "$action" &&
  grep -q -F -e 'hashlib.sha256' "$action" &&
  grep -q -F -e 'checksum mismatch' "$action" &&
  ! grep -q -F -e 'sha256sum -c' "$action"; then
  ok
else
  bad "setup-bazelisk lost portable checksum verification (sha256sum plus shasum plus python3, no sha256sum -c, issue #617)"
fi

# Windows exe handling: .exe asset installs as bazel.exe.
if grep -q -F -e 'bazelisk-windows-amd64.exe' "$action" &&
  grep -q -F -e 'out_name="bazel.exe"' "$action" &&
  grep -q -F -e 'out_name="bazel"' "$action"; then
  ok
else
  bad "setup-bazelisk lost Windows .exe handling (bazelisk-windows-amd64.exe to bazel.exe, issue #617)"
fi

# No per-OS copy-paste: workflows never embed a Bazelisk download URL;
# the single action owns it (Dockerfile plus docs are the only tracked
# copies; ghcr admissibility greps the Dockerfile, not a second installer).
if ! grep -rn -F -e 'bazelisk/releases/download' --include='*.yml' .github/workflows/ 2>/dev/null | grep -q . &&
  ! grep -rn -i -E -e 'curl.*bazelisk' --include='*.yml' .github/workflows/ 2>/dev/null | grep -v -F -e 'setup-bazelisk' | grep -q .; then
  ok
else
  bad "a workflow embedded a Bazelisk download outside the single portable action (issue #617)"
fi

# Docs in place: portable bootstrap documents all five assets plus shas
# with retry plus portable hash plus no-sudo install, and no sudo remains.
if grep -q -F -e 'bazelisk-linux-amd64' "$docs" &&
  grep -q -F -e 'bazelisk-linux-arm64' "$docs" &&
  grep -q -F -e 'bazelisk-darwin-amd64' "$docs" &&
  grep -q -F -e 'bazelisk-darwin-arm64' "$docs" &&
  grep -q -F -e 'bazelisk-windows-amd64.exe' "$docs" &&
  grep -q -F -e 'Pinned Bazelisk launcher (portable' "$docs" &&
  grep -q -F -e 'issue #617' "$docs" &&
  grep -q -F -e '--retry 3' "$docs" &&
  grep -q -F -e 'shasum -a 256' "$docs" &&
  grep -q -F -e '$HOME/.local/bin' "$docs" &&
  ! grep -q -F -e 'sudo install' "$docs"; then
  ok
else
  bad "local-workflows.md lost the portable bootstrap record (five assets plus retry plus portable hash plus no-sudo, issue #617)"
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
