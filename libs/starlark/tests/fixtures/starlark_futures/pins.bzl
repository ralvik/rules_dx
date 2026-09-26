PER_CHECK_FILTERING = "wont-fix"
RICHER_MATCHERS = "supported"
ASPECT_SUBJECTS = "supported"
TOOLCHAIN_SUBJECTS = "deferred"
CONFIGURATION_SUBJECTS = "supported"
OUTPUT_GROUP_SUBJECTS = "deferred"
ACTION_SUBJECTS = "deferred"
PER_FUNCTION_TARGETS = "wont-fix"
RUST_ORCHESTRATION_BEP = "wont-fix"

RICHER_MATCHERS_USE_CASE = "greet plus pair-error plus admitted-list plus subject-fields plus fingerprint via matchers.bzl"
RICHER_MATCHERS_SURFACE = "expect_equal plus expect_true plus expect_false plus expect_contains plus expect_match"
RICHER_MATCHERS_ISSUE = "qualified"

ASPECT_SUBJECTS_USE_CASE = "leaf plus group with deps via aspect_subjects.bzl"
ASPECT_SUBJECTS_SURFACE = "DxAspectInfo plus dx_aspect_note plus aspect_field observations"
ASPECT_SUBJECTS_ISSUE = "qualified"

TOOLCHAIN_SUBJECTS_USE_CASE = "platform plus toolchain mapping plus resolved report via toolchain_subjects.bzl"
TOOLCHAIN_SUBJECTS_SURFACE = "DxSubjectInfo fields plus DefaultInfo basenames only, no ToolchainInfo"
TOOLCHAIN_SUBJECTS_ISSUE = "use case pinned, stays deferred"

OUTPUT_GROUP_SUBJECTS_USE_CASE = "group-to-files mapping plus resolved report via output_group_subjects.bzl"
OUTPUT_GROUP_SUBJECTS_SURFACE = "DxSubjectInfo fields plus DefaultInfo basenames only, no OutputGroupInfo"
OUTPUT_GROUP_SUBJECTS_ISSUE = "use case pinned, stays deferred"

CONFIGURATION_SUBJECTS_USE_CASE = "leaf plus group with select plus fragment plus transition via config_subjects.bzl"
CONFIGURATION_SUBJECTS_SURFACE = "DxConfigInfo plus config_value plus select plus fragment plus transition plus config_field observations"
CONFIGURATION_SUBJECTS_ISSUE = "qualified"

ACTION_SUBJECTS_USE_CASE = "mnemonic-to-outputs mapping plus resolved report via action_subjects.bzl"
ACTION_SUBJECTS_SURFACE = "DxSubjectInfo fields plus DefaultInfo basenames only, no registered actions"
ACTION_SUBJECTS_ISSUE = "use case pinned, stays deferred"

PROVISIONAL_RULE = "provisional pending concrete use cases"
SUCCESSOR_REQUIREMENT = "concrete use case plus fixtures plus successor issue"

REJECTED_SECOND_INTERPRETER = "second Starlark interpreter rejected"
REJECTED_TEST_FILTER_PARSING = "per-check --test_filter parsing rejected"
REJECTED_NESTED_BAZEL = "nested Bazel invocation rejected"
REJECTED_MATRIX_AS_COVERAGE = "behavioral matrix as line coverage rejected"

NO_SUPPORTED = "no Supported claim"
SEED_ONLY = "qualified seed-only"
