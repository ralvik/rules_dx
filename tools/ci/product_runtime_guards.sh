#!/usr/bin/env bash
# Product Python/shell to Rust boundary guard.
#
# Product logic migrates to Rust; the toolchain, fixtures, upstream
# linters plus shims, Bazel-imposed shell, and Starlark stay permanently.
# New product py_binary/sh_binary needs a decision before it lands.
#
# This harness machine-checks the boundary statically on a clean tree:
# the accepted record, the direct-target allowlists, the deploy-macro
# rule kinds, the must-stay pins, and the wiring that keeps the gate
# fail-closed. Phases shrink the allowlist slice by slice with their own
# accepted successor records.
#
# Versioned here, run by CI via `bazel run //tools/ci:product_runtime_guards`,
# following //tools/ci:audit_update_guards.
set -euo pipefail

# Shared workspace + runfiles helpers.
# Bootstrap via tools/sh/bootstrap.sh (issue #654): runfiles forest first, then source tree.
source "${RUNFILES_DIR:-/dev/null}/_main/tools/sh/bootstrap.sh" 2>/dev/null || source "${TEST_SRCDIR:-/dev/null}/_main/tools/sh/bootstrap.sh" 2>/dev/null || source "$0.runfiles/_main/tools/sh/bootstrap.sh" 2>/dev/null || source "${BASH_SOURCE[0]}.runfiles/_main/tools/sh/bootstrap.sh" 2>/dev/null || source "$(git rev-parse --show-toplevel 2>/dev/null)/tools/sh/bootstrap.sh"
dx_bootstrap "tools/sh/lib.sh"
dx_bootstrap "tools/sh/guards.sh"

dx_cd_workspace

dx_test_init

# Accepted boundary record owns the migration scope plus phase index.
dx_guards_contains docs/decisions/0026-rust-product-code.md "product boundary record missing (want ADR 0026 Accepted with guard plus phases)" \
  '## Status' \
  'Accepted.' \
  'product_runtime_guards' \
  'archiver/hasher' \
  'preset' \
  'depcheck' \
  'SBOM/BCR' \
  'deploy launcher' \
  'CI-drivers' \
  'ADR 0028'

# Deferred stance is accepted in ADR 0028 (Phase 6): CI drivers stay bash,
# update.py stays Python, opportunistic migration only.
dx_guards_contains docs/decisions/0028-deferred-ci-drivers-update.md "deferred stance record missing (want ADR 0028 Accepted with CI-drivers plus update.py)" \
  '## Status' \
  'Accepted.' \
  'tools/ci' \
  'quality/artifacts:update' \
  '#667' \
  'opportunistically'

# Decision log indexes the boundary plus depcheck plus deferred plus deploy-release successors.
dx_guard_contains docs/decisions/README.md '0026-rust-product-code.md' "decision log lost ADR 0026 (want 0026-rust-product-code.md)"
dx_guard_contains docs/decisions/README.md '0027-depcheck-rust.md' "decision log lost ADR 0027 (want 0027-depcheck-rust.md)"
dx_guard_contains docs/decisions/README.md '0028-deferred-ci-drivers-update.md' "decision log lost ADR 0028 (want 0028-deferred-ci-drivers-update.md)"
dx_guard_contains docs/decisions/README.md '0029-deploy-release-rust.md' "decision log lost ADR 0029 (want 0029-deploy-release-rust.md)"

# Tool matrix links the boundary plus deferred stance instead of restating them.
dx_guards_contains docs/testing/tools.md "tool matrix lost the product boundary link (want ADR 0026 plus ADR 0028 plus product_runtime_guards)" \
  'ADR 0026' \
  'ADR 0028' \
  'product_runtime_guards'

# Direct product py_binary allowlist stays exact (5 targets): hermetic
# npm_packer plus update plus the three must-stay
# linter shims (archiver/hasher delivered Rust under #760, preset
# delivered Rust under #761, SBOM/BCR gens delivered Rust under #763). A new
# product py_binary fails here until its accepted successor updates both
# this row and ADR 0026. Depcheck stays a filegroup run via sh_test, not
# a py_binary, and migrates as its own phase.
py_names="$(grep -h -A1 -e '^[[:space:]]*py_binary(' deploy/rules/BUILD.bazel deploy/release/BUILD.bazel tools/bazelrc/BUILD.bazel quality/artifacts/BUILD.bazel quality/tools/python/BUILD.bazel 2>/dev/null | grep -e 'name = ' | sed -e 's/.*name = //' -e 's/[",]//g' | sort | tr '\n' ' ' | sed -e 's/ $//')"
if [[ "$py_names" == "flake8 npm_packer pydoclint pylint update" ]]; then
  ok
else
  bad "product py_binary allowlist drifted (want npm_packer plus update plus flake8/pylint/pydoclint shims with archiver/hasher plus preset plus sbom/bcr Rust, got: $py_names)"
fi

# Archiver/hasher stay Rust (delivered Phase 1): no return to Python.
if [[ ! -f "deploy/rules/archiver.py" ]] &&
  [[ ! -f "deploy/rules/hasher.py" ]] &&
  grep -q -F -e 'name = "archiver"' deploy/rules/BUILD.bazel &&
  grep -q -F -e 'name = "hasher"' deploy/rules/BUILD.bazel &&
  grep -q -F -e 'rust_binary(' deploy/rules/BUILD.bazel; then
  ok
else
  bad "archiver/hasher Rust delivery regressed (want no archiver.py/hasher.py with rust_binary archiver/hasher)"
fi

# Preset stays Rust (delivered Phase 2): no return to Python.
if [[ ! -f "tools/bazelrc/preset.py" ]] &&
  grep -q -F -e 'name = "preset_update"' tools/bazelrc/BUILD.bazel &&
  grep -q -F -e 'rust_binary(' tools/bazelrc/BUILD.bazel; then
  ok
else
  bad "preset Rust delivery regressed (want no preset.py with rust_binary preset_update)"
fi

# SBOM/BCR gens stay Rust (delivered Phase 4): no return to Python.
if [[ ! -f "deploy/release/sbom_spdx_gen.py" ]] &&
  [[ ! -f "deploy/release/sbom_prov_gen.py" ]] &&
  [[ ! -f "deploy/release/bcr_source_gen.py" ]] &&
  grep -q -F -e 'name = "sbom_spdx_gen"' deploy/release/BUILD.bazel &&
  grep -q -F -e 'name = "sbom_prov_gen"' deploy/release/BUILD.bazel &&
  grep -q -F -e 'name = "bcr_source_gen"' deploy/release/BUILD.bazel &&
  grep -q -F -e 'rust_binary(' deploy/release/BUILD.bazel; then
  ok
else
  bad "sbom/bcr Rust delivery regressed (want no sbom_spdx_gen.py/sbom_prov_gen.py/bcr_source_gen.py with rust_binary sbom/bcr gens)"
fi

# No product py_binary outside the four allowlisted BUILD files (the
# tools/sh entry is the hermetic_grep helper, delivered #1153 and
# recorded in this row). New product code must reuse the pinned tools,
# not add a fifth file, until its accepted successor updates this row
# and ADR 0026.
py_files="$(grep -rl -e '^[[:space:]]*py_binary(' --include='BUILD.bazel' deploy tools quality env 2>/dev/null | sed -e 's|^\./||' | sort | tr '\n' ' ' | sed -e 's/ $//')"
if [[ "$py_files" == "deploy/rules/BUILD.bazel quality/artifacts/BUILD.bazel quality/tools/python/BUILD.bazel tools/sh/BUILD.bazel" ]]; then
  ok
else
  bad "product py_binary file set drifted (want exactly the four allowlisted BUILD files, got: $py_files)"
fi

# Deploy-macro launcher kinds stay py_binary (nine files): each phase
# migrates its macro to rust_binary with its own accepted successor.
py_bzl_fail=""
for f in archive github pypi crates npm nuget maven oci octopus; do
  if ! grep -q -F -e 'py_binary(' "deploy/rules/$f.bzl"; then
    py_bzl_fail="$py_bzl_fail $f:lost-py_binary"
  fi
done
if [[ -z "$py_bzl_fail" ]]; then
  ok
else
  bad "deploy-macro py_binary kinds drifted:$py_bzl_fail"
fi

# Release-macro launcher kinds are Rust (two files): BCR plus signing
# delivered Rust under ADR 0029 (no return to shell).
if grep -q -F -e 'rust_binary(' deploy/release/bcr.bzl &&
  grep -q -F -e 'rust_binary(' deploy/release/signing.bzl &&
  ! grep -q -e '^[[:space:]]*sh_binary(' deploy/release/bcr.bzl &&
  ! grep -q -e '^[[:space:]]*sh_binary(' deploy/release/signing.bzl; then
  ok
else
  bad "release-macro Rust delivery regressed (want rust_binary bcr.bzl plus signing.bzl with no sh_binary per ADR 0029)"
fi

# Direct product sh_binary allowlist stays exact (five targets): POSIX
# fixtures plus the coverage reporter (dx_verify plus release_driver
# delivered Rust under ADR 0029). A new product sh_binary fails here
# until its accepted successor updates both this row and ADR 0026.
sh_names="$(grep -h -A1 -e '^[[:space:]]*sh_binary(' deploy/rules/BUILD.bazel deploy/release/BUILD.bazel deploy/install/BUILD.bazel env/BUILD.bazel tools/coverage/BUILD.bazel 2>/dev/null | grep -e 'name = ' | sed -e 's/.*name = //' -e 's/[",]//g' | sort | tr '\n' ' ' | sed -e 's/ $//')"
if [[ "$sh_names" == "coverage_comment deploy_app deploy_program doctor tool_sh" ]]; then
  ok
else
  bad "product sh_binary allowlist drifted (want deploy fixtures plus tool_sh/doctor plus coverage_comment with dx_verify/release_driver Rust per ADR 0029, got: $sh_names)"
fi

# No product sh_binary outside the three allowlisted BUILD files
# (deploy/install plus deploy/release went Rust under ADR 0029).
sh_files="$(grep -rl -e '^[[:space:]]*sh_binary(' --include='BUILD.bazel' deploy env tools/coverage 2>/dev/null | sed -e 's|^\./||' | sort | tr '\n' ' ' | sed -e 's/ $//')"
if [[ "$sh_files" == "deploy/rules/BUILD.bazel env/BUILD.bazel tools/coverage/BUILD.bazel" ]]; then
  ok
else
  bad "product sh_binary file set drifted (want exactly the three allowlisted BUILD files with install/release Rust per ADR 0029, got: $sh_files)"
fi

# Must-stay linter shims stay py_binary (Rust cannot import the libs).
dx_guards_contains quality/tools/python/BUILD.bazel "linter shims lost their must-stay py_binary (want flake8 plus pylint plus pydoclint)" \
  'name = "flake8"' \
  'name = "pylint"' \
  'name = "pydoclint"' \
  'flake8_main.py' \
  'pylint_main.py' \
  'pydoclint_main.py'
dx_guard_file quality/tools/python/flake8_main.py "must-stay flake8 shim missing (ADR 0026)"
dx_guard_file quality/tools/python/pylint_main.py "must-stay pylint shim missing (ADR 0026)"
dx_guard_file quality/tools/python/pydoclint_main.py "must-stay pydoclint shim missing (ADR 0026)"

# POSIX fixtures stay portable with no Linux constraint.
if ! grep -A4 -e 'name = "tool_sh"' env/BUILD.bazel | grep -q -F -e 'target_compatible_with' &&
  ! grep -A4 -e 'name = "doctor"' env/BUILD.bazel | grep -q -F -e 'target_compatible_with' &&
  ! grep -A6 -e 'name = "deploy_program"' deploy/rules/BUILD.bazel | grep -q -F -e 'target_compatible_with' &&
  ! grep -A6 -e 'name = "deploy_app"' deploy/rules/BUILD.bazel | grep -q -F -e 'target_compatible_with'; then
  ok
else
  bad "a POSIX fixture gained a Linux-only label (must stay portable per ADR 0026)"
fi

# Python toolchain stays (ADR 0010): aspect_rules_py plus rules_python.
dx_guards_contains MODULE.bazel "python toolchain drifted (want aspect_rules_py plus rules_python per ADR 0026)" \
  'aspect_rules_py' \
  'rules_python'

# Depcheck checker is Rust (delivered Phase 3 per ADR 0027): no Python
# sources or shell wrapper tests, rust_binary present. The package's
# sh_tests are lock-consistency proofs (#1077) that invoke the Rust
# binary, not checker wrappers.
if [[ ! -f "tools/depcheck/depcheck.py" ]] &&
  [[ ! -f "tools/depcheck/consistency_test.sh" ]] &&
  [[ ! -f "tools/depcheck/usage_test.sh" ]] &&
  grep -q -F -e 'name = "depcheck_lib"' tools/depcheck/BUILD.bazel &&
  grep -q -F -e 'name = "depcheck"' tools/depcheck/BUILD.bazel &&
  grep -q -F -e 'rust_binary(' tools/depcheck/BUILD.bazel &&
  ! grep -q -e '^[[:space:]]*py_binary(' tools/depcheck/BUILD.bazel; then
  ok
else
  bad "depcheck Rust delivery regressed (want depcheck_lib plus rust_binary depcheck with no Python checker or shell wrapper per ADR 0027)"
fi

# update.py deferral stays py_binary per ADR 0028 until #667 decides otherwise.
dx_guard_contains quality/artifacts/BUILD.bazel 'name = "update"' "update.py deferral lost (want quality/artifacts:update py_binary per ADR 0028)"

# Deploy/release shell is Rust (delivered Phase 5 per ADR 0029): no shell
# launchers, no shell verifies, rust_binary plus portable rust_test present.
if [[ ! -f "deploy/release/bcr_deploy.sh" ]] &&
  [[ ! -f "deploy/release/sign_deploy.sh" ]] &&
  [[ ! -f "deploy/release/release.sh" ]] &&
  [[ ! -f "deploy/release/bcr_verify.sh" ]] &&
  [[ ! -f "deploy/release/signing_verify.sh" ]] &&
  [[ ! -f "deploy/release/release_verify.sh" ]] &&
  [[ ! -f "deploy/release/sbom_verify.sh" ]] &&
  [[ ! -f "deploy/install/dx_verify.sh" ]] &&
  [[ ! -f "deploy/install/dx_verify_test.sh" ]] &&
  grep -q -F -e 'name = "release_driver"' deploy/release/BUILD.bazel &&
  grep -q -F -e 'name = "dx_verify"' deploy/install/BUILD.bazel &&
  grep -q -F -e 'rust_binary(' deploy/install/BUILD.bazel &&
  ! grep -q -e '^[[:space:]]*sh_binary(' deploy/install/BUILD.bazel &&
  ! grep -q -e '^[[:space:]]*sh_test(' deploy/install/BUILD.bazel &&
  ! grep -q -e '^[[:space:]]*sh_test(' deploy/release/BUILD.bazel; then
  ok
else
  bad "deploy/release Rust delivery regressed (want no shell launchers/verifies with rust_binary release_driver/dx_verify plus portable rust_test per ADR 0029)"
fi

# CI drivers stay shell here per ADR 0028 (harness-wide migration owned by #667, not this umbrella).
if [[ "$(ls tools/ci/*.sh 2>/dev/null | wc -l)" -ge 119 ]]; then
  ok
else
  bad "tools/ci driver count dropped below 119 (CI-driver stance defers to #667 per ADR 0028, not this umbrella)"
fi

# Wiring: BUILD owns the harness target.
dx_guard_contains tools/ci/ci_targets_b.bzl 'name = "product_runtime_guards"' "tools/ci wiring lost product_runtime_guards (want ci_targets_b.bzl target)"

# Wiring: dogfood-freshness runs the gate.
dx_guard_contains tools/ci/dogfood_freshness.sh '//tools/ci:product_runtime_guards' "CI lost the product gate (want dogfood_freshness product_runtime_guards)"

dx_test_summary "product runtime guards harness"
