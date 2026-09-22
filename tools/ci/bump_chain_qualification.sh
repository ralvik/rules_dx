#!/usr/bin/env bash
# Bump then update chaining `dx bump` qualification harness.
#
# Qualifies the automatic chaining decision with fixtures plus docs:
# - automatic: `dx bump <set:package> <version>` widens one requirement
#   then chains the resolver-owned refresh without a manual second step
#   (Cargo full `CARGO_BAZEL_REPIN=1 bazel build
#   //rust/tests/fixtures/hello:hello`, npm selective
#   `bazel run @pnpm//:pnpm -- update [<pkg>]` for the widened package,
#   Go full noop with no launch, Maven full `REPIN=1 bazel run @maven//:pin`,
#   NuGet full `paket2bazel` regen; Bazel/GitHub Actions stay file-only with
#   flag-diff review plus build and no launch); refresh failures keep the
#   widen with no rollback and exit 1 with `update_failed`; dry-run plans
#   both without touching the tree or launching; private resolvers stay
#   rejected;
# - fixtures: `cli/bump/tests/fixtures/bump_chain/` (`pins.bzl` plus
#   `bump_chain.expected`) pins disposition, per-set chaining, selector
#   shapes, file-only sets, failure contract, and rejected routes;
# - record: ADR 0024 owns the chaining rationale; the command docs carry
#   the automatic note with the fixture link;
# - scope: bump plus resolver refresh only, no lock format change. Seed
#   only: platform plus consumer plus release evidence stays owned gap;
#   no Supported claim.
#
# Versioned here, run by CI via `bazel run //tools/ci:bump_chain_qualification`,
# following //tools/ci:selective_update_qualification.
set -euo pipefail

# Shared workspace + runfiles helpers.
# Bootstrap via tools/sh/bootstrap.sh (issue #654): runfiles forest first, then source tree.
source "${RUNFILES_DIR:-/dev/null}/_main/tools/sh/bootstrap.sh" 2>/dev/null || source "${TEST_SRCDIR:-/dev/null}/_main/tools/sh/bootstrap.sh" 2>/dev/null || source "$0.runfiles/_main/tools/sh/bootstrap.sh" 2>/dev/null || source "${BASH_SOURCE[0]}.runfiles/_main/tools/sh/bootstrap.sh" 2>/dev/null || source "$(git rev-parse --show-toplevel 2>/dev/null)/tools/sh/bootstrap.sh"
dx_bootstrap "tools/sh/lib.sh"

dx_cd_workspace

dx_test_init

adr="docs/decisions/0024-selective-update.md"
pins="cli/bump/tests/fixtures/bump_chain/pins.bzl"
expected="cli/bump/tests/fixtures/bump_chain/bump_chain.expected"
fixture_build="cli/bump/tests/fixtures/bump_chain/BUILD.bazel"
bump_lib="cli/bump/src/lib.rs"
bump_request="cli/bump/src/request.rs"
bump_request_tests="cli/bump/src/request_tests.rs"
bump_version="cli/bump/src/version.rs"
bump_sets="cli/bump/src/sets.rs"
bump_exec="cli/cli/src/exec/bump.rs"
command_doc="docs/cli/commands/audit-update-bazel.md"
targets_c="tools/ci/ci_targets_c.bzl"
freshness="tools/ci/dogfood_freshness.sh"

# Bump lib owns the automatic chaining contract (never a manual second step).
if grep -q -F -e 'chains automatically' "$bump_lib" &&
  grep -q -F -e 'issue #638' "$bump_lib" &&
  grep -q -F -e 'dx update cargo' "$bump_lib" &&
  grep -q -F -e 'dx update npm:<package>' "$bump_lib"; then
  ok
else
  bad "bump lib.rs lost its automatic chaining contract under #638"
fi

# Bump request plans the per-set refresh selector automatically.
if grep -q -F -e 'refresh_selector' "$bump_request" &&
  grep -q -F -e 'automatically' "$bump_request" &&
  grep -q -F -e 'issue #638' "$bump_request" &&
  grep -q -F -e 'refresh_selector_chains_automatically_per_set' "$bump_request_tests"; then
  ok
else
  bad "bump request.rs lost its refresh_selector automatic chaining under #638"
fi

# Bump request summary names the automatic refresh (never manual).
if grep -q -F -e 'then refresh via `dx update' "$bump_request" &&
  grep -q -F -e 'automatically' "$bump_request"; then
  ok
else
  bad "bump request.rs lost its automatic refresh summary under #638"
fi

# Bump exec chains through the approved backend with the runner (never private).
if grep -q -F -e 'dx_update::backend::plan' "$bump_exec" &&
  grep -q -F -e 'runner.run' "$bump_exec" &&
  grep -q -F -e 'BackendPlan::Run' "$bump_exec" &&
  grep -q -F -e 'BackendPlan::Noop' "$bump_exec"; then
  ok
else
  bad "exec/bump.rs lost its approved-backend plus runner chaining under #638"
fi

# Bump exec maps Cargo/Maven/NuGet full, npm selective, Go noop (never batch, never private).
if grep -q -F -e 'SetId::Cargo' "$bump_exec" &&
  grep -q -F -e 'SetRequest::Full' "$bump_exec" &&
  grep -q -F -e 'SetRequest::Packages' "$bump_exec" &&
  grep -q -F -e 'SetId::Npm' "$bump_exec" &&
  grep -q -F -e 'SetId::Go' "$bump_exec" &&
  grep -q -F -e 'SetId::Maven' "$bump_exec" &&
  grep -q -F -e 'SetId::NuGet' "$bump_exec"; then
  ok
else
  bad "exec/bump.rs lost its Cargo/Maven/NuGet-full plus npm-selective plus Go-noop mapping under #638"
fi

# Bump exec reports widen plus automatic refresh success (never manual hint).
if grep -q -F -e 'and refreshed' "$bump_exec" &&
  grep -q -F -e 'automatically' "$bump_exec" &&
  grep -q -F -e 'update_set_success' "$bump_exec"; then
  ok
else
  bad "exec/bump.rs lost its widen-plus-automatic-refresh success report under #638"
fi

# Bump exec keeps the widen on refresh failure with update_failed (no rollback).
if grep -q -F -e 'widen kept' "$bump_exec" &&
  grep -q -F -e 'CODE_UPDATE_FAILED' "$bump_exec" &&
  grep -q -F -e 'bump_refresh_failed' "$bump_exec"; then
  ok
else
  bad "exec/bump.rs lost its widen-kept plus update_failed failure contract under #638"
fi

# Bump exec pins the chaining tests (cargo/maven/nuget full, npm selective, go noop, failures).
if grep -q -F -e 'live_npm_chains_selective_refresh_automatically' "$bump_exec" &&
  grep -q -F -e 'live_go_chains_noop_without_launch' "$bump_exec" &&
  grep -q -F -e 'live_maven_chains_full_refresh_automatically' "$bump_exec" &&
  grep -q -F -e 'live_nuget_chains_full_refresh_automatically' "$bump_exec" &&
  grep -q -F -e 'live_refresh_failure_keeps_widen_and_reports_update_failed' "$bump_exec" &&
  grep -q -F -e 'dry_run_chains_without_launch_or_write' "$bump_exec"; then
  ok
else
  bad "exec/bump.rs lost its chaining test pins under #638"
fi

# Major-bump=>migrate hint (issue #931): bump(major) plans print the
# missing-manifest hint with exit mapping, pinned here plus migrate.
if grep -q -F -e 'generic_major_bump_hint' "$bump_exec" &&
  grep -q -F -e 'major_bump_hint' "$bump_request" &&
  grep -q -F -e 'is_major_bump' "$bump_version" &&
  grep -q -F -e 'major_bump_migrate_hint' "$bump_version" &&
  grep -q -F -e 'major_bump_plans_carry_migrate_hint_with_exit_mapping' "$bump_exec" &&
  grep -q -F -e 'dx migrate --from' "$bump_exec" &&
  grep -q -F -e 'migrate_failed' "$bump_exec" &&
  grep -q -F -e 'missing-versions' "$bump_exec"; then
  ok
else
  bad "exec/bump.rs lost its major-bump=>migrate hint with exit mapping under #931"
fi

# Fixture pins stay present with disposition plus per-set chaining plus selectors.
if [[ -f "$pins" && -f "$expected" && -f "$fixture_build" ]] &&
  grep -q -F -e 'BUMP_CHAIN = "automatic"' "$pins" &&
  grep -q -F -e 'BUMP_CHAIN_CARGO = "dx update cargo"' "$pins" &&
  grep -q -F -e 'BUMP_CHAIN_NPM = "dx update npm:<pkg>"' "$pins" &&
  grep -q -F -e 'BUMP_CHAIN_GO = "dx update go"' "$pins" &&
  grep -q -F -e 'BUMP_CHAIN_MAVEN = "dx update maven"' "$pins" &&
  grep -q -F -e 'BUMP_CHAIN_NUGET = "dx update nuget"' "$pins" &&
  grep -q -F -e 'cargo:anyhow refreshes cargo' "$pins" &&
  grep -q -F -e 'npm:jest refreshes npm:jest' "$pins" &&
  grep -q -F -e 'maven:junit:junit refreshes maven' "$pins"; then
  ok
else
  bad "bump_chain pins fixture lost its automatic plus per-set plus selector wiring under #638"
fi

# Pins record file-only plus failure plus rejected plus honesty under #638.
if grep -q -F -e 'file-only' "$pins" &&
  grep -q -F -e 'widen kept in manifest' "$pins" &&
  grep -q -F -e 'update_failed' "$pins" &&
  grep -q -F -e 'manual second step rejected' "$pins" &&
  grep -q -F -e 'private resolver rejected' "$pins" &&
  grep -q -F -e 'qualified seed-only under issue #638' "$pins" &&
  grep -q -F -e 'no Supported claim' "$pins"; then
  ok
else
  bad "bump_chain pins.bzl lost its file-only plus failure plus rejected plus honesty wiring under #638"
fi

# Major-bump=>migrate fixture pins (issue #931).
if grep -q -F -e 'BUMP_MAJOR_HINT' "$pins" &&
  grep -q -F -e 'major bump' "$pins" &&
  grep -q -F -e 'dx migrate --from' "$pins" &&
  grep -q -F -e 'migrate_failed' "$pins" &&
  grep -q -F -e 'missing-versions' "$pins" &&
  grep -q -F -e 'migrate-v1-to-v2.json' "$pins"; then
  ok
else
  bad "bump_chain pins.bzl lost its major-bump=>migrate hint wiring under #931"
fi

# Expected fixture pins the chaining plus failure plus honesty lines.
if grep -q -F -e 'cargo widen then automatic' "$expected" &&
  grep -q -F -e 'npm widen then automatic' "$expected" &&
  grep -q -F -e 'go widen then automatic' "$expected" &&
  grep -q -F -e 'maven widen then automatic' "$expected" &&
  grep -q -F -e 'nuget widen then automatic' "$expected" &&
  grep -q -F -e 'file-only' "$expected" &&
  grep -q -F -e 'widen kept in manifest' "$expected" &&
  grep -q -F -e 'manual second step rejected' "$expected" &&
  grep -q -F -e 'qualified seed-only under issue #638' "$expected" &&
  grep -q -F -e 'no Supported claim' "$expected"; then
  ok
else
  bad "bump_chain.expected lost its chaining plus failure plus honesty lines under #638"
fi

# Expected fixture pins the major-bump=>migrate hint (issue #931).
if grep -q -F -e 'major bump 1.2.3 -> 2.0.0' "$expected" &&
  grep -q -F -e 'migrate-v1-to-v2.json' "$expected" &&
  grep -q -F -e 'migrate_failed' "$expected" &&
  grep -q -F -e 'missing-versions' "$expected"; then
  ok
else
  bad "bump_chain.expected lost its major-bump=>migrate hint lines under #931"
fi

# Fixture BUILD exports pins plus expected with corpus coverage.
if grep -q -F -e 'pins.bzl' "$fixture_build" &&
  grep -q -F -e 'bump_chain.expected' "$fixture_build" &&
  grep -q -F -e 'corpus_starlark' "$fixture_build"; then
  ok
else
  bad "bump_chain BUILD.bazel lost its pins plus expected exports with corpus under #638"
fi

# Command docs carry the automatic chaining note with the fixture link.
if grep -q -F -e 'Lock refresh chains automatically' "$command_doc" &&
  grep -q -F -e 'issue #638' "$command_doc" &&
  grep -q -F -e 'cli/bump/tests/fixtures/bump_chain/' "$command_doc" &&
  grep -q -F -e 'dx update npm:<pkg>' "$command_doc"; then
  ok
else
  bad "audit-update-bazel.md lost its automatic chaining #638 note with fixture link"
fi

# Command docs carry the major-bump=>migrate hint (issue #931).
if grep -q -F -e 'Major bumps hint migrate' "$command_doc" &&
  grep -q -F -e 'issue #931' "$command_doc" &&
  grep -q -F -e 'dx migrate --from' "$command_doc" &&
  grep -q -F -e 'migrate_failed' "$command_doc" &&
  grep -q -F -e 'missing-versions' "$command_doc"; then
  ok
else
  bad "audit-update-bazel.md lost its major-bump=>migrate hint under #931"
fi

# ADR 0024 owns the automatic chaining rationale (never manual).
if grep -q -F -e 'chains automatically' "$adr" &&
  grep -q -F -e 'issue #638' "$adr" &&
  grep -q -F -e 'cli/bump/tests/fixtures/bump_chain/' "$adr"; then
  ok
else
  bad "ADR 0024 lost its automatic chaining record under #638"
fi

# BUILD owns the harness target plus dogfood-freshness wires it.
if grep -q -F -e 'name = "bump_chain_qualification"' "$targets_c" &&
  grep -q -F -e 'bump_chain_qualification.sh' "$targets_c" &&
  grep -q -F -e 'bazel run --noshow_progress //tools/ci:bump_chain_qualification' "$freshness"; then
  ok
else
  bad "tools/ci/ci_targets_c.bzl or dogfood_freshness.sh lost the bump_chain_qualification wiring (want target plus dogfood-freshness)"
fi

dx_test_summary "bump chain qualification harness"
