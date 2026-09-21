"""Starlark testing futures pins.

Contract: `docs/testing/starlark.md#future-not-implemented`,
`docs/decisions/0009-starlark-testing.md`.
Fixture: `libs/starlark/tests/fixtures/starlark_futures/` via
`bazel run //tools/ci:starlark_futures_qualification`.
"""

# Per-future dispositions (two wont-fix filtering/targets, one wont-fix
# orchestration, one supported matchers under #790, five deferred subjects).
PER_CHECK_FILTERING = "wont-fix"
RICHER_MATCHERS = "supported"
ASPECT_SUBJECTS = "deferred"
TOOLCHAIN_SUBJECTS = "deferred"
CONFIGURATION_SUBJECTS = "deferred"
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

# ADR 0009 provisional coverage: configuration includes transitions,
# action includes registered-action, broader subjects (targets, actions,
# files, depsets, runfiles) stay provisional with the same deferred
# dispositions above; do not treat them as available API. Matchers left
# provisional under #588 and are now supported under #790.
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
