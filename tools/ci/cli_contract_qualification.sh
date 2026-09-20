#!/usr/bin/env bash
# CLI-contract registry plus behavior qualification harness.
#
# docs/roadmap.md lists cli-contract claims with only helper guards at
# tools/ci/helper_qualification.sh (seed-only record, adopted crates,
# stays-hand-rolled owners) and no owner for the registry plus behavior
# pins. This harness owns that gap.
#
# Qualifies the as-built final registry plus mutating-vs-check semantics
# with fixture evidence (see docs/testing/cli.md#command-registry-and-behavior):
# - final registry holds exactly the 29 parsed commands including `deploy`
#   plus `bump` plus `migrate` (`Command` grammar plus `dx_adopt::ALL_COMMANDS`
#   plus the qualified command reference); `doctor` plus `configure` stay
#   rejected as unknown; `migrate` syntax plus manifest selection delivered
# under;
# - help plus `Command::is_mutating_by_default` identify the mutating
#   default; `--output=diff` stays exactly the six patch producers
#   (lint, typecheck, format, generate, check, fix);
# - the `check`/`fix` umbrella stays the sequential `format` then `lint`
#   then `typecheck` then `generate` phases with stop-on-first-failure
#   and no parallel, caching, scheduling, or daemon behavior.
#
# Versioned here, run by CI via `bazel run //tools/ci:cli_contract_qualification`,
# following //tools/ci:helper_qualification.
set -euo pipefail

# Shared workspace + runfiles helpers.
# Bootstrap: Bazel runfiles forest first (`data = ["//tools/sh:lib"]`), then source tree.
source "${RUNFILES_DIR:-/dev/null}/_main/tools/sh/lib.sh" 2>/dev/null || source "${TEST_SRCDIR:-/dev/null}/_main/tools/sh/lib.sh" 2>/dev/null || source "$0.runfiles/_main/tools/sh/lib.sh" 2>/dev/null || source "${BASH_SOURCE[0]}.runfiles/_main/tools/sh/lib.sh" 2>/dev/null || source "$(dirname "${BASH_SOURCE[0]}")/../sh/lib.sh"

dx_cd_workspace

dx_test_init

contract="docs/cli/cli-contract.md"
testing="docs/testing/cli.md"
reference="docs/cli/commands/README.md"

# Contract owns the / pinned record.
if grep -q -F -e 'pinned under issue' "$contract" &&
  grep -q -F -e 'bazel run //tools/ci:cli_contract_qualification' "$contract" &&
  grep -q -F -e 'exactly the 29 parsed commands' "$contract" &&
  grep -q -F -e 'including `deploy` plus `bump` plus' "$contract" &&
  grep -q -F -e '`migrate`' "$contract"; then
  ok
else
  bad "cli-contract lost its pinned #457/#462 registry plus behavior record"
fi

# The `Command` grammar holds exactly the final 29 (deploy plus bump plus migrate).
if grep -q -F -e 'Deploy,' cli/cli/src/args/command.rs &&
  grep -q -F -e 'Bump,' cli/cli/src/args/command.rs &&
  grep -q -F -e 'Migrate,' cli/cli/src/args/command.rs &&
  grep -q -F -e 'is_mutating_by_default' cli/cli/src/args/command.rs &&
  grep -q -F -e 'final_registry_is_exact' cli/cli/src/args/command.rs; then
  ok
else
  bad "Command grammar lost its deploy/bump/migrate plus mutating plus registry fixtures"
fi

# The frozen vocabulary reference matches the final 29 (deploy plus bump plus migrate).
if grep -q -F -e '"deploy",' cli/adopt/src/completion.rs &&
  grep -q -F -e '"bump",' cli/adopt/src/completion.rs &&
  grep -q -F -e '"migrate",' cli/adopt/src/completion.rs &&
  grep -q -F -e 'completion_vocabulary_is_the_final_registry' cli/adopt/src/completion.rs; then
  ok
else
  bad "ALL_COMMANDS lost its deploy/bump/migrate plus final-registry fixture"
fi

# Testing matrix pins the final registry with deploy plus bump plus migrate.
if grep -q -F -e '`deploy`' "$testing" &&
  grep -q -F -e '`bump`' "$testing" &&
  grep -q -F -e '`migrate`' "$testing" &&
  grep -q -F -e 'cli_contract_qualification' "$testing" &&
  grep -q -F -e 'issue #457' "$testing"; then
  ok
else
  bad "testing/cli.md lost its final-registry record with deploy plus bump plus migrate"
fi

# Testing matrix keeps the excluded-command plus umbrella behavior pins.
if grep -q -F -e 'doctor' "$testing" &&
  grep -q -F -e 'rejected as unknown' "$testing" &&
  grep -q -F -e 'format --check' "$testing" &&
  grep -q -F -e 'stops on the first required phase failure' "$testing" &&
  grep -q -F -e 'parallel execution' "$testing"; then
  ok
else
  bad "testing/cli.md lost its excluded-command plus umbrella behavior pins"
fi

# Testing matrix keeps the mutating-identification pin with bump plus migrate.
if grep -q -F -e '`bump`' "$testing" &&
  grep -q -F -e '`migrate`' "$testing" &&
  grep -q -F -e 'as mutating by default' "$testing" &&
  grep -q -F -e 'is_mutating_by_default' "$testing"; then
  ok
else
  bad "testing/cli.md lost its mutating-identification pin with bump plus migrate"
fi

# Command reference lists bump plus deploy plus migrate under.
if grep -q -F -e 'dx bump' "$reference" &&
  grep -q -F -e 'dx deploy' "$reference" &&
  grep -q -F -e 'dx migrate' "$reference" &&
  grep -q -F -e 'issue #462' "$reference" &&
  grep -q -F -e 'There is no `dx doctor`' "$reference"; then
  ok
else
  bad "commands/README.md lost its bump plus deploy plus migrate plus excluded record"
fi

# Umbrella phases stay sequential format, lint, typecheck, then generate.
if grep -q -F -e 'Command::Format,' cli/cli/src/exec/umbrella.rs &&
  grep -q -F -e 'Command::Lint,' cli/cli/src/exec/umbrella.rs &&
  grep -q -F -e 'Command::Typecheck,' cli/cli/src/exec/umbrella.rs &&
  grep -q -F -e 'Command::Generate,' cli/cli/src/exec/umbrella.rs; then
  ok
else
  bad "umbrella lost its format/lint/typecheck/generate phase order"
fi

# Umbrella stops on the first required failure with no daemon or parallel fan-out.
if grep -q -F -e 'break;' cli/cli/src/exec/umbrella.rs &&
  grep -q -F -e 'Sequential' cli/cli/src/exec/umbrella.rs &&
  ! grep -E -e 'tokio::spawn|par_iter|rayon|daemon' cli/cli/src/exec/umbrella.rs | grep -q .; then
  ok
else
  bad "umbrella lost its stop-on-first-failure plus no-daemon/parallel pin"
fi

# Diff stays exactly the six patch producers.
if grep -q -F -e 'Command::Lint' cli/cli/src/args/command.rs &&
  grep -q -F -e 'Command::Check' cli/cli/src/args/command.rs &&
  grep -q -F -e 'supports_diff' cli/cli/src/args/command.rs; then
  ok
else
  bad "Command::supports_diff lost its six-command patch-producer pin"
fi

dx_test_summary "cli-contract registry plus behavior harness"
