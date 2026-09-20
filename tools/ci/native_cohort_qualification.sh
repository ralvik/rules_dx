#!/usr/bin/env bash
# Native-cohort qualification harness.
#
# Qualifies the as-built native quality-cohort record with fixture
# evidence and owned gaps, without claiming Supported and without a false
# adapter claim:
# - delivered: decided split native route for clang-format/clang-tidy via
#   the qualified hermetic-llvm LLVM tool targets (authoritative-toolchain
#   class, no separate acquisition; clang-tidy compile-commands context
#   keeps target-coupled wiring versus check-only as recorded open work,
#   never silent) plus cppcheck as a standalone checksummed-artifact
#   candidate (`--xml --xml-version=2` on stderr), and decided split Go
#   route for gofumpt (strict gofmt superset, standalone artifact) plus
#   `govet` from the authoritative Go toolchain (no separate acquisition)
#   plus staticcheck/errcheck as standalone artifacts (staticcheck default
#   checks with the `SA`-only shortcut rejected, neither provisional;
# versions plus rule-sets qualified seed-only),
#   `govet`/errcheck text-parse `file:line[:col]: message`), whole-file
#   rewrite versus check-only fix modes with the provisional
#   sandbox-apply-and-diff flow, initial artifact research rows as
#   observations for digests (versions qualified seed-only under issue
# , provisional adapter-input notes (clang-tidy
#   `--export-fixes` YAML, staticcheck SARIF/JSON shapes, C/C++
#   MSVC-interop plus SDK licensing staying with the Windows platform
# issue), native-config defaults qualified seed-only
#   (staticcheck default checks, govet default analyzers with errcheck
#   complementary, clang-tidy default checks, cppcheck default enablement
#   as upstream built-in defaults with no hidden preset),
#   parity-deferred c/cpp/go with owner plus frozen
#   route, classification-only taxonomy with no curated defaults and no
#   native-config binding;
# - open under with honest records: exact artifact digests
#   plus toolchain qualification, parser plus runner-matrix pass/fail plus
#   fix/format evidence per adapter-backed class, native-config
#   qualification against the native-config contract, platform plus
#   consumer plus release evidence. REAL_ADAPTERS claims c/cpp/go only
#   when green.
#
# Versioned here, run by CI via `bazel run //tools/ci:native_cohort_qualification`,
# following //tools/ci:scala_dotnet_cohort_qualification.
set -euo pipefail

# Shared workspace + runfiles helpers.
# Bootstrap via tools/sh/bootstrap.sh (issue #654): runfiles forest first, then source tree.
source "${RUNFILES_DIR:-/dev/null}/_main/tools/sh/bootstrap.sh" 2>/dev/null || source "${TEST_SRCDIR:-/dev/null}/_main/tools/sh/bootstrap.sh" 2>/dev/null || source "$0.runfiles/_main/tools/sh/bootstrap.sh" 2>/dev/null || source "${BASH_SOURCE[0]}.runfiles/_main/tools/sh/bootstrap.sh" 2>/dev/null || source "$(git rev-parse --show-toplevel 2>/dev/null)/tools/sh/bootstrap.sh"
dx_bootstrap "tools/sh/lib.sh"

dx_cd_workspace

dx_test_init

support="docs/product/support-matrix.md"
baseline="docs/tools/tool-baseline.md"
acquisition="docs/tools/tool-acquisition.md"
integrations="docs/quality/tool-integrations.md"
native_doc="docs/quality/native-configuration.md"
adapters="quality/adapters.bzl"
parity="quality/parity_tests.bzl"
curated="quality/curated_defaults.bzl"
native="quality/native_config.bzl"
matrix="quality/testdata/runner_matrix_cases.bzl"
subjects="quality/testdata/BUILD.bazel"
aspects="quality/real_aspects.bzl"

# No false adapter claim for the native cohort: none of the cohort tool
# IDs appear in REAL_ADAPTERS. Classification exists in
# REAL_CLASS_TO_FAMILY; adapter claim does not.
cohort_claim=""
for tool in clang-format clang-tidy cppcheck gofmt gofumpt staticcheck govet errcheck; do
  if grep -q -F -e "\"$tool\":" "$adapters"; then
    cohort_claim="$cohort_claim $tool:claimed"
  fi
done
if [[ -z "$cohort_claim" ]]; then
  ok
else
  bad "false adapter claim for native cohort:$cohort_claim"
fi

# Parity deferrals own c/cpp/go with owner plus frozen route plus the
# live-successor record (closed owns nothing here).
if grep -q -F -e '"c": ["ADR 0019"' "$parity" &&
  grep -q -F -e '"cpp": ["ADR 0019"' "$parity" &&
  grep -q -F -e '"go": ["ADR 0019"' "$parity" &&
  grep -q -F -e 'authoritative hermetic-llvm toolchain (clang-format, clang-tidy); cppcheck standalone artifact' "$parity" &&
  grep -q -F -e 'authoritative Go toolchain (gofmt/gofumpt); staticcheck/govet standalone artifacts' "$parity" &&
  grep -q -F -e 'issue #418' "$parity"; then
  ok
else
  bad "parity deferrals lost the native c/cpp/go owner plus frozen route plus #418 record"
fi

# Every cohort class stays classified in the frozen taxonomy, one family each.
if grep -q -F -e '"c": "cc"' "$adapters" &&
  grep -q -F -e '"cpp": "cc"' "$adapters" &&
  grep -q -F -e '"go": "go"' "$adapters"; then
  ok
else
  bad "frozen taxonomy lost the c/cpp/go classification"
fi

# Classification-only today: cc/go families carry no curated defaults
# (curated membership unchanged; no native default selected).
cohort_curated=""
for family in '"cc": {' '"go": {'; do
  if grep -q -F -e "$family" "$curated"; then
    cohort_curated="$cohort_curated $family:claimed"
  fi
done
if [[ -z "$cohort_curated" ]]; then
  ok
else
  bad "curated defaults claim a native family before adapters land:$cohort_curated"
fi

# No hidden native native-config preset: no cohort binding exists in the
# typed native-config rules (adapters run pinned upstream defaults until
# qualifies checked-in policy against the native-config contract; the
# provisional clang-tidy/cppcheck/staticcheck suggestions stay review
# inputs, never supplied configs).
cohort_config=""
for tool in clang-format clang-tidy cppcheck gofumpt staticcheck govet errcheck; do
  if grep -q -F -e "${tool}_config" "$native"; then
    cohort_config="$cohort_config $tool:preset"
  fi
done
if [[ -z "$cohort_config" ]]; then
  ok
else
  bad "native-config carries a hidden native preset:$cohort_config"
fi

# No false green claim: the runner matrix carries no native cells yet, so
# REAL_ADAPTERS cannot claim c/cpp/go (claims land only with green
# pass/fail plus fix/format evidence per adapter-backed class). The
# `matrix_go_` prefix also covers a future `matrix_go_module_` cell:
# modfmt stays out of scope, so such a cell re-scopes this guard.
if ! grep -q -F -e 'matrix_c_' "$matrix" &&
  ! grep -q -F -e 'matrix_cpp_' "$matrix" &&
  ! grep -q -F -e 'matrix_go_' "$matrix"; then
  ok
else
  bad "runner matrix claims a native cell without adapter qualification"
fi

# Tool acquisition keeps the decided split native route for
# clang-format/clang-tidy/cppcheck with the compile-commands decision
# recorded explicitly plus MSVC-interop scoping and no false claim;
# versions qualified under, digests plus adapters stay pending
# under (live successor to closed for the c/cpp classes).
if grep -q -F -e 'Decided route: clang-format, clang-tidy, and cppcheck take the' "$acquisition" &&
  grep -q -F -e 'no separate acquisition' "$acquisition" &&
  grep -q -F -e 'compile-commands' "$acquisition" &&
  grep -q -F -e 'stays open under issue #418' "$acquisition" &&
  grep -q -F -e '--xml --xml-version=2' "$acquisition" &&
  grep -q -F -e 'no adapter claims `c` or `cpp` yet' "$acquisition" &&
  grep -q -F -e '(open under issue #418)' "$acquisition" &&
  grep -q -F -e 'MSVC-interop and SDK licensing stay with' "$acquisition" &&
  grep -q -F -e 'qualified seed-only under issue #487' "$acquisition" &&
  grep -q -F -e 'digests plus adapter mappings stay owned under issue #418' "$acquisition"; then
  ok
else
  bad "tool-acquisition lost its decided native route with #487 versions plus #418 digests/adapters split"
fi

# Tool acquisition keeps the decided split Go route for
# gofumpt/staticcheck/govet/errcheck with the SA-only conflict resolved
# (default checks qualified, SA-only shortcut rejected) and no false claim;
# versions qualified under, digests plus adapters stay pending under.
if grep -q -F -e 'Decided route: gofumpt, staticcheck, govet, and errcheck take the' "$acquisition" &&
  grep -q -F -e 'strict superset of gofmt' "$acquisition" &&
  grep -q -F -e 'SA`-only shortcut is rejected without qualification' "$acquisition" &&
  grep -q -F -e 'qualified seed-only under issue #487' "$acquisition" &&
  grep -q -F -e 'digests plus adapter mappings stay owned under issue #418' "$acquisition" &&
  grep -q -F -e 'sandbox-apply-and-diff' "$acquisition" &&
  grep -q -F -e 'file:line[:col]: message' "$acquisition" &&
  grep -q -F -e 'no adapter claims `go`' "$acquisition" &&
  grep -q -F -e '(open under issue #418)' "$acquisition"; then
  ok
else
  bad "tool-acquisition lost its decided Go route with #487 versions plus #418 digests/adapters split"
fi

# Tool acquisition keeps initial artifact research rows for the cohort as
# observations for digests (versions qualified seed-only),
# with byte-identity risk explicit.
cohort_research=""
for tool in '| clang-format |' '| clang-tidy |' '| cppcheck |' '| gofumpt |' '| staticcheck |' '| govet |' '| errcheck |'; do
  grep -q -F -e "$tool" "$acquisition" || cohort_research="$cohort_research $tool:missing"
done
if [[ -z "$cohort_research" ]] &&
  grep -q -F -e 'owned by issue #418' "$acquisition" &&
  grep -q -F -e 'observations, not pins' "$acquisition" &&
  grep -q -F -e 'maintainer acquisition must establish and record byte identity' "$acquisition"; then
  ok
else
  bad "tool-acquisition lost a native research row or its observations-not-pins honesty:$cohort_research"
fi

# Tool integrations keep the native adapter-input notes:
# clang-tidy target-coupled versus check-only recorded not silent, cppcheck
# XML on stderr, staticcheck default checks qualified with the SA-only
# shortcut rejected, govet/errcheck text-parse, MSVC-interop scoping,
# versions qualified under with digests as observations, no adapter claim.
if grep -q -F -e '**Native cohort (issue #418' "$integrations" &&
  grep -q -F -e 'no adapter claims `c`, `cpp`, or `go` yet' "$integrations" &&
  grep -q -F -e 'target-coupled wiring versus check-only decision is recorded here, not silent' "$integrations" &&
  grep -q -F -e '--xml --xml-version=2' "$integrations" &&
  grep -q -F -e 'shortcut is rejected without qualification' "$integrations" &&
  grep -q -F -e 'qualified seed-only under issue #487' "$integrations" &&
  grep -q -F -e 'adapters stay owned under issue #418' "$integrations" &&
  grep -q -F -e 'file:line[:col]: message' "$integrations" &&
  grep -q -F -e 'MSVC-interop and SDK licensing stay with the Windows' "$integrations" &&
  grep -q -F -e 'observations,' "$integrations" &&
  grep -q -F -e 'not pins' "$integrations"; then
  ok
else
  bad "tool-integrations lost its native adapter-input notes with #487 versions plus #418 adapters split"
fi

# Support matrix keeps the native routes plus qualified native-config defaults
# plus adapter-input notes plus cohort tracking, all citing for
# adapters/digests without approving hidden presets or claiming support.
if grep -q -F -e 'split native route (issue #418' "$support" &&
  grep -q -F -e 'Go route (issue #418' "$support" &&
  grep -q -F -e 'qualified seed-only under issue #487' "$support" &&
  grep -q -F -e 'cc/tests/fixtures/native_quality/pins.bzl' "$support" &&
  grep -q -F -e 'no hidden preset' "$support" &&
  grep -q -F -e 'issue #418' "$support" &&
  grep -q -F -e 'remaining cohorts stay in' "$support" &&
  grep -q -F -e '(open under issue #418)' "$support" &&
  grep -q -F -e 'under issue #418' "$support" &&
  grep -q -F -e 'owned by issue #418' "$support" &&
  grep -q -F -e 'itemized under issue #418' "$support" &&
  grep -q -F -e '(issue #418)' "$support" &&
  grep -q -F -e 'to issue #418' "$support"; then
  ok
else
  bad "support-matrix lost its native routes, qualified defaults, adapter notes, or #418 cohort tracking"
fi

# Tool baseline keeps the Go/C/C++ coverage rows (integration inventory,
# not a support claim).
if grep -q -F -e '| C and C++ | clang-format | clang-tidy, cppcheck |' "$baseline" &&
  grep -q -F -e '| Go | gofmt, gofumpt |' "$baseline"; then
  ok
else
  bad "tool-baseline lost its Go/C/C++ coverage rows"
fi

# Curated posture unchanged: naming an integration does not enable it by
# default; enabled tools use pinned native defaults unless checked-in
# native config supplies policy (no hidden preset authorized here).
if grep -q -F -e 'Naming a tool in the matrix does not enable it by default' "$baseline" &&
  grep -q -F -e 'Tool selection does not' "$baseline" &&
  grep -q -F -e 'defines no hidden rule' "$native_doc"; then
  ok
else
  bad "curated-default posture lost its no-hidden-preset honesty"
fi

# Functional: parity gate shape still fails closed on drift.
if grep -q -F -e 'REAL_ADAPTERS = {' "$adapters" &&
  grep -q -F -e 'REAL_CLASS_TO_FAMILY = {' "$adapters" &&
  grep -q -F -e 'no adapter class is unclassified' "$parity" &&
  grep -q -F -e 'no classified class lacks a disposition' "$parity"; then
  ok
else
  bad "parity gate lost its fail-closed shape"
fi

dx_test_summary "Native-cohort qualification harness"
