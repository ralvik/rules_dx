#!/usr/bin/env bash
# Exact-target discovery qualification harness (issue #475).
#
# Qualifies the owned gap from support-matrix 133-142: target resolution
# is the all-direct-owners query strategy but the exact-target path was
# unowned. Query-only without a contract stays rejected.
# - decided: resolver-owned exact labels flow to the upstream target
#   interfaces, preserving exact context. Files resolve through one
#   unconfigured `bazel query` (`kind('rule', rdeps(//..., set(...), 1))`,
#   depth exactly 1, deterministic sorted set, nearest enclosing package
#   file labels); dirs become recursive `//path/...` patterns without
#   filesystem enumeration; explicit labels pass through untouched with
#   no query. Those exact owners are the discovery input to upstream
#   `gen_rust_project`/`flycheck` TARGETS, never raw paths to the
#   dynamic Path/Buildfile discovery that widens to `//pkg:all`.
# - upstream shape: `RustAnalyzerArg::Path/Buildfile` plus
#   `buildfile_to_targets` (`//pkg:all`, root `//...`) in
#   `tools/rust_analyzer/rust_project.rs` at pinned rules_rs v0.0.109;
#   generation/flycheck keep the exact TARGETS interfaces proven by
#   //rust/ide:ide_acquisition_test. Project-owned crate graph plus
#   internal RustAnalyzerInfo stay rejected as discovery substitutes.
# - fixtures: `rust/tests/fixtures/discovery/pins.bzl` pins the
#   identities; `rust/tests/fixtures/hello` proves exact isolation
#   (`src/lib.rs` to `hello_lib` only, `src/main.rs` to `hello` only);
#   resolver FakeQuery fixtures pin the query shape; `rust/env` plans
#   stay provider-derived focused-target plans, not exact-target proof.
# - open owned gaps: platform plus consumer plus release evidence, no
#   `Supported` claim. C++ action-derived snapshot stays open proof,
#   not claimed here. Compatibility is resolution only.
#
# Versioned here, run by CI via `bazel run //tools/ci:exact_target_qualification`,
# following //tools/ci:bindgen_qualification.
set -euo pipefail

# Shared workspace + runfiles helpers (issue #319).
# Bootstrap: Bazel runfiles forest first (`data = ["//tools/sh:lib"]`), then source tree.
source "${RUNFILES_DIR:-/dev/null}/_main/tools/sh/lib.sh" 2>/dev/null || source "${TEST_SRCDIR:-/dev/null}/_main/tools/sh/lib.sh" 2>/dev/null || source "$0.runfiles/_main/tools/sh/lib.sh" 2>/dev/null || source "${BASH_SOURCE[0]}.runfiles/_main/tools/sh/lib.sh" 2>/dev/null || source "$(dirname "${BASH_SOURCE[0]}")/../sh/lib.sh"

dx_cd_workspace

dx_test_init

pins="rust/tests/fixtures/discovery/pins.bzl"
fixture_build="rust/tests/fixtures/discovery/BUILD.bazel"
contract="docs/cli/target-resolution.md"
native="docs/native-toolchains.md"
matrix="docs/product/support-matrix.md"
rust_env="docs/environments/rust.md"
env_readme="docs/environments/README.md"
gen_readme="docs/generation/README.md"
build="tools/ci/BUILD.bazel"
ci=".github/workflows/ci.yml"
verify="docs/testing/verification-matrix.md"
classify="cli/cli/src/resolve/classify.rs"
query="cli/cli/src/resolve/query.rs"
entry="cli/cli/src/resolve/entry.rs"
ide_test="rust/ide/ide_acquisition_test.sh"

# Fixture pair plus pins stay present (issue #475).
if [[ -f "$pins" && -f "$fixture_build" ]]; then
  ok
else
  bad "discovery fixture missing (want $pins plus $fixture_build)"
fi

# Pins record the upstream discovery identities plus rejected substitutes.
if grep -q -F -e 'RULES_RS_VERSION = "v0.0.109"' "$pins" &&
  grep -q -F -e 'b55b132af0c9951807c926768e40222330348632' "$pins" &&
  grep -q -F -e 'tools/rust_analyzer/rust_project.rs' "$pins" &&
  grep -q -F -e '"Path"' "$pins" &&
  grep -q -F -e '"Buildfile"' "$pins" &&
  grep -q -F -e '//pkg:all' "$pins" &&
  grep -q -F -e 'gen_rust_project' "$pins" &&
  grep -q -F -e 'flycheck' "$pins" &&
  grep -q -F -e 'query-only without contract' "$pins" &&
  grep -q -F -e 'project-owned crate graph' "$pins" &&
  grep -q -F -e 'RustAnalyzerInfo' "$pins"; then
  ok
else
  bad "pins.bzl lost its upstream discovery identities plus rejected substitutes under issue #475"
fi

# Resolver keeps the all-direct-owners query shape (no cquery, no aspect).
if grep -q -F -e "kind('rule', rdeps(//..., set(" "$classify" &&
  grep -q -F -e 'rdeps(//..., set(' "$query" &&
  grep -q -F -e 'ownership_set_expression' "$query" &&
  grep -q -F -e 'quote_set' "$query" &&
  ! grep -q -F -e 'cquery' "$classify" &&
  ! grep -q -F -e 'cquery' "$query"; then
  ok
else
  bad "cli resolve lost its all-direct-owners unconfigured query shape (want rdeps depth-1 plus deterministic set, no cquery)"
fi

# Resolver keeps exact-target classification: labels pass through, dirs
# become recursive patterns, files use the nearest enclosing package.
if grep -q -F -e 'dir_pattern' "$classify" &&
  grep -q -F -e '//...".to_owned()' "$classify" &&
  grep -q -F -e 'file_label' "$classify" &&
  grep -q -F -e 'labels.push' "$classify" &&
  grep -q -F -e 'ResolvedOwners' "$entry" &&
  grep -q -F -e 'resolve_for_test' "$entry"; then
  ok
else
  bad "cli resolve lost its exact-target classification (want label pass-through plus dir patterns plus nearest-package file labels plus ResolvedOwners)"
fi

# Contract doc owns the exact-target discovery section with fixtures.
if grep -q -F -e '## Exact-Target Discovery' "$contract" &&
  grep -q -F -e 'query-only without contract' "$contract" &&
  grep -q -F -e 'gen_rust_project' "$contract" &&
  grep -q -F -e 'pins.bzl' "$contract" &&
  grep -q -F -e 'hello_lib' "$contract" &&
  grep -q -F -e 'RustAnalyzerInfo' "$contract"; then
  ok
else
  bad "docs/cli/target-resolution.md lost its Exact-Target Discovery contract with fixtures under issue #475"
fi

# Native plan owns the qualified discovery record plus the question row.
if grep -q -F -e 'qualified seed-only under issue #475' "$native" &&
  grep -q -F -e 'exact_target_qualification' "$native" &&
  grep -q -F -e 'rust/tests/fixtures/discovery/pins.bzl' "$native" &&
  grep -q -F -e 'Can IDE setup preserve exact context' "$native" &&
  grep -q -F -e 'issue #475' "$native" &&
  grep -q -F -e 'rust_project.rs' "$native"; then
  ok
else
  bad "docs/native-toolchains.md lost its qualified exact-target record with fixtures plus pins under issue #475"
fi

# Support matrix keeps the gap owned with the qualified wording.
if grep -q -F -e 'exact-target discovery' "$matrix" &&
  grep -q -F -e 'qualified seed-only under issue #475' "$matrix" &&
  grep -q -F -e '#475' "$matrix" &&
  grep -q -F -e 'exact_target_qualification' "$matrix"; then
  ok
else
  bad "docs/product/support-matrix.md lost its qualified exact-target gap wording under issue #475"
fi

# Rust environment keeps the focused vs exact-target distinction.
if grep -q -F -e 'exact-target' "$rust_env" &&
  grep -q -F -e 'issue #475' "$rust_env" &&
  grep -q -F -e 'exact_target_qualification' "$rust_env"; then
  ok
else
  bad "docs/environments/rust.md lost its exact-target discovery record under issue #475"
fi

# Environments README pins the qualified discovery alongside the plan pin.
if grep -q -F -e 'issue #475' "$env_readme" &&
  grep -q -F -e 'exact-target discovery' "$env_readme" &&
  grep -q -F -e 'exact_target_qualification' "$env_readme"; then
  ok
else
  bad "docs/environments/README.md lost its qualified exact-target record under issue #475"
fi

# Generation README pins the qualified discovery alongside the other gaps.
if grep -q -F -e 'issue #475' "$gen_readme" &&
  grep -q -F -e 'exact-target discovery' "$gen_readme" &&
  grep -q -F -e 'exact_target_qualification' "$gen_readme"; then
  ok
else
  bad "docs/generation/README.md lost its qualified exact-target record under issue #475"
fi

# IDE acquisition still proves the TARGETS interfaces (exact flow endpoint).
if grep -q -F -e 'TARGETS' "$ide_test" &&
  grep -q -F -e 'gen_rust_project' "$ide_test" &&
  grep -q -F -e 'flycheck' "$ide_test"; then
  ok
else
  bad "rust/ide/ide_acquisition_test.sh lost its TARGETS plus flycheck exact-interface proof"
fi

# BUILD owns the harness target.
if grep -q -F -e 'name = "exact_target_qualification"' "$build"; then
  ok
else
  bad "tools/ci/BUILD.bazel lost the exact_target_qualification target"
fi

# CI wires the harness in dogfood-freshness.
if grep -q -F -e 'bazel run --noshow_progress //tools/ci:exact_target_qualification' "$ci"; then
  ok
else
  bad "ci.yml lost the exact_target_qualification step (want dogfood-freshness)"
fi

# Verification matrix owns the qualified seed-only record under #475.
if grep -q -F -e 'exact_target_qualification' "$verify" &&
  grep -q -F -e 'qualified seed-only under #475' "$verify" &&
  grep -q -F -e 'bazel run //tools/ci:exact_target_qualification' "$verify" &&
  grep -q -F -e '`exact_target_qualification` 16/16' "$verify"; then
  ok
else
  bad "verification-matrix lost its #475 exact-target qualified record"
fi

# Live proof: exact isolation holds (lib vs bin), not package-wide :all.
lib_owners="$(bazel query "kind('rule', rdeps(//rust/tests/fixtures/hello/..., set(//rust/tests/fixtures/hello:src/lib.rs), 1))" 2>/dev/null || true)"
bin_owners="$(bazel query "kind('rule', rdeps(//rust/tests/fixtures/hello/..., set(//rust/tests/fixtures/hello:src/main.rs), 1))" 2>/dev/null || true)"
if echo "$lib_owners" | grep -q -F -e '//rust/tests/fixtures/hello:hello_lib' &&
  ! echo "$lib_owners" | grep -q -F -e '//rust/tests/fixtures/hello:hello$' &&
  echo "$bin_owners" | grep -q -F -e '//rust/tests/fixtures/hello:hello' &&
  ! echo "$bin_owners" | grep -q -F -e '//rust/tests/fixtures/hello:hello_lib'; then
  ok
else
  bad "exact isolation failed (want src/lib.rs to hello_lib only plus src/main.rs to hello only, not :all widening)"
fi

# Live proof: IDE target interfaces are acquirable (gen_rust_project TARGETS plus flycheck).
if bazel test //rust/ide:ide_acquisition_test --noshow_progress >/dev/null 2>&1; then
  ok
else
  bad "ide_acquisition_test failed (want gen_rust_project TARGETS plus flycheck green)"
fi

dx_test_summary "exact-target discovery qualification harness"
