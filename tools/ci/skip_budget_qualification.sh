#!/usr/bin/env bash
# Per-host skip-budget qualification harness.
#
# Machine-checks the Linux-only bash-harness skip budget with inventory
# evidence and owned gaps, without claiming Supported or a portable
# harness:
# - delivered: every bash `sh_binary`/`sh_test` stays Linux-only per the
#   shell contract, so Linux cells run the full scope while the macOS
#   arm64 plus Windows x86_64 cells skip the
#   same scope honestly; the skip volume is inventoried here (31 `sh_test`
#   test skips plus 127 `sh_binary` build skips, 162 Linux-only labels)
#   with a fail-closed per-host budget (Linux cells 0 skips, non-Linux
#   cells at most the pinned inventory);
# - CI reporting: every per-host `test //...` job reports its cell skip
#   volume to its step summary (Linux cells as real runs, non-Linux cells
#   as budgeted skips), so qualification never passes on silent skips;
# - docs in place: `docs/testing/tools.md` owns the per-host real-runs vs
#   skips table, `docs/product/promotion-checklist.md` wires the budget as
#   platform evidence, the verification matrix owns this harness entry;
# - open with honest records: porting the harness (hermetic py_binary/Rust
#   or POSIX fixtures) stays future work under this issue; the budget only
#   measures and caps the skip hole, and coverage cells gate Rust code
#   unaffected by sh skips with no union.
#
# Versioned here, run by CI via `bazel run //tools/ci:skip_budget_qualification`,
# following //tools/ci:ci_matrix_qualification.
set -euo pipefail

# Shared workspace + runfiles helpers.
# Bootstrap via tools/sh/bootstrap.sh (issue #654): runfiles forest first, then source tree.
source "${RUNFILES_DIR:-/dev/null}/_main/tools/sh/bootstrap.sh" 2>/dev/null || source "${TEST_SRCDIR:-/dev/null}/_main/tools/sh/bootstrap.sh" 2>/dev/null || source "$0.runfiles/_main/tools/sh/bootstrap.sh" 2>/dev/null || source "${BASH_SOURCE[0]}.runfiles/_main/tools/sh/bootstrap.sh" 2>/dev/null || source "$(git rev-parse --show-toplevel 2>/dev/null)/tools/sh/bootstrap.sh"
dx_bootstrap "tools/sh/lib.sh"

dx_cd_workspace

dx_test_init

ci=".github/workflows/ci.yml"
build="tools/ci/ci_targets_b.bzl"
prove="tools/ci/prove.sh"
dogfood="tools/ci/dogfood_freshness.sh"
tools_doc="docs/testing/tools.md"
checklist="docs/product/promotion-checklist.md"
verify="docs/testing/verification-matrix.md"
dx_mkscratch skip_scratch

# Budgets: per-host skip caps. Linux cells must run with 0 skips; each
# non-Linux cell skips at most the pinned Linux-only inventory. Growing
# the Linux-only harness beyond these budgets fails here and must bump
# the budget plus the tools.md inventory in the same reviewed PR.
shtest_budget="31"
shbin_budget="127"
labels_budget="162"

# Docs own the skip-budget decision with the pinned inventory plus this
# harness, never a silent skip.
if grep -q -F -e 'skip budget (issue #769' "$tools_doc" &&
  grep -q -F -e '31 `sh_test`' "$tools_doc" &&
  grep -q -F -e '127 `sh_binary`' "$tools_doc" &&
  grep -q -F -e '162 Linux-only labels' "$tools_doc" &&
  grep -q -F -e 'bazel run //tools/ci:skip_budget_qualification' "$tools_doc"; then
  ok
else
  bad "docs/testing/tools.md lost the issue #769 skip-budget record with pinned inventory plus harness pin"
fi

# Docs own the per-host real-runs vs skips table: Linux cells run, the
# macOS arm64 plus Windows skip honestly with budgeted volumes.
if grep -q -F -e 'Real runs vs skips' "$tools_doc" &&
  grep -q -F -e 'seed linux_x86_64' "$tools_doc" &&
  grep -q -F -e 'linux_arm64' "$tools_doc" &&
  grep -q -F -e 'macos_arm64' "$tools_doc" &&
  ! grep -q -F -e 'macos_x86_64' "$tools_doc" &&
  grep -q -F -e 'windows_x86_64' "$tools_doc" &&
  grep -q -F -e 'real run' "$tools_doc" &&
  grep -q -F -e '31 skips' "$tools_doc"; then
  ok
else
  bad "docs/testing/tools.md lost the per-host real-runs vs skips table (issue #769; x86_64 removed per #976)"
fi

# Test skip inventory stays within budget: every sh_test is Linux-only,
# so each non-Linux cell skips exactly this many tests.
shtest_total="$(bazel query 'kind(sh_test, //...)' 2>/dev/null | wc -l | tr -d ' ')"
shtest_linux="$(bazel query 'attr(target_compatible_with, "@platforms//os:linux", kind(sh_test, //...))' 2>/dev/null | wc -l | tr -d ' ')"
if [[ "$shtest_total" == "$shtest_linux" ]] && [[ "$shtest_linux" -le "$shtest_budget" ]]; then
  ok
else
  bad "sh_test skip inventory exceeded budget (want all $shtest_total sh_test Linux-only within $shtest_budget, found $shtest_linux Linux-only, issue #769)"
fi

# Build skip inventory stays within budget: Linux-only sh_binary count
# caps each non-Linux cell's build skips.
shbin_linux="$(bazel query 'attr(target_compatible_with, "@platforms//os:linux", kind(sh_binary, //...))' 2>/dev/null | wc -l | tr -d ' ')"
if [[ "$shbin_linux" -le "$shbin_budget" ]]; then
  ok
else
  bad "sh_binary skip inventory exceeded budget (want <= $shbin_budget Linux-only sh_binary, found $shbin_linux, issue #769)"
fi

# Total Linux-only labels stay within budget (shell contract keeps the
# lower bound; this budget keeps the upper bound).
labels="$(grep -r -F -e 'target_compatible_with = ["@platforms//os:linux"]' --include='BUILD.bazel' --include='*.bzl' . 2>/dev/null | wc -l | tr -d ' ')"
if [[ "$labels" -le "$labels_budget" ]]; then
  ok
else
  bad "Linux-only labels exceeded budget (want <= $labels_budget, found $labels, issue #769)"
fi

# No portable sh_test hides outside the budget: non-Linux test skips
# equal the total sh_test scope, never a silent subset.
shtest_portable="$((shtest_total - shtest_linux))"
if [[ "$shtest_portable" == "0" ]]; then
  ok
else
  bad "a portable sh_test appeared outside the Linux-only inventory (found $shtest_portable; pin it in tools.md real-runs vs skips, issue #769)"
fi

# Portable sh_binary inventory stays pinned: exactly the six POSIX plus
# generated shims run everywhere; everything else is budgeted above.
bazel query 'kind(sh_binary, //...)' 2>/dev/null | LC_ALL=C sort >"$skip_scratch/all_shbin.txt"
bazel query 'attr(target_compatible_with, "@platforms//os:linux", kind(sh_binary, //...))' 2>/dev/null | LC_ALL=C sort >"$skip_scratch/linux_shbin.txt"
portable_shbin="$(comm -23 "$skip_scratch/all_shbin.txt" "$skip_scratch/linux_shbin.txt" | wc -l | tr -d ' ')"
if [[ "$portable_shbin" == "6" ]] &&
  grep -q -F -e '//env:tool_sh' "$skip_scratch/all_shbin.txt" &&
  grep -q -F -e '//env:doctor' "$skip_scratch/all_shbin.txt"; then
  ok
else
  bad "portable sh_binary inventory drifted (want exactly the 6 POSIX plus generated shims, found $portable_shbin portable, issue #769)"
fi

# CI reports skip volume on every per-host test job: seed plus arm64
# plus macos arm64 plus windows each carry the report step.
if [[ "$(grep -c -F -e 'Report skip volume (Linux-only sh harness, issue #769)' "$ci")" == "4" ]]; then
  ok
else
  bad "ci.yml lost a per-host skip-volume report step (want 4 test jobs reporting, issue #769; x86_64 removed per #976)"
fi

# Linux cells report real runs with 0 skips: seed plus arm64 summaries
# claim execution, never skips.
if [[ "$(grep -c -F -e 'real run, 0 skips' "$ci")" -ge "2" ]]; then
  ok
else
  bad "ci.yml lost the Linux real-run 0-skips record (want seed plus arm64, issue #769)"
fi

# Non-Linux cells report budgeted skips: macos arm64 plus windows name
# the 31 sh_test skips instead of passing silently.
if [[ "$(grep -c -F -e '31 sh_test skips' "$ci")" -ge "2" ]]; then
  ok
else
  bad "ci.yml lost the non-Linux budgeted-skips record (want macos arm64 plus windows with 31 sh_test skips, issue #769; x86_64 removed per #976)"
fi

# Promotion checklist wires the budget as platform evidence per cell,
# so a silent skip never promotes.
if grep -q -F -e 'skip budget' "$checklist" &&
  grep -q -F -e 'issue #769' "$checklist" &&
  grep -q -F -e 'bazel run //tools/ci:skip_budget_qualification' "$checklist"; then
  ok
else
  bad "promotion-checklist.md lost its issue #769 skip-budget platform-evidence wiring"
fi

# Verification matrix owns the harness entry in the prove battery.
if grep -q -F -e ':skip_budget_qualification' "$verify" &&
  grep -q -F -e 'issue #769' "$verify" &&
  grep -q -F -e 'bazel run //tools/ci:skip_budget_qualification' "$verify"; then
  ok
else
  bad "verification-matrix.md lost its skip_budget_qualification battery entry under #769"
fi

# Verification matrix Green lists this harness count.
if grep -q -F -e '`skip_budget_qualification` 16/16' "$verify"; then
  ok
else
  bad "verification-matrix.md Green lost skip_budget_qualification 16/16"
fi

# BUILD owns the harness target.
if grep -q -F -e 'name = "skip_budget_qualification"' "$build" &&
  grep -q -F -e 'skip_budget_qualification.sh' "$build"; then
  ok
else
  bad "tools/ci/ci_targets_b.bzl lost the skip_budget_qualification target"
fi

# CI wires the harness in prove plus dogfood-freshness.
if grep -q -F -e 'bazel run --noshow_progress //tools/ci:skip_budget_qualification' "$prove" &&
  grep -q -F -e 'bazel run --noshow_progress //tools/ci:skip_budget_qualification' "$dogfood"; then
  ok
else
  bad "tools/ci/prove.sh or dogfood_freshness.sh lost the skip_budget_qualification wiring (want both, issue #769)"
fi

# Live proof: the queried inventory matches the documented inventory,
# so the budget gates the real scope, not a stale pin.
if [[ "$shtest_linux" == "31" ]] && [[ "$shbin_linux" == "127" ]] && [[ "$labels" == "162" ]]; then
  ok
else
  bad "live inventory drifted from the tools.md pin (want 31 sh_test plus 127 sh_binary plus 162 labels, found $shtest_linux plus $shbin_linux plus $labels; update budget plus docs in one reviewed PR, issue #769)"
fi

dx_test_summary "per-host skip budget qualification harness"
