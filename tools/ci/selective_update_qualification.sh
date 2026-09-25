#!/usr/bin/env bash
# Selective `dx update` per-set qualification harness.
#
# Qualifies the per-set selective-versus-full decision with fixtures plus
# decision record:
# - decided: npm selective SUPPORTED via the Bazel-pinned pnpm
#   (`bazel run @pnpm//:pnpm -- update [<pkg>...]`); Cargo, Maven, NuGet,
#   and Go selective WONT-FIX in V1 (whole-lock repin only, Go pins track
#   Gazelle for the shared extension), each failing closed as
#   `unsupported` with its full-set hint and never silently substituting
#   a full update;
# - fixtures: `cli/update/tests/fixtures/selective_update/` (`pins.bzl`
#   plus `selective_update.expected`) pins dispositions, approved argv,
#   hints, bump follow-ups, and rejected routes;
# - record: ADR 0024 owns the rationale; the command docs carry the
#   per-set table plus the automatic bump follow-up (issue #638);
# - scope: update-only, no lock format change. Seed only: platform plus
#   consumer plus release evidence stays owned gap; no Supported claim.
#
# Versioned here, run by CI via `bazel run //tools/ci:selective_update_qualification`,
# following //tools/ci:rust_library_qualification.
set -euo pipefail

# Shared workspace + runfiles helpers.
# Bootstrap via tools/sh/bootstrap.sh (issue #654): runfiles forest first, then source tree.
source "${RUNFILES_DIR:-/dev/null}/_main/tools/sh/bootstrap.sh" 2>/dev/null || source "${TEST_SRCDIR:-/dev/null}/_main/tools/sh/bootstrap.sh" 2>/dev/null || source "$0.runfiles/_main/tools/sh/bootstrap.sh" 2>/dev/null || source "${BASH_SOURCE[0]}.runfiles/_main/tools/sh/bootstrap.sh" 2>/dev/null || source "$(git rev-parse --show-toplevel 2>/dev/null)/tools/sh/bootstrap.sh"
dx_bootstrap "tools/sh/lib.sh"

dx_cd_workspace

dx_test_init

dx_bash_pin

adr="docs/decisions/0024-selective-update.md"
adr_index="docs/decisions/README.md"
pins="cli/update/tests/fixtures/selective_update/pins.bzl"
expected="cli/update/tests/fixtures/selective_update/selective_update.expected"
fixture_build="cli/update/tests/fixtures/selective_update/BUILD.bazel"
backend="cli/update/src/backend.rs"
selector="cli/update/src/selector.rs"
exec_update="cli/cli/src/exec/update.rs"
exec_tests_a="cli/cli/src/exec/update_tests_a.rs"
bump_exec="cli/cli/src/exec/bump.rs"
command_doc="docs/cli/commands/audit-update-bazel.md"
build="tools/ci/BUILD.bazel"
ci=".github/workflows/ci.yml"

# ADR 0024 exists and stays Accepted under.
if [[ -f "$adr" ]] &&
  grep -q -F -e '# ADR 0024: Selective `dx update` Per-Set Support' "$adr" &&
  grep -q -F -e 'Accepted.' "$adr" &&
  grep -q -F -e 'issue #583' "$adr"; then
  ok
else
  bad "ADR 0024 lost its Accepted selective-update record under #583"
fi

# ADR decides one supported plus four wont-fix sets.
if grep -q -F -e 'npm `SUPPORTED`' "$adr" &&
  grep -q -F -e 'Cargo `WONT-FIX`' "$adr" &&
  grep -q -F -e 'Maven `WONT-FIX`' "$adr" &&
  grep -q -F -e 'NuGet `WONT-FIX`' "$adr" &&
  grep -q -F -e 'Go `WONT-FIX`' "$adr"; then
  ok
else
  bad "ADR 0024 lost its one-supported plus four-wont-fix per-set decision"
fi

# ADR rejects silent substitution plus private emulation.
if grep -q -F -e 'Silent full-update substitution' "$adr" &&
  grep -q -F -e 'Private per-package emulation' "$adr" &&
  grep -q -F -e 'never silently substitutes a full' "$adr"; then
  ok
else
  bad "ADR 0024 lost its rejected silent-substitution plus private-emulation record"
fi

# ADR index owns the 0024 entry.
if grep -q -F -e '0024-selective-update.md' "$adr_index" &&
  grep -q -F -e 'Selective `dx update` Per-Set Support' "$adr_index"; then
  ok
else
  bad "decisions README lost its ADR 0024 index entry"
fi

# Fixture pins stay present with per-set dispositions.
if [[ -f "$pins" && -f "$expected" && -f "$fixture_build" ]] &&
  grep -q -F -e 'SELECTIVE_NPM = "supported"' "$pins" &&
  grep -q -F -e 'SELECTIVE_CARGO = "wont-fix"' "$pins" &&
  grep -q -F -e 'SELECTIVE_MAVEN = "wont-fix"' "$pins" &&
  grep -q -F -e 'SELECTIVE_NUGET = "wont-fix"' "$pins" &&
  grep -q -F -e 'SELECTIVE_GO = "wont-fix"' "$pins"; then
  ok
else
  bad "selective_update pins fixture lost its one-supported plus four-wont-fix dispositions under #583"
fi

# Pins record approved argv plus fail-closed hints plus bump follow-ups plus rejected.
if grep -q -F -e 'bazel run @pnpm//:pnpm -- update' "$pins" &&
  grep -q -F -e 'CARGO_BAZEL_REPIN=1 bazel build //rust/tests/fixtures/hello:hello' "$pins" &&
  grep -q -F -e 'REPIN=1 bazel run @maven//:pin' "$pins" &&
  grep -q -F -e 'paket2bazel' "$pins" &&
  grep -q -F -e 'use `dx update cargo` for the set' "$pins" &&
  grep -q -F -e 'use `dx update maven` for the set' "$pins" &&
  grep -q -F -e 'use `dx update nuget` for the set' "$pins" &&
  grep -q -F -e 'go pins track Gazelle for the shared go_deps extension' "$pins" &&
  grep -q -F -e 'silent full-update substitution rejected' "$pins" &&
  grep -q -F -e 'qualified seed-only under issue #583' "$pins"; then
  ok
else
  bad "selective_update pins.bzl lost its argv plus hints plus rejected wiring under #583"
fi

# Expected fixture pins the five per-set lines plus honesty lines.
if grep -q -F -e 'npm selective supported' "$expected" &&
  grep -q -F -e 'cargo selective wont-fix' "$expected" &&
  grep -q -F -e 'maven selective wont-fix' "$expected" &&
  grep -q -F -e 'nuget selective wont-fix' "$expected" &&
  grep -q -F -e 'go selective wont-fix' "$expected" &&
  grep -q -F -e 'silent full-update substitution rejected' "$expected" &&
  grep -q -F -e 'qualified seed-only under issue #583' "$expected" &&
  grep -q -F -e 'no Supported claim' "$expected"; then
  ok
else
  bad "selective_update.expected lost its five per-set plus honesty lines under #583"
fi

# Fixture BUILD exports pins plus expected with corpus coverage.
if grep -q -F -e 'pins.bzl' "$fixture_build" &&
  grep -q -F -e 'selective_update.expected' "$fixture_build" &&
  grep -q -F -e 'corpus_starlark' "$fixture_build"; then
  ok
else
  bad "selective_update BUILD.bazel lost its pins plus expected exports with corpus under #583"
fi

# Backend keeps exactly one selective executor (npm delegates packages to pnpm).
if grep -q -F -e '(SetId::Npm, SetRequest::Packages(packages))' "$backend" &&
  grep -q -F -e 'bazel", "run", "@pnpm//:pnpm", "--", "update"' "$backend" &&
  grep -q -F -e 'npm_selective_delegates_packages_to_pnpm' "$backend"; then
  ok
else
  bad "backend.rs lost its npm selective delegation to pnpm under #583"
fi

# Backend keeps four fail-closed Unsupported arms (never a silent full substitution).
if grep -q -F -e '(SetId::Cargo, SetRequest::Packages(_))' "$backend" &&
  grep -q -F -e '(SetId::Maven, SetRequest::Packages(_))' "$backend" &&
  grep -q -F -e '(SetId::NuGet, SetRequest::Packages(_))' "$backend" &&
  grep -q -F -e '(SetId::Go, SetRequest::Packages(_))' "$backend" &&
  grep -q -F -e 'BackendError::Unsupported' "$backend" &&
  grep -q -F -e 'non_npm_selective_reports_unsupported_never_full' "$backend" &&
  grep -q -F -e 'use `dx update cargo` for the set' "$backend" &&
  grep -q -F -e 'use `dx update maven` for the set' "$backend" &&
  grep -q -F -e 'use `dx update nuget` for the set' "$backend" &&
  grep -q -F -e 'go pins track Gazelle for the shared go_deps extension' "$backend"; then
  ok
else
  bad "backend.rs lost its four wont-fix Unsupported arms with full-set hints under #583"
fi

# Selector keeps package-only sets selective with full-wins and execution-time unsupported.
if grep -q -F -e 'Packages(Vec<String>)' "$selector" &&
  grep -q -F -e 'full wins' "$selector" &&
  grep -q -F -e 'other backends report `unsupported` at execution time' "$selector" &&
  grep -q -F -e 'package_only_sets_stay_selective_and_sorted' "$selector"; then
  ok
else
  bad "selector.rs lost its package-only selective plus full-wins plus execution-time unsupported record"
fi

# Live execution maps Unsupported to update_failed without launching; npm selective runs once.
if grep -q -F -e 'BackendError::Unsupported' "$exec_update" &&
  grep -q -F -e 'unsupported update:' "$exec_update" &&
  grep -q -F -e 'live_unsupported_selective_fails_without_launch' "$exec_tests_a" &&
  grep -q -F -e 'live_selective_npm_runs_once_with_packages' "$exec_tests_a"; then
  ok
else
  bad "exec/update.rs lost its unsupported-fails-without-launch plus npm-selective-runs-once execution"
fi

# Command docs carry the per-set table with the ADR plus fixture links.
if grep -q -F -e '| npm | Supported' "$command_doc" &&
  grep -q -F -e '| cargo | Wont-fix' "$command_doc" &&
  grep -q -F -e '| maven | Wont-fix' "$command_doc" &&
  grep -q -F -e '| nuget | Wont-fix' "$command_doc" &&
  grep -q -F -e '| go | Wont-fix' "$command_doc" &&
  grep -q -F -e '0024-selective-update.md' "$command_doc" &&
  grep -q -F -e 'cli/update/tests/fixtures/selective_update/' "$command_doc"; then
  ok
else
  bad "audit-update-bazel.md lost its per-set selective table with ADR plus fixture links under #583"
fi

# Command docs keep the automatic bump follow-up with selective-permitted npm (issue #638).
if grep -q -F -e 'Lock refresh chains automatically' "$command_doc" &&
  grep -q -F -e 'dx update cargo' "$command_doc" &&
  grep -q -F -e 'dx update npm:<pkg>' "$command_doc" &&
  grep -q -F -e 'Selective' "$command_doc"; then
  ok
else
  bad "audit-update-bazel.md lost its automatic bump follow-up with selective-permitted npm (issue #638)"
fi

# Bump execution chains the resolver-owned refresh automatically (issue #638).
if grep -q -F -e 'and refreshed' "$bump_exec" &&
  grep -q -F -e 'automatically' "$bump_exec" &&
  grep -q -F -e 'needs_update_refresh' "$bump_exec" &&
  grep -q -F -e 'runner.run' "$bump_exec"; then
  ok
else
  bad "exec/bump.rs lost its automatic resolver-owned follow-up record (issue #638)"
fi

# ci_targets_c.bzl owns the harness target plus dogfood-freshness wires it.
if grep -q -F -e 'name = "selective_update_qualification"' "tools/ci/ci_targets_c.bzl" &&
  grep -q -F -e 'selective_update_qualification.sh' "tools/ci/ci_targets_c.bzl" &&
  grep -q -F -e 'bazel run --noshow_progress //tools/ci:selective_update_qualification' tools/ci/dogfood_freshness.sh; then
  ok
else
  bad "tools/ci/ci_targets_c.bzl or dogfood_freshness.sh lost the selective_update_qualification wiring (want target plus dogfood-freshness)"
fi

dx_test_summary "selective update qualification harness"
