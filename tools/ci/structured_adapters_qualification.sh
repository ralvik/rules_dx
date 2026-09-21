#!/usr/bin/env bash
# Structured adapters qualification harness (issue #799).
#
# Qualifies the as-built Structured adapter delivery with fixture evidence
# and owned gaps, without claiming Supported:
# - delivered: three adapters (`buf` format plus lint protobuf via native
#   JSONL/diff, `qmlformat` format qml via path listing, `qmllint` lint
#   qml via JSON) over the decided routes (checksummed
#   native/self-contained artifact for buf with no target compiler
#   context plus execution-platform laziness; authoritative Qt
#   distribution for qmlformat/qmllint, Qt-last ordering decided);
#   per-tool fixtures with pins plus samples; Layer-2 matrix pass plus
#   fail cells per tool (format via fake shell doubles seed-only, lint
#   via delegated recorded diagnostics like Clippy/rustc); parsers with
#   pass plus fail samples; native-config bindings for the three tools;
#   runner dispatch plus fix flows (formatters whole-file rewrite, lint
#   check-only via sandbox-apply-and-diff); Layer-2 runner-matrix cells
#   delivered;
# - open owned gaps: exact artifact digests plus Qt distribution
#   identity plus licensing plus platform artifacts, platform plus
#   consumer plus release evidence, no Supported claim.
#
# Versioned here, run by CI via `bazel run //tools/ci:structured_adapters_qualification`,
# following //tools/ci:scala_dotnet_adapters_qualification.
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
matrix="quality/testdata/runner_matrix_structured.bzl"
cases="quality/testdata/runner_matrix_cases.bzl"
subjects="quality/testdata/BUILD.bazel"
real_rs="quality/runner/src/real.rs"
commands="quality/adapter/src/commands.rs"
integrations="docs/quality/tool-integrations.md"
runner_doc="docs/quality/runner-matrix.md"
verify="docs/testing/verification-matrix.md"
targets="tools/ci/ci_targets_d.bzl"
dogfood="tools/ci/dogfood_freshness.sh"

# Per-tool fixture dirs stay present (buf plus qmlformat plus qmllint).
if [[ -d "quality/tests/fixtures/buf" && -d "quality/tests/fixtures/qmlformat" && -d "quality/tests/fixtures/qmllint" ]]; then
  ok
else
  bad "structured adapter fixtures missing (want buf plus qmlformat plus qmllint dirs)"
fi

# Fixtures pin versions plus routes plus invocation shapes.
if grep -q -F -e 'BUF_VERSION = "1.72.0"' quality/tests/fixtures/buf/pins.bzl &&
  grep -q -F -e 'QMLFORMAT_QT_OBSERVED = "Qt 6.11.2"' quality/tests/fixtures/qmlformat/pins.bzl &&
  grep -q -F -e 'QMLLINT_QT_OBSERVED = "Qt 6.11.1"' quality/tests/fixtures/qmllint/pins.bzl &&
  grep -q -F -e 'self-contained per-platform binaries' quality/tests/fixtures/buf/pins.bzl &&
  grep -q -F -e 'qualified Qt distribution' quality/tests/fixtures/qmlformat/pins.bzl &&
  grep -q -F -e 'qualified Qt distribution' quality/tests/fixtures/qmllint/pins.bzl; then
  ok
else
  bad "structured fixtures lost their version plus route pins under issue #799"
fi

# REAL_ADAPTERS claims all three cohort tools with the right families.
if grep -q -F -e '"buf": {' "$adapters" &&
  grep -q -F -e '"qmlformat": {"format": ["qml"]}' "$adapters" &&
  grep -q -F -e '"qmllint": {"lint": ["qml"]}' "$adapters"; then
  ok
else
  bad "REAL_ADAPTERS lost a Structured cohort claim (want all three under issue #799)"
fi

# Parity no longer defers the two delivered classes.
if ! grep -q -F -e '"protobuf":' "$parity" &&
  ! grep -q -F -e '"qml":' "$parity"; then
  ok
else
  bad "parity still defers a delivered Structured class (want none under issue #799)"
fi

# Matrix carries all eight cohort cells.
cohort_matrix=""
for cell in matrix_protobuf_format_pass matrix_protobuf_format_fail matrix_protobuf_lint_pass matrix_protobuf_lint_fail matrix_qml_format_pass matrix_qml_format_fail matrix_qml_lint_pass matrix_qml_lint_fail; do
  grep -q -F -e "$cell" "$matrix" || cohort_matrix="$cohort_matrix $cell:missing"
done
if [[ -z "$cohort_matrix" ]] && grep -q -F -e 'STRUCTURED_CASES' "$cases"; then
  ok
else
  bad "runner matrix lost Structured cells:$cohort_matrix (want all eight under issue #799)"
fi

# Every cohort tool keeps its parser with pass plus fail samples.
cohort_parser=""
for tool in buf qmlformat qmllint; do
  [[ -f "quality/adapter/src/parsers/$tool.rs" ]] || cohort_parser="$cohort_parser $tool:missing"
done
if [[ -z "$cohort_parser" ]] &&
  grep -q -F -e 'pub fn parse_buf_lint' quality/adapter/src/parsers/buf.rs &&
  grep -q -F -e 'pub fn parse_buf_format' quality/adapter/src/parsers/buf.rs &&
  grep -q -F -e 'pub fn parse_qmlformat' quality/adapter/src/parsers/qmlformat.rs &&
  grep -q -F -e 'pub fn parse_qmllint' quality/adapter/src/parsers/qmllint.rs; then
  ok
else
  bad "parsers lost a Structured tool:$cohort_parser (want all three under issue #799)"
fi

# Native-config binds the three tools.
if grep -q -F -e 'buf_config' "$native" &&
  grep -q -F -e 'qmlformat_config' "$native" &&
  grep -q -F -e 'qmllint_config' "$native"; then
  ok
else
  bad "native-config lost a Structured binding (want buf plus qmlformat plus qmllint under issue #799)"
fi

# Runner dispatches all three tools.
cohort_dispatch=""
for tool in '"buf"' '"qmlformat"' '"qmllint"'; do
  grep -q -F -e "$tool" "$real_rs" || cohort_dispatch="$cohort_dispatch $tool:missing"
done
if [[ -z "$cohort_dispatch" ]]; then
  ok
else
  bad "runner lost Structured dispatch:$cohort_dispatch (want all three under issue #799)"
fi

# Commands pin the invocation shapes.
if grep -q -F -e 'pub fn buf_lint_check' "$commands" &&
  grep -q -F -e 'pub fn buf_format_check' "$commands" &&
  grep -q -F -e 'pub fn buf_format_fix' "$commands" &&
  grep -q -F -e 'pub fn qmlformat_check' "$commands" &&
  grep -q -F -e 'pub fn qmlformat_fix' "$commands" &&
  grep -q -F -e 'pub fn qmllint_check' "$commands"; then
  ok
else
  bad "commands lost a Structured invocation shape (want all six under issue #799)"
fi

# Runner-matrix doc owns the two cohort sections.
if grep -q -F -e '## Protobuf (`protobuf`: buf format plus lint)' "$runner_doc" &&
  grep -q -F -e '## QML (`qml`: qmlformat format, qmllint lint)' "$runner_doc" &&
  grep -q -F -e 'delivered under #799' "$runner_doc"; then
  ok
else
  bad "runner-matrix doc lost its Structured sections under issue #799"
fi

# Tool integrations claim the three adapters over the decided routes.
if grep -q -F -e 'adapters `buf` (format plus lint `protobuf`)' "$integrations" &&
  grep -q -F -e '`qmlformat` (format `qml`)' "$integrations" &&
  grep -q -F -e '`qmllint` (lint `qml`)' "$integrations" &&
  grep -q -F -e 'adapters qualified seed-only under #799' "$integrations" &&
  grep -q -F -e 'structured_adapters_qualification' "$integrations"; then
  ok
else
  bad "tool-integrations lost its Structured adapter claims under issue #799"
fi

# Verification matrix owns the harness in dogfood-freshness plus Green.
if grep -q -F -e ':structured_adapters_qualification' "$verify" &&
  grep -q -F -e '`structured_adapters_qualification` 16/16' "$verify"; then
  ok
else
  bad "verification-matrix lost its #799 adapters harness record (want dogfood plus Green 16/16)"
fi

# Targets own the harness plus dogfood wires it.
if grep -q -F -e 'name = "structured_adapters_qualification"' "$targets" &&
  grep -q -F -e 'bazel run --noshow_progress //tools/ci:structured_adapters_qualification' "$dogfood"; then
  ok
else
  bad "ci_targets or dogfood lost the structured_adapters_qualification wiring (want target plus dogfood-freshness)"
fi

# Fake format doubles stay present for the seed-only matrix cells.
if [[ -f "quality/testdata/fake_buf_format.sh" && -f "quality/testdata/fake_qmlformat.sh" ]] &&
  grep -q -F -e 'fake_buf_format' "$subjects" &&
  grep -q -F -e 'fake_qmlformat' "$subjects"; then
  ok
else
  bad "matrix fake doubles missing (want two fake_*.sh plus BUILD targets under issue #799)"
fi

# Live proof: adapter plus runner unit suites stay green.
if bazel test //quality/adapter:quality_adapter_test //quality/runner:all --noshow_progress >/dev/null 2>&1; then
  ok
else
  bad "structured adapters live proof failed (want //quality/adapter:quality_adapter_test plus //quality/runner:all green)"
fi

# Live proof: the eight cohort matrix cells stay green.
if bazel test //quality/testdata:matrix_protobuf_format_pass //quality/testdata:matrix_protobuf_format_fail //quality/testdata:matrix_protobuf_lint_pass //quality/testdata:matrix_protobuf_lint_fail //quality/testdata:matrix_qml_format_pass //quality/testdata:matrix_qml_format_fail //quality/testdata:matrix_qml_lint_pass //quality/testdata:matrix_qml_lint_fail --noshow_progress >/dev/null 2>&1; then
  ok
else
  bad "structured matrix live proof failed (want eight cohort cells green)"
fi

dx_test_summary "structured adapters qualification harness"
