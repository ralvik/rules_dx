#!/usr/bin/env bash
# Hand-rolled helpers qualification harness (issue #315).
#
# Qualifies the as-built per-helper decisions with fixture evidence and owned
# gaps, without claiming unreviewed migrations:
# - adopted with fixture evidence: digest via hex/blake3/sha2, diff via
#   similar, SPDX parse via spdx, date calendar via chrono, scratch via
#   tempfile, dir sizing and walks via walkdir/ignore/globset;
# - stays hand-rolled with owned reasons: atomic write plus lock via
#   std::fs::File::try_lock plus tempfile, path-ladder classifier, LCOV
#   parser plus ignore scanner with inventory plus verdict, SPDX lattice plus
#   date shape gate plus scratch discipline wrappers;
# - open owned gap: upstream re-evaluation on new crate versions plus any
#   future migration.
#
# Versioned here, run by CI via `bazel run //tools/ci:helper_qualification`,
# following //tools/ci:file_family_qualification.
set -euo pipefail

# Shared workspace + runfiles helpers (issue #319).
# Bootstrap: Bazel runfiles forest first (`data = ["//tools/sh:lib"]`), then source tree.
source "${RUNFILES_DIR:-/dev/null}/_main/tools/sh/lib.sh" 2>/dev/null || source "${TEST_SRCDIR:-/dev/null}/_main/tools/sh/lib.sh" 2>/dev/null || source "$0.runfiles/_main/tools/sh/lib.sh" 2>/dev/null || source "${BASH_SOURCE[0]}.runfiles/_main/tools/sh/lib.sh" 2>/dev/null || source "$(dirname "${BASH_SOURCE[0]}")/../sh/lib.sh"

workspace="$(dx_workspace_root)"
cd "$workspace"

pass=0
fail=0
ok() { pass=$((pass + 1)); }
bad() { echo "FAIL: $1" >&2; fail=$((fail + 1)); }

contract="docs/cli/cli-contract.md"
verify="docs/testing/verification-matrix.md"

# Contract owns the qualified seed-only record under #315.
if grep -q -F -e 'qualified seed-only under issue #315' "$contract" \
  && grep -q -F -e 'bazel run //tools/ci:helper_qualification' "$contract" \
  && grep -q -F -e 'upstream' "$contract" \
  && grep -q -F -e 'stays owned gap' "$contract"; then
  ok
else
  bad "cli-contract lost its qualified seed-only record under #315"
fi

# Contract names adopted crates plus stays-hand-rolled owners.
if grep -q -F -e 'digest via `hex`/`blake3`/`sha2`' "$contract" \
  && grep -q -F -e 'diff via `similar`' "$contract" \
  && grep -q -F -e 'SPDX parse via `spdx`' "$contract" \
  && grep -q -F -e 'date calendar via `chrono`' "$contract" \
  && grep -q -F -e 'scratch via `tempfile`' "$contract" \
  && grep -q -F -e '`walkdir`/`ignore`/`globset`' "$contract"; then
  ok
else
  bad "cli-contract lost its adopted-crate list"
fi

if grep -q -F -e 'atomic' "$contract" \
  && grep -q -F -e 'path-ladder' "$contract" \
  && grep -q -F -e 'LCOV parser plus ignore scanner' "$contract"; then
  ok
else
  bad "cli-contract lost its stays-hand-rolled owner list"
fi

# Atomic write plus lock stays hand-rolled with owned reason.
if grep -q -F -e 'issue #315' cli/atomic_fs/src/lib.rs \
  && grep -q -F -e 'stays hand-rolled' cli/atomic_fs/src/lib.rs \
  && grep -q -F -e 'file.try_lock()' cli/atomic_fs/src/lib.rs \
  && grep -q -F -e 'NamedTempFile' cli/atomic_fs/src/lib.rs \
  && grep -q -F -e 'fslock' cli/atomic_fs/src/lib.rs; then
  ok
else
  bad "atomic_fs lost its #315 stays-hand-rolled reason"
fi

# Atomic Cargo/BUILD pins tempfile with no fs2/fslock/atomic-write-file.
if grep -q -F -e 'tempfile = "3"' cli/atomic_fs/Cargo.toml \
  && ! grep -E -e 'fs2|fslock|atomic-write' cli/atomic_fs/Cargo.toml | grep -q . \
  && grep -q -F -e '"tempfile"' cli/atomic_fs/BUILD.bazel \
  && ! grep -E -e 'fs2|fslock|atomic-write' cli/atomic_fs/BUILD.bazel | grep -q .; then
  ok
else
  bad "atomic_fs Cargo/BUILD lost its tempfile-only pin"
fi

# Path ladder stays hand-rolled with owned reason.
if grep -q -F -e 'issue #315' cli/path/src/lib.rs \
  && grep -q -F -e 'stays hand-rolled' cli/path/src/lib.rs \
  && grep -q -F -e 'PathProblem' cli/path/src/lib.rs \
  && grep -q -F -e 'path-clean' cli/path/src/lib.rs \
  && grep -q -F -e 'camino' cli/path/src/lib.rs; then
  ok
else
  bad "path lost its #315 stays-hand-rolled reason"
fi

# Path Cargo/BUILD stays dependency-free with no path-clean/camino.
if ! grep -E -e 'path-clean|camino' cli/path/Cargo.toml | grep -q . \
  && grep -q -F -e 'deps = []' cli/path/BUILD.bazel; then
  ok
else
  bad "path gained a path-clean/camino dependency"
fi

# Digest adopted: Cargo plus BUILD pin hex/blake3/sha2.
if grep -q -F -e 'blake3 = "1"' cli/digest/Cargo.toml \
  && grep -q -F -e 'hex = "0.4"' cli/digest/Cargo.toml \
  && grep -q -F -e 'sha2 = "0.10"' cli/digest/Cargo.toml \
  && grep -q -F -e '"blake3"' cli/digest/BUILD.bazel \
  && grep -q -F -e '"hex"' cli/digest/BUILD.bazel \
  && grep -q -F -e '"sha2"' cli/digest/BUILD.bazel; then
  ok
else
  bad "digest Cargo/BUILD lost its hex/blake3/sha2 pins"
fi

# Digest uses upstream crates with no manual digit loop.
if grep -q -F -e 'issue #315' cli/digest/src/lib.rs \
  && grep -q -F -e 'adopted' cli/digest/src/lib.rs \
  && grep -q -F -e 'hex::encode' cli/digest/src/lib.rs \
  && grep -q -F -e 'hex::decode' cli/digest/src/lib.rs \
  && grep -q -F -e 'blake3::hash' cli/digest/src/lib.rs \
  && grep -q -F -e 'Sha256::new' cli/digest/src/lib.rs; then
  ok
else
  bad "digest lost its #315 adopted upstream-use evidence"
fi

# Diff adopted: Cargo plus BUILD pin similar.
if grep -q -F -e 'similar = "3"' cli/diff/Cargo.toml \
  && grep -q -F -e '"similar"' cli/diff/BUILD.bazel \
  && grep -q -F -e 'issue #315' cli/diff/src/lib.rs \
  && grep -q -F -e 'similar::TextDiff' cli/diff/src/lib.rs; then
  ok
else
  bad "diff lost its similar adopted evidence"
fi

# LCOV parser plus ignore scanner stays hand-rolled with owned reason.
if grep -q -F -e 'issue #315' cli/lcov/src/lib.rs \
  && grep -q -F -e 'stays hand-rolled' cli/lcov/src/lib.rs \
  && grep -q -F -e 'parse_lcov' cli/lcov/src/lib.rs \
  && grep -q -F -e 'cargo-llvm-cov' cli/lcov/src/lib.rs; then
  ok
else
  bad "lcov lost its #315 stays-hand-rolled reason"
fi

# LCOV Cargo stays thiserror-only with SF/DA plus reason: evidence.
if grep -q -F -e 'thiserror = "2"' cli/lcov/Cargo.toml \
  && ! grep -E -e '^[[:space:]]*lcov[[:space:]]*=' cli/lcov/Cargo.toml | grep -q . \
  && ! grep -F -e 'cargo-llvm' cli/lcov/Cargo.toml | grep -q . \
  && grep -q -F -e 'strip_prefix("SF:")' cli/lcov/src/parse.rs \
  && grep -q -F -e 'strip_prefix("DA:")' cli/lcov/src/parse.rs \
  && grep -q -F -e 'reason:' cli/lcov/src/ignores.rs; then
  ok
else
  bad "lcov gained a parser dependency or lost SF/DA plus reason evidence"
fi

# SPDX parse adopted: Cargo plus BUILD pin spdx.
if grep -q -F -e 'spdx = ' cli/audit/Cargo.toml \
  && grep -q -F -e '"spdx"' cli/audit/BUILD.bazel \
  && grep -q -F -e 'issue #315' cli/audit/src/license_expr.rs \
  && grep -q -F -e 'adopted' cli/audit/src/license_expr.rs \
  && grep -q -F -e 'spdx::Expression::parse' cli/audit/src/license_expr.rs; then
  ok
else
  bad "audit license_expr lost its spdx adopted evidence"
fi

# Date calendar adopted: Cargo plus BUILD pin chrono.
if grep -q -F -e 'chrono = ' cli/audit/Cargo.toml \
  && grep -q -F -e '"chrono"' cli/audit/BUILD.bazel \
  && grep -q -F -e 'issue #315' cli/audit/src/exception.rs \
  && grep -q -F -e 'NaiveDate::parse_from_str' cli/audit/src/exception.rs \
  && grep -q -F -e 'is_date_shape' cli/audit/src/exception.rs; then
  ok
else
  bad "audit exception lost its chrono adopted plus shape-gate evidence"
fi

# Scratch adopted: Cargo plus BUILD pin tempfile with direct Builder use.
if grep -q -F -e 'tempfile = "3"' cli/test_scratch/Cargo.toml \
  && grep -q -F -e '"tempfile"' cli/test_scratch/BUILD.bazel \
  && grep -q -F -e 'issue #315' cli/test_scratch/src/lib.rs \
  && grep -q -F -e 'adopted' cli/test_scratch/src/lib.rs \
  && grep -q -F -e 'tempfile::Builder' cli/test_scratch/src/lib.rs \
  && grep -q -F -e 'TempDir' cli/test_scratch/src/lib.rs; then
  ok
else
  bad "test_scratch lost its tempfile adopted evidence"
fi

# Dir sizing adopted: Cargo plus BUILD pin walkdir.
if grep -q -F -e 'walkdir = "2"' cli/clean/Cargo.toml \
  && grep -q -F -e '"walkdir"' cli/clean/BUILD.bazel \
  && grep -q -F -e 'issue #315' cli/clean/src/bytes.rs \
  && grep -q -F -e 'walkdir::WalkDir' cli/clean/src/bytes.rs; then
  ok
else
  bad "clean bytes lost its walkdir adopted evidence"
fi

# Walks adopted: ignore plus globset with fixture evidence.
if grep -q -F -e 'ignore = "0.4"' cli/clean/Cargo.toml \
  && grep -q -F -e 'globset = "0.4"' cli/clean/Cargo.toml \
  && grep -q -F -e '"ignore"' cli/clean/BUILD.bazel \
  && grep -q -F -e '"globset"' cli/clean/BUILD.bazel \
  && grep -q -F -e 'issue #315' cli/clean/src/inventory.rs \
  && grep -q -F -e 'ignore::WalkBuilder' cli/clean/src/inventory.rs \
  && grep -q -F -e 'GlobSetBuilder' cli/clean/src/inventory.rs; then
  ok
else
  bad "clean inventory lost its ignore/globset adopted evidence"
fi

# No hand-rolled stays without a #315 reason record.
missing=""
for f in cli/atomic_fs/src/lib.rs cli/path/src/lib.rs cli/digest/src/lib.rs cli/diff/src/lib.rs cli/lcov/src/lib.rs cli/audit/src/license_expr.rs cli/audit/src/exception.rs cli/test_scratch/src/lib.rs cli/clean/src/bytes.rs cli/clean/src/inventory.rs; do
  if ! grep -q -F -e '#315' "$f"; then
    missing="$missing $f"
  fi
done
if [[ -z "$missing" ]]; then
  ok
else
  bad "helpers missing a #315 decision record:$missing"
fi

# Functional: digest lowercase-only policy still pinned (re-encode check).
if grep -q -F -e 'hex::encode(&bytes) == text' cli/digest/src/lib.rs \
  && grep -q -F -e 'parse_hex(&good.to_uppercase()).is_err()' cli/digest/src/lib.rs; then
  ok
else
  bad "digest lost its lowercase-only re-encode pin"
fi

# Functional: path ladder order still pinned (absolute beats backslash).
if grep -q -F -e 'first problem in ladder order wins' cli/path/src/lib.rs \
  && grep -q -F -e 'Absolute' cli/path/src/lib.rs \
  && grep -q -F -e 'DotDot' cli/path/src/lib.rs; then
  ok
else
  bad "path lost its ladder-order pin"
fi

# Functional: LCOV gate still unions duplicates and requires reason:.
if grep -q -F -e 'maximum' cli/lcov/src/parse.rs \
  && grep -q -F -e 'MissingReason' cli/lcov/src/lib.rs \
  && grep -q -F -e '100%' cli/lcov/src/lib.rs; then
  ok
else
  bad "lcov lost its union plus reason plus 100% verdict pins"
fi

# Functional: SPDX fail-closed plus chrono leap-day evidence stays.
if grep -q -F -e 'LicenseExpr::Unknown' cli/audit/src/license_expr.rs \
  && grep -q -F -e 'leap day valid' cli/audit/src/exception.rs \
  && grep -q -F -e '2028-02-29' cli/audit/src/exception.rs; then
  ok
else
  bad "audit lost its SPDX fail-closed plus leap-day evidence"
fi

# Verification matrix keeps the #315 qualified record with owned gaps.
if grep -q -F -e 'helper_qualification' "$verify" \
  && grep -q -F -e 'qualified seed-only under #315' "$verify" \
  && grep -q -F -e 'upstream' "$verify"; then
  ok
else
  bad "verification-matrix lost its #315 helper qualification record"
fi

echo "helper qualification harness: $pass passed, $fail failed"
[[ "$fail" == "0" ]]
