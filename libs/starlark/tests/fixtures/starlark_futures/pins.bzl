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

# admitted-list plus subject-fields plus fingerprint via matchers.bzl,
# proven by //libs/starlark/tests:matcher_unit.
RICHER_MATCHERS_USE_CASE = "greet plus pair-error plus admitted-list plus subject-fields plus fingerprint via matchers.bzl"
RICHER_MATCHERS_SURFACE = "expect_equal plus expect_true plus expect_false plus expect_contains plus expect_match"
RICHER_MATCHERS_ISSUE = "qualified under issue #790"

# aspect_subjects.bzl, proven by //libs/starlark/tests:aspect_subject_analysis.
ASPECT_SUBJECTS_USE_CASE = "leaf plus group with deps via aspect_subjects.bzl"
ASPECT_SUBJECTS_SURFACE = "DxAspectInfo plus dx_aspect_note plus aspect_field observations"
ASPECT_SUBJECTS_ISSUE = "qualified under issue #791"

# plus resolved report via toolchain_subjects.bzl, proven by
# //libs/starlark/tests:toolchain_unit. Direct toolchain observation stays
# deferred; analysis observes DxSubjectInfo fields plus DefaultInfo basenames
# only.
TOOLCHAIN_SUBJECTS_USE_CASE = "platform plus toolchain mapping plus resolved report via toolchain_subjects.bzl"
TOOLCHAIN_SUBJECTS_SURFACE = "DxSubjectInfo fields plus DefaultInfo basenames only, no ToolchainInfo"
TOOLCHAIN_SUBJECTS_ISSUE = "use case pinned under issue #792, stays deferred"

# resolved report via output_group_subjects.bzl, proven by
# //libs/starlark/tests:output_group_unit. Direct OutputGroupInfo
# observation stays deferred; analysis observes DxSubjectInfo fields plus
# DefaultInfo basenames only, wrapper forwarding does not imply observation.
OUTPUT_GROUP_SUBJECTS_USE_CASE = "group-to-files mapping plus resolved report via output_group_subjects.bzl"
OUTPUT_GROUP_SUBJECTS_SURFACE = "DxSubjectInfo fields plus DefaultInfo basenames only, no OutputGroupInfo"
OUTPUT_GROUP_SUBJECTS_ISSUE = "use case pinned under issue #794, stays deferred"

# configurable select plus platform fragment plus flip transition via
# config_subjects.bzl, proven by //libs/starlark/tests:config_subject_analysis.
CONFIGURATION_SUBJECTS_USE_CASE = "leaf plus group with select plus fragment plus transition via config_subjects.bzl"
CONFIGURATION_SUBJECTS_SURFACE = "DxConfigInfo plus config_value plus select plus fragment plus transition plus config_field observations"
CONFIGURATION_SUBJECTS_ISSUE = "qualified under issue #793"

# resolved report via action_subjects.bzl, proven by
ACTION_SUBJECTS_USE_CASE = "mnemonic-to-outputs mapping plus resolved report via action_subjects.bzl"
ACTION_SUBJECTS_SURFACE = "DxSubjectInfo fields plus DefaultInfo basenames only, no registered actions"
ACTION_SUBJECTS_ISSUE = "use case pinned under issue #795, stays deferred"

# ADR 0009 provisional coverage: configuration includes transitions,
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
