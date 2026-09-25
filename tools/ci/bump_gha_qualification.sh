#!/usr/bin/env bash
# Bump GHA tag-to-SHA auto resolution qualification harness.
#
# Qualifies the automatic tag resolution decision with fixtures plus docs:
# - auto: GitHub Actions tags auto-resolve to SHA via the upstream GitHub
#   releases client before the file edit (planned in `dx_bump::gha`);
#   shapes validate through `version::parse` (GitTag plus 40/64-char
#   GitCommit), never custom version code; fetching stays in the upstream
#   client plus the scheduled `bump.yml` runner (`gh api`, never custom
#   HTTP); unknown tags fail closed with no invented SHA; direct tags fail
#   closed in `dx bump` (NeedsSha) until the runner resolves; manual SHA
#   only stays rejected as the sole route for the automatic goal;
# - fixtures: `cli/bump/tests/fixtures/bump_gha/` (`pins.bzl` plus
#   `bump_gha.expected`) pins disposition, client, snapshots, shapes,
#   resolve, unknown, NeedsSha, and rejected routes;
# - record: the command docs carry the auto note with the fixture link;
#   the automation policy carries the auto resolution;
# - scope: automation only, no lock format change. Seed only: platform
#   plus consumer plus release evidence stays owned gap; no Supported claim.
#
# Versioned here, run by CI via `bazel run //tools/ci:bump_gha_qualification`,
# following //tools/ci:bump_discovery_qualification.
set -euo pipefail

# Shared workspace + runfiles helpers.
# Bootstrap via tools/sh/bootstrap.sh (issue #654): runfiles forest first, then source tree.
source "${RUNFILES_DIR:-/dev/null}/_main/tools/sh/bootstrap.sh" 2>/dev/null || source "${TEST_SRCDIR:-/dev/null}/_main/tools/sh/bootstrap.sh" 2>/dev/null || source "$0.runfiles/_main/tools/sh/bootstrap.sh" 2>/dev/null || source "${BASH_SOURCE[0]}.runfiles/_main/tools/sh/bootstrap.sh" 2>/dev/null || source "$(git rev-parse --show-toplevel 2>/dev/null)/tools/sh/bootstrap.sh"
dx_bootstrap "tools/sh/lib.sh"

dx_cd_workspace

dx_test_init

dx_bash_pin

gha="cli/bump/src/gha.rs"
bump_lib="cli/bump/src/lib.rs"
bump_build="cli/bump/BUILD.bazel"
pins="cli/bump/tests/fixtures/bump_gha/pins.bzl"
expected="cli/bump/tests/fixtures/bump_gha/bump_gha.expected"
fixture_build="cli/bump/tests/fixtures/bump_gha/BUILD.bazel"
command_doc="docs/cli/commands/audit-update-bazel.md"
automation="docs/contributing/automation.md"
bump_workflow=".github/workflows/bump.yml"
request="cli/bump/src/request.rs"
build="tools/ci/BUILD.bazel"
ci=".github/workflows/ci.yml"

# GHA owns the upstream-client contract with no custom HTTP under #640.
if grep -q -F -e 'never custom HTTP' "$gha" &&
  grep -q -F -e 'issue #640' "$gha" &&
  grep -q -F -e 'GitHub releases' "$gha"; then
  ok
else
  bad "gha.rs lost its upstream-client plus never-custom-HTTP contract under #640"
fi

# GHA exposes the upstream-client plus snapshot plus resolve entrypoints.
if grep -q -F -e 'pub fn upstream_client' "$gha" &&
  grep -q -F -e 'pub fn parse_snapshot' "$gha" &&
  grep -q -F -e 'pub fn resolve_tag' "$gha" &&
  grep -q -F -e 'pub struct TagSnapshot' "$gha"; then
  ok
else
  bad "gha.rs lost its upstream_client plus parse_snapshot plus resolve_tag entrypoints under #640"
fi

# GHA validates shapes through version::parse (never custom version code).
if grep -q -F -e 'version::parse' "$gha" &&
  grep -q -F -e 'GitTag' "$gha" &&
  grep -q -F -e 'GitCommit' "$gha" &&
  grep -q -F -e 'never custom version' "$gha"; then
  ok
else
  bad "gha.rs lost its version::parse plus GitTag plus GitCommit shape pins under #640"
fi

# GHA pins its tests (client, auto resolve, unknown closed, malformed, SHA).
if grep -q -F -e 'upstream_client_is_github_releases' "$gha" &&
  grep -q -F -e 'tag_auto_resolves_to_sha_from_upstream_snapshot' "$gha" &&
  grep -q -F -e 'unknown_tag_fails_closed_without_inventing_sha' "$gha" &&
  grep -q -F -e 'invalid_snapshot_sha_fails_closed' "$gha"; then
  ok
else
  bad "gha.rs lost its client plus auto plus unknown plus SHA test pins under #640"
fi

# Bump lib owns the gha module with the #640 contract; direct tags stay
# NeedsSha until the runner resolves (nothing widened).
if grep -q -F -e 'pub mod gha' "$bump_lib" &&
  grep -q -F -e 'issue #640' "$bump_lib" &&
  grep -q -F -e 'dx_bump::gha' "$bump_workflow" &&
  grep -q -F -e 'NeedsSha' "$request"; then
  ok
else
  bad "bump lib.rs lost its gha module plus #640 contract"
fi

# Bump BUILD owns the gha source.
if grep -q -F -e 'src/gha.rs' "$bump_build"; then
  ok
else
  bad "cli/bump/BUILD.bazel lost its src/gha.rs wiring under #640"
fi

# Fixture pins stay present with auto disposition plus client.
if [[ -f "$pins" && -f "$expected" && -f "$fixture_build" ]] &&
  grep -q -F -e 'BUMP_GHA = "auto"' "$pins" &&
  grep -q -F -e 'BUMP_GHA_CLIENT = "GitHub releases"' "$pins"; then
  ok
else
  bad "bump_gha pins fixture lost its auto plus client wiring under #640"
fi

# Pins record snapshots plus shapes (tag via GitTag, SHA via GitCommit).
if grep -q -F -e 'actions/checkout v5 auto-resolves' "$pins" &&
  grep -q -F -e 'actions/cache v4 auto-resolves' "$pins" &&
  grep -q -F -e 'via version::parse GitTag' "$pins" &&
  grep -q -F -e 'via version::parse GitCommit' "$pins"; then
  ok
else
  bad "bump_gha pins.bzl lost its snapshot plus shape wiring under #640"
fi

# Pins record resolve plus unknown plus NeedsSha (one tag per run).
if grep -q -F -e 'resolve_tag matches owner/repo plus tag' "$pins" &&
  grep -q -F -e 'one tag per run' "$pins" &&
  grep -q -F -e 'unknown tag fails closed' "$pins" &&
  grep -q -F -e 'needs SHA resolution' "$pins"; then
  ok
else
  bad "bump_gha pins.bzl lost its resolve plus unknown plus NeedsSha wiring under #640"
fi

# Pins record rejected routes plus honesty under #640.
if grep -q -F -e 'manual SHA only rejected' "$pins" &&
  grep -q -F -e 'custom HTTP rejected' "$pins" &&
  grep -q -F -e 'invented SHA rejected' "$pins" &&
  grep -q -F -e 'qualified seed-only under issue #640' "$pins" &&
  grep -q -F -e 'no Supported claim' "$pins"; then
  ok
else
  bad "bump_gha pins.bzl lost its rejected plus honesty wiring under #640"
fi

# Expected fixture pins auto plus rejected plus honesty lines.
if grep -q -F -e 'auto-resolves to SHA via GitHub releases' "$expected" &&
  grep -q -F -e 'unknown tag fails closed' "$expected" &&
  grep -q -F -e 'manual SHA only rejected' "$expected" &&
  grep -q -F -e 'invented SHA rejected' "$expected" &&
  grep -q -F -e 'qualified seed-only under issue #640' "$expected" &&
  grep -q -F -e 'no Supported claim' "$expected"; then
  ok
else
  bad "bump_gha.expected lost its auto plus rejected plus honesty lines under #640"
fi

# Fixture BUILD exports pins plus expected with corpus coverage.
if grep -q -F -e 'pins.bzl' "$fixture_build" &&
  grep -q -F -e 'bump_gha.expected' "$fixture_build" &&
  grep -q -F -e 'corpus_starlark' "$fixture_build"; then
  ok
else
  bad "bump_gha BUILD.bazel lost its pins plus expected exports with corpus under #640"
fi

# Command docs carry the auto note with the fixture link under #640.
if grep -q -F -e 'dx_bump::gha' "$command_doc" &&
  grep -q -F -e 'closed #640' "$command_doc" &&
  grep -q -F -e 'cli/bump/tests/fixtures/bump_gha/' "$command_doc" &&
  grep -q -F -e 'manual SHA' "$command_doc"; then
  ok
else
  bad "audit-update-bazel.md lost its auto #640 note with fixture link"
fi

# Automation policy carries the upstream-client auto resolution under #640.
if grep -q -F -e 'dx_bump::gha' "$automation" &&
  grep -q -F -e 'closed #640' "$automation" &&
  grep -q -F -e 'cli/bump/tests/fixtures/bump_gha/' "$automation" &&
  grep -q -F -e 'manual SHA only rejected' "$automation"; then
  ok
else
  bad "automation.md lost its auto resolution record under #640"
fi

# Scheduled workflow auto-resolves tags via gh api (never curl/wget/custom HTTP).
if grep -q -F -e 'gh api' "$bump_workflow" &&
  grep -q -F -e 'dx_bump::gha' "$bump_workflow" &&
  grep -q -F -e 'auto-resolve' "$bump_workflow" &&
  grep -q -F -e 'never invent' "$bump_workflow" &&
  ! grep -q -F -e 'curl ' "$bump_workflow" &&
  ! grep -q -F -e 'wget ' "$bump_workflow"; then
  ok
else
  bad "bump.yml lost its tag auto-resolution via gh api under #640"
fi

# BUILD owns the harness target plus CI wires it records seed-only evidence.
if grep -q -F -e 'name = "bump_gha_qualification"' "tools/ci/ci_targets_c.bzl" &&
  grep -q -F -e 'bump_gha_qualification.sh' "tools/ci/ci_targets_c.bzl" &&
  grep -q -F -e 'bazel run --noshow_progress //tools/ci:bump_gha_qualification' tools/ci/dogfood_freshness.sh; then
  ok
else
  bad "tools/ci/ci_targets_c.bzl or dogfood_freshness.sh lost the bump_gha_qualification wiring (want target plus dogfood-freshness)"
fi

dx_test_summary "bump gha qualification harness"
