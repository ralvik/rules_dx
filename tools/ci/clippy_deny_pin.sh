#!/usr/bin/env bash
# Clippy deny pin for issue #955.
#
# Per-crate `deny(clippy::expect_used, clippy::unwrap_used, ...)` rolls out
# per crate with no global enforcement: new crates could land without the
# deny and regress the `expect`/`unwrap`-free non-test invariant. This
# harness pins the exact first-party crate roots carrying the deny plus the
# `too_many_*`/`unused_imports` blanket-allow ban, so additions fail closed.
#
# Versioned here, run by CI via `bazel run //tools/ci:clippy_deny_pin`,
# following //tools/ci:source_hygiene.
set -euo pipefail

# Shared workspace + runfiles helpers.
# Bootstrap via tools/sh/bootstrap.sh (issue #654): runfiles forest first, then source tree.
source "${RUNFILES_DIR:-/dev/null}/_main/tools/sh/bootstrap.sh" 2>/dev/null || source "${TEST_SRCDIR:-/dev/null}/_main/tools/sh/bootstrap.sh" 2>/dev/null || source "$0.runfiles/_main/tools/sh/bootstrap.sh" 2>/dev/null || source "${BASH_SOURCE[0]}.runfiles/_main/tools/sh/bootstrap.sh" 2>/dev/null || source "$(git rev-parse --show-toplevel 2>/dev/null)/tools/sh/bootstrap.sh"
dx_bootstrap "tools/sh/lib.sh"

dx_cd_workspace

dx_test_init

# --- Pinned first-party crate roots carrying the deny ---
# Every `src/lib.rs` / `src/main.rs` outside testdata/fixtures/tests must
# appear here and carry `#![cfg_attr(not(test), deny(... expect_used ...))]`.
# Add new crates here in the same PR that adds their sources.
expected=(
  "cli/adopt/src/lib.rs"
  "cli/apply/src/lib.rs"
  "cli/atomic_fs/src/lib.rs"
  "cli/audit/src/lib.rs"
  "cli/bep/src/lib.rs"
  "cli/bump/src/lib.rs"
  "cli/ci/src/lib.rs"
  "cli/clean/src/lib.rs"
  "cli/cli/src/lib.rs"
  "cli/cli/src/main.rs"
  "cli/codegen/src/lib.rs"
  "cli/diff/src/lib.rs"
  "cli/digest/src/lib.rs"
  "cli/docgen/src/lib.rs"
  "cli/env/src/lib.rs"
  "cli/env/src/main.rs"
  "cli/env_plan/src/lib.rs"
  "cli/fingerprint/src/lib.rs"
  "cli/lcov/src/lib.rs"
  "cli/output/src/lib.rs"
  "cli/path/src/lib.rs"
  "cli/process/src/lib.rs"
  "cli/proto_validate/src/lib.rs"
  "cli/qualification/src/lib.rs"
  "cli/roots/src/lib.rs"
  "cli/schema/src/lib.rs"
  "cli/setup/src/lib.rs"
  "cli/test_scratch/src/lib.rs"
  "cli/update/src/lib.rs"
  "deploy/install/src/lib.rs"
  "deploy/install/src/main.rs"
  "deploy/release/src/lib.rs"
  "deploy/rules/src/lib.rs"
  "docs/adapters/src/lib.rs"
  "docs/ir/ir/src/lib.rs"
  "env/env_shard/src/lib.rs"
  "env/env_shard/src/main.rs"
  "generation/codegen_shard/src/lib.rs"
  "generation/codegen_shard/src/main.rs"
  "generation/result/src/lib.rs"
  "quality/adapter/src/lib.rs"
  "quality/evaluator/src/lib.rs"
  "quality/evaluator/src/main.rs"
  "quality/markdown/src/lib.rs"
  "quality/markdown/src/main.rs"
  "quality/result/src/lib.rs"
  "quality/runner/src/lib.rs"
  "quality/runner/src/main.rs"
  "tools/bazelrc/src/lib.rs"
  "tools/coverage/src/main.rs"
  "tools/depcheck/src/lib.rs"
  "tools/depcheck/src/main.rs"
)

# Every pinned root exists and carries the deny.
missing=""
for f in "${expected[@]}"; do
  if [[ ! -f "$f" ]]; then
    missing="$missing $f(missing)"
  elif ! grep -q -F -e 'clippy::expect_used' "$f" ||
    ! grep -q -F -e 'clippy::unwrap_used' "$f" ||
    ! grep -q -F -e 'cfg_attr' "$f"; then
    missing="$missing $f(no-deny)"
  else
    ok
  fi
done
if [[ -n "$missing" ]]; then
  bad "clippy deny pin lost roots:$missing (want every pinned crate root carrying cfg_attr(not(test), deny(... expect_used ... unwrap_used ...)), issue #955)"
else
  ok
fi

# No unpinned first-party crate root exists outside the pin (new crates fail closed).
dx_mkscratch clippy_scratch
find cli quality tools docs deploy env generation -name "lib.rs" -o -name "main.rs" 2>/dev/null |
  grep -v -F -e 'testdata' |
  grep -v -F -e '/tests/' |
  grep -v -F -e 'fixtures' |
  LC_ALL=C sort -u >"$clippy_scratch/found.txt"
printf '%s\n' "${expected[@]}" | LC_ALL=C sort -u >"$clippy_scratch/expected.txt"
unpinned=""
while IFS= read -r f; do
  rel="${f#./}"
  if ! grep -q -F -x -e "$rel" "$clippy_scratch/expected.txt"; then
    unpinned="$unpinned $rel"
  fi
done <"$clippy_scratch/found.txt"
if [[ -z "$unpinned" ]]; then
  ok
else
  bad "unpinned crate roots:$unpinned (want new src/lib.rs/src/main.rs added to tools/ci/clippy_deny_pin.sh plus the deny, issue #955)"
fi
# Pin stays honest: every entry is still a real crate root.
stale=""
for f in "${expected[@]}"; do
  if ! grep -q -F -x -e "$f" "$clippy_scratch/found.txt"; then
    stale="$stale $f"
  fi
done
if [[ -z "$stale" ]]; then
  ok
else
  bad "stale pin entries:$stale (want pin matching the tree, issue #955)"
fi

# Blanket allows stay out of first-party non-test code: params structs and
# extraction replace `too_many_*`, per-use scoping replaces `unused_imports`.
if ! grep -rn -F -e 'allow(clippy::too_many_arguments)' --include='*.rs' cli/ quality/ tools/ docs/ deploy/ env/ generation/ 2>/dev/null | grep -v -F -e 'testdata' | grep -q .; then
  ok
else
  bad "allow(clippy::too_many_arguments) reappeared (want params struct, issue #955)"
fi
if ! grep -rn -F -e 'allow(clippy::too_many_lines)' --include='*.rs' cli/ quality/ tools/ docs/ deploy/ env/ generation/ 2>/dev/null | grep -v -F -e 'testdata' | grep -q .; then
  ok
else
  bad "allow(clippy::too_many_lines) reappeared (want extraction, issue #955)"
fi
if ! grep -rn -F -e 'allow(unused_imports)' --include='*.rs' cli/ quality/ tools/ docs/ deploy/ env/ generation/ 2>/dev/null | grep -v -F -e 'testdata' | grep -q .; then
  ok
else
  bad "allow(unused_imports) reappeared (want per-use scoping or deletion, issue #955)"
fi

# clippy.toml stays the single source of truth and points at this pin.
if grep -q -F -e 'clippy_deny_pin' clippy.toml; then
  ok
else
  bad "clippy.toml lost its clippy_deny_pin pointer (want single source of truth plus CI pin, issue #955)"
fi

dx_test_summary "clippy deny pin (issue #955)"
