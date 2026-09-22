#!/usr/bin/env bash
# Starlark testing futures qualification harness.
#
# Qualifies the nine per-future decisions under ADR 0009
# (remaining subjects provisional pending concrete use cases;
# richer matchers graduated under #790, aspect subjects under #791,
# toolchain use case pinned under #792 staying deferred,
# output-group use case pinned under #794 staying deferred,
# configuration subjects graduated under #793,
# action use case pinned under #795 staying deferred):
# - wont-fix: per-check filtering (target granularity is contract),
#   per-function targets (explicit macro instantiation is contract),
#   Rust orchestration with BEP (single invocation, no nested Bazel);
# - supported: richer matchers (expect_equal plus expect_true plus
#   expect_false plus expect_contains plus expect_match, qualified under
#   #790 with the greet plus pair-error plus admitted-list plus
#   subject-fields plus fingerprint use case) plus aspect subjects
#   (DxAspectInfo plus dx_aspect_note plus aspect_field observations,
#   qualified under #791 with the leaf plus group use case) plus
#   configuration subjects (DxConfigInfo plus config_field observations
#   with select plus fragment plus transition, qualified under #793 with
#   the leaf plus group use case);
# - deferred with use case: toolchain (platform plus toolchain mapping plus
#   resolved report via toolchain_subjects.bzl proven by toolchain_unit,
#   pinned under #792, stays deferred) plus output-group (group-to-files
#   mapping plus resolved report via output_group_subjects.bzl proven by
#   output_group_unit, pinned under #794, stays deferred) plus action
#   (mnemonic-to-outputs mapping plus resolved report via
#   action_subjects.bzl proven by action_unit, pinned under #795, stays
#   deferred);
# - rejected: second Starlark interpreter, per-check --test_filter
#   parsing, nested Bazel invocation, behavioral matrix as line coverage;
# - fixtures: `libs/starlark/tests/fixtures/starlark_futures/` (`pins.bzl`
#   plus `starlark_futures.expected` plus `matchers.bzl` plus
#   `aspect_subjects.bzl` plus `toolchain_subjects.bzl` plus
#   `output_group_subjects.bzl` plus `config_subjects.bzl` plus
#   `action_subjects.bzl`) pins dispositions
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

dx_bash_pin

doc="docs/testing/starlark.md"
adr="docs/decisions/0009-starlark-testing.md"
defs="libs/starlark/defs.bzl"
matrix="libs/starlark/behavioral_matrix.md"
pins="libs/starlark/tests/fixtures/starlark_futures/pins.bzl"
expected="libs/starlark/tests/fixtures/starlark_futures/starlark_futures.expected"
matchers="libs/starlark/tests/fixtures/starlark_futures/matchers.bzl"
aspects="libs/starlark/tests/fixtures/starlark_futures/aspect_subjects.bzl"
toolchains="libs/starlark/tests/fixtures/starlark_futures/toolchain_subjects.bzl"
output_groups="libs/starlark/tests/fixtures/starlark_futures/output_group_subjects.bzl"
configs="libs/starlark/tests/fixtures/starlark_futures/config_subjects.bzl"
actions="libs/starlark/tests/fixtures/starlark_futures/action_subjects.bzl"
matcher_tests="libs/starlark/tests/matcher_tests.bzl"
aspect_tests="libs/starlark/tests/aspect_tests.bzl"
toolchain_tests="libs/starlark/tests/toolchain_tests.bzl"
output_group_tests="libs/starlark/tests/output_group_tests.bzl"
config_tests="libs/starlark/tests/config_tests.bzl"
action_tests="libs/starlark/tests/action_tests.bzl"
analysis_tests="libs/starlark/tests/analysis_tests.bzl"
tests_build="libs/starlark/tests/BUILD.bazel"
matrix_tests="libs/starlark/tests/matrix_tests.bzl"
fixture_build="libs/starlark/tests/fixtures/starlark_futures/BUILD.bazel"
build="tools/ci/ci_targets_c.bzl"
dogfood="tools/ci/dogfood_freshness.sh"

# Docs decide the nine futures under with ADR plus fixtures plus qualification.
if grep -q -F -e 'Decided under closed #588 plus #790 plus #791 plus #792 plus #793 plus #794 plus #795' "$doc" &&
  grep -q -F -e 'remaining subjects provisional pending concrete use cases' "$doc" &&
  grep -q -F -e 'libs/starlark/tests/fixtures/starlark_futures/' "$doc" &&
  grep -q -F -e 'bazel run //tools/ci:starlark_futures_qualification' "$doc" &&
  grep -q -F -e 'second Starlark interpreter is rejected' "$doc" &&
  grep -q -F -e 'line coverage is rejected' "$doc"; then
  ok
else
  bad "starlark.md lost its #588 plus #790 plus #791 plus #792 plus #793 plus #794 plus #795 decided header with ADR plus fixtures plus qualification plus rejected routes"
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

# Docs graduate aspect subjects under #791 with the leaf plus group use case.
if grep -q -F -e 'Aspect subjects graduated under #791' "$doc" &&
  grep -q -F -e '`dx_aspect_note`' "$doc" &&
  grep -q -F -e '`aspect_field`' "$doc" &&
  grep -q -F -e 'aspect_subjects.bzl' "$doc" &&
  grep -q -F -e '//libs/starlark/tests:aspect_subject_analysis' "$doc"; then
  ok
else
  bad "starlark.md lost its aspect-subjects graduation under #791 with leaf plus group use case"
fi

# Docs graduate configuration subjects under #793 with select plus fragment plus transition.
if grep -q -F -e 'Configuration subjects graduated under #793' "$doc" &&
  grep -q -F -e 'DxConfigInfo' "$doc" &&
  grep -q -F -e '`config_field`' "$doc" &&
  grep -q -F -e 'config_subjects.bzl' "$doc" &&
  grep -q -F -e '//libs/starlark/tests:config_subject_analysis' "$doc"; then
  ok
else
  bad "starlark.md lost its configuration-subjects graduation under #793 with select plus fragment plus transition"
fi

# Docs keep the remaining deferred subjects with registered-action coverage.
if grep -q -F -e 'Toolchain subjects stay deferred' "$doc" &&
  grep -q -F -e 'Output-group subjects stay deferred' "$doc" &&
  grep -q -F -e 'Action subjects stay deferred' "$doc" &&
  grep -q -F -e 'including registered-action' "$doc" &&
  ! grep -q -F -e 'Aspect subjects stay deferred' "$doc" &&
  ! grep -q -F -e 'Configuration subjects stay deferred' "$doc"; then
  ok
else
  bad "starlark.md lost its deferred record under #588 plus #790 plus #791 plus #792 plus #793 plus #794 plus #795"
fi

# Docs pin the toolchain use case under #792 while staying deferred.
if grep -q -F -e 'toolchain_subjects.bzl' "$doc" &&
  grep -q -F -e '//libs/starlark/tests:toolchain_unit' "$doc" &&
  grep -q -F -e 'stays deferred' "$doc"; then
  ok
else
  bad "starlark.md lost its toolchain use-case pin under #792 with stays deferred"
fi

# Docs pin the output-group use case under #794 while staying deferred.
if grep -q -F -e 'output_group_subjects.bzl' "$doc" &&
  grep -q -F -e '//libs/starlark/tests:output_group_unit' "$doc" &&
  grep -q -F -e 'Output-group subjects stay deferred' "$doc"; then
  ok
else
  bad "starlark.md lost its output-group use-case pin under #794 with stays deferred"
fi

# Docs pin the action use case under #795 while staying deferred.
if grep -q -F -e 'action_subjects.bzl' "$doc" &&
  grep -q -F -e '//libs/starlark/tests:action_unit' "$doc" &&
  grep -q -F -e 'Action subjects stay deferred' "$doc"; then
  ok
else
  bad "starlark.md lost its action use-case pin under #795 with stays deferred"
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

# ADR 0009 graduates aspect subjects under #791 with deferred remainder.
if grep -q -F -e 'Aspect subjects graduated under #791' "$adr" &&
  grep -q -F -e 'DxAspectInfo' "$adr" &&
  grep -q -F -e 'dx_aspect_note' "$adr" &&
  grep -q -F -e 'graduated under #791' "$adr"; then
  ok
else
  bad "ADR 0009 lost its aspect-subjects graduation under #791"
fi

# ADR 0009 graduates configuration subjects under #793 with deferred remainder.
if grep -q -F -e 'configuration subjects graduated under' "$adr" &&
  grep -q -F -e 'DxConfigInfo' "$adr" &&
  grep -q -F -e 'config_flip_transition' "$adr" &&
  grep -q -F -e 'graduated under #793' "$adr"; then
  ok
else
  bad "ADR 0009 lost its configuration-subjects graduation under #793"
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
  bad "defs.bzl must not handle test_filter per check under #588 plus #790 plus #791 plus #792 plus #793 plus #794 plus #795"
fi

# Code observes DefaultInfo plus DxSubjectInfo plus DxAspectInfo plus DxConfigInfo with no broader subject plumbing.
if grep -q -F -e 'DxSubjectInfo' "$defs" &&
  grep -q -F -e 'DxAspectInfo' "$defs" &&
  grep -q -F -e 'DxConfigInfo' "$defs" &&
  grep -q -F -e 'dx_aspect_note' "$defs" &&
  grep -q -F -e 'aspect_field' "$defs" &&
  grep -q -F -e 'config_field' "$defs" &&
  grep -q -F -e 'DefaultInfo' "$defs" &&
  ! grep -q -F -e 'OutputGroupInfo' "$defs" &&
  ! grep -q -F -e 'Toolchain' "$defs"; then
  ok
else
  bad "defs.bzl lost its DefaultInfo plus DxSubjectInfo plus DxAspectInfo plus DxConfigInfo observation under #588 plus #790 plus #791 plus #792 plus #793 plus #794 plus #795"
fi

# Code keeps one-call-one-target dispatch with no nested Bazel.
if grep -q -F -e '_MODES' "$defs" &&
  grep -q -F -e 'def starlark_test' "$defs" &&
  grep -q -F -e 'One macro call is one addressable' "$defs" &&
  ! grep -q -F -e 'bazel run' "$defs"; then
  ok
else
  bad "defs.bzl lost its one-call-one-target dispatch with no nested Bazel under #588 plus #790 plus #791 plus #792 plus #793 plus #794 plus #795"
fi

# Fixture pins stay present with nine dispositions plus matcher plus aspect plus toolchain plus output-group plus configuration plus action use cases plus #790 plus #791 plus #792 plus #793 plus #794 plus #795.
if [[ -f "$pins" && -f "$expected" && -f "$fixture_build" && -f "$matchers" && -f "$aspects" && -f "$toolchains" && -f "$output_groups" && -f "$configs" && -f "$actions" ]] &&
  grep -q -F -e 'PER_CHECK_FILTERING = "wont-fix"' "$pins" &&
  grep -q -F -e 'RICHER_MATCHERS = "supported"' "$pins" &&
  grep -q -F -e 'RICHER_MATCHERS_USE_CASE' "$pins" &&
  grep -q -F -e 'RICHER_MATCHERS_SURFACE' "$pins" &&
  grep -q -F -e 'qualified under issue #790' "$pins" &&
  grep -q -F -e 'ASPECT_SUBJECTS = "supported"' "$pins" &&
  grep -q -F -e 'ASPECT_SUBJECTS_USE_CASE' "$pins" &&
  grep -q -F -e 'ASPECT_SUBJECTS_SURFACE' "$pins" &&
  grep -q -F -e 'qualified under issue #791' "$pins" &&
  grep -q -F -e 'TOOLCHAIN_SUBJECTS_USE_CASE' "$pins" &&
  grep -q -F -e 'TOOLCHAIN_SUBJECTS_SURFACE' "$pins" &&
  grep -q -F -e 'use case pinned under issue #792' "$pins" &&
  grep -q -F -e 'OUTPUT_GROUP_SUBJECTS_USE_CASE' "$pins" &&
  grep -q -F -e 'OUTPUT_GROUP_SUBJECTS_SURFACE' "$pins" &&
  grep -q -F -e 'use case pinned under issue #794' "$pins" &&
  grep -q -F -e 'ACTION_SUBJECTS_USE_CASE' "$pins" &&
  grep -q -F -e 'ACTION_SUBJECTS_SURFACE' "$pins" &&
  grep -q -F -e 'use case pinned under issue #795' "$pins" &&
  grep -q -F -e 'TOOLCHAIN_SUBJECTS = "deferred"' "$pins" &&
  grep -q -F -e 'CONFIGURATION_SUBJECTS = "supported"' "$pins" &&
  grep -q -F -e 'CONFIGURATION_SUBJECTS_USE_CASE' "$pins" &&
  grep -q -F -e 'CONFIGURATION_SUBJECTS_SURFACE' "$pins" &&
  grep -q -F -e 'qualified under issue #793' "$pins" &&
  grep -q -F -e 'OUTPUT_GROUP_SUBJECTS = "deferred"' "$pins" &&
  grep -q -F -e 'ACTION_SUBJECTS = "deferred"' "$pins" &&
  grep -q -F -e 'PER_FUNCTION_TARGETS = "wont-fix"' "$pins" &&
  grep -q -F -e 'RUST_ORCHESTRATION_BEP = "wont-fix"' "$pins" &&
  grep -q -F -e 'provisional pending concrete use cases' "$pins" &&
  grep -q -F -e 'REJECTED_NESTED_BAZEL' "$pins"; then
  ok
else
  bad "starlark_futures pins fixture lost its nine dispositions plus matcher plus aspect plus toolchain plus output-group plus configuration plus action use cases under #790 plus #791 plus #792 plus #793 plus #794 plus #795"
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

# Fixture aspect_subjects.bzl carries the concrete use case.
if grep -q -F -e 'aspect_leaf' "$aspects" &&
  grep -q -F -e 'aspect_group' "$aspects" &&
  grep -q -F -e 'DxSubjectInfo' "$aspects" &&
  grep -q -F -e 'issue #791' "$aspects"; then
  ok
else
  bad "starlark_futures aspect_subjects.bzl lost its #791 leaf plus group use case"
fi

# Fixture toolchain_subjects.bzl carries the concrete use case pinned under #792 staying deferred.
if grep -q -F -e 'def admitted_platforms' "$toolchains" &&
  grep -q -F -e 'def admitted_toolchains' "$toolchains" &&
  grep -q -F -e 'def resolve_toolchain' "$toolchains" &&
  grep -q -F -e 'def toolchain_report' "$toolchains" &&
  grep -q -F -e 'def toolchain_subject_fields' "$toolchains" &&
  grep -q -F -e 'def toolchain_fingerprint_like' "$toolchains" &&
  grep -q -F -e 'def is_supported_platform' "$toolchains" &&
  grep -q -F -e 'issue #792' "$toolchains" &&
  grep -q -F -e 'stays deferred' "$toolchains"; then
  ok
else
  bad "starlark_futures toolchain_subjects.bzl lost its #792 platform plus toolchain use case"
fi

# Fixture output_group_subjects.bzl carries the concrete use case pinned under #794 staying deferred.
if grep -q -F -e 'def admitted_output_groups' "$output_groups" &&
  grep -q -F -e 'def admitted_output_files' "$output_groups" &&
  grep -q -F -e 'def resolve_output_group' "$output_groups" &&
  grep -q -F -e 'def output_group_report' "$output_groups" &&
  grep -q -F -e 'def output_group_subject_fields' "$output_groups" &&
  grep -q -F -e 'def output_group_fingerprint_like' "$output_groups" &&
  grep -q -F -e 'def is_supported_output_group' "$output_groups" &&
  grep -q -F -e 'issue #794' "$output_groups" &&
  grep -q -F -e 'stays deferred' "$output_groups"; then
  ok
else
  bad "starlark_futures output_group_subjects.bzl lost its #794 group-to-files use case"
fi

# Fixture config_subjects.bzl carries the concrete use case.
if grep -q -F -e 'config_leaf' "$configs" &&
  grep -q -F -e 'config_group' "$configs" &&
  grep -q -F -e 'config_flip_transition' "$configs" &&
  grep -q -F -e 'DxConfigInfo' "$configs" &&
  grep -q -F -e 'issue #793' "$configs"; then
  ok
else
  bad "starlark_futures config_subjects.bzl lost its #793 leaf plus group use case"
fi

# Fixture action_subjects.bzl carries the concrete use case pinned under #795 staying deferred.
if grep -q -F -e 'def admitted_actions' "$actions" &&
  grep -q -F -e 'def admitted_action_outputs' "$actions" &&
  grep -q -F -e 'def resolve_action' "$actions" &&
  grep -q -F -e 'def action_report' "$actions" &&
  grep -q -F -e 'def action_subject_fields' "$actions" &&
  grep -q -F -e 'def action_fingerprint_like' "$actions" &&
  grep -q -F -e 'def is_supported_action' "$actions" &&
  grep -q -F -e 'issue #795' "$actions" &&
  grep -q -F -e 'stays deferred' "$actions"; then
  ok
else
  bad "starlark_futures action_subjects.bzl lost its #795 mnemonic-to-outputs use case"
fi

# Expected fixture pins richer matchers plus aspect plus configuration supported plus toolchain plus output-group plus action deferred with use cases plus remaining futures.
if grep -q -F -e 'per-check filtering wont-fix' "$expected" &&
  grep -q -F -e 'richer matchers supported' "$expected" &&
  grep -q -F -e 'qualified under #790' "$expected" &&
  grep -q -F -e 'aspect subjects supported' "$expected" &&
  grep -q -F -e 'qualified under #791' "$expected" &&
  grep -q -F -e 'toolchain subjects deferred' "$expected" &&
  grep -q -F -e 'pinned under #792' "$expected" &&
  grep -q -F -e 'toolchain_subjects.bzl' "$expected" &&
  grep -q -F -e '//libs/starlark/tests:toolchain_unit' "$expected" &&
  grep -q -F -e 'configuration subjects supported' "$expected" &&
  grep -q -F -e 'qualified under #793' "$expected" &&
  grep -q -F -e 'output-group subjects deferred' "$expected" &&
  grep -q -F -e 'pinned under #794' "$expected" &&
  grep -q -F -e 'output_group_subjects.bzl' "$expected" &&
  grep -q -F -e '//libs/starlark/tests:output_group_unit' "$expected" &&
  grep -q -F -e 'action subjects deferred' "$expected" &&
  grep -q -F -e 'pinned under #795' "$expected" &&
  grep -q -F -e 'action_subjects.bzl' "$expected" &&
  grep -q -F -e '//libs/starlark/tests:action_unit' "$expected" &&
  grep -q -F -e 'per-function targets wont-fix' "$expected" &&
  grep -q -F -e 'Rust orchestration with BEP wont-fix' "$expected" &&
  grep -q -F -e 'nested Bazel invocation rejected' "$expected" &&
  grep -q -F -e 'no Supported claim' "$expected"; then
  ok
else
  bad "starlark_futures.expected lost its supported-matchers plus supported-aspect plus supported-configuration plus toolchain-pinned plus output-group-pinned plus action-pinned plus remaining futures under #790 plus #791 plus #792 plus #793 plus #794 plus #795"
fi

# Fixture BUILD exports pins plus expected plus matchers plus aspect plus toolchain plus output-group plus configuration plus action subjects with corpus coverage.
if grep -q -F -e 'pins.bzl' "$fixture_build" &&
  grep -q -F -e 'starlark_futures.expected' "$fixture_build" &&
  grep -q -F -e 'matchers.bzl' "$fixture_build" &&
  grep -q -F -e 'aspect_subjects.bzl' "$fixture_build" &&
  grep -q -F -e 'toolchain_subjects.bzl' "$fixture_build" &&
  grep -q -F -e 'output_group_subjects.bzl' "$fixture_build" &&
  grep -q -F -e 'config_subjects.bzl' "$fixture_build" &&
  grep -q -F -e 'action_subjects.bzl' "$fixture_build" &&
  grep -q -F -e 'config_value' "$fixture_build" &&
  grep -q -F -e 'is_flipped' "$fixture_build" &&
  grep -q -F -e 'corpus_starlark' "$fixture_build"; then
  ok
else
  bad "starlark_futures BUILD.bazel lost its pins plus expected plus matchers plus aspect plus toolchain plus output-group plus configuration plus action exports with corpus under #790 plus #791 plus #792 plus #793 plus #794 plus #795"
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

# Matrix maps aspect subjects and validation pins them.
if grep -q -F -e 'matrix-item: aspect-subjects' "$matrix" &&
  grep -q -F -e 'DxAspectInfo' "$matrix" &&
  grep -q -F -e 'dx_aspect_note' "$matrix" &&
  grep -q -F -e 'matrix-item: aspect-subjects' "$matrix_tests" &&
  grep -q -F -e 'aspect_leaf' "$matrix_tests" &&
  grep -q -F -e 'DxAspectInfo' "$defs" &&
  grep -q -F -e 'aspect_field' "$defs"; then
  ok
else
  bad "behavioral matrix plus validation lost its #791 aspect-subjects mapping"
fi

# Matrix maps configuration subjects and validation pins them.
if grep -q -F -e 'matrix-item: configuration-subjects' "$matrix" &&
  grep -q -F -e 'DxConfigInfo' "$matrix" &&
  grep -q -F -e 'config_flip_transition' "$matrix" &&
  grep -q -F -e 'matrix-item: configuration-subjects' "$matrix_tests" &&
  grep -q -F -e 'config_leaf' "$matrix_tests" &&
  grep -q -F -e 'DxConfigInfo' "$defs" &&
  grep -q -F -e 'config_field' "$defs"; then
  ok
else
  bad "behavioral matrix plus validation lost its #793 configuration-subjects mapping"
fi

# Tests prove aspect subjects through aspect_subject_analysis in the suite.
if grep -q -F -e 'def aspect_subject_tests' "$aspect_tests" &&
  grep -q -F -e 'aspect_leaf_under_test' "$aspect_tests" &&
  grep -q -F -e 'aspect_group_under_test' "$aspect_tests" &&
  grep -q -F -e 'aspect_subject_analysis' "$tests_build" &&
  grep -q -F -e '":aspect_subject_analysis"' "$tests_build" &&
  grep -q -F -e 'aspect_field' "$analysis_tests"; then
  ok
else
  bad "libs/starlark/tests lost its aspect_subject_analysis proof under #791"
fi

# Tests prove the toolchain use case through toolchain_unit in the suite.
if grep -q -F -e 'def toolchain_unit_tests' "$toolchain_tests" &&
  grep -q -F -e 'expect_equal' "$toolchain_tests" &&
  grep -q -F -e 'expect_true' "$toolchain_tests" &&
  grep -q -F -e 'expect_false' "$toolchain_tests" &&
  grep -q -F -e 'expect_contains' "$toolchain_tests" &&
  grep -q -F -e 'expect_match' "$toolchain_tests" &&
  grep -q -F -e 'resolve_toolchain' "$toolchain_tests" &&
  grep -q -F -e 'toolchain_unit' "$tests_build" &&
  grep -q -F -e '":toolchain_unit"' "$tests_build"; then
  ok
else
  bad "libs/starlark/tests lost its toolchain_unit proof under #792"
fi

# Tests prove the output-group use case through output_group_unit in the suite.
if grep -q -F -e 'def output_group_unit_tests' "$output_group_tests" &&
  grep -q -F -e 'expect_equal' "$output_group_tests" &&
  grep -q -F -e 'expect_true' "$output_group_tests" &&
  grep -q -F -e 'expect_false' "$output_group_tests" &&
  grep -q -F -e 'expect_contains' "$output_group_tests" &&
  grep -q -F -e 'expect_match' "$output_group_tests" &&
  grep -q -F -e 'resolve_output_group' "$output_group_tests" &&
  grep -q -F -e 'output_group_unit' "$tests_build" &&
  grep -q -F -e '":output_group_unit"' "$tests_build"; then
  ok
else
  bad "libs/starlark/tests lost its output_group_unit proof under #794"
fi

# Tests prove configuration subjects through config_subject_analysis in the suite.
if grep -q -F -e 'def config_subject_tests' "$config_tests" &&
  grep -q -F -e 'config_leaf_under_test' "$config_tests" &&
  grep -q -F -e 'config_group_under_test' "$config_tests" &&
  grep -q -F -e 'config_subject_analysis' "$tests_build" &&
  grep -q -F -e '":config_subject_analysis"' "$tests_build" &&
  grep -q -F -e 'config_field' "$config_tests"; then
  ok
else
  bad "libs/starlark/tests lost its config_subject_analysis proof under #793"
fi

# Tests prove the action use case through action_unit in the suite.
if grep -q -F -e 'def action_unit_tests' "$action_tests" &&
  grep -q -F -e 'expect_equal' "$action_tests" &&
  grep -q -F -e 'expect_true' "$action_tests" &&
  grep -q -F -e 'expect_false' "$action_tests" &&
  grep -q -F -e 'expect_contains' "$action_tests" &&
  grep -q -F -e 'expect_match' "$action_tests" &&
  grep -q -F -e 'resolve_action' "$action_tests" &&
  grep -q -F -e 'action_unit' "$tests_build" &&
  grep -q -F -e '":action_unit"' "$tests_build"; then
  ok
else
  bad "libs/starlark/tests lost its action_unit proof under #795"
fi

# BUILD owns the harness target plus CI wires it in dogfood-freshness.
if grep -q -F -e 'name = "starlark_futures_qualification"' "$build" &&
  grep -q -F -e 'starlark_futures_qualification.sh' "$build" &&
  grep -q -F -e 'bazel run --noshow_progress //tools/ci:starlark_futures_qualification' "$dogfood"; then
  ok
else
  bad "tools/ci/ci_targets_c.bzl or dogfood_freshness.sh lost the starlark_futures_qualification wiring (want target plus dogfood-freshness)"
fi

dx_test_summary "starlark futures qualification harness"
