#!/usr/bin/env bash
# Renovate parity fixture (closed issue #3 delivered, snapshot workflow issue #322): the
# repository's own `renovate.json` is a snapshot of the `dx init` scaffold
# output, so the config we ship is the one we run. A drifting hand copy is
# worse than none.
#
# Snapshot testing, not brittle equality: `snapshot_diff` fails with a
# unified diff, while `UPDATE_EXPECT=1` refreshes the golden instead of
# failing (bazel test --test_env=UPDATE_EXPECT). Schema validation below
# pins the contract fields so scaffold drift fails at the source even when
# the snapshot is refreshed.
#
# Shell sources have no corpus class. Process-spawning tests stay out of
# the coverage denominator per the repo coverage preset.
set -euo pipefail

# Shared snapshot helper (issue #322).
# Bootstrap: Bazel runfiles forest first (`data = ["//tools/sh:snapshot"]`), then source tree.
source "${RUNFILES_DIR:-/dev/null}/_main/tools/sh/snapshot.sh" 2>/dev/null || source "${TEST_SRCDIR:-/dev/null}/_main/tools/sh/snapshot.sh" 2>/dev/null || source "$0.runfiles/_main/tools/sh/snapshot.sh" 2>/dev/null || source "${BASH_SOURCE[0]}.runfiles/_main/tools/sh/snapshot.sh" 2>/dev/null || source "$(dirname "${BASH_SOURCE[0]}")/tools/sh/snapshot.sh"

# Shared CI helpers (issues #319, #323).
# Bootstrap: Bazel runfiles forest first (`data = ["//tools/sh:lib"]`), then source tree.
source "${RUNFILES_DIR:-/dev/null}/_main/tools/sh/lib.sh" 2>/dev/null || source "${TEST_SRCDIR:-/dev/null}/_main/tools/sh/lib.sh" 2>/dev/null || source "$0.runfiles/_main/tools/sh/lib.sh" 2>/dev/null || source "${BASH_SOURCE[0]}.runfiles/_main/tools/sh/lib.sh" 2>/dev/null || source "$(dirname "${BASH_SOURCE[0]}")/tools/sh/lib.sh"

# Portable helpers via tools/sh/lib.sh dx_realpath/dx_mkscratch (issues #299, #323).

expected="$(dx_realpath "$1")"
dx_bin="$(dx_realpath "$2")"

dx_mkscratch scratch

# The `dx` binary resolves its workspace through a `MODULE.bazel`
# marker (or `--workspace` override): seed a stub workspace so `init`
# plans against an empty tree instead of this repository.
touch "$scratch/MODULE.bazel"
"${dx_bin}" --workspace "$scratch" init --quiet >/dev/null

actual="$scratch/renovate.json"

# Schema validation (issue #322): pin the contract fields on both the
# scaffold output and the checked-in file, mirroring
# `cli/adopt/src/scaffold.rs::renovate_scaffold_ships_full_manager_set_automerge_off`.
# Exact bytes stay in the snapshot below; this fails first on shape drift.
for f in "$expected" "$actual"; do
  snapshot_json_validates "$f"
  python3 - "$f" <<'PY'
import json, sys
doc = json.load(open(sys.argv[1]))
assert doc["$schema"] == "https://docs.renovatebot.com/renovate-schema.json", "missing $schema"
assert doc["extends"] == ["config:recommended"], "extends must stay config:recommended"
assert isinstance(doc.get("schedule"), list) and doc["schedule"], "schedule must stay non-empty"
assert doc.get("labels") == ["dependencies"], "labels must stay ['dependencies']"
managers = doc.get("enabledManagers", [])
for want in ["bazel", "cargo", "github-actions", "gomod", "npm"]:
    assert want in managers, f"missing manager {want}: {managers}"
rules = doc.get("packageRules", [])
assert any(r.get("matchManagers") == ["bazel"] for r in rules), "missing bazel packageRule"
assert doc.get("automerge") is False, "automerge must stay off"
assert doc.get("platformAutomerge") is False, "platformAutomerge must stay off"
assert doc.get("prCreation") == "not-pending", "prCreation must stay not-pending"
assert doc.get("dependencyDashboard") is True, "dependencyDashboard must stay true"
PY
done
echo "renovate schema: scaffold and checked-in definition carry the pinned contract"

snapshot_diff "$expected" "$actual" "renovate.json"
echo "renovate parity: scaffold output matches checked-in definition (snapshot)"
