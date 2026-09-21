#!/usr/bin/env bash
# Starlark testing futures qualification harness.
#
# Qualifies the nine per-future decisions under ADR 0009
# (remaining subjects provisional pending concrete use cases;
# richer matchers graduated under #790):
# - wont-fix: per-check filtering (target granularity is contract),
#   per-function targets (explicit macro instantiation is contract),
#   Rust orchestration with BEP (single invocation, no nested Bazel);
# - supported: richer matchers (expect_equal plus expect_true plus
#   expect_false plus expect_contains plus expect_match, qualified under
#   #790 with the greet plus pair-error plus admitted-list plus
#   subject-fields plus fingerprint use case);
# - deferred: aspect plus toolchain plus configuration (including
#   transitions) plus output-group plus action (including
#   registered-action) subjects, each pending a concrete use case plus
#   fixtures plus successor issue;
# - rejected: second Starlark interpreter, per-check --test_filter
#   parsing, nested Bazel invocation, behavioral matrix as line coverage;
# - fixtures: `libs/starlark/tests/fixtures/starlark_futures/` (`pins.bzl`
#   plus `starlark_futures.expected` plus `matchers.bzl`) pins dispositions
#   plus rejected routes plus honesty;
# - scope: test framework only, no Bazel semantics change.
#   Seed only: no Supported claim.
#
# Versioned here, run by CI via `bazel run //tools/ci:starlark_futures_qualification`,
# following //tools/ci:update_events_qualification.
set -euo pipefail

# Shared workspace + runfiles helpers.
# Bootstrap via tools/sh/bootstrap.sh (issue #654): runfiles forest first, then source tree.
source "${RUNFILES_DIR:-/dev/null}/_main/tools/sh/bootstrap.sh" 2>/dev/null || source "${TEST_SRCDIR:-/dev/null}/_main/tools/sh/bootstrap.sh" 2>/dev/null || source "$0.runfiles/_main/tools/sh/bootstrap.sh" 2>/dev/null || source "${BASH_SOURCE[0]}.runfiles/_main/tools/sh/bootstrap.sh" 2>/dev/null || source "$(git rev-parse --show-toplevel 2>/dev/null)/tools/sh/bootstrap.sh"
dx_bootstrap "tools/sh/lib.sh"

dx_cd_workspace

dx_test_init

doc="docs/testing/starlark.md"
adr="docs/decisions/0009-starlark-testing.md"
defs="libs/starlark/defs.bzl"
matrix="libs/starlark/behavioral_matrix.md"
pins="libs/starlark/tests/fixtures/starlark_futures/pins.bzl"
expected="libs/starlark/tests/fixtures/starlark_futures/starlark_futures.expected"
matchers="libs/starlark/tests/fixtures/starlark_futures/matchers.bzl"
matcher_tests="libs/starlark/tests/matcher_tests.bzl"
tests_build="libs/starlark/tests/BUILD.bazel"
matrix_tests="libs/starlark/tests/matrix_tests.bzl"
fixture_build="libs/starlark/tests/fixtures/starlark_futures/BUILD.bazel"
build="tools/ci/ci_targets_c.bzl"
dogfood="tools/ci/dogfood_freshness.sh"
verify="docs/testing/verification-matrix.md"

# Docs decide the nine futures under with ADR plus fixtures plus qualification.
if grep -q -F -e 'Decided under closed #588 plus #790' "$doc" &&
  grep -q -F -e 'remaining subjects provisional pending concrete use cases' "$doc" &&
  grep -q -F -e 'libs/starlark/tests/fixtures/starlark_futures/' "$doc" &&
  grep -q -F -e 'bazel run //tools/ci:starlark_futures_qualification' "$doc" &&
  grep -q -F -e 'second Starlark interpreter is rejected' "$doc" &&
  grep -q -F -e 'line coverage is rejected' "$doc"; then
  ok
else
  bad "starlark.md lost its #588 plus #790 decided header with ADR plus fixtures plus qualification plus rejected routes"
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

# Docs graduate richer matchers under #790 with the five constructors.
if grep -q -F -e 'Richer matchers graduated under #790' "$doc" &&
  grep -q -F -e '`expect_true`' "$doc" &&
  grep -q -F -e '`expect_contains`' "$doc" &&
  grep -q -F -e '`expect_match`' "$doc" &&
  grep -q -F -e 'matchers.bzl' "$doc" &&
  grep -q -F -e 'no larger matcher library' "$doc"; then
  ok
else
  bad "starlark.md lost its richer-matchers graduation under #790 with five constructors plus use case"
fi

# Docs author the five constructors with the futures use case.
if grep -q -F -e 'expect_true(name, actual)' "$doc" &&
  grep -q -F -e 'expect_contains(name, haystack, needle)' "$doc" &&
  grep -q -F -e 'expect_match(name, value, want)' "$doc" &&
  grep -q -F -e '//libs/starlark/tests:matcher_unit' "$doc"; then
  ok
else
  bad "starlark.md lost its five-constructor authoring with matcher_unit use case"
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
  bad "starlark.md lost its five subject-family deferred record under #588 plus #790"
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

# ADR 0009 qualifies the five constructors with no larger library claim.
if grep -q -F -e 'five constructors' "$adr" &&
  grep -q -F -e 'expect_contains' "$adr" &&
  grep -q -F -e 'expect_match' "$adr" &&
  grep -q -F -e 'graduated under #790' "$adr" &&
  grep -q -F -e 'no larger matcher library' "$adr" &&
  grep -q -F -e 'ships no Rust orchestration' "$adr" &&
  grep -q -F -e 'no nested Bazel' "$adr"; then
  ok
else
  bad "ADR 0009 lost its five-constructor graduation under #790 with no larger library"
fi

# Code ships the five matcher constructors with kinded records.
if grep -q -F -e 'def expect_equal' "$defs" &&
  grep -q -F -e 'def expect_true' "$defs" &&
  grep -q -F -e 'def expect_false' "$defs" &&
  grep -q -F -e 'def expect_contains' "$defs" &&
  grep -q -F -e 'def expect_match' "$defs"; then
  ok
else
  bad "defs.bzl lost its five-constructor matcher surface under #790"
fi

# Code renders each kind through the generated runner with accumulation.
if grep -q -F -e 'check_true' "$defs" &&
  grep -q -F -e 'check_false' "$defs" &&
  grep -q -F -e 'check_contains' "$defs" &&
  grep -q -F -e 'check_match' "$defs" &&
  grep -q -F -e 'kind' "$defs"; then
  ok
else
  bad "defs.bzl lost its per-kind runner rendering under #790"
fi

# Code keeps no per-check filtering: runner never handles test_filter.
if ! grep -q -F -e 'test_filter' "$defs" &&
  ! grep -q -F -e 'TEST_FILTER' "$defs"; then
  ok
else
  bad "defs.bzl must not handle test_filter per check under #588 plus #790"
fi

# Code observes DefaultInfo plus DxSubjectInfo only with no broader subject plumbing.
if grep -q -F -e 'DxSubjectInfo' "$defs" &&
  grep -q -F -e 'DefaultInfo' "$defs" &&
  ! grep -q -F -e 'OutputGroupInfo' "$defs" &&
  ! grep -q -F -e 'Toolchain' "$defs"; then
  ok
else
  bad "defs.bzl lost its DefaultInfo plus DxSubjectInfo-only observation under #588 plus #790"
fi

# Code keeps one-call-one-target dispatch with no nested Bazel.
if grep -q -F -e '_MODES' "$defs" &&
  grep -q -F -e 'def starlark_test' "$defs" &&
  grep -q -F -e 'One macro call is one addressable' "$defs" &&
  ! grep -q -F -e 'bazel run' "$defs"; then
  ok
else
  bad "defs.bzl lost its one-call-one-target dispatch with no nested Bazel under #588 plus #790"
fi

# Fixture pins stay present with nine dispositions plus matcher use case plus #790.
if [[ -f "$pins" && -f "$expected" && -f "$fixture_build" && -f "$matchers" ]] &&
  grep -q -F -e 'PER_CHECK_FILTERING = "wont-fix"' "$pins" &&
  grep -q -F -e 'RICHER_MATCHERS = "supported"' "$pins" &&
  grep -q -F -e 'RICHER_MATCHERS_USE_CASE' "$pins" &&
  grep -q -F -e 'RICHER_MATCHERS_SURFACE' "$pins" &&
  grep -q -F -e 'qualified under issue #790' "$pins" &&
  grep -q -F -e 'ASPECT_SUBJECTS = "deferred"' "$pins" &&
  grep -q -F -e 'TOOLCHAIN_SUBJECTS = "deferred"' "$pins" &&
  grep -q -F -e 'CONFIGURATION_SUBJECTS = "deferred"' "$pins" &&
  grep -q -F -e 'OUTPUT_GROUP_SUBJECTS = "deferred"' "$pins" &&
  grep -q -F -e 'ACTION_SUBJECTS = "deferred"' "$pins" &&
  grep -q -F -e 'PER_FUNCTION_TARGETS = "wont-fix"' "$pins" &&
  grep -q -F -e 'RUST_ORCHESTRATION_BEP = "wont-fix"' "$pins" &&
  grep -q -F -e 'provisional pending concrete use cases' "$pins" &&
  grep -q -F -e 'REJECTED_NESTED_BAZEL' "$pins"; then
  ok
else
  bad "starlark_futures pins fixture lost its nine dispositions plus matcher use case under #790"
fi

# Fixture matchers.bzl carries the concrete use case.
if grep -q -F -e 'def greet_report' "$matchers" &&
  grep -q -F -e 'def pair_error' "$matchers" &&
  grep -q -F -e 'def admitted_pairs' "$matchers" &&
  grep -q -F -e 'def subject_fields' "$matchers" &&
  grep -q -F -e 'def fingerprint_like' "$matchers" &&
  grep -q -F -e 'def is_even' "$matchers" &&
  grep -q -F -e 'issue #790' "$matchers"; then
  ok
else
  bad "starlark_futures matchers.bzl lost its #790 use-case subjects"
fi

# Expected fixture pins richer matchers supported plus remaining futures.
if grep -q -F -e 'per-check filtering wont-fix' "$expected" &&
  grep -q -F -e 'richer matchers supported' "$expected" &&
  grep -q -F -e 'qualified under #790' "$expected" &&
  grep -q -F -e 'aspect subjects deferred' "$expected" &&
  grep -q -F -e 'toolchain subjects deferred' "$expected" &&
  grep -q -F -e 'configuration subjects deferred' "$expected" &&
  grep -q -F -e 'output-group subjects deferred' "$expected" &&
  grep -q -F -e 'action subjects deferred' "$expected" &&
  grep -q -F -e 'per-function targets wont-fix' "$expected" &&
  grep -q -F -e 'Rust orchestration with BEP wont-fix' "$expected" &&
  grep -q -F -e 'nested Bazel invocation rejected' "$expected" &&
  grep -q -F -e 'no Supported claim' "$expected"; then
  ok
else
  bad "starlark_futures.expected lost its supported-matchers plus remaining futures under #790"
fi

# Fixture BUILD exports pins plus expected plus matchers with corpus coverage.
if grep -q -F -e 'pins.bzl' "$fixture_build" &&
  grep -q -F -e 'starlark_futures.expected' "$fixture_build" &&
  grep -q -F -e 'matchers.bzl' "$fixture_build" &&
  grep -q -F -e 'corpus_starlark' "$fixture_build"; then
  ok
else
  bad "starlark_futures BUILD.bazel lost its pins plus expected plus matchers exports with corpus under #790"
fi

# Tests prove the matchers through matcher_unit in the suite.
if grep -q -F -e 'def matcher_unit_tests' "$matcher_tests" &&
  grep -q -F -e 'expect_true' "$matcher_tests" &&
  grep -q -F -e 'expect_false' "$matcher_tests" &&
  grep -q -F -e 'expect_contains' "$matcher_tests" &&
  grep -q -F -e 'expect_match' "$matcher_tests" &&
  grep -q -F -e 'matcher_unit' "$tests_build" &&
  grep -q -F -e '":matcher_unit"' "$tests_build"; then
  ok
else
  bad "libs/starlark/tests lost its matcher_unit proof under #790"
fi

# Matrix maps the new matchers and validation pins them.
if grep -q -F -e 'matrix-item: expect-true-false' "$matrix" &&
  grep -q -F -e 'matrix-item: expect-contains' "$matrix" &&
  grep -q -F -e 'matrix-item: expect-match' "$matrix" &&
  grep -q -F -e 'matrix-item: expect-true-false' "$matrix_tests" &&
  grep -q -F -e 'def expect_match' "$defs"; then
  ok
else
  bad "behavioral matrix plus validation lost its #790 matcher mapping"
fi

# BUILD owns the harness target plus CI wires it in dogfood-freshness.
if grep -q -F -e 'name = "starlark_futures_qualification"' "$build" &&
  grep -q -F -e 'starlark_futures_qualification.sh' "$build" &&
  grep -q -F -e 'bazel run --noshow_progress //tools/ci:starlark_futures_qualification' "$dogfood"; then
  ok
else
  bad "tools/ci/ci_targets_c.bzl or dogfood_freshness.sh lost the starlark_futures_qualification wiring (want target plus dogfood-freshness)"
fi

# Verification matrix owns the harness entry as seed-only fixture evidence.
if grep -q -F -e ':starlark_futures_qualification' "$verify" &&
  grep -q -F -e 'closed #588' "$verify" &&
  grep -q -F -e '#790' "$verify"; then
  ok
else
  bad "verification-matrix.md lost its starlark_futures_qualification entry under #588 plus #790"
fi

dx_test_summary "starlark futures qualification harness"
