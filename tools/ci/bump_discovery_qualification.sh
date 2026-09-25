#!/usr/bin/env bash
# Bump discovery outdated enumeration qualification harness.
#
# Qualifies the scheduled discovery decision with fixtures plus docs:
# - enumerate: scheduled `bump.yml` runs without inputs enumerate outdated
#   via upstream registry clients (BCR / crates.io / npm registry / Go
#   proxy / Maven Central / NuGet / GitHub releases, never custom HTTP),
#   stable only with prerelease following upstream plus semver ordering
#   plus resolver-governed transitives (planned in `dx_bump::discovery`);
#   manual selector only stays rejected as the sole discovery route;
#   GitHub Actions tags enumerate via GitHub releases but need SHA
#   resolution before the file edit (issue #640 owns auto);
# - fixtures: `cli/bump/tests/fixtures/bump_discovery/` (`pins.bzl` plus
#   `bump_discovery.expected`) pins disposition, per-set clients, policy,
#   selector shapes, GHA note, up-to-date, and rejected routes;
# - record: the command docs carry the discovery note with the fixture
#   link; the automation policy carries the enumeration;
# - scope: automation only, no lock format change. Seed only: platform
#   plus consumer plus release evidence stays owned gap; no Supported claim.
#
# Versioned here, run by CI via `bazel run //tools/ci:bump_discovery_qualification`,
# following //tools/ci:bump_chain_qualification.
set -euo pipefail

# Shared workspace + runfiles helpers.
# Bootstrap via tools/sh/bootstrap.sh (issue #654): runfiles forest first, then source tree.
source "${RUNFILES_DIR:-/dev/null}/_main/tools/sh/bootstrap.sh" 2>/dev/null || source "${TEST_SRCDIR:-/dev/null}/_main/tools/sh/bootstrap.sh" 2>/dev/null || source "$0.runfiles/_main/tools/sh/bootstrap.sh" 2>/dev/null || source "${BASH_SOURCE[0]}.runfiles/_main/tools/sh/bootstrap.sh" 2>/dev/null || source "$(git rev-parse --show-toplevel 2>/dev/null)/tools/sh/bootstrap.sh"
dx_bootstrap "tools/sh/lib.sh"

dx_cd_workspace

dx_test_init

dx_bash_pin

discovery="cli/bump/src/discovery.rs"
bump_lib="cli/bump/src/lib.rs"
bump_build="cli/bump/BUILD.bazel"
pins="cli/bump/tests/fixtures/bump_discovery/pins.bzl"
expected="cli/bump/tests/fixtures/bump_discovery/bump_discovery.expected"
fixture_build="cli/bump/tests/fixtures/bump_discovery/BUILD.bazel"
command_doc="docs/cli/commands/audit-update-bazel.md"
automation="docs/contributing/automation.md"
bump_workflow=".github/workflows/bump.yml"
build="tools/ci/BUILD.bazel"
ci=".github/workflows/ci.yml"

# Discovery owns the upstream-client contract with no custom HTTP under #639.
if grep -q -F -e 'never custom HTTP' "$discovery" &&
  grep -q -F -e 'issue #639' "$discovery" &&
  grep -q -F -e 'BCR' "$discovery" &&
  grep -q -F -e 'crates.io' "$discovery"; then
  ok
else
  bad "discovery.rs lost its upstream-client plus never-custom-HTTP contract under #639"
fi

# Discovery exposes the registry-client plus latest-stable plus ordering entrypoints.
if grep -q -F -e 'pub fn registry_client' "$discovery" &&
  grep -q -F -e 'pub fn latest_stable' "$discovery" &&
  grep -q -F -e 'pub fn collect_outdated' "$discovery" &&
  grep -q -F -e 'pub fn next_outdated' "$discovery"; then
  ok
else
  bad "discovery.rs lost its registry_client plus latest_stable plus collect/next entrypoints under #639"
fi

# Discovery pins stable-only plus upstream prerelease plus semver ordering.
if grep -q -F -e 'stable only' "$discovery" &&
  grep -q -F -e 'prerelease_follows_upstream' "$discovery" &&
  grep -q -F -e 'version::compare' "$discovery" &&
  grep -q -F -e 'resolver-governed' "$discovery"; then
  ok
else
  bad "discovery.rs lost its stable-only plus upstream-prerelease plus semver-ordering pins under #639"
fi

# Discovery pins its tests (clients, stable filtering, ordering, GHA note).
if grep -q -F -e 'semver_sets_parse_declared_with_client_mapping' "$discovery" &&
  grep -q -F -e 'stable_only_prerelease_never_wins' "$discovery" &&
  grep -q -F -e 'collect_orders_by_selector_never_batch' "$discovery" &&
  grep -q -F -e 'github_actions_has_no_semver_current' "$discovery"; then
  ok
else
  bad "discovery.rs lost its client plus stable plus ordering plus GHA test pins under #639"
fi

# Bump lib owns the discovery module with the #639 contract.
if grep -q -F -e 'pub mod discovery' "$bump_lib" &&
  grep -q -F -e 'issue #639' "$bump_lib" &&
  grep -q -F -e 'dx_bump::discovery' "$bump_workflow"; then
  ok
else
  bad "bump lib.rs lost its discovery module plus #639 contract"
fi

# Bump BUILD owns the discovery source.
if grep -q -F -e 'src/discovery.rs' "$bump_build"; then
  ok
else
  bad "cli/bump/BUILD.bazel lost its src/discovery.rs wiring under #639"
fi

# Fixture pins stay present with enumerate disposition plus per-set clients.
if [[ -f "$pins" && -f "$expected" && -f "$fixture_build" ]] &&
  grep -q -F -e 'BUMP_DISCOVERY = "enumerate"' "$pins" &&
  grep -q -F -e 'BUMP_DISCOVERY_CARGO = "crates.io"' "$pins" &&
  grep -q -F -e 'BUMP_DISCOVERY_NPM = "npm registry"' "$pins" &&
  grep -q -F -e 'BUMP_DISCOVERY_GO = "Go proxy"' "$pins" &&
  grep -q -F -e 'BUMP_DISCOVERY_MAVEN = "Maven Central"' "$pins" &&
  grep -q -F -e 'BUMP_DISCOVERY_NUGET = "NuGet"' "$pins" &&
  grep -q -F -e 'BUMP_DISCOVERY_GHA = "GitHub releases"' "$pins"; then
  ok
else
  bad "bump_discovery pins fixture lost its enumerate plus per-set client wiring under #639"
fi

# Pins record policy (stable, prerelease, ordering, transitives, next).
if grep -q -F -e 'stable only' "$pins" &&
  grep -q -F -e 'prerelease follows upstream' "$pins" &&
  grep -q -F -e 'upstream semver comparison' "$pins" &&
  grep -q -F -e 'transitives stay resolver-governed' "$pins" &&
  grep -q -F -e 'never batch' "$pins"; then
  ok
else
  bad "bump_discovery pins.bzl lost its policy wiring under #639"
fi

# Pins record selector shapes plus GHA note plus up-to-date.
if grep -q -F -e 'cargo:anyhow outdated' "$pins" &&
  grep -q -F -e 'npm:jest outdated' "$pins" &&
  grep -q -F -e 'maven:junit:junit outdated' "$pins" &&
  grep -q -F -e 'need SHA resolution' "$pins" &&
  grep -q -F -e 'up-to-date has no candidate' "$pins"; then
  ok
else
  bad "bump_discovery pins.bzl lost its selector plus GHA plus up-to-date wiring under #639"
fi

# Pins record rejected routes plus honesty under #639.
if grep -q -F -e 'manual selector only rejected' "$pins" &&
  grep -q -F -e 'custom HTTP rejected' "$pins" &&
  grep -q -F -e 'private resolver rejected' "$pins" &&
  grep -q -F -e 'qualified seed-only under issue #639' "$pins" &&
  grep -q -F -e 'no Supported claim' "$pins"; then
  ok
else
  bad "bump_discovery pins.bzl lost its rejected plus honesty wiring under #639"
fi

# Expected fixture pins clients plus policy plus rejected plus honesty lines.
if grep -q -F -e 'via BCR stable only' "$expected" &&
  grep -q -F -e 'via crates.io stable only' "$expected" &&
  grep -q -F -e 'via npm registry stable only' "$expected" &&
  grep -q -F -e 'need SHA resolution' "$expected" &&
  grep -q -F -e 'manual selector only rejected' "$expected" &&
  grep -q -F -e 'custom HTTP rejected' "$expected" &&
  grep -q -F -e 'qualified seed-only under issue #639' "$expected" &&
  grep -q -F -e 'no Supported claim' "$expected"; then
  ok
else
  bad "bump_discovery.expected lost its client plus policy plus honesty lines under #639"
fi

# Fixture BUILD exports pins plus expected with corpus coverage.
if grep -q -F -e 'pins.bzl' "$fixture_build" &&
  grep -q -F -e 'bump_discovery.expected' "$fixture_build" &&
  grep -q -F -e 'corpus_starlark' "$fixture_build"; then
  ok
else
  bad "bump_discovery BUILD.bazel lost its pins plus expected exports with corpus under #639"
fi

# Command docs carry the discovery note with the fixture link under #639.
if grep -q -F -e 'dx_bump::discovery' "$command_doc" &&
  grep -q -F -e 'closed #639' "$command_doc" &&
  grep -q -F -e 'cli/bump/tests/fixtures/bump_discovery/' "$command_doc" &&
  grep -q -F -e 'manual selector only rejected' "$command_doc"; then
  ok
else
  bad "audit-update-bazel.md lost its discovery #639 note with fixture link"
fi

# Automation policy carries the upstream-client enumeration under #639.
if grep -q -F -e 'dx_bump::discovery' "$automation" &&
  grep -q -F -e 'closed #639' "$automation" &&
  grep -q -F -e 'cli/bump/tests/fixtures/bump_discovery/' "$automation" &&
  grep -q -F -e 'manual selector only rejected' "$automation"; then
  ok
else
  bad "automation.md lost its discovery enumeration record under #639"
fi

# Scheduled workflow enumerates outdated via upstream clients (never echo-only, never custom HTTP).
if grep -q -F -e 'enumerating outdated' "$bump_workflow" &&
  grep -q -F -e 'upstream registry clients' "$bump_workflow" &&
  grep -q -F -e 'stable only' "$bump_workflow" &&
  grep -q -F -e 'dx_bump::discovery' "$bump_workflow" &&
  grep -q -F -e 'cargo search' "$bump_workflow" &&
  grep -q -F -e 'npm view' "$bump_workflow" &&
  grep -q -F -e 'go list' "$bump_workflow" &&
  ! grep -q -F -e 'curl ' "$bump_workflow" &&
  ! grep -q -F -e 'wget ' "$bump_workflow"; then
  ok
else
  bad "bump.yml lost its outdated enumeration via upstream clients under #639"
fi

# BUILD owns the harness target plus CI wires it records seed-only evidence.
if grep -q -F -e 'name = "bump_discovery_qualification"' "tools/ci/ci_targets_c.bzl" &&
  grep -q -F -e 'bump_discovery_qualification.sh' "tools/ci/ci_targets_c.bzl" &&
  grep -q -F -e 'bazel run --noshow_progress //tools/ci:bump_discovery_qualification' tools/ci/dogfood_freshness.sh; then
  ok
else
  bad "tools/ci/ci_targets_c.bzl or dogfood_freshness.sh lost the bump_discovery_qualification wiring (want target plus dogfood-freshness)"
fi

dx_test_summary "bump discovery qualification harness"
