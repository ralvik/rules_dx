#!/usr/bin/env bash
# Selective Go per-module `dx update` qualification harness.
#
# Qualifies the Go wont-fix decision with fixtures plus docs:
# - wont-fix: full stays an intentional no-op success (the main workspace
#   has no `go.mod` by design; the pinned `go_deps.from_file` module lock
#   `third_party/go/go.mod` plus `go.sum` tracks Gazelle, so there is no
#   launch); per-module `go:<module-path>` (e.g.
#   `go:github.com/google/go-cmp/cmp`) parses in the selector but has no
#   per-module update flag, so execution fails closed as `unsupported`
#   with the `dx bump gomod:<module> <version>` hint and never silently
#   substitutes the no-op; private `go get` plus `go mod tidy` stays
#   rejected as a private resolver;
# - fixtures: `cli/update/tests/fixtures/selective_go/` (`pins.bzl`
#   plus `selective_go.expected`) pins disposition, full no-op lock, selector
#   shape, hint, bump follow-up, and rejected routes;
# - record: ADR 0024 owns the per-set rationale; the command docs carry
#   the Go per-module note with the fixture link;
# - scope: update-only, no lock format change. Seed only: platform plus
#   consumer plus release evidence stays owned gap; no Supported claim.
#
# Versioned here, run by CI via `bazel run //tools/ci:selective_go_qualification`,
# following //tools/ci:selective_nuget_qualification.
set -euo pipefail

# Shared workspace + runfiles helpers.
# Bootstrap via tools/sh/bootstrap.sh (issue #654): runfiles forest first, then source tree.
source "${RUNFILES_DIR:-/dev/null}/_main/tools/sh/bootstrap.sh" 2>/dev/null || source "${TEST_SRCDIR:-/dev/null}/_main/tools/sh/bootstrap.sh" 2>/dev/null || source "$0.runfiles/_main/tools/sh/bootstrap.sh" 2>/dev/null || source "${BASH_SOURCE[0]}.runfiles/_main/tools/sh/bootstrap.sh" 2>/dev/null || source "$(git rev-parse --show-toplevel 2>/dev/null)/tools/sh/bootstrap.sh"
dx_bootstrap "tools/sh/lib.sh"

dx_cd_workspace

dx_test_init

adr="docs/decisions/0024-selective-update.md"
pins="cli/update/tests/fixtures/selective_go/pins.bzl"
expected="cli/update/tests/fixtures/selective_go/selective_go.expected"
fixture_build="cli/update/tests/fixtures/selective_go/BUILD.bazel"
backend="cli/update/src/backend.rs"
selector="cli/update/src/selector.rs"
sets="cli/update/src/sets.rs"
exec_update="cli/cli/src/exec/update.rs"
bump_exec="cli/cli/src/exec/bump.rs"
command_doc="docs/cli/commands/audit-update-bazel.md"
build="tools/ci/BUILD.bazel"
ci=".github/workflows/ci.yml"
module_lock="third_party/go/go.mod"

# Backend keeps the Go full intentional no-op (never a dx lockfile).
if grep -q -F -e '(SetId::Go, SetRequest::Full)' "$backend" &&
  grep -q -F -e 'BackendPlan::Noop' "$backend" &&
  grep -q -F -e 'go_full_is_pinned_noop_success' "$backend"; then
  ok
else
  bad "backend.rs lost its Go full pinned no-op success under #636"
fi

# Backend keeps the Go selective fail-closed arm with the bump hint.
if grep -q -F -e '(SetId::Go, SetRequest::Packages(_))' "$backend" &&
  grep -q -F -e 'BackendError::Unsupported' "$backend" &&
  grep -q -F -e 'dx bump gomod' "$backend"; then
  ok
else
  bad "backend.rs lost its Go selective Unsupported arm with bump hint under #636"
fi

# Backend pins the dedicated Go selective wont-fix test (never a full substitution).
if grep -q -F -e 'go_selective_reports_unsupported_never_full' "$backend" &&
  grep -q -F -e 'go get' "$backend"; then
  ok
else
  bad "backend.rs lost its dedicated go_selective_reports_unsupported_never_full pin under #636"
fi

# Backend keeps the shared never-full contract covering Go.
if grep -q -F -e 'non_npm_selective_reports_unsupported_never_full' "$backend"; then
  ok
else
  bad "backend.rs lost its shared non_npm never-full contract covering Go under #636"
fi

# Selector parses go:<module-path> with upstream-native module-path validation.
if grep -q -F -e 'go:github.com/google/go-cmp/cmp' "$selector" &&
  grep -q -F -e 'go_selective_parses_module_path_and_stays_selective' "$selector" &&
  grep -q -F -e 'go module paths use' "$selector"; then
  ok
else
  bad "selector.rs lost its go:github.com/google/go-cmp/cmp parse plus validation under #636"
fi

# Live execution fails go:github.com/google/go-cmp/cmp without launching the updater.
if grep -q -F -e 'go:github.com/google/go-cmp/cmp' "$exec_update" &&
  grep -q -F -e 'live_unsupported_go_selective_fails_without_launch' "$exec_update" &&
  grep -q -F -e 'BackendError::Unsupported' "$exec_update"; then
  ok
else
  bad "exec/update.rs lost its go per-module fails-without-launch execution under #636"
fi

# Fixture pins stay present with disposition plus full no-op plus selector plus hint.
if [[ -f "$pins" && -f "$expected" && -f "$fixture_build" ]] &&
  grep -q -F -e 'SELECTIVE_GO = "wont-fix"' "$pins" &&
  grep -q -F -e 'GO_FULL = "noop"' "$pins" &&
  grep -q -F -e 'go_deps.from_file' "$pins" &&
  grep -q -F -e 'go:github.com/google/go-cmp/cmp' "$pins" &&
  grep -q -F -e 'dx bump gomod' "$pins" &&
  grep -q -F -e 'BUMP_FOLLOWUP_GO = "dx update go"' "$pins"; then
  ok
else
  bad "selective_go pins fixture lost its wont-fix plus noop plus selector plus hint wiring under #636"
fi

# Pins record rejected routes plus honesty under #636.
if grep -q -F -e 'silent full-update substitution rejected' "$pins" &&
  grep -q -F -e 'private go get rejected' "$pins" &&
  grep -q -F -e 'private go mod tidy rejected' "$pins" &&
  grep -q -F -e 'qualified seed-only under issue #636' "$pins" &&
  grep -q -F -e 'no Supported claim' "$pins"; then
  ok
else
  bad "selective_go pins.bzl lost its rejected plus honesty wiring under #636"
fi

# Expected fixture pins the full noop plus wont-fix plus rejected plus honesty lines.
if grep -q -F -e 'go full noop' "$expected" &&
  grep -q -F -e 'go selective wont-fix' "$expected" &&
  grep -q -F -e 'go:github.com/google/go-cmp/cmp parses then fails closed' "$expected" &&
  grep -q -F -e 'silent full-update substitution rejected' "$expected" &&
  grep -q -F -e 'private go get rejected' "$expected" &&
  grep -q -F -e 'qualified seed-only under issue #636' "$expected" &&
  grep -q -F -e 'no Supported claim' "$expected"; then
  ok
else
  bad "selective_go.expected lost its full plus wont-fix plus honesty lines under #636"
fi

# Fixture BUILD exports pins plus expected with corpus coverage.
if grep -q -F -e 'pins.bzl' "$fixture_build" &&
  grep -q -F -e 'selective_go.expected' "$fixture_build" &&
  grep -q -F -e 'corpus_starlark' "$fixture_build"; then
  ok
else
  bad "selective_go BUILD.bazel lost its pins plus expected exports with corpus under #636"
fi

# Command docs carry the go table row plus the per-module #636 note with fixture link.
if grep -q -F -e '| go | Wont-fix' "$command_doc" &&
  grep -q -F -e 'go:github.com/google/go-cmp/cmp' "$command_doc" &&
  grep -q -F -e 'issue #636' "$command_doc" &&
  grep -q -F -e 'cli/update/tests/fixtures/selective_go/' "$command_doc" &&
  grep -q -F -e 'go get' "$command_doc"; then
  ok
else
  bad "audit-update-bazel.md lost its Go per-module #636 note with fixture link"
fi

# ADR 0024 still owns the Go wont-fix rationale plus private-emulation rejection.
if grep -q -F -e 'Go `WONT-FIX`' "$adr" &&
  grep -q -F -e 'go_deps.from_file' "$adr" &&
  grep -q -F -e 'go get' "$adr"; then
  ok
else
  bad "ADR 0024 lost its Go wont-fix plus from_file plus go-get record"
fi

# Bump follow-up chains automatically resolver-owned for Go (issue #638).
if grep -q -F -e 'dx update go' "$command_doc" &&
  grep -q -F -e 'and refreshed' "$bump_exec" &&
  grep -q -F -e 'automatically' "$bump_exec" &&
  grep -q -F -e 'needs_update_refresh' "$bump_exec" &&
  grep -q -F -e 'BackendPlan::Noop' "$bump_exec"; then
  ok
else
  bad "bump follow-up lost its automatic resolver-owned dx update go record (issue #638)"
fi

# Set registry plus module lock keep the Go from_file wiring.
if grep -q -F -e 'third_party/go/go.mod' "$sets" &&
  grep -q -F -e 'go_deps.from_file' "$sets" &&
  [[ -f "$module_lock" ]] &&
  grep -q -F -e 'module rules_dx/third_party/go' "$module_lock"; then
  ok
else
  bad "sets.rs or third_party/go/go.mod lost its Go from_file wiring under #636"
fi

# BUILD owns the harness target plus CI wires it in dogfood-freshness.
if grep -q -F -e 'name = "selective_go_qualification"' "$build" &&
  grep -q -F -e 'selective_go_qualification.sh' "$build" &&
  grep -q -F -e 'bazel run --noshow_progress //tools/ci:selective_go_qualification' "$ci"; then
  ok
else
  bad "tools/ci/BUILD.bazel or ci.yml lost the selective_go_qualification wiring (want target plus dogfood-freshness)"
fi

dx_test_summary "selective go qualification harness"
