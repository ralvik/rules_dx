#!/usr/bin/env bash
# Devcontainer boot manual plus wont-fix qualification harness.
#
# Owns the shape-only devcontainer check left with boot as an open gap:
# - seed boot: linux/amd64 manual verify only (devcontainer CLI build plus
#   postCreateCommand execution asserting the managed environment
#   materializes: `bazel run //dx:env` then `dx setup`, no full build);
#   CI stays parity plus definition shape only;
# - procedure: local `docker build` plus `devcontainer up` on every change
#   to `.devcontainer/Dockerfile.prebuilt` or
#   `.devcontainer/devcontainer.json` plus before any gated GHCR push,
#   with evidence in the same reviewed PR; sole maintainer owns every row
#   until delegation;
# - wont-fix: non-Linux runs plus linux/arm64 boot plus arm64 prebuilt
#   variant (scaffold json arch-independent, amd64-only seed slice pins
#   the amd64 Bazelisk launcher; natively qualified per #410-#414, not
#   container boot); no multi-platform container support claimed;
# - qualification: locally or on demand with customer flows only (bazel
#   run harness plus on-demand local docker plus devcontainer up, builds
#   nothing, boots nothing here);
# - rejected: boot job in CI, non-customer; no extra CI job; keep CI
#   customer-only;
# - fixtures: `tools/ci/tests/fixtures/devcontainer_boot/` (`pins.bzl`
#   plus `devcontainer_boot.expected`) pins manual plus wont-fix plus
#   rejected plus honesty;
# - scope: infra only. Seed only: no Supported claim.
#
# Versioned here, run on demand via `bazel run //tools/ci:devcontainer_boot_qualification`,
# following //tools/ci:ghcr_rebuild_rotation_qualification.
set -euo pipefail

# Shared workspace + runfiles helpers.
# Bootstrap via tools/sh/bootstrap.sh (issue #654): runfiles forest first, then source tree.
source "${RUNFILES_DIR:-/dev/null}/_main/tools/sh/bootstrap.sh" 2>/dev/null || source "${TEST_SRCDIR:-/dev/null}/_main/tools/sh/bootstrap.sh" 2>/dev/null || source "$0.runfiles/_main/tools/sh/bootstrap.sh" 2>/dev/null || source "${BASH_SOURCE[0]}.runfiles/_main/tools/sh/bootstrap.sh" 2>/dev/null || source "$(git rev-parse --show-toplevel 2>/dev/null)/tools/sh/bootstrap.sh"
dx_bootstrap "tools/sh/lib.sh"

dx_cd_workspace

dx_test_init

dx_bash_pin

pins="tools/ci/tests/fixtures/devcontainer_boot/pins.bzl"
expected="tools/ci/tests/fixtures/devcontainer_boot/devcontainer_boot.expected"
fixture_build="tools/ci/tests/fixtures/devcontainer_boot/BUILD.bazel"
contract="docs/contributing/devcontainer.md"
test_matrix="docs/testing/github-ci.md"
build="tools/ci/BUILD.bazel"
ci=".github/workflows/ci.yml"
dockerfile=".devcontainer/Dockerfile.prebuilt"
definition=".devcontainer/devcontainer.json"
ghcr=".github/workflows/ghcr.yml"

# Fixture files stay present.
if [[ -f "$pins" && -f "$expected" && -f "$fixture_build" ]]; then
  ok
else
  bad "devcontainer boot fixture missing (want $pins plus devcontainer_boot.expected plus BUILD.bazel)"
fi

# Pins record the seed-host manual boot scope (linux/amd64, CLI plus postCreate, no full build, CI shape-only).
if grep -q -F -e 'linux/amd64 seed-host boot only' "$pins" &&
  grep -q -F -e 'devcontainer CLI build plus postCreateCommand execution asserting the managed environment materializes' "$pins" &&
  grep -q -F -e 'bazel run //dx:env then dx setup' "$pins" &&
  grep -q -F -e 'no full build on create' "$pins" &&
  grep -q -F -e 'devcontainer-check stays parity plus definition shape' "$pins"; then
  ok
else
  bad "pins.bzl lost its seed-host manual boot scope (linux/amd64 plus CLI plus postCreate plus no-full-build plus shape-only, issue #648)"
fi

# Pins record the manual verify procedure (local build plus up plus triggers plus PR evidence plus owner).
if grep -q -F -e 'docker build -f .devcontainer/Dockerfile.prebuilt -t dx-devcontainer:local .' "$pins" &&
  grep -q -F -e 'devcontainer up --workspace-folder .' "$pins" &&
  grep -q -F -e 'on every change to .devcontainer/Dockerfile.prebuilt or .devcontainer/devcontainer.json plus before any gated GHCR push' "$pins" &&
  grep -q -F -e 'with evidence in the same reviewed PR' "$pins" &&
  grep -q -F -e 'sole maintainer owns every row until delegation' "$pins"; then
  ok
else
  bad "pins.bzl lost its manual verify procedure (local docker build plus devcontainer up plus triggers plus PR evidence plus owner, issue #648)"
fi

# Pins record the non-Linux/arm64 wont-fix scope.
if grep -q -F -e 'non-Linux runs wont-fix' "$pins" &&
  grep -q -F -e 'linux/arm64 boot wont-fix' "$pins" &&
  grep -q -F -e 'arm64 prebuilt variant wont-fix' "$pins" &&
  grep -q -F -e 'scaffold devcontainer.json arch-independent' "$pins" &&
  grep -q -F -e 'amd64-only seed slice pins the amd64 Bazelisk launcher' "$pins" &&
  grep -q -F -e 'natively qualified per #410 plus #411 plus #412 plus #414 not container boot' "$pins"; then
  ok
else
  bad "pins.bzl lost its non-Linux/arm64 wont-fix scope (non-Linux plus arm64 boot plus arm64 variant plus arch-independent plus seed slice plus native-only, issue #648)"
fi

# Pins record the customer-flows-only qualification.
if grep -q -F -e 'qualified locally or on demand with customer flows only' "$pins" &&
  grep -q -F -e 'bazel run //tools/ci:devcontainer_boot_qualification' "$pins" &&
  grep -q -F -e 'static pins builds nothing boots nothing' "$pins" &&
  grep -q -F -e 'plus on-demand local docker build plus devcontainer up' "$pins"; then
  ok
else
  bad "pins.bzl lost its customer-flows-only qualification (local/on-demand plus harness plus static plus on-demand boot, issue #648)"
fi

# Pins record the rejected boot job plus customer-only boundary.
if grep -q -F -e 'boot job in CI rejected non-customer' "$pins" &&
  grep -q -F -e 'no extra CI job' "$pins" &&
  grep -q -F -e 'keep CI customer-only' "$pins"; then
  ok
else
  bad "pins.bzl lost its boot-job rejection plus no-extra-job plus customer-only pins under #648"
fi

# Pins record honesty (seed-only, infra only, no Supported).
if grep -q -F -e 'qualified seed-only under issue #648' "$pins" &&
  grep -q -F -e 'no Supported claim' "$pins" &&
  grep -q -F -e 'infra only' "$pins"; then
  ok
else
  bad "pins.bzl lost its seed-only plus infra-only plus no-Supported honesty under #648"
fi

# Expected fixture pins manual plus wont-fix plus rejected.
if grep -q -F -e 'Devcontainer boot manual plus wont-fix (issue #648)' "$expected" &&
  grep -q -F -e 'linux/amd64' "$expected" &&
  grep -q -F -e 'docker build -f' "$expected" &&
  grep -q -F -e 'devcontainer up --workspace-folder .' "$expected" &&
  grep -q -F -e 'wont-fix' "$expected" &&
  grep -q -F -e 'Boot job in CI rejected' "$expected" &&
  grep -q -F -e 'Qualified seed-only under issue #648' "$expected"; then
  ok
else
  bad "devcontainer_boot.expected lost its seed plus manual plus wont-fix plus rejected lines under #648"
fi

# Fixture BUILD exports pins plus expected with corpus coverage.
if grep -q -F -e 'pins.bzl' "$fixture_build" &&
  grep -q -F -e 'devcontainer_boot.expected' "$fixture_build" &&
  grep -q -F -e 'corpus_starlark' "$fixture_build"; then
  ok
else
  bad "devcontainer_boot BUILD.bazel lost its pins plus expected exports with corpus under #648"
fi

# Contract owns the manual boot plus wont-fix with fixture proof.
if grep -q -F -e 'Container boot manual' "$contract" &&
  grep -q -F -e 'manually verified on every Dockerfile or definition' "$contract" &&
  grep -q -F -e 'devcontainer up --workspace-folder .' "$contract" &&
  grep -q -F -e 'wont-fix' "$contract" &&
  grep -q -F -e 'Boot job in CI rejected' "$contract" &&
  grep -q -F -e 'keep CI' "$contract" &&
  grep -q -F -e 'tools/ci/tests/fixtures/devcontainer_boot/pins.bzl' "$contract" &&
  grep -q -F -e 'bazel run //tools/ci:devcontainer_boot_qualification' "$contract" &&
  grep -q -F -e 'issue #648' "$contract"; then
  ok
else
  bad "devcontainer.md lost its container boot manual plus wont-fix plus rejected record with fixture proof under #648"
fi

# Test matrix records the on-demand customer-flows-only qualification.
if grep -q -F -e 'Devcontainer boot manual' "$test_matrix" &&
  grep -q -F -e 'issue #648' "$test_matrix" &&
  grep -q -F -e 'bazel run //tools/ci:devcontainer_boot_qualification' "$test_matrix"; then
  ok
else
  bad "testing/github-ci.md lost its #648 devcontainer boot manual on-demand qualification record"
fi

# BUILD owns the harness target plus CI wires it records seed-only evidence.
if grep -q -F -e 'name = "devcontainer_boot_qualification"' "tools/ci/ci_targets_b.bzl" &&
  grep -q -F -e 'devcontainer_boot_qualification.sh' "tools/ci/ci_targets_b.bzl" &&
  grep -q -F -e 'bazel run --noshow_progress //tools/ci:devcontainer_boot_qualification' tools/ci/dogfood_freshness.sh; then
  ok
else
  bad "tools/ci/ci_targets_b.bzl or dogfood_freshness.sh lost the devcontainer_boot_qualification wiring (want target plus audit step)"
fi

# As-built shape stays check-only with no boot job in CI.
if grep -q -F -e 'devcontainer_parity_test' "$ci" &&
  grep -q -F -e 'postCreateCommand' "$ci" &&
  ! grep -q -F -e 'devcontainer up' "$ci" &&
  ! grep -q -F -e 'devcontainer build' "$ci" &&
  ! grep -q -E -e '^FROM [^ ]+:latest' "$dockerfile" &&
  grep -q -F -e 'bazel run //dx:env' "$definition" &&
  ! grep -q -F -e 'bazel build //...' "$definition"; then
  ok
else
  bad "as-built devcontainer shape drifted or CI gained a boot job (want parity plus shape plus no devcontainer up/build plus pinned base plus bootstrap postCreate, issue #648)"
fi

# As-built wont-fix stays single-sourced (amd64 seed slice, arch-independent scaffold).
if grep -q -E -e '^FROM [^ ]+@sha256:[0-9a-f]{64}' "$dockerfile" &&
  grep -q -F -e 'bazelisk-linux-amd64' "$dockerfile" &&
  grep -q -F -e 'USE_BAZEL_VERSION=9.2.0' "$dockerfile" &&
  grep -q -F -e '"image": "mcr.microsoft.com/devcontainers/base' "$definition" &&
  grep -q -F -e 'ghcr.io/devcontainers/features/bazel:1' "$definition" &&
  ! grep -q -F -e '"image": "ghcr.io' "$definition"; then
  ok
else
  bad "as-built amd64 seed slice plus arch-independent scaffold drifted (want digest plus amd64 Bazelisk plus 9.2.0 plus base image plus bazel feature, issue #648)"
fi

# GHCR route stays build-only on PRs with no boot and no extra job.
if grep -q -F -e '.devcontainer/Dockerfile.prebuilt' "$ghcr" &&
  ! grep -q -F -e 'devcontainer up' "$ghcr" &&
  ! grep -q -F -e 'devcontainer build' "$ghcr" &&
  ! grep -q -E -e '^  (push|schedule):' "$ghcr"; then
  ok
else
  bad "ghcr.yml gained a boot step or push/schedule trigger (want build-only PR scope with no devcontainer up/build, issue #648)"
fi

# Live proof: customer build flow only, no boot, no push.
if bazel build //tools/ci/tests/fixtures/devcontainer_boot/... --noshow_progress >/dev/null 2>&1; then
  ok
else
  bad "devcontainer-boot fixture failed to build (want green via customer build flow, issue #648)"
fi

dx_test_summary "devcontainer boot qualification harness"
