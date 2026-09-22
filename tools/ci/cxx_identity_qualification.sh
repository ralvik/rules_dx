#!/usr/bin/env bash
# CXX graph identity qualification harness.
#
# The CXX generator had no owner after closed: the native plan recorded
# that CXX's Bazel module registers direct rules_rust toolchains, risking a
# second Rust graph beside the single crate_universe `crates` graph.
# This harness pins the decided outcome on the as-built stack (stock
# rules_rust 0.74.0, Linux x86_64 seed host):
# - decided: single-graph identity. The `cxx` library crate and the
#   `cxxbridge-cmd` codegen binary resolve from the single crate_universe
#   `crates` graph at identical versions (`cxx == cxxbridge-cmd == 1.0.200`,
#   the inspected CXX release). The generator tool is
#   `@crates//:cxxbridge-cmd` on the exec platform, never `@cxx.rs//:codegen`.
#   No `cxx.rs` Bazel module lands in MODULE.bazel: its direct rules_rust
#   0.74.0 plus toolchain 1.98.1 plus `crates.io`/`vendor` repos would be a
#   second Rust graph. Ad-hoc identity (mixed versions, separate codegen
#   repo) stays rejected.
# - fixtures: `rust/tests/fixtures/cxx_identity/` (version pin in
#   `cxx_bridge.bzl` plus `Cargo.toml` metadata, `cxx_identity` lib plus
#   `cxx_identity_test` plus `bridge` cc_library plus `bridge_test` under
#   `bazel test //...`, versioned alias surviving `dx generate --check`).
# - scope: graph only. The pin lives under `[package.metadata]` until
#   MODULE.bazel wires the manifest into crate_universe (an unwired
#   `crate_deps(["cxx"])` fails analysis); full `cxxbridge-cmd` execution
# plus corpus wiring stays under; platform plus consumer plus release
#   evidence stays open; no Supported claim.
#
# Versioned here, run by CI via `bazel run //tools/ci:cxx_identity_qualification`,
# following //tools/ci:cc_optout_qualification.
set -euo pipefail

# Shared workspace + runfiles helpers.
# Bootstrap via tools/sh/bootstrap.sh (issue #654): runfiles forest first, then source tree.
source "${RUNFILES_DIR:-/dev/null}/_main/tools/sh/bootstrap.sh" 2>/dev/null || source "${TEST_SRCDIR:-/dev/null}/_main/tools/sh/bootstrap.sh" 2>/dev/null || source "$0.runfiles/_main/tools/sh/bootstrap.sh" 2>/dev/null || source "${BASH_SOURCE[0]}.runfiles/_main/tools/sh/bootstrap.sh" 2>/dev/null || source "$(git rev-parse --show-toplevel 2>/dev/null)/tools/sh/bootstrap.sh"
dx_bootstrap "tools/sh/lib.sh"

dx_cd_workspace

dx_test_init

verify="docs/testing/verification-matrix.md"
verify_remaining="docs/testing/verification-matrix-remaining.md"
gen_rust="docs/generation/rust.md"
gen_readme="docs/generation/README.md"
native="docs/native-toolchains.md"
matrix="docs/product/support-matrix.md"
ci=".github/workflows/ci.yml"
build="tools/ci/BUILD.bazel"
module="MODULE.bazel"
fixture_build="rust/tests/fixtures/cxx_identity/BUILD.bazel"
fixture_bridge="rust/tests/fixtures/cxx_identity/cxx_bridge.bzl"
fixture_manifest="rust/tests/fixtures/cxx_identity/Cargo.toml"
fixture_lib="rust/tests/fixtures/cxx_identity/src/lib.rs"

# Planned work lives in GitHub issues only (docs/roadmap.md removed under #981).
if [[ ! -f "docs/roadmap.md" ]]; then
  ok
else
  bad "docs/roadmap.md still exists (planned work lives in GitHub issues only, #981)"
fi

# Remaining matrix owns the qualified seed-only record under.
if grep -q -F -e 'cxx_identity_qualification' "$verify_remaining" &&
  grep -q -F -e 'qualified seed-only under #474' "$verify_remaining" &&
  grep -q -F -e 'bazel run //tools/ci:cxx_identity_qualification' "$verify_remaining"; then
  ok
else
  bad "verification-matrix-remaining lost its #474 CXX identity qualified record"
fi

# Verification matrix lists the harness in dogfood-freshness.
if grep -q -F -e ':cxx_identity_qualification' "$verify"; then
  ok
else
  bad "verification-matrix dogfood-freshness lost :cxx_identity_qualification"
fi

# Verification matrix Green lists the harness count.
if grep -q -F -e '`cxx_identity_qualification` 18/18' "$verify"; then
  ok
else
  bad "verification-matrix Green lost cxx_identity_qualification 18/18"
fi

# BUILD owns the harness target.
if grep -q -F -e 'name = "cxx_identity_qualification"' "$build"; then
  ok
else
  bad "tools/ci/BUILD.bazel lost the cxx_identity_qualification target"
fi

# CI wires the harness in dogfood-freshness.
if grep -q -F -e 'bazel run --noshow_progress //tools/ci:cxx_identity_qualification' "$ci"; then
  ok
else
  bad "ci.yml lost the cxx_identity_qualification step (want dogfood-freshness)"
fi

# Fixture files stay present (identity Starlark plus manifest plus lib plus C++ stubs).
if [[ -f "$fixture_build" && -f "$fixture_bridge" && -f "$fixture_manifest" && -f "$fixture_lib" &&
  -f "rust/tests/fixtures/cxx_identity/bridge.cc" && -f "rust/tests/fixtures/cxx_identity/bridge.h" &&
  -f "rust/tests/fixtures/cxx_identity/bridge_test.cc" ]]; then
  ok
else
  bad "cxx_identity fixture lost files (want BUILD.bazel plus cxx_bridge.bzl plus Cargo.toml plus src/lib.rs plus bridge stubs)"
fi

# Identity Starlark pins identical versions with the single-graph tool and records the rejection.
if grep -q -F -e 'CXX_VERSION = "1.0.200"' "$fixture_bridge" &&
  grep -q -F -e 'CXXBRIDGE_CMD_VERSION = "1.0.200"' "$fixture_bridge" &&
  grep -q -F -e '@crates//:cxxbridge-cmd' "$fixture_bridge" &&
  grep -q -F -e 'rejected' "$fixture_bridge"; then
  ok
else
  bad "cxx_bridge.bzl lost its single-graph identity (want identical 1.0.200 pins plus @crates//:cxxbridge-cmd with recorded rejection)"
fi

# Cargo manifest pins the same identical versions under metadata (unwired until).
if grep -q -F -e '[package.metadata.cxx-identity]' "$fixture_manifest" &&
  grep -q -F -e 'cxx = "=1.0.200"' "$fixture_manifest" &&
  grep -q -F -e 'cxxbridge-cmd = "=1.0.200"' "$fixture_manifest"; then
  ok
else
  bad "cxx_identity Cargo.toml lost its metadata identity pin (want cxx plus cxxbridge-cmd at =1.0.200)"
fi

# Fixture BUILD composes the Rust plus C++ sides through the wrapper contracts.
if grep -q -F -e 'load(":cxx_bridge.bzl", "CXX_VERSION")' "$fixture_build" &&
  grep -q -F -e 'name = "cxx_identity"' "$fixture_build" &&
  grep -q -F -e 'name = "cxx_identity_test"' "$fixture_build" &&
  grep -q -F -e 'name = "bridge"' "$fixture_build" &&
  grep -q -F -e 'name = "bridge_test"' "$fixture_build"; then
  ok
else
  bad "cxx_identity BUILD.bazel lost its bridge composition (want cxx_bridge load plus cxx_identity plus bridge plus both tests)"
fi

# MODULE.bazel keeps the single Rust graph (no cxx.rs second graph, one rust_toolchains).
if ! grep -q -F -e 'cxx.rs' "$module" &&
  [[ "$(grep -c -F -e 'use_repo(rust, "rust_toolchains")' "$module")" == "1" ]]; then
  ok
else
  bad "MODULE.bazel gained a second Rust graph (want no cxx.rs plus exactly one rust_toolchains repo)"
fi

# Generation contract owns the CXX identity decision under.
if grep -q -F -e 'CXX graph identity (decided under issue #474' "$gen_rust" &&
  grep -q -F -e '@crates//:cxxbridge-cmd' "$gen_rust" &&
  grep -q -F -e 'never `@cxx.rs//:codegen`' "$gen_rust"; then
  ok
else
  bad "generation/rust.md lost its #474 CXX single-graph record with @crates//:cxxbridge-cmd"
fi

# Generation README owns the pinned identity record.
if grep -q -F -e 'CXX graph identity' "$gen_readme" &&
  grep -q -F -e 'issue #474' "$gen_readme"; then
  ok
else
  bad "generation README lost its #474 CXX identity pinned record"
fi

# Native plan owns the qualification (identity decided, corpus cites fixture).
if grep -q -F -e '#474' "$native" &&
  grep -q -F -e 'CXX graph identity' "$native" &&
  grep -q -F -e 'cxx_identity_qualification' "$native"; then
  ok
else
  bad "native-toolchains lost its #474 CXX identity qualification with cxx_identity fixture"
fi

# Support matrix keeps the CXX identity line decided under with no Supported claim.
if grep -q -F -e 'CXX graph identity decided' "$matrix" &&
  grep -q -F -e '#474' "$matrix" &&
  ! grep -q -E -e '^\| .* \| Supported' "$matrix"; then
  ok
else
  bad "support-matrix lost its decided #474 CXX identity line (want decided plus #474, no Supported)"
fi

# Live proof: the bridge composition builds (Rust lib plus C++ lib).
if bazel build //rust/tests/fixtures/cxx_identity:cxx_identity //rust/tests/fixtures/cxx_identity:bridge --noshow_progress >/dev/null 2>&1; then
  ok
else
  bad "cxx_identity fixture failed to build (want Rust lib plus C++ bridge green on the single graph)"
fi

# Live proof: both sides test green (Rust unit plus C++ bridge).
if bazel test //rust/tests/fixtures/cxx_identity:cxx_identity_test //rust/tests/fixtures/cxx_identity:bridge_test --noshow_progress >/dev/null 2>&1; then
  ok
else
  bad "cxx_identity tests failed (want Rust plus C++ bridge tests green)"
fi

# Generation stability: the fixture survives `dx generate --check`.
if bazel run //cli/cli:dx -- generate --check //rust/tests/fixtures/cxx_identity/... >/dev/null 2>&1; then
  ok
else
  bad "dx generate --check failed on cxx_identity (want single-graph fixture stable)"
fi

dx_test_summary "cxx identity qualification harness"
