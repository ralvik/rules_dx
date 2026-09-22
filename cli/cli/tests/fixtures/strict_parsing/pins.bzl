"""Strict clap parsing fixture for the `dx` CLI surface.

Contract: `docs/cli/cli-contract.md#invocation-shape`.
Fixture: `cli/cli/tests/fixtures/strict_parsing/` via
`bazel run //tools/ci:cli_strict_qualification`.
Unit fixtures: `cli/cli/src/args/strict_tests.rs` (`dx_cli_test` strict filter).
"""

# Strict: exact `--long` names only; unknown flags fail as UnknownOption
# with whole-token echo (including attached `=value`), never silently ignored.
STRICT_UNKNOWN_REJECTED = [
    "--jobs=4",
    "-q",
    "--dry-run=yes",
    "--quiet=x",
    "--check=x",
    "--bazel=yes",
    "--force",
    "-",
    "--bogus",
    "--bogus=1",
]
STRICT_UNKNOWN_ECHO_WHOLE_TOKEN = True

# Strict: missing option values fail as MissingValue naming the bare flag;
# hyphen-led tokens are never consumed as values.
STRICT_MISSING_REJECTED = [
    "--output",
    "--workspace",
    "--min-coverage",
    "--port",
    "--from",
]
STRICT_HYPHEN_VALUES_NOT_CONSUMED = True
STRICT_MISSING_NAMES_BARE_FLAG = True

# Strict: invalid values fail with contract shapes, never silently ignored.
STRICT_BAD_OUTPUT_REJECTED = ["yaml"]
STRICT_BAD_FAIL_ON_REJECTED = ["never"]
STRICT_BAD_REPORT_REJECTED = ["sarif", "=out.sarif", "sarif=", ""]
STRICT_BAD_MIN_COVERAGE_REJECTED = ["eighty", "101"]

# Strict: typo hints come from the same grammar (commands plus longs).
STRICT_COMMAND_SUGGESTION = {"lintt": "lint"}
STRICT_OPTION_SUGGESTION = {"--ouptut=json": "--output"}

# Strict: known flags on the wrong command fail as UnsupportedOption,
# never silently ignored.
STRICT_UNSUPPORTED_REJECTED = [
    "build --serve",
    "lint --min-coverage",
    "build --check",
    "lint --bazel",
]

# Strict: `dx bazel` tails forward verbatim; dx-owned options before the
# command word still fail fast.
STRICT_BAZEL_FORWARDS_VERBATIM = True
STRICT_BAZEL_PREFIX_OPTIONS_REJECTED = True

# Strict: `dx help [command]` verb redirects to the same generated help as
# flags (`dx --help`, `dx <cmd> --help`); clap keeps `disable_help_subcommand`.
STRICT_HELP_VERB_REDIRECT = True
STRICT_HELP_FLAG_ONLY = ["--help", "-h"]

# Strict: generated help from the same grammar (parsing plus help plus
# completions plus man pages); per-command help pins usage plus scopes
# plus exits plus output for all 32 commands.
STRICT_COMMAND_COUNT = 32
STRICT_GENERATED_HELP_FROM_GRAMMAR = True

# Rejected substitutes (never accepted as the resolution).
REJECTED_SILENT_IGNORE = "silent ignore rejected"
REJECTED_PREFIX_INFERENCE = "prefix inference rejected"
REJECTED_HYPHEN_CONSUMPTION = "hyphen-value consumption rejected"

# Honesty lines (never pinned as supported here).
NO_SUPPORTED = "no Supported claim"
SEED_ONLY = "qualified seed-only under issue #810"
