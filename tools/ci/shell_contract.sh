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

workspace="$(dx_workspace_root)"
cd "$workspace"

pass=0
fail=0
ok() { pass=$((pass + 1)); }
bad() { echo "FAIL: $1" >&2; fail=$((fail + 1)); }

# Docs record the decided contract, not an open tracker.
if grep -q -F -e 'is bash-only on the Linux seed host (decided' docs/testing/tools.md \
  && grep -q -F -e '//tools/ci:shell_contract' docs/testing/tools.md \
  && grep -q -F -e 'target_compatible_with = ["@platforms//os:linux"]' docs/testing/tools.md; then
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
if ! grep -A4 -e 'name = "tool_sh"' env/BUILD.bazel | grep -q -F -e 'target_compatible_with' \
  && ! grep -A4 -e 'name = "doctor"' env/BUILD.bazel | grep -q -F -e 'target_compatible_with' \
  && ! grep -A4 -e 'name = "pass"' integration/clean/BUILD.bazel | grep -q -F -e 'target_compatible_with' \
  && ! grep -A4 -e 'name = "fail"' integration/dirty/BUILD.bazel | grep -q -F -e 'target_compatible_with'; then
  ok
else
  bad "a POSIX fixture gained a Linux-only label (must stay portable)"
fi

# Deploy POSIX fixtures stay portable while bash verifiers stay Linux-only.
if ! grep -A6 -e 'name = "deploy_program"' deploy/rules/BUILD.bazel | grep -q -F -e 'target_compatible_with' \
  && ! grep -A6 -e 'name = "deploy_app"' deploy/rules/BUILD.bazel | grep -q -F -e 'target_compatible_with' \
  && grep -A16 -e 'name = "release_demo_verify"' deploy/rules/BUILD.bazel | grep -q -F -e 'target_compatible_with' \
  && grep -A16 -e 'name = "github_demo_verify"' deploy/rules/BUILD.bazel | grep -q -F -e 'target_compatible_with'; then
  ok
else
  bad "deploy POSIX/bash split broke (fixtures portable, verifiers Linux-only)"
fi

# No bare `$(realpath`: every resolution goes through portable_realpath.
# (Self-excluded: this script names the banned form in its own pattern.)
if ! grep -rn -F -e '$(realpath' --include='*.sh' --exclude='shell_contract.sh' --exclude-dir='bazel-*' --exclude-dir='.git' . | grep -q .; then
  ok
else
  bad "bare \$(realpath ...) survives; use portable_realpath (issue #299)"
fi

# Every portable_realpath definition probes realpath, readlink -f, python3.
probes="$(grep -rl -F -e 'portable_realpath() {' --include='*.sh' --exclude-dir='bazel-*' --exclude-dir='.git' . | wc -l)"
if [[ "$probes" -ge 19 ]] \
  && grep -q -F -e 'command -v realpath' cli/env/bootstrap_test.sh \
  && grep -q -F -e "python3 -c 'import os,sys" cli/env/bootstrap_test.sh; then
  ok
else
  bad "portable_realpath coverage regressed (found $probes definitions, want >= 19)"
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

# Portable timing helpers exist where EPOCHREALTIME was used.
if grep -q -F -e 'now_secs() {' perf/bench.sh \
  && grep -q -F -e 'now_secs() {' perf/rules_lint_comparison.sh; then
  ok
else
  bad "portable now_secs() helper missing in perf harnesses"
fi

# Every sha256sum site has a shasum fallback (deploy already had it;
# gazelle + perf gained it here).
if grep -q -F -e 'shasum -a 256' deploy/rules/archive_verify.sh \
  && grep -q -F -e 'shasum -a 256' deploy/rules/archive_deploy.sh \
  && grep -q -F -e 'shasum -a 256' gazelle/rust/idempotent_test.sh \
  && grep -q -F -e 'shasum -a 256' gazelle/rust/native_config_test.sh \
  && grep -q -F -e 'shasum -a 256' perf/rules_lint_comparison.sh; then
  ok
else
  bad "a sha256sum site lost its shasum -a 256 fallback"
fi

# Windows stays out: no .ps1/.bat, no rules_powershell.
if [[ -z "$(find . -name '*.ps1' -not -path './bazel-*' -print -quit)" ]] \
  && [[ -z "$(find . -name '*.bat' -not -path './bazel-*' -print -quit)" ]] \
  && ! grep -rn -F -e 'rules_powershell' --include='*.bzl' --include='MODULE.bazel' --include='*.bazel' . | grep -q .; then
  ok
else
  bad "a Windows shell artifact appeared (stays out per ADR 0014 until the backend unblocks)"
fi

echo "shell contract harness: $pass passed, $fail failed"
[[ "$fail" == "0" ]]
