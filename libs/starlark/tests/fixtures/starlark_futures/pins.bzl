"""Starlark testing futures pins.

Contract: `docs/testing/starlark.md#future-not-implemented`,
`docs/decisions/0009-starlark-testing.md`.
Fixture: `libs/starlark/tests/fixtures/starlark_futures/` via
`bazel run //tools/ci:starlark_futures_qualification`.
"""

# Per-future dispositions (two wont-fix filtering/targets, one wont-fix
# orchestration, three supported matchers/aspect/configuration under
# #790/#791/#793, three deferred subjects).
PER_CHECK_FILTERING = "wont-fix"
RICHER_MATCHERS = "supported"
ASPECT_SUBJECTS = "supported"
TOOLCHAIN_SUBJECTS = "deferred"
CONFIGURATION_SUBJECTS = "supported"
OUTPUT_GROUP_SUBJECTS = "deferred"
ACTION_SUBJECTS = "deferred"
PER_FUNCTION_TARGETS = "wont-fix"
RUST_ORCHESTRATION_BEP = "wont-fix"

# Richer matchers use case (issue #790): greet plus pair-error plus
# admitted-list plus subject-fields plus fingerprint via matchers.bzl,
# proven by //libs/starlark/tests:matcher_unit.
RICHER_MATCHERS_USE_CASE = "greet plus pair-error plus admitted-list plus subject-fields plus fingerprint via matchers.bzl"
RICHER_MATCHERS_SURFACE = "expect_equal plus expect_true plus expect_false plus expect_contains plus expect_match"
RICHER_MATCHERS_ISSUE = "qualified under issue #790"

# Aspect subjects use case (issue #791): leaf plus group with deps via
# aspect_subjects.bzl, proven by //libs/starlark/tests:aspect_subject_analysis.
ASPECT_SUBJECTS_USE_CASE = "leaf plus group with deps via aspect_subjects.bzl"
ASPECT_SUBJECTS_SURFACE = "DxAspectInfo plus dx_aspect_note plus aspect_field observations"
ASPECT_SUBJECTS_ISSUE = "qualified under issue #791"

# Toolchain subjects use case (issue #792): platform plus toolchain mapping
# plus resolved report via toolchain_subjects.bzl, proven by
# //libs/starlark/tests:toolchain_unit. Direct toolchain observation stays
# deferred; analysis observes DxSubjectInfo fields plus DefaultInfo basenames
# only.
TOOLCHAIN_SUBJECTS_USE_CASE = "platform plus toolchain mapping plus resolved report via toolchain_subjects.bzl"
TOOLCHAIN_SUBJECTS_SURFACE = "DxSubjectInfo fields plus DefaultInfo basenames only, no ToolchainInfo"
TOOLCHAIN_SUBJECTS_ISSUE = "use case pinned under issue #792, stays deferred"

# Output-group subjects use case (issue #794): group-to-files mapping plus
# resolved report via output_group_subjects.bzl, proven by
# //libs/starlark/tests:output_group_unit. Direct OutputGroupInfo
# observation stays deferred; analysis observes DxSubjectInfo fields plus
# DefaultInfo basenames only, wrapper forwarding does not imply observation.
OUTPUT_GROUP_SUBJECTS_USE_CASE = "group-to-files mapping plus resolved report via output_group_subjects.bzl"
OUTPUT_GROUP_SUBJECTS_SURFACE = "DxSubjectInfo fields plus DefaultInfo basenames only, no OutputGroupInfo"
OUTPUT_GROUP_SUBJECTS_ISSUE = "use case pinned under issue #794, stays deferred"

# Configuration subjects use case (issue #793): leaf plus group with
# configurable select plus platform fragment plus flip transition via
# config_subjects.bzl, proven by //libs/starlark/tests:config_subject_analysis.
CONFIGURATION_SUBJECTS_USE_CASE = "leaf plus group with select plus fragment plus transition via config_subjects.bzl"
CONFIGURATION_SUBJECTS_SURFACE = "DxConfigInfo plus config_value plus select plus fragment plus transition plus config_field observations"
CONFIGURATION_SUBJECTS_ISSUE = "qualified under issue #793"

# Action subjects use case (issue #795): mnemonic-to-outputs mapping plus
# resolved report via action_subjects.bzl, proven by
# //libs/starlark/tests:action_unit. Registered-action observation stays
# deferred; analysis observes DxSubjectInfo fields plus DefaultInfo
# basenames only, actions are proven via execution-mode file_checks or
# aquery evidence.
ACTION_SUBJECTS_USE_CASE = "mnemonic-to-outputs mapping plus resolved report via action_subjects.bzl"
ACTION_SUBJECTS_SURFACE = "DxSubjectInfo fields plus DefaultInfo basenames only, no registered actions"
ACTION_SUBJECTS_ISSUE = "use case pinned under issue #795, stays deferred"

# ADR 0009 provisional coverage: configuration includes transitions,
# action includes registered-action, broader subjects (targets, actions,
# files, depsets, runfiles) stay provisional with the same deferred
# dispositions above; do not treat them as available API. Matchers left
# provisional under #588 and are now supported under #790; aspects left
# provisional under #588 and are now supported under #791; toolchain use case
# pinned under #792 but stays deferred; output-group use case pinned under
# #794 but stays deferred; action use case pinned under #795 but stays
# deferred; configuration left provisional under #588 and is
# now supported under #793.
PROVISIONAL_RULE = "provisional pending concrete use cases"
SUCCESSOR_REQUIREMENT = "concrete use case plus fixtures plus successor issue"

# Rejected routes (never pinned as supported here).
REJECTED_SECOND_INTERPRETER = "second Starlark interpreter rejected"
REJECTED_TEST_FILTER_PARSING = "per-check --test_filter parsing rejected"
REJECTED_NESTED_BAZEL = "nested Bazel invocation rejected"
REJECTED_MATRIX_AS_COVERAGE = "behavioral matrix as line coverage rejected"

# Honesty lines (never pinned as supported here).
NO_SUPPORTED = "no Supported claim"
SEED_ONLY = "qualified seed-only under issue #588"
