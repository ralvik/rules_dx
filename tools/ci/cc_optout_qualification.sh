#!/usr/bin/env bash
# CC opt-out linker qualification harness.
#
# The kept `use_cc_toolchain = False` opt-out had no owner after 
# closed: the native plan recorded a source-derived no-linker failure path
# from the rules_rs pinned patched toolchain with no standalone linker.
# This harness pins the decided outcome on the as-built stack (stock
# rules_rust 0.74.0, Linux x86_64 seed host):
# - decided: the kept opt-out executes successfully for pure-Rust scripts.
#   The execution action carries no C++ toolchain inputs (CC/CXX/AR point
#   at the upstream no_cc/no_cxx/no_ar stubs, CFLAGS/CXXFLAGS/LDFLAGS are
#   empty) while LD falls back to the sysroot rust-lld. The script binary
#   itself still compiles through the normal Rust toolchain; opt-out
#   removes only execution-action CC inputs, never the compilation linker.
# - fixtures: `rust/tests/fixtures/cc_optout/` (pure-Rust build.rs stamping
#   a cfg, `cc_optout` lib plus `cc_optout_test` under `bazel test //...`,
#   kept `use_cc_toolchain = 0  # keep` surviving `dx generate --check`).
# - scope: pure-Rust scripts only. Scripts needing CC still fail clearly
# when opted out; bindgen/CXX/exact-target stay owned under -;
#   platform plus consumer plus release evidence stays open; no Supported
#   claim. Silent kept opt-out stays rejected: the opt-out is explicit,
#   kept, and fixture-proven here.
#
# Versioned here, run by CI via `bazel run //tools/ci:cc_optout_qualification`,
# following //tools/ci:rustfmt_edition_qualification.
set -euo pipefail

# Shared workspace + runfiles helpers.
# Bootstrap via tools/sh/bootstrap.sh (issue #654): runfiles forest first, then source tree.
source "${RUNFILES_DIR:-/dev/null}/_main/tools/sh/bootstrap.sh" 2>/dev/null || source "${TEST_SRCDIR:-/dev/null}/_main/tools/sh/bootstrap.sh" 2>/dev/null || source "$0.runfiles/_main/tools/sh/bootstrap.sh" 2>/dev/null || source "${BASH_SOURCE[0]}.runfiles/_main/tools/sh/bootstrap.sh" 2>/dev/null || source "$(git rev-parse --show-toplevel 2>/dev/null)/tools/sh/bootstrap.sh"
dx_bootstrap "tools/sh/lib.sh"

dx_cd_workspace

dx_test_init

roadmap="docs/roadmap.md"
verify="docs/testing/verification-matrix.md"
verify_remaining="docs/testing/verification-matrix-remaining.md"
gen_rust="docs/generation/rust.md"
gen_readme="docs/generation/README.md"
native="docs/native-toolchains.md"
matrix="docs/product/support-matrix.md"
ci=".github/workflows/ci.yml"
build="tools/ci/BUILD.bazel"
fixture_build="rust/tests/fixtures/cc_optout/BUILD.bazel"
fixture_script="rust/tests/fixtures/cc_optout/build.rs"
fixture_lib="rust/tests/fixtures/cc_optout/src/lib.rs"
fixture_manifest="rust/tests/fixtures/cc_optout/Cargo.toml"

# Roadmap owns the Seed-host-delivered history record under Cleanup-completed.
if grep -q -F -e 'CC opt-out linker Seed-host-delivered (closed #471' "$roadmap"; then
  ok
else
  bad "roadmap lost its CC opt-out Seed-host-delivered record under closed #471"
fi

# Remaining matrix owns the qualified seed-only record under.
if grep -q -F -e 'cc_optout_qualification' "$verify_remaining" &&
  grep -q -F -e 'qualified seed-only under #471' "$verify_remaining" &&
  grep -q -F -e 'bazel run //tools/ci:cc_optout_qualification' "$verify_remaining"; then
  ok
else
  bad "verification-matrix-remaining lost its #471 CC opt-out qualified record"
fi

# Verification matrix lists the harness in dogfood-freshness.
if grep -q -F -e ':cc_optout_qualification' "$verify"; then
  ok
else
  bad "verification-matrix dogfood-freshness lost :cc_optout_qualification"
fi

# Verification matrix Green lists the harness count.
if grep -q -F -e '`cc_optout_qualification` 18/18' "$verify"; then
  ok
else
  bad "verification-matrix Green lost cc_optout_qualification 18/18"
fi

# BUILD owns the harness target.
if grep -q -F -e 'name = "cc_optout_qualification"' "$build"; then
  ok
else
  bad "tools/ci/BUILD.bazel lost the cc_optout_qualification target"
fi

# CI wires the harness in dogfood-freshness.
if grep -q -F -e 'bazel run --noshow_progress //tools/ci:cc_optout_qualification' "$ci"; then
  ok
else
  bad "ci.yml lost the cc_optout_qualification step (want dogfood-freshness)"
fi

# Fixture files stay present (handwritten pure-Rust script plus lib plus manifest).
if [[ -f "$fixture_build" && -f "$fixture_script" && -f "$fixture_lib" && -f "$fixture_manifest" ]]; then
  ok
else
  bad "cc_optout fixture lost files (want BUILD.bazel plus build.rs plus src/lib.rs plus Cargo.toml)"
fi

# Fixture keeps the explicit opt-out with # keep plus hermetic defaults.
if grep -q -F -e 'use_cc_toolchain = 0,  # keep' "$fixture_build" &&
  grep -q -F -e 'use_default_shell_env = 0' "$fixture_build" &&
  grep -q -F -e 'emit_warnings = True' "$fixture_build"; then
  ok
else
  bad "cc_optout fixture lost its kept opt-out (want use_cc_toolchain = 0 with # keep plus shell-env 0 plus warnings True)"
fi

# Fixture script stays pure-Rust (no cc tool use, stamps a cfg).
if grep -q -F -e 'cargo:rustc-cfg=has_cc_optout_stamp' "$fixture_script" &&
  ! grep -q -F -e 'cc::' "$fixture_script"; then
  ok
else
  bad "cc_optout build.rs lost its pure-Rust stamp (want rustc-cfg without cc::)"
fi

# Generation contract owns the opt-out failure-path decision under.
if grep -q -F -e 'issue #471' "$gen_rust" &&
  grep -q -F -e 'script execution action' "$gen_rust" &&
  grep -q -F -e 'no_cc' "$gen_rust"; then
  ok
else
  bad "generation/rust.md lost its #471 opt-out execution-action record with no_cc"
fi

# Generation README owns the pinned opt-out record.
if grep -q -F -e 'CC opt-out' "$gen_readme" &&
  grep -q -F -e 'issue #471' "$gen_readme"; then
  ok
else
  bad "generation README lost its #471 CC opt-out pinned record"
fi

# Native plan owns the qualification (failure path closed, corpus cites fixture).
if grep -q -F -e '#471' "$native" &&
  grep -q -F -e 'Can the kept CC opt-out execute successfully?' "$native" &&
  grep -q -F -e 'cc_optout' "$native"; then
  ok
else
  bad "native-toolchains lost its #471 opt-out qualification with cc_optout fixture"
fi

# Support matrix keeps the CC opt-out line owned under with no Supported claim.
if grep -q -F -e 'kept CC opt-out linker' "$matrix" &&
  grep -q -F -e '#471' "$matrix" &&
  ! grep -q -E -e '^\| .* \| Supported' "$matrix"; then
  ok
else
  bad "support-matrix lost its owned #471 CC opt-out line (want kept opt-out plus #471, no Supported)"
fi

# Gazelle default stays hermetic on (opt-out is explicit keep only).
if grep -q -F -e 'use_cc_toolchain", 1' gazelle/rust/lang.go &&
  grep -q -F -e 'use_cc_toolchain' gazelle/rust/lang_test.go; then
  ok
else
  bad "Gazelle lost its use_cc_toolchain default-on proof (want lang.go 1 plus lang_test.go)"
fi

# Live proof: the kept opt-out builds (script execution plus lib).
if bazel build //rust/tests/fixtures/cc_optout:cc_optout_build_script //rust/tests/fixtures/cc_optout:cc_optout --noshow_progress >/dev/null 2>&1; then
  ok
else
  bad "cc_optout fixture failed to build (want script plus lib green with kept opt-out)"
fi

# Live proof: the stamped crate test passes (cfg reached the lib).
if bazel test //rust/tests/fixtures/cc_optout:cc_optout_test --noshow_progress >/dev/null 2>&1; then
  ok
else
  bad "cc_optout_test failed (want stamped cfg test green)"
fi

# Live proof: the execution action carries no CC inputs (stubs plus empty flags, sysroot LD).
optout_env="$(bazel aquery 'mnemonic(CargoBuildScriptRun, //rust/tests/fixtures/cc_optout/...)' --output=text --noshow_progress 2>/dev/null || true)"
if [[ "$optout_env" == *"no_cc"* && "$optout_env" == *"no_cxx"* && "$optout_env" == *"no_ar"* &&
  "$optout_env" == *"rust-lld"* && "$optout_env" == *"CFLAGS="* && "$optout_env" == *"LDFLAGS="* ]]; then
  ok
else
  bad "opt-out execution action lost its no-CC markers (want no_cc plus no_cxx plus no_ar plus rust-lld plus empty CFLAGS/LDFLAGS)"
fi

# Generation stability: the kept opt-out survives `dx generate --check`.
if bazel run //cli/cli:dx -- generate --check //rust/tests/fixtures/cc_optout/... >/dev/null 2>&1; then
  ok
else
  bad "dx generate --check failed on cc_optout (want kept opt-out stable)"
fi

dx_test_summary "cc optout qualification harness"
