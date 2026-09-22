#!/usr/bin/env bash
# Selective Cargo per-crate `dx update` qualification harness.
#
# Qualifies the Cargo wont-fix decision with fixtures plus docs:
# - wont-fix: per-crate `cargo:<crate>` (e.g. `cargo:anyhow`) parses in
#   the selector but the approved `crate_universe` repin
#   (`CARGO_BAZEL_REPIN=1 bazel build //rust/tests/fixtures/hello:hello`)
#   has no per-crate flag, so execution fails closed as `unsupported`
#   with the `dx update cargo` hint and never silently substitutes a
#   full update; private `cargo update -p` stays rejected as a private
#   resolver;
# - fixtures: `cli/update/tests/fixtures/selective_cargo/` (`pins.bzl`
#   plus `selective_cargo.expected`) pins disposition, approved full
#   argv, selector shape, hint, bump follow-up, and rejected routes;
# - record: ADR 0024 owns the per-set rationale; the command docs carry
#   the Cargo per-crate note with the fixture link;
# - scope: update-only, no lock format change. Seed only: platform plus
#   consumer plus release evidence stays owned gap; no Supported claim.
#
# Versioned here, run by CI via `bazel run //tools/ci:selective_cargo_qualification`,
# following //tools/ci:selective_update_qualification.
set -euo pipefail

# Shared workspace + runfiles helpers.
# Bootstrap via tools/sh/bootstrap.sh (issue #654): runfiles forest first, then source tree.
source "${RUNFILES_DIR:-/dev/null}/_main/tools/sh/bootstrap.sh" 2>/dev/null || source "${TEST_SRCDIR:-/dev/null}/_main/tools/sh/bootstrap.sh" 2>/dev/null || source "$0.runfiles/_main/tools/sh/bootstrap.sh" 2>/dev/null || source "${BASH_SOURCE[0]}.runfiles/_main/tools/sh/bootstrap.sh" 2>/dev/null || source "$(git rev-parse --show-toplevel 2>/dev/null)/tools/sh/bootstrap.sh"
dx_bootstrap "tools/sh/lib.sh"

dx_cd_workspace

dx_test_init

adr="docs/decisions/0024-selective-update.md"
pins="cli/update/tests/fixtures/selective_cargo/pins.bzl"
expected="cli/update/tests/fixtures/selective_cargo/selective_cargo.expected"
fixture_build="cli/update/tests/fixtures/selective_cargo/BUILD.bazel"
backend="cli/update/src/backend.rs"
selector="cli/update/src/selector.rs"
exec_update="cli/cli/src/exec/update.rs"
bump_exec="cli/cli/src/exec/bump.rs"
command_doc="docs/cli/commands/audit-update-bazel.md"
build="tools/ci/BUILD.bazel"
ci=".github/workflows/ci.yml"

# Backend keeps the approved Cargo full repin (never a dx lockfile).
if grep -q -F -e '(SetId::Cargo, SetRequest::Full)' "$backend" &&
  grep -q -F -e 'CARGO_BAZEL_REPIN' "$backend" &&
  grep -q -F -e '//rust/tests/fixtures/hello:hello' "$backend"; then
  ok
else
  bad "backend.rs lost its approved Cargo full crate_universe repin under #633"
fi

# Backend keeps the Cargo selective fail-closed arm with the full-set hint.
if grep -q -F -e '(SetId::Cargo, SetRequest::Packages(_))' "$backend" &&
  grep -q -F -e 'BackendError::Unsupported' "$backend" &&
  grep -q -F -e 'use `dx update cargo` for the set' "$backend"; then
  ok
else
  bad "backend.rs lost its Cargo selective Unsupported arm with full-set hint under #633"
fi

# Backend pins the dedicated Cargo selective wont-fix test (never a full substitution).
if grep -q -F -e 'cargo_selective_reports_unsupported_never_full' "$backend" &&
  grep -q -F -e 'cargo update -p' "$backend"; then
  ok
else
  bad "backend.rs lost its dedicated cargo_selective_reports_unsupported_never_full pin under #633"
fi

# Backend keeps the shared never-full contract covering Cargo.
if grep -q -F -e 'non_npm_selective_reports_unsupported_never_full' "$backend"; then
  ok
else
  bad "backend.rs lost its shared non_npm never-full contract covering Cargo under #633"
fi

# Selector parses cargo:<crate> with upstream-native crate-name validation.
if grep -q -F -e 'cargo:anyhow' "$selector" &&
  grep -q -F -e 'cargo crate names use' "$selector"; then
  ok
else
  bad "selector.rs lost its cargo:anyhow parse plus crate-name validation under #633"
fi

# Live execution fails cargo:anyhow without launching the updater.
if grep -q -F -e 'cargo:anyhow' "$exec_update" &&
  grep -q -F -e 'live_unsupported_selective_fails_without_launch' "$exec_update" &&
  grep -q -F -e 'BackendError::Unsupported' "$exec_update"; then
  ok
else
  bad "exec/update.rs lost its cargo:anyhow fails-without-launch execution under #633"
fi

# Fixture pins stay present with disposition plus full plus selector plus hint.
if [[ -f "$pins" && -f "$expected" && -f "$fixture_build" ]] &&
  grep -q -F -e 'SELECTIVE_CARGO = "wont-fix"' "$pins" &&
  grep -q -F -e 'CARGO_BAZEL_REPIN=1 bazel build //rust/tests/fixtures/hello:hello' "$pins" &&
  grep -q -F -e 'cargo:anyhow' "$pins" &&
  grep -q -F -e 'use `dx update cargo` for the set' "$pins" &&
  grep -q -F -e 'BUMP_FOLLOWUP_CARGO = "dx update cargo"' "$pins"; then
  ok
else
  bad "selective_cargo pins fixture lost its wont-fix plus full plus selector plus hint wiring under #633"
fi

# Pins record rejected routes plus honesty under #633.
if grep -q -F -e 'silent full-update substitution rejected' "$pins" &&
  grep -q -F -e 'private cargo update -p rejected' "$pins" &&
  grep -q -F -e 'qualified seed-only under issue #633' "$pins" &&
  grep -q -F -e 'no Supported claim' "$pins"; then
  ok
else
  bad "selective_cargo pins.bzl lost its rejected plus honesty wiring under #633"
fi

# Expected fixture pins the full plus wont-fix plus rejected plus honesty lines.
if grep -q -F -e 'cargo full supported' "$expected" &&
  grep -q -F -e 'cargo selective wont-fix' "$expected" &&
  grep -q -F -e 'cargo:anyhow parses then fails closed' "$expected" &&
  grep -q -F -e 'silent full-update substitution rejected' "$expected" &&
  grep -q -F -e 'private cargo update -p rejected' "$expected" &&
  grep -q -F -e 'qualified seed-only under issue #633' "$expected" &&
  grep -q -F -e 'no Supported claim' "$expected"; then
  ok
else
  bad "selective_cargo.expected lost its full plus wont-fix plus honesty lines under #633"
fi

# Fixture BUILD exports pins plus expected with corpus coverage.
if grep -q -F -e 'pins.bzl' "$fixture_build" &&
  grep -q -F -e 'selective_cargo.expected' "$fixture_build" &&
  grep -q -F -e 'corpus_starlark' "$fixture_build"; then
  ok
else
  bad "selective_cargo BUILD.bazel lost its pins plus expected exports with corpus under #633"
fi

# Command docs carry the cargo table row plus the per-crate #633 note with fixture link.
if grep -q -F -e '| cargo | Wont-fix' "$command_doc" &&
  grep -q -F -e 'cargo:anyhow' "$command_doc" &&
  grep -q -F -e 'issue #633' "$command_doc" &&
  grep -q -F -e 'cli/update/tests/fixtures/selective_cargo/' "$command_doc" &&
  grep -q -F -e 'cargo update -p' "$command_doc"; then
  ok
else
  bad "audit-update-bazel.md lost its Cargo per-crate #633 note with fixture link"
fi

# ADR 0024 still owns the Cargo wont-fix rationale plus private-emulation rejection.
if grep -q -F -e 'Cargo `WONT-FIX`' "$adr" &&
  grep -q -F -e 'crate_universe' "$adr" &&
  grep -q -F -e 'cargo update -p' "$adr"; then
  ok
else
  bad "ADR 0024 lost its Cargo wont-fix plus crate_universe plus cargo-update-p record"
fi

# Bump follow-up chains automatically resolver-owned for Cargo (issue #638).
if grep -q -F -e 'dx update cargo' "$command_doc" &&
  grep -q -F -e 'and refreshed' "$bump_exec" &&
  grep -q -F -e 'automatically' "$bump_exec" &&
  grep -q -F -e 'needs_update_refresh' "$bump_exec" &&
  grep -q -F -e 'runner.run' "$bump_exec"; then
  ok
else
  bad "bump follow-up lost its automatic resolver-owned dx update cargo record (issue #638)"
fi

# BUILD owns the harness target plus CI wires it in dogfood-freshness.
if grep -q -F -e 'name = "selective_cargo_qualification"' "$build" &&
  grep -q -F -e 'selective_cargo_qualification.sh' "$build" &&
  grep -q -F -e 'bazel run --noshow_progress //tools/ci:selective_cargo_qualification' "$ci"; then
  ok
else
  bad "tools/ci/BUILD.bazel or ci.yml lost the selective_cargo_qualification wiring (want target plus dogfood-freshness)"
fi

# Backend keeps the never-names-a-dx-lockfile invariant on the Cargo path.
if grep -q -F -e 'argv_never_names_a_dx_lockfile' "$backend"; then
  ok
else
  bad "backend.rs lost its never-names-a-dx-lockfile invariant under #633"
fi

dx_test_summary "selective cargo qualification harness"
