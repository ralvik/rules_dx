#!/usr/bin/env bash
# Selective NuGet per-package `dx update` qualification harness.
#
# Qualifies the NuGet wont-fix decision with fixtures plus docs:
# - wont-fix: per-package `nuget:<id>` (e.g. `nuget:FSharp.Core`) parses in
#   the selector but the approved `paket2bazel` regen
#   (`bazel run @rules_dotnet//tools/paket2bazel -- --dependencies-file
#   third_party/dotnet/paket.dependencies --output-folder
#   third_party/dotnet/deps`) has no per-id flag, so execution fails closed
#   as `unsupported` with the `dx update nuget` hint and never silently
#   substitutes a full update; private `paket.lock` surgery stays rejected
#   as a private resolver;
# - fixtures: `cli/update/tests/fixtures/selective_nuget/` (`pins.bzl`
#   plus `selective_nuget.expected`) pins disposition, approved full
#   argv, selector shape, hint, bump follow-up, and rejected routes;
# - record: ADR 0024 owns the per-set rationale; the command docs carry
#   the NuGet per-package note with the fixture link;
# - scope: update-only, no lock format change. Seed only: platform plus
#   consumer plus release evidence stays owned gap; no Supported claim.
#
# Versioned here, run by CI via `bazel run //tools/ci:selective_nuget_qualification`,
# following //tools/ci:selective_cargo_qualification.
set -euo pipefail

# Shared workspace + runfiles helpers.
# Bootstrap via tools/sh/bootstrap.sh (issue #654): runfiles forest first, then source tree.
source "${RUNFILES_DIR:-/dev/null}/_main/tools/sh/bootstrap.sh" 2>/dev/null || source "${TEST_SRCDIR:-/dev/null}/_main/tools/sh/bootstrap.sh" 2>/dev/null || source "$0.runfiles/_main/tools/sh/bootstrap.sh" 2>/dev/null || source "${BASH_SOURCE[0]}.runfiles/_main/tools/sh/bootstrap.sh" 2>/dev/null || source "$(git rev-parse --show-toplevel 2>/dev/null)/tools/sh/bootstrap.sh"
dx_bootstrap "tools/sh/lib.sh"

dx_cd_workspace

dx_test_init

dx_bash_pin

adr="docs/decisions/0024-selective-update.md"
pins="cli/update/tests/fixtures/selective_nuget/pins.bzl"
expected="cli/update/tests/fixtures/selective_nuget/selective_nuget.expected"
fixture_build="cli/update/tests/fixtures/selective_nuget/BUILD.bazel"
backend="cli/update/src/backend.rs"
selector="cli/update/src/selector.rs"
exec_update="cli/cli/src/exec/update.rs"
exec_tests_a="cli/cli/src/exec/update_tests_a.rs"
bump_exec="cli/cli/src/exec/bump.rs"
command_doc="docs/cli/commands/audit-update-bazel.md"
build="tools/ci/BUILD.bazel"
ci=".github/workflows/ci.yml"

# Backend keeps the approved NuGet full regen (never a dx lockfile).
if grep -q -F -e '(SetId::NuGet, SetRequest::Full)' "$backend" &&
  grep -q -F -e 'paket2bazel' "$backend" &&
  grep -q -F -e 'third_party/dotnet/paket.dependencies' "$backend"; then
  ok
else
  bad "backend.rs lost its approved NuGet full paket2bazel regen under #635"
fi

# Backend keeps the NuGet selective fail-closed arm with the full-set hint.
if grep -q -F -e '(SetId::NuGet, SetRequest::Packages(_))' "$backend" &&
  grep -q -F -e 'BackendError::Unsupported' "$backend" &&
  grep -q -F -e 'use `dx update nuget` for the set' "$backend"; then
  ok
else
  bad "backend.rs lost its NuGet selective Unsupported arm with full-set hint under #635"
fi

# Backend pins the dedicated NuGet selective wont-fix test (never a full substitution).
if grep -q -F -e 'nuget_selective_reports_unsupported_never_full' "$backend" &&
  grep -q -F -e 'paket.lock' "$backend"; then
  ok
else
  bad "backend.rs lost its dedicated nuget_selective_reports_unsupported_never_full pin under #635"
fi

# Backend keeps the shared never-full contract covering NuGet.
if grep -q -F -e 'non_npm_selective_reports_unsupported_never_full' "$backend"; then
  ok
else
  bad "backend.rs lost its shared non_npm never-full contract covering NuGet under #635"
fi

# Selector parses nuget:<id> with upstream-native id validation.
if grep -q -F -e 'nuget:FSharp.Core' "$selector" &&
  grep -q -F -e 'nuget ids use' "$selector"; then
  ok
else
  bad "selector.rs lost its nuget:FSharp.Core parse plus id validation under #635"
fi

# Live execution fails nuget:FSharp.Core without launching the updater.
if grep -q -F -e 'nuget:FSharp.Core' "$exec_tests_a" &&
  grep -q -F -e 'live_unsupported_nuget_selective_fails_without_launch' "$exec_tests_a" &&
  grep -q -F -e 'BackendError::Unsupported' "$exec_update"; then
  ok
else
  bad "exec/update.rs lost its nuget:FSharp.Core fails-without-launch execution under #635"
fi

# Fixture pins stay present with disposition plus full plus selector plus hint.
if [[ -f "$pins" && -f "$expected" && -f "$fixture_build" ]] &&
  grep -q -F -e 'SELECTIVE_NUGET = "wont-fix"' "$pins" &&
  grep -q -F -e 'paket2bazel' "$pins" &&
  grep -q -F -e 'nuget:FSharp.Core' "$pins" &&
  grep -q -F -e 'use `dx update nuget` for the set' "$pins" &&
  grep -q -F -e 'BUMP_FOLLOWUP_NUGET = "dx update nuget"' "$pins"; then
  ok
else
  bad "selective_nuget pins fixture lost its wont-fix plus full plus selector plus hint wiring under #635"
fi

# Pins record rejected routes plus honesty under #635.
if grep -q -F -e 'silent full-update substitution rejected' "$pins" &&
  grep -q -F -e 'private paket.lock surgery rejected' "$pins" &&
  grep -q -F -e 'qualified seed-only under issue #635' "$pins" &&
  grep -q -F -e 'no Supported claim' "$pins"; then
  ok
else
  bad "selective_nuget pins.bzl lost its rejected plus honesty wiring under #635"
fi

# Expected fixture pins the full plus wont-fix plus rejected plus honesty lines.
if grep -q -F -e 'nuget full supported' "$expected" &&
  grep -q -F -e 'nuget selective wont-fix' "$expected" &&
  grep -q -F -e 'nuget:FSharp.Core parses then fails closed' "$expected" &&
  grep -q -F -e 'silent full-update substitution rejected' "$expected" &&
  grep -q -F -e 'private paket.lock surgery rejected' "$expected" &&
  grep -q -F -e 'qualified seed-only under issue #635' "$expected" &&
  grep -q -F -e 'no Supported claim' "$expected"; then
  ok
else
  bad "selective_nuget.expected lost its full plus wont-fix plus honesty lines under #635"
fi

# Fixture BUILD exports pins plus expected with corpus coverage.
if grep -q -F -e 'pins.bzl' "$fixture_build" &&
  grep -q -F -e 'selective_nuget.expected' "$fixture_build" &&
  grep -q -F -e 'corpus_starlark' "$fixture_build"; then
  ok
else
  bad "selective_nuget BUILD.bazel lost its pins plus expected exports with corpus under #635"
fi

# Command docs carry the nuget table row plus the per-package #635 note with fixture link.
if grep -q -F -e '| nuget | Wont-fix' "$command_doc" &&
  grep -q -F -e 'nuget:FSharp.Core' "$command_doc" &&
  grep -q -F -e 'issue #635' "$command_doc" &&
  grep -q -F -e 'cli/update/tests/fixtures/selective_nuget/' "$command_doc" &&
  grep -q -F -e 'paket.lock' "$command_doc"; then
  ok
else
  bad "audit-update-bazel.md lost its NuGet per-package #635 note with fixture link"
fi

# ADR 0024 still owns the NuGet wont-fix rationale plus private-emulation rejection.
if grep -q -F -e 'NuGet `WONT-FIX`' "$adr" &&
  grep -q -F -e 'paket2bazel' "$adr" &&
  grep -q -F -e 'paket.lock' "$adr"; then
  ok
else
  bad "ADR 0024 lost its NuGet wont-fix plus paket2bazel plus paket-lock record"
fi

# Bump follow-up chains automatically resolver-owned for NuGet-adjacent sets (issue #638).
if grep -q -F -e 'dx update cargo' "$command_doc" &&
  grep -q -F -e 'and refreshed' "$bump_exec" &&
  grep -q -F -e 'automatically' "$bump_exec" &&
  grep -q -F -e 'needs_update_refresh' "$bump_exec"; then
  ok
else
  bad "bump follow-up lost its automatic resolver-owned record (issue #638)"
fi

# ci_targets_c.bzl owns the harness target plus dogfood-freshness wires it.
if grep -q -F -e 'name = "selective_nuget_qualification"' "tools/ci/ci_targets_c.bzl" &&
  grep -q -F -e 'selective_nuget_qualification.sh' "tools/ci/ci_targets_c.bzl" &&
  grep -q -F -e 'bazel run --noshow_progress //tools/ci:selective_nuget_qualification' tools/ci/dogfood_freshness.sh; then
  ok
else
  bad "tools/ci/ci_targets_c.bzl or dogfood_freshness.sh lost the selective_nuget_qualification wiring (want target plus dogfood-freshness)"
fi

# Backend keeps the never-names-a-dx-lockfile invariant on the NuGet path.
if grep -q -F -e 'argv_never_names_a_dx_lockfile' "$backend"; then
  ok
else
  bad "backend.rs lost its never-names-a-dx-lockfile invariant under #635"
fi

dx_test_summary "selective nuget qualification harness"
