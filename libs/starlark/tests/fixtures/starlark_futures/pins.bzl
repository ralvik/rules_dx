"""Starlark testing futures pins.

Contract: `docs/testing/starlark.md#future-not-implemented`,
`docs/decisions/0009-starlark-testing.md`.
Fixture: `libs/starlark/tests/fixtures/starlark_futures/` via
`bazel run //tools/ci:starlark_futures_qualification`.
"""

# Per-future dispositions (two wont-fix filtering/targets, one wont-fix
# orchestration, six deferred subjects/matchers).
PER_CHECK_FILTERING = "wont-fix"
RICHER_MATCHERS = "deferred"
ASPECT_SUBJECTS = "deferred"
TOOLCHAIN_SUBJECTS = "deferred"
CONFIGURATION_SUBJECTS = "deferred"
OUTPUT_GROUP_SUBJECTS = "deferred"
ACTION_SUBJECTS = "deferred"
PER_FUNCTION_TARGETS = "wont-fix"
RUST_ORCHESTRATION_BEP = "wont-fix"

# ADR 0009 provisional coverage: configuration includes transitions,
# action includes registered-action, broader subjects (targets, actions,
# files, depsets, runfiles) plus matchers stay provisional with the same
# deferred dispositions above; do not treat them as available API.
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
