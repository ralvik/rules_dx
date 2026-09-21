"""Help-output goldens fixture for the `dx` CLI surface.

Contract: `docs/cli/cli-contract.md#invocation-shape`.
Fixture: `cli/cli/tests/fixtures/help_goldens/` via
`bazel run //tools/ci:cli_strict_qualification`.
Goldens: `top_help.golden` plus per-command goldens generated from the
same `Cli` grammar that parses invocations (`dx --help` plus
`dx <command> --help` flag-only; no `help` verb; `dx bazel --help`
forwards verbatim so its golden rides `render_command_help`).
Unit fixtures: `cli/cli/src/args/strict_tests.rs` plus
`cli/cli/src/args/help.rs` help tests.
"""

# Goldens generated from the same grammar (never hand-maintained flag lists).
HELP_TOP_GOLDEN = "top_help.golden"
HELP_REPRESENTATIVE_GOLDENS = [
    "lint_help.golden",
    "build_help.golden",
    "clean_help.golden",
    "bazel_help.golden",
    "docs_help.golden",
]

# Top help pins brand plus commands plus flags plus scopes plus exits plus output.
HELP_TOP_INVARIANTS = [
    "dx - Transparent UI over Bazel",
    "Commands:",
    "--workspace",
    "--output",
    "--fail-on",
    "Exit codes",
]

# Per-command help pins usage plus scopes plus exits plus output plus owned flags.
HELP_COMMAND_INVARIANTS = [
    "Usage:",
    "Scopes:",
    "Exit codes:",
    "Output:",
    "Per-command flags:",
]

# Help is flag-only with no verb; bazel tail forwards verbatim.
HELP_FLAG_ONLY = ["--help", "-h"]
HELP_NO_VERB = True
HELP_BAZEL_VERBATIM = True

# All 32 commands have generated help from the same source.
HELP_COMMAND_COUNT = 32
HELP_GENERATED_FROM_GRAMMAR = True

# Honesty lines (never pinned as supported here).
NO_SUPPORTED = "no Supported claim"
SEED_ONLY = "qualified seed-only under issue #810"
