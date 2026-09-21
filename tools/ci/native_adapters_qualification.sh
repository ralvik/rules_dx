#!/usr/bin/env bash
# Native adapters qualification harness (issue #798).
#
# Qualifies the as-built native adapter delivery with fixture evidence
# and owned gaps, without claiming Supported:
# - delivered: seven adapters (`clang_format` format c plus cpp,
#   `clang_tidy` lint c plus cpp via delegated text diagnostics,
#   `cppcheck` lint c plus cpp via delegated XML diagnostics, `gofumpt`
#   format go, `staticcheck` lint go via delegated JSON diagnostics,
#   `govet` lint go via delegated text diagnostics, `errcheck` lint go
#   via delegated text diagnostics complementary) over the decided routes
#   (split native route via the qualified hermetic-llvm LLVM tool targets
#   with no separate acquisition plus cppcheck as a standalone
#   checksummed artifact; split Go route with gofumpt as a strict gofmt
#   superset plus staticcheck/govet/errcheck with errcheck
#   complementary); per-tool fixtures with pins plus samples; Layer-2
#   matrix pass plus fail cells per tool (format via fake shell doubles
#   seed-only, lint via delegated recorded diagnostics like
#   Clippy/rustc); parsers with pass plus fail samples; native-config
#   bindings for the four configurable tools; runner dispatch plus fix
#   flows (formatters whole-file rewrite, lint check-only via
#   sandbox-apply-and-diff); Layer-2 verification cells flipped from Open
#   (adapter-less) to Delivered;
# - open owned gaps: exact artifact digests plus hermetic-llvm/Go
#   toolchain bounds, platform plus consumer plus release evidence, no
#   Supported claim.
#
# Versioned here, run by CI via `bazel run //tools/ci:native_adapters_qualification`,
# following //tools/ci:native_quality_qualification.
set -euo pipefail

# Shared workspace + runfiles helpers.
# Bootstrap via tools/sh/bootstrap.sh (issue #654): runfiles forest first, then source tree.
source "${RUNFILES_DIR:-/dev/null}/_main/tools/sh/bootstrap.sh" 2>/dev/null || source "${TEST_SRCDIR:-/dev/null}/_main/tools/sh/bootstrap.sh" 2>/dev/null || source "$0.runfiles/_main/tools/sh/bootstrap.sh" 2>/dev/null || source "${BASH_SOURCE[0]}.runfiles/_main/tools/sh/bootstrap.sh" 2>/dev/null || source "$(git rev-parse --show-toplevel 2>/dev/null)/tools/sh/bootstrap.sh"
dx_bootstrap "tools/sh/lib.sh"

dx_cd_workspace

dx_test_init

adapters="quality/adapters.bzl"
parity="quality/parity_tests.bzl"
native="quality/native_config.bzl"
matrix="quality/testdata/runner_matrix_native.bzl"
cases="quality/testdata/runner_matrix_cases.bzl"
subjects="quality/testdata/BUILD.bazel"
real_rs="quality/runner/src/real.rs"
commands="quality/adapter/src/commands.rs"
integrations="docs/quality/tool-integrations.md"
runner_doc="docs/quality/runner-matrix.md"
verify="docs/testing/verification-matrix.md"
targets="tools/ci/ci_targets_d.bzl"
dogfood="tools/ci/dogfood_freshness.sh"

# Per-tool fixture dirs stay present (three C/C++ plus four Go fixtures).
if [[ -d "cc/tests/fixtures/clang_format" && -d "cc/tests/fixtures/clang_tidy" && -d "cc/tests/fixtures/cppcheck" && -d "go/tests/fixtures/gofumpt" && -d "go/tests/fixtures/staticcheck" && -d "go/tests/fixtures/govet" && -d "go/tests/fixtures/errcheck" ]]; then
  ok
else
  bad "native adapter fixtures missing (want clang_format plus clang_tidy plus cppcheck plus gofumpt plus staticcheck plus govet plus errcheck dirs)"
fi

# Format fixtures pin versions plus routes plus invocation shapes.
if grep -q -F -e 'CLANG_FORMAT_VERSION = "hermetic-llvm v0.8.19 (LLVM 23.1.0)"' cc/tests/fixtures/clang_format/pins.bzl &&
  grep -q -F -e 'GOFUMPT_VERSION = "v0.11.0"' go/tests/fixtures/gofumpt/pins.bzl &&
  grep -q -F -e 'authoritative hermetic-llvm LLVM tool targets' cc/tests/fixtures/clang_format/pins.bzl &&
  grep -q -F -e 'strict superset of gofmt' go/tests/fixtures/gofumpt/pins.bzl; then
  ok
else
  bad "format fixtures lost their version plus route pins under issue #798"
fi

# Lint fixtures pin versions plus routes plus invocation shapes.
if grep -q -F -e 'CLANG_TIDY_VERSION = "hermetic-llvm v0.8.19 (LLVM 23.1.0)"' cc/tests/fixtures/clang_tidy/pins.bzl &&
  grep -q -F -e 'CPPCHECK_VERSION = "2.21.0"' cc/tests/fixtures/cppcheck/pins.bzl &&
  grep -q -F -e 'STATICCHECK_VERSION = "2026.2"' go/tests/fixtures/staticcheck/pins.bzl &&
  grep -q -F -e 'GOVET_TOOLCHAIN_VERSION = "1.26.6"' go/tests/fixtures/govet/pins.bzl &&
  grep -q -F -e 'ERRCHECK_VERSION = "v1.20.0"' go/tests/fixtures/errcheck/pins.bzl &&
  grep -q -F -e 'check-only' cc/tests/fixtures/clang_tidy/pins.bzl &&
  grep -q -F -e 'complementary' go/tests/fixtures/errcheck/pins.bzl; then
  ok
else
  bad "lint fixtures lost their version plus route pins under issue #798"
fi

# REAL_ADAPTERS claims all seven cohort tools with the right families.
if grep -q -F -e '"clang_format": {"format": ["c", "cpp"]}' "$adapters" &&
  grep -q -F -e '"clang_tidy": {"lint": ["c", "cpp"]}' "$adapters" &&
  grep -q -F -e '"cppcheck": {"lint": ["c", "cpp"]}' "$adapters" &&
  grep -q -F -e '"gofumpt": {"format": ["go"]}' "$adapters" &&
  grep -q -F -e '"staticcheck": {"lint": ["go"]}' "$adapters" &&
  grep -q -F -e '"govet": {"lint": ["go"]}' "$adapters" &&
  grep -q -F -e '"errcheck": {"lint": ["go"]}' "$adapters"; then
  ok
else
  bad "REAL_ADAPTERS lost a native cohort claim (want all seven under issue #798)"
fi

# Parity no longer defers the three delivered classes.
if ! grep -q -F -e '"c":' "$parity" &&
  ! grep -q -F -e '"cpp":' "$parity" &&
  ! grep -q -F -e '"go":' "$parity"; then
  ok
else
  bad "parity still defers a delivered native class (want none under issue #798)"
fi

# Matrix carries all sixteen cohort cells.
cohort_matrix=""
for cell in matrix_c_format_pass matrix_c_format_fail matrix_cpp_format_pass matrix_cpp_format_fail matrix_c_lint_pass matrix_c_lint_fail matrix_cpp_lint_pass matrix_cpp_lint_fail matrix_go_format_pass matrix_go_format_fail matrix_go_staticcheck_pass matrix_go_staticcheck_fail matrix_go_govet_pass matrix_go_govet_fail matrix_go_errcheck_pass matrix_go_errcheck_fail; do
  grep -q -F -e "$cell" "$matrix" || cohort_matrix="$cohort_matrix $cell:missing"
done
if [[ -z "$cohort_matrix" ]] && grep -q -F -e 'NATIVE_CASES' "$cases"; then
  ok
else
  bad "runner matrix lost native cells:$cohort_matrix (want all sixteen under issue #798)"
fi

# Every cohort tool keeps its parser with pass plus fail samples.
cohort_parser=""
for tool in clang_format clang_tidy cppcheck gofumpt staticcheck govet errcheck; do
  [[ -f "quality/adapter/src/parsers/$tool.rs" ]] || cohort_parser="$cohort_parser $tool:missing"
done
if [[ -z "$cohort_parser" ]] &&
  grep -q -F -e 'pub fn parse_clang_format' quality/adapter/src/parsers/clang_format.rs &&
  grep -q -F -e 'pub fn parse_clang_tidy' quality/adapter/src/parsers/clang_tidy.rs &&
  grep -q -F -e 'pub fn parse_cppcheck' quality/adapter/src/parsers/cppcheck.rs &&
  grep -q -F -e 'pub fn parse_gofumpt' quality/adapter/src/parsers/gofumpt.rs &&
  grep -q -F -e 'pub fn parse_staticcheck' quality/adapter/src/parsers/staticcheck.rs &&
  grep -q -F -e 'pub fn parse_govet' quality/adapter/src/parsers/govet.rs &&
  grep -q -F -e 'pub fn parse_errcheck' quality/adapter/src/parsers/errcheck.rs; then
  ok
else
  bad "parsers lost a native tool:$cohort_parser (want all seven under issue #798)"
fi

# Native-config binds the four configurable tools.
if grep -q -F -e 'clang_format_config' "$native" &&
  grep -q -F -e 'clang_tidy_config' "$native" &&
  grep -q -F -e 'cppcheck_config' "$native" &&
  grep -q -F -e 'staticcheck_config' "$native"; then
  ok
else
  bad "native-config lost a native binding (want clang_format plus clang_tidy plus cppcheck plus staticcheck under issue #798)"
fi

# Runner dispatches all seven tools.
cohort_dispatch=""
for tool in '"clang_format"' '"clang_tidy"' '"cppcheck"' '"gofumpt"' '"staticcheck"' '"govet"' '"errcheck"'; do
  grep -q -F -e "$tool" "$real_rs" || cohort_dispatch="$cohort_dispatch $tool:missing"
done
if [[ -z "$cohort_dispatch" ]]; then
  ok
else
  bad "runner lost native dispatch:$cohort_dispatch (want all seven under issue #798)"
fi

# Commands pin the nine invocation shapes.
if grep -q -F -e 'pub fn clang_format_check' "$commands" &&
  grep -q -F -e 'pub fn clang_format_fix' "$commands" &&
  grep -q -F -e 'pub fn gofumpt_check' "$commands" &&
  grep -q -F -e 'pub fn gofumpt_fix' "$commands" &&
  grep -q -F -e 'pub fn clang_tidy_check' "$commands" &&
  grep -q -F -e 'pub fn cppcheck_check' "$commands" &&
  grep -q -F -e 'pub fn staticcheck_check' "$commands" &&
  grep -q -F -e 'pub fn govet_check' "$commands" &&
  grep -q -F -e 'pub fn errcheck_check' "$commands"; then
  ok
else
  bad "commands lost a native invocation shape (want all nine under issue #798)"
fi

# Runner-matrix doc owns the three cohort sections.
if grep -q -F -e '## C (`c`: clang-format format, clang-tidy lint)' "$runner_doc" &&
  grep -q -F -e '## C++ (`cpp`: clang-format format, cppcheck lint)' "$runner_doc" &&
  grep -q -F -e '## Go (`go`: gofumpt format, staticcheck/govet/errcheck lint)' "$runner_doc" &&
  grep -q -F -e 'opt-in adapters delivered under #798' "$runner_doc"; then
  ok
else
  bad "runner-matrix doc lost its native sections under issue #798"
fi

# Tool integrations claim the seven adapters over the decided routes.
if grep -q -F -e 'adapters `clang_format` (format `c`' "$integrations" &&
  grep -q -F -e '`cppcheck` (lint `c`, `cpp`)' "$integrations" &&
  grep -q -F -e '`gofumpt` (format `go`)' "$integrations" &&
  grep -q -F -e 'adapters qualified seed-only' "$integrations" &&
  grep -q -F -e 'under #798' "$integrations" &&
  grep -q -F -e 'native_adapters_qualification' "$integrations"; then
  ok
else
  bad "tool-integrations lost its native adapter claims under issue #798"
fi

# Verification matrix flips the two Layer-2 cells to Delivered.
if grep -q -F -e '| Go | Delivered (code ownership) | Delivered |' "$verify" &&
  grep -q -F -e '| C++ | Delivered (code ownership) | Delivered |' "$verify" &&
  grep -q -F -e 'Native Layer-2 delivered' "$verify" &&
  grep -q -F -e 'under #798' "$verify"; then
  ok
else
  bad "verification-matrix lost its native Layer-2 Delivered flip under issue #798"
fi

# Verification matrix lists the harness in dogfood-freshness plus Green.
if grep -q -F -e ':native_adapters_qualification' "$verify" &&
  grep -q -F -e '`native_adapters_qualification` 19/19' "$verify"; then
  ok
else
  bad "verification-matrix lost its #798 adapters harness record (want dogfood plus Green 19/19)"
fi

# Targets own the harness plus dogfood wires it.
if grep -q -F -e 'name = "native_adapters_qualification"' "$targets" &&
  grep -q -F -e 'bazel run --noshow_progress //tools/ci:native_adapters_qualification' "$dogfood"; then
  ok
else
  bad "ci_targets or dogfood lost the native_adapters_qualification wiring (want target plus dogfood-freshness)"
fi

# Fake format doubles stay present for the seed-only matrix cells.
if [[ -f "quality/testdata/fake_clang_format.sh" && -f "quality/testdata/fake_gofumpt.sh" ]] &&
  grep -q -F -e 'fake_clang_format' "$subjects" &&
  grep -q -F -e 'fake_gofumpt' "$subjects"; then
  ok
else
  bad "matrix fake doubles missing (want two fake_*.sh plus BUILD targets under issue #798)"
fi

# Live proof: adapter plus runner unit suites stay green.
if bazel test //quality/adapter:all //quality/runner:all --noshow_progress >/dev/null 2>&1; then
  ok
else
  bad "native adapters live proof failed (want //quality/adapter:all plus //quality/runner:all green)"
fi

# Live proof: the sixteen cohort matrix cells stay green.
if bazel test //quality/testdata:matrix_c_format_pass //quality/testdata:matrix_c_format_fail //quality/testdata:matrix_cpp_format_pass //quality/testdata:matrix_cpp_format_fail //quality/testdata:matrix_c_lint_pass //quality/testdata:matrix_c_lint_fail //quality/testdata:matrix_cpp_lint_pass //quality/testdata:matrix_cpp_lint_fail //quality/testdata:matrix_go_format_pass //quality/testdata:matrix_go_format_fail //quality/testdata:matrix_go_staticcheck_pass //quality/testdata:matrix_go_staticcheck_fail //quality/testdata:matrix_go_govet_pass //quality/testdata:matrix_go_govet_fail //quality/testdata:matrix_go_errcheck_pass //quality/testdata:matrix_go_errcheck_fail --noshow_progress >/dev/null 2>&1; then
  ok
else
  bad "native matrix live proof failed (want sixteen cohort cells green)"
fi

# Live proof: foundation fixtures stay green (adapters change only).
if bazel test //cc/tests/fixtures/hello:hello_test //go/tests/fixtures/hello:hello_test --noshow_progress >/dev/null 2>&1; then
  ok
else
  bad "native adapters live proof failed (want cc plus go hello green)"
fi

dx_test_summary "native adapters qualification harness"
