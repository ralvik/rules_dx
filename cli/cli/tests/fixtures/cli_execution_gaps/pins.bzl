"""CLI execution/reporting gaps fixture (issue #590).

Contract: `docs/cli/cli-contract.md#execution-reporting-gaps`,
`docs/cli/commands/watch.md`, `docs/cli/output-protocol.md`,
`docs/cli/standard-reports.md`.
Fixture: `cli/cli/tests/fixtures/cli_execution_gaps/` via
`bazel run //tools/ci:cli_execution_gaps_qualification`.

Decides the four CLI-only gaps that previously had no owning tracker:
watch coverage plus CI, Bazel startup plus test-binary arg-forwarding,
standard-report command/format matrix, and parallel-execution scope.
All four stay wont-fix with fail-closed contract matrices; silent
substitution across commands stays rejected. CLI-only; no Bazel
semantics change. Seed only: no Supported claim.
"""

# Watch: exactly the 8 thin-loop commands stay watchable (ADR 0017/0018).
WATCHABLE_COMMANDS = [
    "build",
    "test",
    "run",
    "lint",
    "typecheck",
    "format",
    "check",
    "fix",
]
WATCHABLE_COUNT = 8

# Watch: the remaining 21 registry commands stay not watchable.
NOT_WATCHABLE_COMMANDS = [
    "audit",
    "bazel",
    "bump",
    "clean",
    "codegen",
    "completion",
    "coverage",
    "deps",
    "deploy",
    "env",
    "generate",
    "hooks",
    "init",
    "migrate",
    "owners",
    "setup",
    "status",
    "update",
    "version",
    "watch",
    "why",
]
NOT_WATCHABLE_COUNT = 21

# Watch locality: CI use stays refused, debounce stays 200ms.
WATCH_REFUSES_CI = True
WATCH_DEBOUNCE_MS = 200
WATCH_REUSES_WRAPPED_VERBATIM = True

# Arg-forwarding: Bazel startup options stay rejected on workflow
# commands with guidance to use `dx bazel`.
STARTUP_OPTIONS_REJECTED = [
    "bazelrc",
    "home_rc",
    "nohome_rc",
    "system_rc",
    "nosystem_rc",
    "output_base",
    "output_user_root",
    "host_jvm_args",
    "server_jvm_out",
]
STARTUP_DISPOSITION = "wont-fix"

# Arg-forwarding: test-binary args stay rejected on workflow commands.
TEST_BINARY_ARGS_REJECTED = ["test_arg"]
TEST_BINARY_DISPOSITION = "wont-fix"

# Arg-forwarding: only `dx bazel` forwards unchanged; `dx run` forwards
# after `--` to the application binary, not to Bazel.
BAZEL_FORWARDS_UNCHANGED = True
RUN_FORWARDS_TO_APP = True
OTHER_WORKFLOW_FORWARDS_TO_BAZEL_COMMAND_OPTIONS = True

# Report matrix: per-command supported standard-report formats.
REPORT_LINT = ["sarif"]
REPORT_TYPECHECK = ["sarif"]
REPORT_TEST = ["junit"]
REPORT_COVERAGE = ["lcov"]
REPORT_AUDIT = ["sarif", "spdx"]
REPORT_CHECK = ["sarif"]
REPORT_FIX = ["sarif"]
REPORT_NONE = [
    "build",
    "run",
    "deploy",
    "format",
    "generate",
    "clean",
    "update",
    "bump",
    "migrate",
    "codegen",
    "env",
    "setup",
    "init",
    "hooks",
    "status",
    "version",
    "watch",
    "owners",
    "deps",
    "why",
    "completion",
    "bazel",
]
REPORT_DISPOSITION = "wont-fix"

# Parallelism: every multi-phase/multi-set/multi-target plan stays
# sequential with no parallel, caching, scheduling, or daemon behavior.
PARALLEL_CHECK_FIX = "sequential format then lint then typecheck then generate"
PARALLEL_WATCH = "one iteration at a time"
PARALLEL_UPDATE = "sequential per-set with continuation, no parallel"
PARALLEL_RUN = "sequential multirun in scope order"
PARALLEL_DISPOSITION = "wont-fix"

# Rejected substitutes (never accepted as the #590 resolution).
REJECTED_SILENT_SUBSTITUTION = "silent substitution across commands rejected"
REJECTED_WATCH_DAEMON = "watch daemon rejected"
REJECTED_PARALLEL_UMBRELLA = "parallel umbrella rejected"
REJECTED_INFERRED_FORWARDS = "inferred option-class forwarding rejected"
REJECTED_INVENTED_REPORTS = "invented standard reports rejected"

# Honesty lines (never pinned as supported here).
NO_SUPPORTED = "no Supported claim"
SEED_ONLY = "qualified seed-only under issue #590"
