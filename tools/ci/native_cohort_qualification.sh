#!/usr/bin/env bash
# Native-cohort qualification harness.
#
# Qualifies the as-built native quality-cohort record with fixture
# evidence and owned gaps, without claiming Supported:
# - delivered: seven adapters (`clang_format` format c plus cpp plus cuda,
#   `clang_tidy` lint c plus cpp check-only via delegated text
#   diagnostics, `cppcheck` lint c plus cpp via delegated XML,
#   `gofumpt` format go, `staticcheck` lint go via delegated JSON,
#   `govet` lint go via delegated text, `errcheck` lint go via delegated
#   text complementary) over the decided split native route for
#   clang-format/clang-tidy via the qualified hermetic-llvm LLVM tool
#   targets (authoritative-toolchain class, no separate acquisition;
#   clang-tidy compile-commands context rides the authoritative target
#   when present with check-only text diagnostics, never silent) plus
#   cppcheck as a standalone checksummed artifact
#   (`--xml --xml-version=2` on stderr), and decided split Go route for
#   gofumpt (strict gofmt superset, standalone artifact) plus `govet`
#   from the authoritative Go toolchain (no separate acquisition) plus
#   staticcheck/errcheck as standalone artifacts (staticcheck default
#   checks with the `SA`-only shortcut rejected; versions plus rule-sets
#   qualified seed-only), `govet`/errcheck text-parse
#   `file:line:col: message`, whole-file rewrite versus check-only fix
#   modes with the provisional sandbox-apply-and-diff flow, initial
#   artifact research rows as observations for digests (versions qualified
#   seed-only under issue #487), adapter-input notes (clang-tidy
#   check-only decided, staticcheck JSON shape, C/C++ MSVC-interop plus
#   SDK licensing staying with the Windows platform issue),
#   native-config defaults qualified seed-only (staticcheck default
#   checks, govet default analyzers with errcheck complementary,
#   clang-tidy default checks, cppcheck default enablement as upstream
#   built-in defaults with no hidden preset), parity-delivered c/cpp/cuda/go,
#   taxonomy with native bindings;
# - open owned gaps: exact artifact digests plus toolchain qualification,
#   platform plus consumer plus release evidence. REAL_ADAPTERS claims
#   c/cpp/cuda/go green.
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

# Adapters delivered under #798 (successor to closed #418): all seven
# cohort tools appear in REAL_ADAPTERS.
cohort_claim=""
for tool in clang_format clang_tidy cppcheck gofumpt staticcheck govet errcheck; do
  grep -q -F -e "\"$tool\":" "$adapters" || cohort_claim="$cohort_claim $tool:missing"
done
if [[ -z "$cohort_claim" ]]; then
  ok
else
  bad "native adapter delivery missing:$cohort_claim (want all seven under #798)"
fi

# Parity no longer defers the delivered native classes (closed #418 owns
# nothing here; delivery under #798).
if ! grep -q -F -e '"c":' "$parity" &&
  ! grep -q -F -e '"cpp":' "$parity" &&
  ! grep -q -F -e '"cuda":' "$parity" &&
  ! grep -q -F -e '"go":' "$parity"; then
  ok
else
  bad "parity still defers delivered native classes (want none under #798)"
fi

# Every cohort class stays classified in the frozen taxonomy, one family each.
if grep -q -F -e '"c": "cc"' "$adapters" &&
  grep -q -F -e '"cpp": "cc"' "$adapters" &&
  grep -q -F -e '"cuda": "cuda"' "$adapters" &&
  grep -q -F -e '"go": "go"' "$adapters"; then
  ok
else
  bad "frozen taxonomy lost the c/cpp/cuda/go classification"
fi

# Classification-only today: cc/cuda/go families carry no curated defaults
# (curated membership unchanged; no native default selected).
cohort_curated=""
for family in '"cc": {' '"cuda": {' '"go": {'; do
  if grep -q -F -e "$family" "$curated"; then
    cohort_curated="$cohort_curated $family:claimed"
  fi
done
if [[ -z "$cohort_curated" ]]; then
  ok
else
  bad "curated defaults claim a native family before adapters land:$cohort_curated"
fi

# Native native-config bindings delivered under #798.
cohort_config=""
for tool in clang_format clang_tidy cppcheck staticcheck; do
  grep -q -F -e "${tool}_config" "$native" || cohort_config="$cohort_config $tool:missing"
done
if [[ -z "$cohort_config" ]]; then
  ok
else
  bad "native-config lost native bindings:$cohort_config (want all four under #798)"
fi

# Matrix carries native cells with green pass/fail plus fix/format
# evidence per adapter-backed class (delivery under #798).
if grep -q -F -e 'NATIVE_CASES' "$matrix"; then
  ok
else
  bad "runner matrix lost native cells (want NATIVE_CASES under #798)"
fi

# Tool acquisition keeps the decided split native route for
# clang-format/clang-tidy/cppcheck with the check-only decision recorded
# explicitly plus MSVC-interop scoping; versions qualified under #487,
# digests plus adapters owned under #798 (live successor to closed #418
# for the c/cpp classes).
if grep -q -F -e 'Decided route: clang-format, clang-tidy, and cppcheck take the' "$acquisition" &&
  grep -q -F -e 'no separate acquisition' "$acquisition" &&
  grep -q -F -e 'compile-commands' "$acquisition" &&
  grep -q -F -e 'stays open under #798 (successor to closed #418)' "$acquisition" &&
  grep -q -F -e '--xml --xml-version=2' "$acquisition" &&
  grep -q -F -e 'no adapter claims `c` or `cpp` yet' "$acquisition" &&
  grep -q -F -e '(open under #798, successor to closed #418)' "$acquisition" &&
  grep -q -F -e 'MSVC-interop and SDK licensing stay with' "$acquisition" &&
  grep -q -F -e 'qualified seed-only under issue #487' "$acquisition" &&
  grep -q -F -e 'digests plus adapter mappings stay owned under #798 (successor to closed #418)' "$acquisition"; then
  ok
else
  bad "tool-acquisition lost its decided native route with #487 versions plus #798 digests/adapters split"
fi

# Tool acquisition keeps the decided split Go route for
# gofumpt/staticcheck/govet/errcheck with the SA-only conflict resolved
# (default checks qualified, SA-only shortcut rejected); versions
# qualified under #487, digests plus adapters owned under #798.
if grep -q -F -e 'Decided route: gofumpt, staticcheck, govet, and errcheck take the' "$acquisition" &&
  grep -q -F -e 'strict superset of gofmt' "$acquisition" &&
  grep -q -F -e '`SA`-only shortcut is rejected without qualification' "$acquisition" &&
  grep -q -F -e 'qualified seed-only under issue #487' "$acquisition" &&
  grep -q -F -e 'digests plus adapter mappings stay owned under #798 (successor to closed #418)' "$acquisition" &&
  grep -q -F -e 'sandbox-apply-and-diff' "$acquisition" &&
  grep -q -F -e 'file:line[:col]: message' "$acquisition" &&
  grep -q -F -e 'no adapter claims `go`' "$acquisition" &&
  grep -q -F -e '(open under #798, successor to closed #418)' "$acquisition"; then
  ok
else
  bad "tool-acquisition lost its decided Go route with #487 versions plus #798 digests/adapters split"
fi

# Tool acquisition keeps initial artifact research rows for the cohort as
# observations for digests (versions qualified seed-only),
# with byte-identity risk explicit.
cohort_research=""
for tool in '| clang-format |' '| clang-tidy |' '| cppcheck |' '| gofumpt |' '| staticcheck |' '| govet |' '| errcheck |'; do
  grep -q -F -e "$tool" "$acquisition" || cohort_research="$cohort_research $tool:missing"
done
if [[ -z "$cohort_research" ]] &&
  grep -q -F -e 'successor to closed #418' "$acquisition" &&
  grep -q -F -e 'observations, not pins' "$acquisition" &&
  grep -q -F -e 'maintainer acquisition must establish and record byte identity' "$acquisition"; then
  ok
else
  bad "tool-acquisition lost a native research row or its observations-not-pins honesty:$cohort_research"
fi

# Tool integrations record the native adapter delivery under #798
# (successor to the #418 provisional notes): decided routes plus the
# check-only decision plus versions qualified under #487, adapters
# qualified under #798.
if grep -q -F -e 'Native cohort (#798' "$integrations" &&
  grep -q -F -e 'adapters `clang_format` (format `c`' "$integrations" &&
  grep -q -F -e 'check-only with delegated text diagnostics' "$integrations" &&
  grep -q -F -e '--xml --xml-version=2' "$integrations" &&
  grep -q -F -e 'shortcut is rejected without qualification' "$integrations" &&
  grep -q -F -e 'qualified seed-only under issue #487' "$integrations" &&
  grep -q -F -e 'adapters qualified seed-only' "$integrations" &&
  grep -q -F -e 'under #798' "$integrations" &&
  grep -q -F -e 'MSVC-interop and SDK licensing stay with the Windows' "$integrations" &&
  grep -q -F -e 'observations,' "$integrations" &&
  grep -q -F -e 'not pins' "$integrations"; then
  ok
else
  bad "tool-integrations lost its native adapter delivery record under #798"
fi

# Support matrix keeps the native coverage rows plus the #798 delivery
# record without claiming support.
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
