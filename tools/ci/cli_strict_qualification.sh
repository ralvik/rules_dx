#!/usr/bin/env bash
# Strict clap parsing plus generated-help qualification harness.
#
# Qualifies the dx CLI strict surface with fixture evidence (issue #810,
# successor to the #316 owned gap for the dx CLI tokenizer only; thin shims
# stay frozen legacy under #316; plus issue #951 help verb redirect plus
# completion/status/doctor discoverability):
# - strict parsing: exact `--long` names only, unknown/missing/bad shapes
#   fail fast with whole-token echo plus bare-flag naming plus grammar-owned
#   suggestions, hyphen-values never consumed, known flags on wrong commands
#   fail as unsupported, `dx bazel` tails forward verbatim, `dx help [command]`
#   verb redirects to the same generated help as `--help`/`-h`;
# - generated help: `--help`/`-h` plus `dx help` render from the same `Cli` grammar that
#   parses (top plus all 32 per-command helps pin usage plus scopes plus
#   exits plus output plus owned flags; top lists completion shells,
#   status names NDJSON shape, completion names `--check`);
# - fixtures: `cli/cli/tests/fixtures/strict_parsing/` (`pins.bzl` plus
#   `strict_parsing.expected`) plus `cli/cli/tests/fixtures/help_goldens/`
#   (`pins.bzl` plus `help_goldens.expected` plus `top_help.golden` plus
#   representative per-command goldens) plus unit fixtures in
#   `cli/cli/src/args/strict_tests.rs` plus `cli/cli/src/args/help.rs`;
# - scope: dx CLI surface only; thin-shim frozen legacy plus Bazel semantics
#   unchanged. Seed only: no Supported claim.
#
# Versioned here, run by CI via `bazel run //tools/ci:cli_strict_qualification`,
# following //tools/ci:cli_execution_gaps_qualification.
set -euo pipefail

# Shared workspace + runfiles helpers.
# Bootstrap via tools/sh/bootstrap.sh (issue #654): runfiles forest first, then source tree.
source "${RUNFILES_DIR:-/dev/null}/_main/tools/sh/bootstrap.sh" 2>/dev/null || source "${TEST_SRCDIR:-/dev/null}/_main/tools/sh/bootstrap.sh" 2>/dev/null || source "$0.runfiles/_main/tools/sh/bootstrap.sh" 2>/dev/null || source "${BASH_SOURCE[0]}.runfiles/_main/tools/sh/bootstrap.sh" 2>/dev/null || source "$(git rev-parse --show-toplevel 2>/dev/null)/tools/sh/bootstrap.sh"
dx_bootstrap "tools/sh/lib.sh"

dx_cd_workspace

dx_test_init

contract="docs/cli/cli-contract.md"
testing="docs/testing/cli.md"
strict_pins="cli/cli/tests/fixtures/strict_parsing/pins.bzl"
strict_expected="cli/cli/tests/fixtures/strict_parsing/strict_parsing.expected"
strict_build="cli/cli/tests/fixtures/strict_parsing/BUILD.bazel"
help_pins="cli/cli/tests/fixtures/help_goldens/pins.bzl"
help_expected="cli/cli/tests/fixtures/help_goldens/help_goldens.expected"
help_build="cli/cli/tests/fixtures/help_goldens/BUILD.bazel"
top_golden="cli/cli/tests/fixtures/help_goldens/top_help.golden"
lint_golden="cli/cli/tests/fixtures/help_goldens/lint_help.golden"
build_golden="cli/cli/tests/fixtures/help_goldens/build_help.golden"
clean_golden="cli/cli/tests/fixtures/help_goldens/clean_help.golden"
bazel_golden="cli/cli/tests/fixtures/help_goldens/bazel_help.golden"
docs_golden="cli/cli/tests/fixtures/help_goldens/docs_help.golden"
grammar="cli/cli/src/args/grammar.rs"
strict_tests="cli/cli/src/args/strict_tests.rs"
help_mod="cli/cli/src/args/help.rs"
build="tools/ci/ci_targets_c.bzl"
ci="tools/ci/dogfood_freshness.sh"

# Contract owns the #810 strict record with fixtures plus goldens plus qualification.
if grep -q -F -e 'strict clap parsing with auto help' "$contract" &&
  grep -q -F -e 'qualified seed-only under issue #810' "$contract" &&
  grep -q -F -e 'bazel run //tools/ci:cli_strict_qualification' "$contract" &&
  grep -q -F -e 'cli/cli/tests/fixtures/strict_parsing/' "$contract" &&
  grep -q -F -e 'cli/cli/tests/fixtures/help_goldens/' "$contract"; then
  ok
else
  bad "cli-contract.md lost its #810 strict parsing plus help-goldens record with fixtures plus qualification"
fi

# Strict fixture pins stay present with unknown plus missing plus bad plus hyphen plus suggestions plus bazel plus help.
if [[ -f "$strict_pins" && -f "$strict_expected" && -f "$strict_build" ]] &&
  grep -q -F -e 'STRICT_UNKNOWN_REJECTED' "$strict_pins" &&
  grep -q -F -e 'STRICT_MISSING_REJECTED' "$strict_pins" &&
  grep -q -F -e 'STRICT_HYPHEN_VALUES_NOT_CONSUMED' "$strict_pins" &&
  grep -q -F -e 'STRICT_BAZEL_FORWARDS_VERBATIM' "$strict_pins" &&
  grep -q -F -e 'STRICT_HELP_VERB_REDIRECT' "$strict_pins" &&
  grep -q -F -e 'STRICT_GENERATED_HELP_FROM_GRAMMAR' "$strict_pins" &&
  grep -q -F -e 'qualified seed-only under issue #810' "$strict_pins"; then
  ok
else
  bad "strict_parsing pins fixture lost its strict wiring under #810"
fi

# Strict expected pins the strict plus rejected plus honesty lines.
if grep -q -F -e 'unknown options rejected' "$strict_expected" &&
  grep -q -F -e 'missing values rejected' "$strict_expected" &&
  grep -q -F -e 'hyphen-led tokens never consumed' "$strict_expected" &&
  grep -q -F -e 'help verb redirect' "$strict_expected" &&
  grep -q -F -e 'qualified seed-only under issue #810' "$strict_expected" &&
  grep -q -F -e 'no Supported claim' "$strict_expected"; then
  ok
else
  bad "strict_parsing.expected lost its strict plus honesty lines under #810"
fi

# Fixture BUILD exports pins plus expected with corpus coverage.
if grep -q -F -e 'pins.bzl' "$strict_build" &&
  grep -q -F -e 'strict_parsing.expected' "$strict_build" &&
  grep -q -F -e 'corpus_starlark' "$strict_build"; then
  ok
else
  bad "strict_parsing BUILD.bazel lost its pins plus expected exports with corpus under #810"
fi

# Help goldens fixture pins stay present with top plus representative plus invariants.
if [[ -f "$help_pins" && -f "$help_expected" && -f "$help_build" ]] &&
  grep -q -F -e 'HELP_TOP_GOLDEN' "$help_pins" &&
  grep -q -F -e 'HELP_REPRESENTATIVE_GOLDENS' "$help_pins" &&
  grep -q -F -e 'HELP_COMMAND_INVARIANTS' "$help_pins" &&
  grep -q -F -e 'HELP_VERB_REDIRECT' "$help_pins" &&
  grep -q -F -e 'HELP_COMMAND_COUNT = 32' "$help_pins" &&
  grep -q -F -e 'qualified seed-only under issue #810' "$help_pins"; then
  ok
else
  bad "help_goldens pins fixture lost its golden wiring under #810"
fi

# Help expected pins the golden plus invariant plus honesty lines.
if grep -q -F -e 'top help golden' "$help_expected" &&
  grep -q -F -e 'Per-command flags' "$help_expected" &&
  grep -q -F -e 'help verb redirect' "$help_expected" &&
  grep -q -F -e 'qualified seed-only under issue #810' "$help_expected" &&
  grep -q -F -e 'no Supported claim' "$help_expected"; then
  ok
else
  bad "help_goldens.expected lost its golden plus honesty lines under #810"
fi

# Help BUILD exports pins plus expected plus goldens with corpus coverage.
if grep -q -F -e 'pins.bzl' "$help_build" &&
  grep -q -F -e 'help_goldens.expected' "$help_build" &&
  grep -q -F -e 'top_help.golden' "$help_build" &&
  grep -q -F -e 'lint_help.golden' "$help_build" &&
  grep -q -F -e 'bazel_help.golden' "$help_build" &&
  grep -q -F -e 'corpus_starlark' "$help_build"; then
  ok
else
  bad "help_goldens BUILD.bazel lost its pins plus expected plus goldens exports with corpus under #810"
fi

# Goldens stay present with generated invariants (brand plus commands plus flags plus sections).
for golden in "$top_golden" "$lint_golden" "$build_golden" "$clean_golden" "$bazel_golden" "$docs_golden"; do
  if [[ ! -f "$golden" ]]; then
    bad "help golden missing: $golden"
  fi
done
if grep -q -F -e 'dx - Transparent UI over Bazel' "$top_golden" &&
  grep -q -F -e 'Commands:' "$top_golden" &&
  grep -q -F -e '--workspace' "$top_golden" &&
  grep -q -F -e 'Exit codes' "$top_golden" &&
  grep -q -F -e 'bash|zsh|fish|powershell' "$top_golden"; then
  ok
else
  bad "top_help.golden lost its generated brand plus commands plus flags under #810"
fi
for golden in "$lint_golden" "$build_golden" "$clean_golden" "$bazel_golden" "$docs_golden"; do
  if grep -q -F -e 'Usage:' "$golden" &&
    grep -q -F -e 'Scopes:' "$golden" &&
    grep -q -F -e 'Exit codes:' "$golden" &&
    grep -q -F -e 'Output:' "$golden" &&
    grep -q -F -e 'Per-command flags:' "$golden" &&
    grep -q -F -e '--workspace' "$golden"; then
    ok
  else
    bad "help golden lost its usage plus scopes plus exits plus output under #810: $golden"
  fi
done

# Grammar pins strict help-subcommand refusal with auto help plus verb redirect.
if grep -q -F -e 'disable_help_subcommand' "$grammar" &&
  grep -q -F -e 'Strict parsing' "$grammar" &&
  grep -q -F -e 'dx help [command]' "$grammar"; then
  ok
else
  bad "grammar.rs lost its #810 strict disable_help_subcommand plus auto-help pin"
fi

# Unit fixtures keep strict plus help-golden coverage.
if grep -q -F -e 'strict_unknown_options_fail_with_whole_token' "$strict_tests" &&
  grep -q -F -e 'strict_missing_values_fail_with_bare_flag' "$strict_tests" &&
  grep -q -F -e 'strict_hyphen_values_are_never_consumed' "$strict_tests" &&
  grep -q -F -e 'strict_help_verb_redirects_to_generated_help' "$strict_tests" &&
  grep -q -F -e 'strict_help_is_generated_from_the_same_grammar' "$strict_tests" &&
  grep -q -F -e 'strict_every_command_help_pins_usage_scopes_exits_output' "$strict_tests"; then
  ok
else
  bad "strict_tests.rs lost its #810 strict plus help fixtures"
fi

# Help module keeps generated rendering from the same grammar plus verb redirect.
if grep -q -F -e 'render_top_help' "$help_mod" &&
  grep -q -F -e 'render_command_help' "$help_mod" &&
  grep -q -F -e 'per_command_flags' "$help_mod" &&
  grep -q -F -e 'help_verb_error_in' "$help_mod"; then
  ok
else
  bad "help.rs lost its generated help rendering pins under #810"
fi

# Testing matrix pins strict plus help goldens under.
if grep -q -F -e 'bazel run //tools/ci:cli_strict_qualification' "$testing" &&
  grep -q -F -e 'issue #810' "$testing"; then
  ok
else
  bad "testing/cli.md lost its #810 strict plus help-goldens pins"
fi

# BUILD owns the harness target plus dogfood wires it.
if grep -q -F -e 'name = "cli_strict_qualification"' "$build" &&
  grep -q -F -e 'cli_strict_qualification.sh' "$build" &&
  grep -q -F -e 'bazel run --noshow_progress //tools/ci:cli_strict_qualification' "$ci"; then
  ok
else
  bad "tools/ci wiring lost the cli_strict_qualification target plus dogfood-freshness under #810"
fi

dx_test_summary "cli strict plus help-goldens qualification harness"
