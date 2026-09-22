#!/usr/bin/env bash
# Shell portability contract guard.
#
# The CI/test harness is bash-only: every bash `sh_binary`/`sh_test`
# carries `target_compatible_with = ["@platforms//os:linux"]`, POSIX
# `#!/bin/sh` fixtures stay portable with no constraint, and the known
# non-portable forms carry macOS best-effort fallbacks with no Linux
# behavior change (realpath probe, shasum fallback, portable sed/cp,
# portable timing). Bootstrap requires bash by design
# (`BASH_SOURCE` plus the 5-way runfiles fallback never run under POSIX
# `sh`; floor is bash 3.2+ with Linux execution). Windows x86_64
# MSVC-compatible is qualified for native `dx`/CI execution under issue
# via `windows-latest` runners with shell `bash` plus portable forms
# only (no `.ps1`/`.bat`, no `rules_powershell`, PowerShell port wont-fix
# under issue #750); product runtime is Rust
# and shell-free except generated deploy launchers plus the doctor shim.
# Guard maintenance owns shared helpers plus snapshot versus grep policy
# (snapshot for golden bytes, `dx_expect_*`/`dx_guard_*` table rows for
# doc/code pins; this harness owns the rule).
#
# This harness machine-checks the contract statically on a clean tree.
# Versioned here, run by CI via `bazel run //tools/ci:shell_contract`.
set -euo pipefail

# Shared workspace + runfiles helpers.
# Bootstrap via tools/sh/bootstrap.sh (issue #654): runfiles forest first, then source tree.
source "${RUNFILES_DIR:-/dev/null}/_main/tools/sh/bootstrap.sh" 2>/dev/null || source "${TEST_SRCDIR:-/dev/null}/_main/tools/sh/bootstrap.sh" 2>/dev/null || source "$0.runfiles/_main/tools/sh/bootstrap.sh" 2>/dev/null || source "${BASH_SOURCE[0]}.runfiles/_main/tools/sh/bootstrap.sh" 2>/dev/null || source "$(git rev-parse --show-toplevel 2>/dev/null)/tools/sh/bootstrap.sh"
dx_bootstrap "tools/sh/lib.sh"
dx_bootstrap "tools/sh/guards.sh"

dx_cd_workspace

dx_test_init

# Docs record the decided contract, not an open tracker.
if grep -q -F -e 'is bash-only (decided' docs/testing/tools.md &&
  grep -q -F -e 'Windows native execution via shell bash under issue' docs/testing/tools.md &&
  grep -q -F -e '//tools/ci:shell_contract' docs/testing/tools.md &&
  grep -q -F -e 'target_compatible_with = ["@platforms//os:linux"]' docs/testing/tools.md; then
  ok
else
  bad "docs/testing/tools.md lost the decided #299/#414 shell contract record"
fi

# Every bash sh target carries the Linux-only label
# the 6 nested-E2E label sites; POSIX fixtures stay portable). Count
# labels vs bash sh targets (BUILD.bazel plus ci_targets split under #652).
labels="$(grep -r -F -e 'target_compatible_with = ["@platforms//os:linux"]' --include='BUILD.bazel' --include='*.bzl' --exclude-dir='bazel-*' --exclude-dir='.git' . | wc -l)"
# Bash sh_* plus devcontainer parity stay Linux-only; threshold keeps the
# contract fail-closed after the E2E deletion.
if [[ "$labels" -ge 69 ]]; then
  ok
else
  bad "want >= 69 Linux-only labels, found $labels"
fi

# POSIX fixtures stay portable: no Linux constraint in their packages'
# POSIX-only targets (env tool/doctor; deleted the
# integration pass/fail fixtures).
if ! grep -A4 -e 'name = "tool_sh"' env/BUILD.bazel | grep -q -F -e 'target_compatible_with' &&
  ! grep -A4 -e 'name = "doctor"' env/BUILD.bazel | grep -q -F -e 'target_compatible_with'; then
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

# Dedup: portable realpath lives once in tools/sh/lib.sh
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

# Portable timing lives once in tools/sh/lib.sh
# dx_now_secs + now_secs alias with EPOCHREALTIME/date fallbacks);
# no per-file copies.
if grep -q -F -e 'dx_now_secs() {' tools/sh/lib.sh &&
  grep -q -F -e 'now_secs() {' tools/sh/lib.sh &&
  grep -q -F -e '${EPOCHREALTIME:-}' tools/sh/lib.sh &&
  [[ "$(grep -rln -F -e 'now_secs() {' --include='*.sh' --exclude-dir='bazel-*' --exclude-dir='.git' . | grep -v -F -e 'tools/sh/lib.sh' | grep -v -F -e 'tools/ci/shell_contract.sh' | wc -l)" == "0" ]]; then
  ok
else
  bad "portable timing must live once in tools/sh/lib.sh (dx_now_secs/now_secs, issue #323)"
fi

# Portable hashing lives once in tools/sh/lib.sh
# dx_sha256_* + dx_tree_sha256 with shasum/python fallbacks); deploy stays
# hermetic python, gazelle uses the lib helpers (depcheck is Rust since ADR 0027).
if grep -q -F -e 'dx_sha256_file() {' tools/sh/lib.sh &&
  grep -q -F -e 'dx_tree_sha256() {' tools/sh/lib.sh &&
  grep -q -F -e 'dx_sha256_check() {' tools/sh/lib.sh &&
  grep -q -F -e 'shasum -a 256' tools/sh/lib.sh &&
  grep -q -F -e 'py_sha256() {' deploy/rules/archive_verify.sh &&
  grep -q -F -e 'dx_sha256_file' gazelle/rust/idempotent_test.sh &&
  grep -q -F -e 'dx_sha256_check' gazelle/rust/native_config_test.sh; then
  ok
else
  bad "portable hashing must live once in tools/sh/lib.sh with gazelle using it and deploy hermetic python (issues #318, #323)"
fi

# Dedup: harness counters live once in tools/sh/lib.sh
# (dx_test_init/ok/bad/dx_test_summary); drivers use them, no per-file
# copies (deploy/install/dx_verify_test.sh stays standalone: deploy
# hermetic scope,; deleted the e2e drivers, so the
# witness pair is coverage_cell plus non_dogfed_paths).
if [[ "$(grep -rln -e '^ok() {' --include='*.sh' --exclude-dir='bazel-*' --exclude-dir='.git' . | grep -v -F -e 'tools/sh/lib.sh' | grep -v -F -e 'deploy/install/dx_verify_test.sh' | grep -v -F -e 'tools/ci/shell_contract.sh' | wc -l)" == "0" ]] &&
  grep -q -F -e 'dx_test_init' tools/ci/coverage_cell.sh &&
  grep -q -F -e 'dx_test_summary' tools/ci/non_dogfed_paths.sh; then
  ok
else
  bad "harness counters must live once in tools/sh/lib.sh (dx_test_init/ok/bad) with drivers using them (issue #323)"
fi

# Dedup: scratch dirs live once in tools/sh/lib.sh (dx_mkscratch
# with EXIT cleanup); drivers use it, no per-file mktemp+trap copies
# (tools/sh/snapshot.sh owns its RETURN-scoped tmp, deploy stays standalone;
# both TMPDIR-aware under issue #750).
if [[ "$(grep -rln -F -e 'scratch="$(mktemp -d)"' --include='*.sh' --exclude-dir='bazel-*' --exclude-dir='.git' . | grep -v -F -e 'deploy/install/dx_verify_test.sh' | grep -v -F -e 'tools/sh/snapshot.sh' | grep -v -F -e 'tools/sh/lib.sh' | grep -v -F -e 'tools/ci/shell_contract.sh' | grep -v -F -e 'tools/ci/coverage_spill.sh' | wc -l)" == "0" ]] &&
  grep -q -F -e 'dx_mkscratch' tools/ci/coverage_cell.sh &&
  grep -q -F -e 'dx_mkscratch' tools/ci/non_dogfed_paths.sh; then
  ok
else
  bad "scratch dirs must use tools/sh/lib.sh dx_mkscratch with EXIT cleanup (issue #323)"
fi

# Dedup: portable sed lives once in tools/sh/lib.sh (dx_replace
# tmpfile+mv, no sed -i); drivers use it for in-place edits
# deleted the e2e drivers, so the witness is release_policy).
if grep -q -F -e 'dx_replace() {' tools/sh/lib.sh &&
  grep -q -F -e 'dx_replace' tools/ci/release_policy.sh; then
  ok
else
  bad "portable sed must live once in tools/sh/lib.sh (dx_replace) with drivers using it (issue #323)"
fi

# Lint: shellcheck config stays (bash, all checks with scoped
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

# Windows shell stays bash-only for the harness: no harness `.ps1`/`.bat`,
# no harness `rules_powershell`; Windows CI jobs run via shell bash with
# portable forms. Product PowerShell application sources are explicitly
# allowed under ADR 0032 (`powershell/`, `examples/adopt-powershell/`,
# `third_party/powershell/` plus their `MODULE.bazel` toolchain): the
# harness shell stays bash-only while the product language ships `.ps1`.
if [[ -z "$(find . -name '*.ps1' -not -path './bazel-*' -not -path './powershell/*' -not -path './examples/adopt-powershell/*' -not -path './third_party/powershell/*' -not -path './quality/tests/fixtures/file_family_adapters/psscriptanalyzer/*' -print -quit)" ]] &&
  [[ -z "$(find . -name '*.bat' -not -path './bazel-*' -print -quit)" ]] &&
  ! grep -rn -F -e 'rules_powershell' --include='*.bzl' --include='MODULE.bazel' --include='*.bazel' . | grep -v -F -e 'powershell/rules/' | grep -v -F -e 'powershell/env/' | grep -v -F -e 'powershell/tests/' | grep -v -F -e 'examples/adopt-powershell' | grep -v -F -e 'third_party/powershell' | grep -v -F -e 'modules/powershell.bzl' | grep -v -F -e 'tools/ci/foundation_maps.sh' | grep -v -F -e 'tools/ci/shell_contract.sh' | grep -v -e 'MODULE.bazel.*rules_powershell' | grep -v -e 'MODULE.bazel.lock' | grep -v -F -e 'docs/' | grep -q .; then
  ok
else
  bad "a Windows harness shell artifact appeared (harness stays bash-only under issue #414, no ps1/bat/powershell outside the ADR 0032 product PowerShell foundation)"
fi

# Windows CI jobs run bash-only: every windows-latest job sets shell bash
# and stays portable-shell clean (no pwsh, no banned forms). This resolves
# the Windows bash-only Linux-only harness gap here, not by
# papering over: the harness stays Linux-only (labels above) while Windows
# execution goes through bash explicitly.
if grep -q -F -e 'runs-on: windows-latest' .github/workflows/ci.yml &&
  grep -q -F -e 'shell: bash' .github/workflows/ci.yml &&
  [[ "$(grep -c -F -e 'runs-on: windows-latest' .github/workflows/ci.yml)" == "$(grep -c -F -e 'shell: bash' .github/workflows/ci.yml)" ]] &&
  ! grep -A30 -e 'runs-on: windows-latest' .github/workflows/ci.yml | grep -E -e 'shell: (pwsh|powershell|cmd)' | grep -q .; then
  ok
else
  bad "windows-latest jobs must run shell bash with portable forms (issue #414 resolves the #323 Windows gap)"
fi

# Portable route: release archives are hermetic Rust
# (archiver dereferences like tar -h, no host tar). No host
# `tar` invocation may appear in deploy runtime or rules.
if ! grep -rn -E -e '(^|[^a-z_])tar( |$| -)' --include='*.sh' deploy/rules/ | grep -v -F -e 'tarfile' | grep -v -e '^[^:]*:[0-9]*: *#' | grep -q . &&
  grep -q -F -e 'like `tar -h`' deploy/rules/src/lib.rs &&
  grep -q -F -e 'no host `tar`' deploy/rules/archive.bzl; then
  ok
else
  bad "deploy archive must stay hermetic Rust with no host tar (issue #320)"
fi

# Stays closed with the contract guard owned:
# docs must record delivered (closed) with the
# //tools/ci:shell_contract pin, never a stay-open tracker.
if grep -q -F -e 'closed issue #323' docs/testing/tools.md &&
  grep -q -F -e '//tools/ci:shell_contract' docs/testing/tools.md &&
  ! grep -q -F -e 'stay open under issue #323' docs/testing/tools.md; then
  ok
else
  bad "docs/testing/tools.md lost the closed-#323 plus shell_contract record (want closed issue #323 with //tools/ci:shell_contract, no stay-open claim, issue #424)"
fi

# Bootstrap floor: every bash driver carries the bash shebang
# plus `set -euo pipefail`; POSIX `#!/bin/sh` fixtures stay exempt with no
# bootstrap.
shebang_fail=""
for f in $(grep -rln -F -e '#!/usr/bin/env bash' --include='*.sh' --exclude-dir='bazel-*' --exclude-dir='.git' .); do
  if ! grep -q -F -e 'set -euo pipefail' "$f"; then
    shebang_fail="$shebang_fail $f"
  fi
done
if [[ -z "$shebang_fail" ]]; then
  ok
else
  bad "bash drivers missing set -euo pipefail:$shebang_fail (issue #450 bootstrap floor)"
fi

# Bootstrap is single-sourced (issue #654): `dx_bootstrap` lives once in
# `tools/sh/bootstrap.sh` (runfiles-first plus `BUILD_WORKSPACE_DIRECTORY`
# plus git top-level plus source-tree walk); drivers share one identical
# loader plus `dx_bootstrap` lines with no depth-adjusted `../` variants.
# (Self-excluded: this script names the helper definition in its own
# pattern below.)
if grep -q -F -e 'dx_bootstrap() {' tools/sh/bootstrap.sh &&
  grep -q -F -e 'git rev-parse --show-toplevel' tools/sh/bootstrap.sh &&
  grep -q -F -e 'BUILD_WORKSPACE_DIRECTORY' tools/sh/bootstrap.sh &&
  grep -q -F -e 'dx_bootstrap "tools/sh/lib.sh"' tools/sh/lib.sh &&
  [[ "$(grep -rln -F -e 'dx_bootstrap() {' --include='*.sh' --exclude-dir='bazel-*' --exclude-dir='.git' . | grep -v -F -e 'tools/sh/bootstrap.sh' | grep -v -F -e 'tools/ci/shell_contract.sh' | wc -l)" == "0" ]]; then
  ok
else
  bad "bootstrap must live once in tools/sh/bootstrap.sh (dx_bootstrap with git plus walk fallbacks, issue #654)"
fi

# No direct tools/sh library source chains survive outside the bootstrap
# loader docs: every library load goes through `dx_bootstrap`.
# (Self-excluded: this script names the retired forms in its own patterns.)
if [[ "$(grep -rn -F -e '_main/tools/sh/lib.sh' --include='*.sh' --exclude='shell_contract.sh' --exclude-dir='bazel-*' --exclude-dir='.git' . | wc -l)" == "0" ]] &&
  [[ "$(grep -rn -F -e '_main/tools/sh/guards.sh' --include='*.sh' --exclude='shell_contract.sh' --exclude-dir='bazel-*' --exclude-dir='.git' . | wc -l)" == "0" ]] &&
  [[ "$(grep -rn -F -e '_main/tools/sh/snapshot.sh' --include='*.sh' --exclude='shell_contract.sh' --exclude-dir='bazel-*' --exclude-dir='.git' . | wc -l)" == "0" ]]; then
  ok
else
  bad "direct tools/sh library source chains survive; load via tools/sh/bootstrap.sh dx_bootstrap (issue #654)"
fi

# No depth-adjusted `dirname BASH_SOURCE` source-tree fallbacks for
# tools/sh libraries: the fallback lives once in `dx_bootstrap`.
# (Self-excluded: this script names the retired forms in its own patterns.)
if [[ "$(grep -rn -F -e 'dirname "${BASH_SOURCE[0]}")/../sh/' --include='*.sh' --exclude='shell_contract.sh' --exclude-dir='bazel-*' --exclude-dir='.git' . | wc -l)" == "0" ]] &&
  [[ "$(grep -rn -F -e 'dirname "${BASH_SOURCE[0]}")/../../tools/sh/' --include='*.sh' --exclude='shell_contract.sh' --exclude-dir='bazel-*' --exclude-dir='.git' . | wc -l)" == "0" ]] &&
  [[ "$(grep -rn -F -e 'dirname "${BASH_SOURCE[0]}")/../tools/sh/' --include='*.sh' --exclude='shell_contract.sh' --exclude-dir='bazel-*' --exclude-dir='.git' . | wc -l)" == "0" ]] &&
  [[ "$(grep -rn -F -e 'dirname "${BASH_SOURCE[0]}")/tools/sh/' --include='*.sh' --exclude='shell_contract.sh' --exclude-dir='bazel-*' --exclude-dir='.git' . | wc -l)" == "0" ]]; then
  ok
else
  bad "depth-adjusted dirname BASH_SOURCE fallbacks survive; share the tools/sh/bootstrap.sh loader (issue #654)"
fi

# Every `dx_bootstrap` call site carries the canonical bootstrap loader.
bootstrap_fail=""
for f in $(grep -rln -F -e 'dx_bootstrap "tools/sh/' --include='*.sh' --exclude-dir='bazel-*' --exclude-dir='.git' .); do
  if ! grep -q -F -e 'tools/sh/bootstrap.sh' "$f"; then
    bootstrap_fail="$bootstrap_fail $f"
  fi
done
if [[ -z "$bootstrap_fail" ]]; then
  ok
else
  bad "dx_bootstrap call site missing the tools/sh/bootstrap.sh loader:$bootstrap_fail (issue #654)"
fi

# Lib-free exceptions stay explicit: POSIX fixtures keep
# `#!/bin/sh` with no lib bootstrap; deploy hermetic runtime keeps
# python-only probes with no lib bootstrap.
if grep -q -F -e '#!/bin/sh' env/tool.sh &&
  grep -q -F -e '#!/bin/sh' env/doctor.sh &&
  grep -q -F -e '#!/bin/sh' deploy/rules/deploy_app.sh &&
  grep -q -F -e '#!/bin/sh' deploy/rules/deploy_program.sh &&
  ! grep -q -F -e 'tools/sh/lib.sh' env/tool.sh &&
  ! grep -q -F -e 'tools/sh/lib.sh' env/doctor.sh &&
  ! grep -q -F -e 'tools/sh/lib.sh' deploy/rules/deploy_app.sh &&
  ! grep -q -F -e 'tools/sh/lib.sh' deploy/rules/deploy_program.sh &&
  grep -q -F -e 'py_realpath' deploy/rules/archive_verify.sh &&
  ! grep -q -F -e 'tools/sh/lib.sh' deploy/rules/archive_verify.sh; then
  ok
else
  bad "lib-free exceptions drifted (want POSIX sh fixtures plus deploy python-only with no lib bootstrap, issue #450)"
fi

# Docs record the bash-only bootstrap floor plus the guard
# maintenance rule (shared helpers plus snapshot versus grep policy).
dx_expect_contains docs/testing/tools.md 'Bootstrap requires bash by design under issue #450' 'data = ["//tools/sh:lib"]' 'Intentional lib-free exceptions'
dx_expect_contains docs/testing/tools.md 'Guard maintenance owns shared helpers plus snapshot versus grep policy under' 'issue #450' 'dx_expect_contains' 'snapshot_diff'

# Guard helpers live once in tools/sh/lib.sh with no per-file
# copies; snapshot owns golden bytes, dx_expect_* owns fixed-string pins.
# (Self-excluded: this script names the helper definitions in its own
# patterns below.)
dx_expect_contains tools/sh/lib.sh 'dx_expect_file() {' 'dx_expect_contains() {' 'dx_expect_absent() {' 'Bootstrap requires bash by design under issue' 'Intentional lib-free exceptions' 'snapshot versus grep policy'
if [[ "$(grep -rln -F -e 'dx_expect_contains() {' --include='*.sh' --exclude-dir='bazel-*' --exclude-dir='.git' . | grep -v -F -e 'tools/sh/lib.sh' | grep -v -F -e 'tools/ci/shell_contract.sh' | wc -l)" == "0" ]] &&
  [[ "$(grep -rln -F -e 'dx_expect_absent() {' --include='*.sh' --exclude-dir='bazel-*' --exclude-dir='.git' . | grep -v -F -e 'tools/sh/lib.sh' | grep -v -F -e 'tools/ci/shell_contract.sh' | wc -l)" == "0" ]] &&
  [[ "$(grep -rln -F -e 'dx_expect_file() {' --include='*.sh' --exclude-dir='bazel-*' --exclude-dir='.git' . | grep -v -F -e 'tools/sh/lib.sh' | grep -v -F -e 'tools/ci/shell_contract.sh' | wc -l)" == "0" ]]; then
  ok
else
  bad "guard helpers must live once in tools/sh/lib.sh with no per-file copies"
fi
dx_expect_contains tools/sh/snapshot.sh 'snapshot versus grep policy' 'Bootstrap requires bash by design under issue'
dx_expect_contains docs/contributing/build-conventions.md 'data = ["//tools/sh:bootstrap", "//tools/sh:lib"]' 'snapshot versus grep policy under issue #450' 'single-sourced bootstrap under issue #654'

# Bootstrap loader still carries the runfiles probes (RUNFILES_DIR plus
# BASH_SOURCE) for every guards call site, via the single-sourced
# `tools/sh/bootstrap.sh` loader (issue #654).
guards_bootstrap_fail=""
for f in $(grep -rln -F -e 'tools/sh/guards.sh' --include='*.sh' --exclude-dir='bazel-*' --exclude-dir='.git' .); do
  if ! grep -q -F -e 'RUNFILES_DIR' "$f" || ! grep -q -F -e 'BASH_SOURCE' "$f"; then
    guards_bootstrap_fail="$guards_bootstrap_fail $f"
  fi
done
if [[ -z "$guards_bootstrap_fail" ]]; then
  ok
else
  bad "guards bootstrap lost the 5-way runfiles fallback:$guards_bootstrap_fail (issue #653)"
fi

# Guard table rows live once in tools/sh/guards.sh with no per-file
# copies; guards own fixed-string/regex plus single-file/tree pins with
# reason context, snapshot owns golden bytes.
# (Self-excluded: this script names the helper definitions in its own
# patterns below.)
dx_guards_contains tools/sh/guards.sh "guard table rows must live once in tools/sh/guards.sh (issue #653)" \
  'dx_guard_contains() {' \
  'dx_guards_contains() {' \
  'dx_guard_tree_absent_re() {' \
  'dx_guards_tree_absent_re() {' \
  'snapshot versus grep policy' \
  'Bootstrap requires bash by design under issue' \
  'Prefer fixed-string'
if [[ "$(grep -rln -F -e 'dx_guard_contains() {' --include='*.sh' --exclude-dir='bazel-*' --exclude-dir='.git' . | grep -v -F -e 'tools/sh/guards.sh' | grep -v -F -e 'tools/ci/shell_contract.sh' | wc -l)" == "0" ]] &&
  [[ "$(grep -rln -F -e 'dx_guards_contains() {' --include='*.sh' --exclude-dir='bazel-*' --exclude-dir='.git' . | grep -v -F -e 'tools/sh/guards.sh' | grep -v -F -e 'tools/ci/shell_contract.sh' | wc -l)" == "0" ]] &&
  [[ "$(grep -rln -F -e 'dx_guard_tree_absent_re() {' --include='*.sh' --exclude-dir='bazel-*' --exclude-dir='.git' . | grep -v -F -e 'tools/sh/guards.sh' | grep -v -F -e 'tools/ci/shell_contract.sh' | wc -l)" == "0" ]] &&
  [[ "$(grep -rln -F -e 'dx_guards_tree_absent_re() {' --include='*.sh' --exclude-dir='bazel-*' --exclude-dir='.git' . | grep -v -F -e 'tools/sh/guards.sh' | grep -v -F -e 'tools/ci/shell_contract.sh' | wc -l)" == "0" ]]; then
  ok
else
  bad "guard table rows must live once in tools/sh/guards.sh with no per-file copies (issue #653)"
fi

# Snapshot versus grep policy stays documented with the when-rules
# (snapshot for byte identity, literal rows for known files, regex only
# for shapes, tree only when the location is unknown).
dx_guards_contains docs/testing/tools.md "snapshot versus grep policy lost its when-rules (want snapshot vs literal vs regex vs tree, issue #653)" \
  'issue #653' \
  'dx_guard_' \
  'when whole-file byte identity matters' \
  'when a few contract' \
  'only for shapes' \
  'prefer single-file pins'
dx_guards_contains tools/sh/guards.sh "guards.sh lost its snapshot versus grep policy (want when-rules, issue #653)" \
  'snapshot versus grep policy' \
  'Prefer fixed-string' \
  'only for shapes' \
  'Prefer single-file pins'
dx_guard_contains docs/contributing/build-conventions.md 'issue #653' "build-conventions lost its guard-rows record (want issue #653)"

# First-wave migration stays table-driven: hygiene plus parity guards
# source guards.sh and use its rows instead of ad-hoc grep chains.
dx_guards_contains tools/ci/build_hygiene.sh "build_hygiene lost its guard-rows migration (want guards.sh plus table rows, issue #653)" \
  'tools/sh/guards.sh' \
  'dx_guards_contains' \
  'dx_guard_contains'
dx_guards_contains tools/ci/warnings_as_errors.sh "warnings_as_errors lost its guard-rows migration (want guards.sh plus table rows, issue #653)" \
  'tools/sh/guards.sh' \
  'dx_guards_contains' \
  'dx_guard_contains'
dx_guards_contains tools/ci/release_hygiene.sh "release_hygiene lost its guard-rows migration (want guards.sh plus table rows, issue #653)" \
  'tools/sh/guards.sh' \
  'dx_guards_contains' \
  'dx_guard_contains'
dx_guards_contains tools/ci/quality_adapters_parity.sh "quality_adapters_parity lost its guard-rows migration (want guards.sh plus table rows, issue #653)" \
  'tools/sh/guards.sh' \
  'dx_guards_contains' \
  'dx_guard_file'

# Shell-harness elimination stays wont-fix under issue #667 (CI/harness
# only, no product behavior; affirms decided #299): docs record the
# inventory plus keep rationale with no wholesale migration, and the
# verification-matrix shell row stays pinned here.
dx_expect_contains docs/testing/tools.md 'issue #667' 'stays wont-fix' 'Wholesale Rust-ify' 'POSIX-only' 'harness-wide' 'shell=bash' '//tools/ci:shell_contract'
dx_expect_contains docs/testing/verification-matrix.md 'issue #667' 'elimination wont-fix'

# Crate-reuse plus scratch/tmp discipline stays delivered under issue #750:
# PowerShell port wont-fix, workflows under RUNNER_TEMP, shell scratch via
# dx_mkscratch (snapshot RETURN plus deploy standalone TMPDIR-aware).
dx_expect_contains docs/testing/tools.md 'issue #750' 'PowerShell port stays wont-fix' 'RUNNER_TEMP' 'dx_mkscratch'

dx_test_summary "shell contract harness"
