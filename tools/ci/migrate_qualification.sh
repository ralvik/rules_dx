#!/usr/bin/env bash
# Migrate syntax plus manifest-selection qualification harness.
#
# Qualifies the as-built `dx migrate` V1 scope with fixture evidence
# (see docs/cli/commands/migrate.md plus docs/testing/cli.md):
# - syntax: `dx migrate --from <version> --to <version> [scope ...]`,
#   both Cargo-flavor semver, upgrade-only gate
#   (`dx_adopt::migrate_is_upgrade` plus `plan_migrate`, major hops stay
#   the `migrate_is_major_bump` subset), prerelease
#   and build metadata ride the same gate, usage errors exit 2;
# - manifest selection: one manifest per major hop
#   (`migrate-v<from_major>-to-v<to_major>.json` via
#   `dx_adopt::migrate_manifest_name`) plus one per full version pair
#   for minor/patch upgrades (`migrate-v<from>-to-v<to>.json` via
#   `dx_adopt::migrate_manifest_name_full`), edit-manifest records applied
#   through the generation write-outcome pattern;
# - execution: `--dry-run` plans without writes (text plus JSON
#   `migrate_planned`), live runs fail closed with `migrate_failed`
#   (exit 1, no writes) until the first manifest lands (module 0.0.0);
# - registry: `migrate` inside the final 31 parsed commands with
#   `--from`/`--to` owned, mutating by default, JSON-capable, diff
#   rejected, help plus docs in place.
#
# Versioned here, run by CI via `bazel run //tools/ci:migrate_qualification`,
# following //tools/ci:cli_contract_qualification.
set -euo pipefail

# Shared workspace + runfiles helpers.
# Bootstrap via tools/sh/bootstrap.sh (issue #654): runfiles forest first, then source tree.
source "${RUNFILES_DIR:-/dev/null}/_main/tools/sh/bootstrap.sh" 2>/dev/null || source "${TEST_SRCDIR:-/dev/null}/_main/tools/sh/bootstrap.sh" 2>/dev/null || source "$0.runfiles/_main/tools/sh/bootstrap.sh" 2>/dev/null || source "${BASH_SOURCE[0]}.runfiles/_main/tools/sh/bootstrap.sh" 2>/dev/null || source "$(git rev-parse --show-toplevel 2>/dev/null)/tools/sh/bootstrap.sh"
dx_bootstrap "tools/sh/lib.sh"

dx_cd_workspace

dx_test_init

migrate_rs="cli/adopt/src/migrate.rs"
migrate_exec="cli/cli/src/exec/migrate.rs"
command_rs="cli/cli/src/args/command.rs"
grammar_rs="cli/cli/src/args/grammar.rs"
parser_rs="cli/cli/src/args/parser.rs"
invocation_rs="cli/cli/src/args/invocation.rs"
help_rs="cli/cli/src/args/help.rs"
registry_rs="cli/cli/src/plan/registry.rs"
common_rs="cli/cli/src/exec/common.rs"
migrate_doc="docs/cli/commands/migrate.md"
testing="docs/testing/cli.md"

# Planning library owns the upgrade-only gate plus manifest selection.
if grep -q -F -e 'pub fn migrate_is_upgrade' "$migrate_rs" &&
  grep -q -F -e 'pub fn migrate_is_major_bump' "$migrate_rs" &&
  grep -q -F -e 'pub fn migrate_manifest_name' "$migrate_rs" &&
  grep -q -F -e 'pub fn migrate_manifest_name_full' "$migrate_rs" &&
  grep -q -F -e 'pub fn plan_migrate' "$migrate_rs" &&
  grep -q -F -e 'migrate_upgrade_gate_accepts_any_upgrade' "$migrate_rs" &&
  grep -q -F -e 'migrate_manifest_selection_is_mechanical' "$migrate_rs" &&
  grep -q -F -e 'migrate_syntax_pins_semver' "$migrate_rs"; then
  ok
else
  bad "migrate planning lost its gate plus manifest plus syntax fixtures"
fi

# Manifest selection stays mechanical per major hop plus per full version pair.
if grep -q -F -e 'migrate-v{from_major}-to-v{to_major}.json' "$migrate_rs" &&
  grep -q -F -e 'migrate-v{from}-to-v{to}.json' "$migrate_rs" &&
  grep -q -F -e 'migrate-v1-to-v2.json' "$migrate_rs" &&
  grep -q -F -e 'migrate-v0-to-v1.json' "$migrate_rs" &&
  grep -q -F -e 'migrate-v1.2.3-to-v1.3.0.json' "$migrate_rs"; then
  ok
else
  bad "migrate manifest selection lost its per-hop plus per-version pin"
fi

# CLI grammar owns --from/--to as migrate-only value options.
if grep -q -F -e '"--from"' "$grammar_rs" &&
  grep -q -F -e '"--to"' "$grammar_rs" &&
  grep -q -F -e 'pub(crate) from' "$grammar_rs" &&
  grep -q -F -e 'pub(crate) to' "$grammar_rs" &&
  grep -q -F -e 'pub from' "$invocation_rs" &&
  grep -q -F -e 'pub to' "$invocation_rs"; then
  ok
else
  bad "migrate grammar lost its --from/--to value-option pin"
fi

# Parser requires both versions for migrate/upgrade and rejects them elsewhere.
if grep -q -F -e 'Command::Migrate' "$parser_rs" &&
  grep -q -F -e '--from <version> --to <version>' "$parser_rs" &&
  grep -q -F -e '--from' "$parser_rs" &&
  grep -q -F -e 'belong to `migrate` plus `upgrade` only' "$parser_rs"; then
  ok
else
  bad "migrate parser lost its required-versions plus ownership pin"
fi

# Command registry holds migrate with mutating-plus-JSON contract.
if grep -q -F -e 'Migrate,' "$command_rs" &&
  grep -q -F -e 'Command::Migrate => "migrate"' "$command_rs" &&
  grep -q -F -e 'final_registry_is_exact' "$command_rs"; then
  ok
else
  bad "migrate registry lost its Command variant plus fixture pin"
fi

# Execution plans dry-run without writes and fails closed live.
if grep -q -F -e 'pub(crate) fn execute_migrate' "$migrate_exec" &&
  grep -q -F -e 'migrate_planned' "$migrate_exec" &&
  grep -q -F -e 'CODE_MIGRATE_FAILED' "$common_rs" &&
  grep -q -F -e 'migrate_failed' "$migrate_exec" &&
  grep -q -F -e 'no migrate manifest' "$migrate_exec"; then
  ok
else
  bad "migrate execution lost its dry-run plus fail-closed pin"
fi

# Registry capability has no standard reports and no workflow aspects.
if grep -q -F -e 'Command::Migrate' "$registry_rs" &&
  grep -q -F -e '"migrate"' "$registry_rs"; then
  ok
else
  bad "migrate registry spec lost its capability pin"
fi

# Help names the migrate usage plus owned flags.
if grep -q -F -e 'migrate --from' "$help_rs" &&
  grep -q -F -e '--from <version> --to <version>' "$help_rs"; then
  ok
else
  bad "migrate help lost its usage plus owned-flag pin"
fi

# Docs stay in place with syntax plus manifest plus fail-closed state.
if grep -q -F -e 'delivered CLI' "$migrate_doc" &&
  grep -q -F -e '--from <version>' "$migrate_doc" &&
  grep -q -F -e 'migrate-v<from_major>-to-v<to_major>.json' "$migrate_doc" &&
  grep -q -F -e 'migrate-v<from>-to-v<to>.json' "$migrate_doc" &&
  grep -q -F -e 'No manifests exist yet' "$migrate_doc" &&
  grep -q -F -e 'migrate_failed' "$migrate_doc" &&
  grep -q -F -e 'issue #462' "$migrate_doc" &&
  grep -q -F -e 'issue #671' "$migrate_doc"; then
  ok
else
  bad "migrate doc lost its syntax plus manifest plus fail-closed record"
fi

# Testing matrix pins the migrate fixtures with the owner.
if grep -q -F -e '`migrate`' "$testing" &&
  grep -q -F -e 'migrate_qualification' "$testing" &&
  grep -q -F -e 'migrate_failed' "$testing" &&
  grep -q -F -e 'issue #462' "$testing" &&
  grep -q -F -e 'issue #671' "$testing"; then
  ok
else
  bad "testing/cli.md lost its migrate fixture plus #462/#671 owner record"
fi

dx_test_summary "migrate syntax plus manifest-selection harness"
