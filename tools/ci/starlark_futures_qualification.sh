#!/usr/bin/env bash
# Starlark testing futures qualification harness (issue #588).
#
# Qualifies the nine per-future decisions under ADR 0009
# (provisional pending concrete use cases):
# - wont-fix: per-check filtering (target granularity is contract),
#   per-function targets (explicit macro instantiation is contract),
#   Rust orchestration with BEP (single invocation, no nested Bazel);
# - deferred: richer matchers plus aspect plus toolchain plus
#   configuration (including transitions) plus output-group plus action
#   (including registered-action) subjects, each pending a concrete use
#   case plus fixtures plus successor issue;
# - rejected: second Starlark interpreter, per-check --test_filter
#   parsing, nested Bazel invocation, behavioral matrix as line coverage;
# - fixtures: `libs/starlark/tests/fixtures/starlark_futures/` (`pins.bzl`
#   plus `starlark_futures.expected`) pins dispositions plus rejected
#   routes plus honesty;
# - scope: test framework only, no Bazel semantics change.
#   Seed only: no Supported claim.
#
# Versioned here, run by CI via `bazel run //tools/ci:starlark_futures_qualification`,
# following //tools/ci:update_events_qualification.
set -euo pipefail

# Shared workspace + runfiles helpers (issue #319).
# Bootstrap: Bazel runfiles forest first (`data = ["//tools/sh:lib"]`), then source tree.
source "${RUNFILES_DIR:-/dev/null}/_main/tools/sh/lib.sh" 2>/dev/null || source "${TEST_SRCDIR:-/dev/null}/_main/tools/sh/lib.sh" 2>/dev/null || source "$0.runfiles/_main/tools/sh/lib.sh" 2>/dev/null || source "${BASH_SOURCE[0]}.runfiles/_main/tools/sh/lib.sh" 2>/dev/null || source "$(dirname "${BASH_SOURCE[0]}")/../sh/lib.sh"

dx_cd_workspace

dx_test_init

doc="docs/testing/starlark.md"
adr="docs/decisions/0009-starlark-testing.md"
defs="libs/starlark/defs.bzl"
pins="libs/starlark/tests/fixtures/starlark_futures/pins.bzl"
expected="libs/starlark/tests/fixtures/starlark_futures/starlark_futures.expected"
fixture_build="libs/starlark/tests/fixtures/starlark_futures/BUILD.bazel"
build="tools/ci/BUILD.bazel"
ci=".github/workflows/ci.yml"
verify="docs/testing/verification-matrix.md"

# Docs decide the nine futures under #588 with ADR plus fixtures plus qualification.
if grep -q -F -e 'Decided under issue #588' "$doc" &&
  grep -q -F -e 'provisional pending concrete use cases' "$doc" &&
  grep -q -F -e 'libs/starlark/tests/fixtures/starlark_futures/' "$doc" &&
  grep -q -F -e 'bazel run //tools/ci:starlark_futures_qualification' "$doc" &&
  grep -q -F -e 'second Starlark interpreter is rejected' "$doc" &&
  grep -q -F -e 'line coverage is rejected' "$doc"; then
  ok
else
  bad "starlark.md lost its #588 decided header with ADR plus fixtures plus qualification plus rejected routes"
fi

# Docs keep per-check filtering wont-fix on target granularity.
if grep -q -F -e 'Per-check filtering stays wont-fix' "$doc" &&
  grep -q -F -e 'target granularity is contract' "$doc" &&
  grep -q -F -e 'does not interpret' "$doc" &&
  grep -q -F -e '`--test_filter` parsing is rejected' "$doc"; then
  ok
else
  bad "starlark.md lost its per-check filtering wont-fix on target granularity"
fi

# Docs keep richer matchers deferred on expect_equal only.
if grep -q -F -e 'Richer matchers stay deferred' "$doc" &&
  grep -q -F -e '`expect_equal` only' "$doc" &&
  grep -q -F -e 'concrete use case plus fixtures plus successor issue' "$doc"; then
  ok
else
  bad "starlark.md lost its richer-matchers deferred on expect_equal only"
fi

# Docs keep all five subject families deferred with transition plus registered-action coverage.
if grep -q -F -e 'Aspect subjects stay deferred' "$doc" &&
  grep -q -F -e 'Toolchain subjects stay deferred' "$doc" &&
  grep -q -F -e 'Configuration subjects stay deferred' "$doc" &&
  grep -q -F -e 'including transitions' "$doc" &&
  grep -q -F -e 'Output-group subjects stay deferred' "$doc" &&
  grep -q -F -e 'Action subjects stay deferred' "$doc" &&
  grep -q -F -e 'including registered-action' "$doc"; then
  ok
else
  bad "starlark.md lost its five subject-family deferred record under #588"
fi

# Docs keep per-function targets wont-fix on explicit macro instantiation.
if grep -q -F -e 'Per-function targets stay wont-fix' "$doc" &&
  grep -q -F -e 'explicit macro instantiation is contract' "$doc" &&
  grep -q -F -e 'static `load()` constraint' "$doc"; then
  ok
else
  bad "starlark.md lost its per-function wont-fix on explicit macro instantiation"
fi

# Docs keep Rust orchestration wont-fix on single invocation with no nested Bazel.
if grep -q -F -e 'Rust orchestration of fixture workspaces with BEP consumption stays' "$doc" &&
  grep -q -F -e 'single invocation, no nested Bazel' "$doc" &&
  grep -q -F -e 'Nested Bazel invocation is rejected' "$doc"; then
  ok
else
  bad "starlark.md lost its Rust orchestration wont-fix on single invocation"
fi

# ADR 0009 stays consulted: provisional pending concrete use cases with no claim.
if grep -q -F -e 'provisional pending concrete use cases' "$adr" &&
  grep -q -F -e 'This record does not claim them' "$adr" &&
  grep -q -F -e 'ships no Rust orchestration' "$adr" &&
  grep -q -F -e 'no nested Bazel' "$adr"; then
  ok
else
  bad "ADR 0009 lost its provisional pending concrete use cases record consulted by #588"
fi

# Code keeps expect_equal only with no richer matcher definitions.
if grep -q -F -e 'def expect_equal' "$defs" &&
  ! grep -q -F -e 'def expect_true' "$defs" &&
  ! grep -q -F -e 'def expect_contains' "$defs" &&
  ! grep -q -F -e 'def expect_match' "$defs"; then
  ok
else
  bad "defs.bzl lost its expect_equal-only matcher surface under #588"
fi

# Code keeps no per-check filtering: runner never handles test_filter.
if ! grep -q -F -e 'test_filter' "$defs" &&
  ! grep -q -F -e 'TEST_FILTER' "$defs"; then
  ok
else
  bad "defs.bzl must not handle test_filter per check under #588"
fi

# Code observes DefaultInfo plus DxSubjectInfo only with no broader subject plumbing.
if grep -q -F -e 'DxSubjectInfo' "$defs" &&
  grep -q -F -e 'DefaultInfo' "$defs" &&
  ! grep -q -F -e 'OutputGroupInfo' "$defs" &&
  ! grep -q -F -e 'Toolchain' "$defs"; then
  ok
else
  bad "defs.bzl lost its DefaultInfo plus DxSubjectInfo-only observation under #588"
fi

# Code keeps one-call-one-target dispatch with no nested Bazel.
if grep -q -F -e '_MODES' "$defs" &&
  grep -q -F -e 'def starlark_test' "$defs" &&
  grep -q -F -e 'One macro call is one addressable' "$defs" &&
  ! grep -q -F -e 'bazel run' "$defs"; then
  ok
else
  bad "defs.bzl lost its one-call-one-target dispatch with no nested Bazel under #588"
fi

# Fixture pins stay present with nine dispositions plus provisional plus rejected plus honesty.
if [[ -f "$pins" && -f "$expected" && -f "$fixture_build" ]] &&
  grep -q -F -e 'PER_CHECK_FILTERING = "wont-fix"' "$pins" &&
  grep -q -F -e 'RICHER_MATCHERS = "deferred"' "$pins" &&
  grep -q -F -e 'ASPECT_SUBJECTS = "deferred"' "$pins" &&
  grep -q -F -e 'TOOLCHAIN_SUBJECTS = "deferred"' "$pins" &&
  grep -q -F -e 'CONFIGURATION_SUBJECTS = "deferred"' "$pins" &&
  grep -q -F -e 'OUTPUT_GROUP_SUBJECTS = "deferred"' "$pins" &&
  grep -q -F -e 'ACTION_SUBJECTS = "deferred"' "$pins" &&
  grep -q -F -e 'PER_FUNCTION_TARGETS = "wont-fix"' "$pins" &&
  grep -q -F -e 'RUST_ORCHESTRATION_BEP = "wont-fix"' "$pins" &&
  grep -q -F -e 'provisional pending concrete use cases' "$pins" &&
  grep -q -F -e 'REJECTED_NESTED_BAZEL' "$pins" &&
  grep -q -F -e 'qualified seed-only under issue #588' "$pins"; then
  ok
else
  bad "starlark_futures pins fixture lost its nine dispositions plus rejected wiring under #588"
fi

# Expected fixture pins the nine futures plus rejected plus honesty lines.
if grep -q -F -e 'per-check filtering wont-fix' "$expected" &&
  grep -q -F -e 'richer matchers deferred' "$expected" &&
  grep -q -F -e 'aspect subjects deferred' "$expected" &&
  grep -q -F -e 'toolchain subjects deferred' "$expected" &&
  grep -q -F -e 'configuration subjects deferred' "$expected" &&
  grep -q -F -e 'output-group subjects deferred' "$expected" &&
  grep -q -F -e 'action subjects deferred' "$expected" &&
  grep -q -F -e 'per-function targets wont-fix' "$expected" &&
  grep -q -F -e 'Rust orchestration with BEP wont-fix' "$expected" &&
  grep -q -F -e 'nested Bazel invocation rejected' "$expected" &&
  grep -q -F -e 'qualified seed-only under issue #588' "$expected" &&
  grep -q -F -e 'no Supported claim' "$expected"; then
  ok
else
  bad "starlark_futures.expected lost its nine futures plus honesty lines under #588"
fi

# Fixture BUILD exports pins plus expected with corpus coverage.
if grep -q -F -e 'pins.bzl' "$fixture_build" &&
  grep -q -F -e 'starlark_futures.expected' "$fixture_build" &&
  grep -q -F -e 'corpus_starlark' "$fixture_build"; then
  ok
else
  bad "starlark_futures BUILD.bazel lost its pins plus expected exports with corpus under #588"
fi

# BUILD owns the harness target plus CI wires it in dogfood-freshness.
if grep -q -F -e 'name = "starlark_futures_qualification"' "$build" &&
  grep -q -F -e 'starlark_futures_qualification.sh' "$build" &&
  grep -q -F -e 'bazel run --noshow_progress //tools/ci:starlark_futures_qualification' "$ci"; then
  ok
else
  bad "tools/ci/BUILD.bazel or ci.yml lost the starlark_futures_qualification wiring (want target plus dogfood-freshness)"
fi

# Verification matrix owns the harness entry as seed-only fixture evidence.
if grep -q -F -e ':starlark_futures_qualification' "$verify" &&
  grep -q -F -e 'issue #588' "$verify"; then
  ok
else
  bad "verification-matrix.md lost its starlark_futures_qualification entry under #588"
fi

dx_test_summary "starlark futures qualification harness"
