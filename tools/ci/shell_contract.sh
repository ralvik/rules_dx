#!/usr/bin/env bash
# Shell portability contract guard (issue #299).
#
# The CI/test harness is bash-only on the Linux seed host: every bash
# `sh_binary`/`sh_test` carries
# `target_compatible_with = ["@platforms//os:linux"]`, POSIX `#!/bin/sh`
# fixtures stay portable with no constraint, and the known non-portable
# forms carry macOS best-effort fallbacks with no Linux behavior change
# (realpath probe, shasum fallback, portable sed/cp, portable timing).
# Windows stays out per ADR 0014 (backend-blocked); product runtime is
# Rust and shell-free except generated deploy launchers plus the doctor shim.
#
# This harness machine-checks the contract statically on a clean tree.
# Versioned here, run by CI via `bazel run //tools/ci:shell_contract`.
set -euo pipefail

# Shared workspace + runfiles helpers (issue #319).
# Bootstrap: Bazel runfiles forest first (`data = ["//tools/sh:lib"]`), then source tree.
source "${RUNFILES_DIR:-/dev/null}/_main/tools/sh/lib.sh" 2>/dev/null || source "${TEST_SRCDIR:-/dev/null}/_main/tools/sh/lib.sh" 2>/dev/null || source "$0.runfiles/_main/tools/sh/lib.sh" 2>/dev/null || source "${BASH_SOURCE[0]}.runfiles/_main/tools/sh/lib.sh" 2>/dev/null || source "$(dirname "${BASH_SOURCE[0]}")/../sh/lib.sh"

dx_cd_workspace

dx_test_init

# Docs record the decided contract, not an open tracker.
if grep -q -F -e 'is bash-only on the Linux seed host (decided' docs/testing/tools.md &&
  grep -q -F -e '//tools/ci:shell_contract' docs/testing/tools.md &&
  grep -q -F -e 'target_compatible_with = ["@platforms//os:linux"]' docs/testing/tools.md; then
  ok
else
  bad "docs/testing/tools.md lost the decided #299 shell contract record"
fi

# Every bash sh target carries the Linux-only label (67 bash targets;
# 6 POSIX fixtures stay portable). Count labels vs bash sh targets.
labels="$(grep -r -F -e 'target_compatible_with = ["@platforms//os:linux"]' --include='BUILD.bazel' --exclude-dir='bazel-*' --exclude-dir='.git' . | wc -l)"
# 67 bash sh_* + devcontainer parity + e2e test_suite = 69 label sites.
if [[ "$labels" -ge 69 ]]; then
  ok
else
  bad "want >= 69 Linux-only labels, found $labels"
fi

# POSIX fixtures stay portable: no Linux constraint in their packages'
# POSIX-only targets (env tool/doctor, integration pass/fail).
if ! grep -A4 -e 'name = "tool_sh"' env/BUILD.bazel | grep -q -F -e 'target_compatible_with' &&
  ! grep -A4 -e 'name = "doctor"' env/BUILD.bazel | grep -q -F -e 'target_compatible_with' &&
  ! grep -A4 -e 'name = "pass"' integration/clean/BUILD.bazel | grep -q -F -e 'target_compatible_with' &&
  ! grep -A4 -e 'name = "fail"' integration/dirty/BUILD.bazel | grep -q -F -e 'target_compatible_with'; then
  ok
else
  bad "a POSIX fixture gained a Linux-only label (must stay portable)"
fi

# Deploy POSIX fixtures stay portable while bash verifiers stay Linux-only.
if ! grep -A6 -e 'name = "deploy_program"' deploy/rules/BUILD.bazel | grep -q -F -e 'target_compatible_with' &&
  ! grep -A6 -e 'name = "deploy_app"' deploy/rules/BUILD.bazel | grep -q -F -e 'target_compatible_with' &&
  grep -A16 -e 'name = "release_demo_verify"' deploy/rules/BUILD.bazel | grep -q -F -e 'target_compatible_with' &&
  grep -A16 -e 'name = "github_demo_verify"' deploy/rules/BUILD.bazel | grep -q -F -e 'target_compatible_with'; then
  ok
else
  bad "deploy POSIX/bash split broke (fixtures portable, verifiers Linux-only)"
fi

# No bare `$(realpath`: every resolution goes through dx_realpath.
# (Self-excluded: this script names the banned form in its own pattern.)
if ! grep -rn -F -e '$(realpath' --include='*.sh' --exclude='shell_contract.sh' --exclude-dir='bazel-*' --exclude-dir='.git' . | grep -q .; then
  ok
else
  bad "bare \$(realpath ...) survives; use dx_realpath from tools/sh/lib.sh (issues #299, #323)"
fi

# Issue #323 dedup: portable realpath lives once in tools/sh/lib.sh
# (dx_realpath + portable_realpath alias probing realpath/readlink/python3);
# drivers use dx_realpath, no per-file copies.
if grep -q -F -e 'dx_realpath() {' tools/sh/lib.sh &&
  grep -q -F -e 'command -v realpath' tools/sh/lib.sh &&
  grep -q -F -e 'readlink -f' tools/sh/lib.sh &&
  grep -q -F -e "python3 -c 'import os,sys" tools/sh/lib.sh &&
  grep -q -F -e 'portable_realpath() {' tools/sh/lib.sh &&
  [[ "$(grep -rl -F -e 'portable_realpath() {' --include='*.sh' --exclude-dir='bazel-*' --exclude-dir='.git' . | grep -v -F -e 'tools/sh/lib.sh' | grep -v -F -e 'tools/ci/shell_contract.sh' | wc -l)" == "0" ]] &&
  grep -q -F -e 'dx_realpath' cli/env/bootstrap_test.sh &&
  grep -q -F -e 'dx_realpath' go/tests/fixtures/hello/hello_output_test.sh; then
  ok
else
  bad "portable realpath must live once in tools/sh/lib.sh (dx_realpath) with drivers using it (issue #323)"
fi

# No `sed -i` in executable code (comments may name it as the banned form).
# (Self-excluded: this script names the banned form in its own pattern.)
if ! grep -rn -e '^[^#]*sed -i' --include='*.sh' --exclude='shell_contract.sh' --exclude-dir='bazel-*' --exclude-dir='.git' . | grep -q .; then
  ok
else
  bad "GNU sed -i survives in executable code; use the tmpfile form (issue #299)"
fi

# No `cp -a` in executable code (comments may name it as the banned form).
# (Self-excluded: this script names the banned form in its own pattern.)
if ! grep -rn -e '^[^#]*cp -a' --include='*.sh' --exclude='shell_contract.sh' --exclude-dir='bazel-*' --exclude-dir='.git' . | grep -q .; then
  ok
else
  bad "GNU cp -a survives in executable code; use cp -RPp (issue #299)"
fi

# No unguarded $EPOCHREALTIME: braced `${EPOCHREALTIME...}` only
# (the `${EPOCHREALTIME:-}` presence test plus the guarded
# `"${EPOCHREALTIME}"` read inside now_secs()), plus comments.
# (Self-excluded: this script names the banned form in its own pattern.)
if ! grep -rn -F -e '$EPOCHREALTIME' --include='*.sh' --exclude='shell_contract.sh' --exclude-dir='bazel-*' --exclude-dir='.git' . | grep -v -F -e '${EPOCHREALTIME' | grep -v -e '^[^:]*:[0-9]*: *#' | grep -q .; then
  ok
else
  bad "unguarded \$EPOCHREALTIME survives; use now_secs() with \${EPOCHREALTIME:-} (issue #299)"
fi

# Portable timing lives once in tools/sh/lib.sh (issue #323 dedup:
# dx_now_secs + now_secs alias with EPOCHREALTIME/date fallbacks);
# perf harnesses use it, no per-file copies.
if grep -q -F -e 'dx_now_secs() {' tools/sh/lib.sh &&
  grep -q -F -e 'now_secs() {' tools/sh/lib.sh &&
  grep -q -F -e '${EPOCHREALTIME:-}' tools/sh/lib.sh &&
  [[ "$(grep -rln -F -e 'now_secs() {' --include='*.sh' --exclude-dir='bazel-*' --exclude-dir='.git' . | grep -v -F -e 'tools/sh/lib.sh' | grep -v -F -e 'tools/ci/shell_contract.sh' | wc -l)" == "0" ]] &&
  grep -q -F -e 'now_secs' perf/bench.sh &&
  grep -q -F -e 'now_secs' perf/rules_lint_comparison.sh; then
  ok
else
  bad "portable timing must live once in tools/sh/lib.sh (dx_now_secs/now_secs) with perf using it (issue #323)"
fi

# Portable hashing lives once in tools/sh/lib.sh (issue #323 dedup:
# dx_sha256_* + dx_tree_sha256 with shasum/python fallbacks); deploy stays
# hermetic python (issue #318), gazelle/perf use the lib helpers.
if grep -q -F -e 'dx_sha256_file() {' tools/sh/lib.sh &&
  grep -q -F -e 'dx_tree_sha256() {' tools/sh/lib.sh &&
  grep -q -F -e 'dx_sha256_check() {' tools/sh/lib.sh &&
  grep -q -F -e 'shasum -a 256' tools/sh/lib.sh &&
  grep -q -F -e 'py_sha256() {' deploy/rules/archive_verify.sh &&
  grep -q -F -e 'dx_sha256_file' gazelle/rust/idempotent_test.sh &&
  grep -q -F -e 'dx_sha256_check' gazelle/rust/native_config_test.sh &&
  grep -q -F -e 'dx_tree_sha256' perf/rules_lint_comparison.sh; then
  ok
else
  bad "portable hashing must live once in tools/sh/lib.sh with gazelle/perf using it and deploy hermetic python (issues #318, #323)"
fi

# Issue #323 dedup: harness counters live once in tools/sh/lib.sh
# (dx_test_init/ok/bad/dx_test_summary); drivers use them, no per-file
# copies (deploy/install/dx_verify_test.sh stays standalone: deploy
# hermetic scope, issue #318).
if [[ "$(grep -rln -e '^ok() {' --include='*.sh' --exclude-dir='bazel-*' --exclude-dir='.git' . | grep -v -F -e 'tools/sh/lib.sh' | grep -v -F -e 'deploy/install/dx_verify_test.sh' | grep -v -F -e 'tools/ci/shell_contract.sh' | wc -l)" == "0" ]] &&
  grep -q -F -e 'dx_test_init' tools/ci/e2e.sh &&
  grep -q -F -e 'dx_test_summary' tools/ci/e2e.sh; then
  ok
else
  bad "harness counters must live once in tools/sh/lib.sh (dx_test_init/ok/bad) with drivers using them (issue #323)"
fi

# Issue #323 dedup: scratch dirs live once in tools/sh/lib.sh (dx_mkscratch
# with EXIT cleanup); drivers use it, no per-file mktemp+trap copies
# (tools/sh/snapshot.sh owns its RETURN-scoped tmp, deploy stays standalone).
if [[ "$(grep -rln -F -e 'scratch="$(mktemp -d)"' --include='*.sh' --exclude-dir='bazel-*' --exclude-dir='.git' . | grep -v -F -e 'deploy/install/dx_verify_test.sh' | grep -v -F -e 'tools/sh/snapshot.sh' | grep -v -F -e 'tools/sh/lib.sh' | grep -v -F -e 'tools/ci/shell_contract.sh' | grep -v -F -e 'tools/ci/coverage_spill.sh' | wc -l)" == "0" ]] &&
  grep -q -F -e 'dx_mkscratch' tools/ci/coverage_cell.sh &&
  grep -q -F -e 'dx_mkscratch' tools/ci/e2e.sh; then
  ok
else
  bad "scratch dirs must use tools/sh/lib.sh dx_mkscratch with EXIT cleanup (issue #323)"
fi

# Issue #323 dedup: portable sed lives once in tools/sh/lib.sh (dx_replace
# tmpfile+mv, no sed -i); drivers use it for in-place edits.
if grep -q -F -e 'dx_replace() {' tools/sh/lib.sh &&
  grep -q -F -e 'dx_replace' tools/ci/e2e.sh &&
  grep -q -F -e 'dx_replace' tools/ci/e2e_format.sh; then
  ok
else
  bad "portable sed must live once in tools/sh/lib.sh (dx_replace) with drivers using it (issue #323)"
fi

# Issue #323 lint: shellcheck config stays (bash, all checks with scoped
# disables) and the harness stays shfmt clean (shfmt -i 2 -ci).
if grep -q -F -e 'shell=bash' .shellcheckrc &&
  grep -q -F -e 'enable=all' .shellcheckrc &&
  grep -q -F -e 'disable=SC1090,SC1091' .shellcheckrc &&
  grep -q -F -e 'shfmt -i 2 -ci' .shellcheckrc &&
  grep -q -F -e 'shfmt -i 2 -ci' tools/sh/lib.sh; then
  ok
else
  bad "shellcheck/shfmt pins missing (.shellcheckrc bash+all plus shfmt -i 2 -ci, issue #323)"
fi

# Windows stays out: no .ps1/.bat, no rules_powershell.
if [[ -z "$(find . -name '*.ps1' -not -path './bazel-*' -print -quit)" ]] &&
  [[ -z "$(find . -name '*.bat' -not -path './bazel-*' -print -quit)" ]] &&
  ! grep -rn -F -e 'rules_powershell' --include='*.bzl' --include='MODULE.bazel' --include='*.bazel' . | grep -q .; then
  ok
else
  bad "a Windows shell artifact appeared (stays out per ADR 0014 until the backend unblocks)"
fi

# Issue #320 portable route: perf harnesses record the actual host via
# dx_perf_host instead of hard-coding the seed label; unknown OS/CPU
# fails fast. The checked-in seed reports stay pinned to linux_x86_64
# (proven by //perf:rules_lint_results_test), so only the harness
# scripts are checked here.
if grep -q -F -e 'dx_perf_host() {' tools/sh/lib.sh &&
  grep -q -F -e 'host="$(dx_perf_host)"' perf/bench.sh &&
  grep -q -F -e 'host="$(dx_perf_host)"' perf/rules_lint_comparison.sh &&
  ! grep -rn -F -e 'host="linux_x86_64"' --include='bench.sh' --include='rules_lint_comparison.sh' perf/ | grep -q .; then
  ok
else
  bad "perf harnesses must use dx_perf_host (issue #320), not a hard-coded linux_x86_64 pin"
fi

# Issue #320 portable route: release archives are hermetic Python
# (archiver.py tarfile dereferences like tar -h, no host tar). No host
# `tar` invocation may appear in deploy runtime or rules.
if ! grep -rn -E -e '(^|[^a-z_])tar( |$| -)' --include='*.sh' deploy/rules/ | grep -v -F -e 'tarfile' | grep -v -e '^[^:]*:[0-9]*: *#' | grep -q . &&
  grep -q -F -e 'No host `tar`' deploy/rules/archiver.py &&
  grep -q -F -e 'no host `tar`' deploy/rules/archive.bzl; then
  ok
else
  bad "deploy archive must stay hermetic Python with no host tar (issue #320)"
fi

dx_test_summary "shell contract harness"
