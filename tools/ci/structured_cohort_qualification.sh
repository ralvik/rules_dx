#!/usr/bin/env bash
# Structured-cohort qualification harness.
#
# Qualifies the as-built structured quality-cohort record with fixture
# evidence and owned gaps, without claiming Supported:
# - delivered under #799 (successor to closed #419): decided
#   checksummed native/self-contained artifact route for `buf`
#   (self-contained per-platform binaries with published checksums, no
#   target compiler context unlike clang-tidy, execution-platform lazy)
#   plus decided authoritative-toolchain route for qmlformat/qmllint
#   from the Qt distribution (Qt-last ordering decided; exact Qt
#   distribution identity, licensing, and platform artifact qualification
#   remain pending), REAL_ADAPTERS claims protobuf/qml via buf,
#   qmlformat, qmllint, JSONL plus diff plus path-listing plus JSON
#   parsers with runner-matrix pass/fail plus fix/format evidence per
#   adapter-backed class, native-config bindings against the
#   native-config contract;
# - open under #799 with honest records: exact artifact digests plus Qt
#   distribution qualification, platform plus consumer plus release
#   evidence.
#
# Versioned here, run by CI via `bazel run //tools/ci:structured_cohort_qualification`,
# following //tools/ci:native_cohort_qualification.
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

# Delivered adapter claims for the structured cohort: the three cohort
# tool IDs appear in REAL_ADAPTERS with protobuf/qml classes.
cohort_claim=""
for tool in buf qmlformat qmllint; do
  grep -q -F -e "\"$tool\":" "$adapters" || cohort_claim="$cohort_claim $tool:missing"
done
if [[ -z "$cohort_claim" ]]; then
  ok
else
  bad "structured adapter delivery missing:$cohort_claim (want all three under #799)"
fi

# Parity no longer defers the delivered protobuf/qml classes (closed #419
# owns nothing here; delivery under #799).
if ! grep -q -F -e '"protobuf":' "$parity" &&
  ! grep -q -F -e '"qml":' "$parity"; then
  ok
else
  bad "parity still defers delivered protobuf/qml (want none under #799)"
fi

# Every cohort class stays classified in the frozen taxonomy, one family each.
if grep -q -F -e '"protobuf": "protobuf"' "$adapters" &&
  grep -q -F -e '"qml": "qml"' "$adapters"; then
  ok
else
  bad "frozen taxonomy lost the protobuf/qml classification"
fi

# Classification-only today: protobuf/qml families carry no curated defaults
# (curated membership unchanged; no native default selected).
cohort_curated=""
for family in '"protobuf": {' '"qml": {'; do
  if grep -q -F -e "$family" "$curated"; then
    cohort_curated="$cohort_curated $family:claimed"
  fi
done
if [[ -z "$cohort_curated" ]]; then
  ok
else
  bad "curated defaults claim a structured family before adapters land:$cohort_curated"
fi

# Delivered native-config bindings: buf plus qmlformat plus qmllint carry
# the typed bindings (no hidden preset).
cohort_config=""
for tool in buf qmlformat qmllint; do
  grep -q -F -e "${tool}_config" "$native" || cohort_config="$cohort_config $tool:missing"
done
if [[ -z "$cohort_config" ]]; then
  ok
else
  bad "native-config lost structured bindings:$cohort_config (want all three under #799)"
fi

# Delivered green claim: the runner matrix carries structured cells with
# pass/fail plus fix/format evidence per adapter-backed class. Cells live
# in runner_matrix_structured.bzl (split from the cases file).
structured_matrix="quality/testdata/runner_matrix_structured.bzl"
if grep -q -F -e 'matrix_protobuf_format_pass' "$structured_matrix" &&
  grep -q -F -e 'matrix_protobuf_format_fail' "$structured_matrix" &&
  grep -q -F -e 'matrix_protobuf_lint_pass' "$structured_matrix" &&
  grep -q -F -e 'matrix_protobuf_lint_fail' "$structured_matrix" &&
  grep -q -F -e 'matrix_qml_format_pass' "$structured_matrix" &&
  grep -q -F -e 'matrix_qml_format_fail' "$structured_matrix" &&
  grep -q -F -e 'matrix_qml_lint_pass' "$structured_matrix" &&
  grep -q -F -e 'matrix_qml_lint_fail' "$structured_matrix"; then
  ok
else
  bad "runner matrix lost delivered structured cells under #799"
fi

# Tool acquisition keeps the decided checksummed buf route with no
# target-compiler context plus execution-platform laziness, delivered
# under #799 (live successor to closed #419 for the protobuf class).
if grep -q -F -e 'Decided route: `buf` takes the checksummed' "$acquisition" &&
  grep -q -F -e 'needs no target compiler context' "$acquisition" &&
  grep -q -F -e 'execution-platform lazy' "$acquisition" &&
  grep -q -F -e 'with `protobuf` claimed via `buf`' "$acquisition" &&
  grep -q -F -e '(delivered under #799' "$acquisition"; then
  ok
else
  bad "tool-acquisition lost its delivered buf route under #799"
fi

# Tool acquisition keeps the decided Qt-last authoritative-toolchain route for
# qmlformat/qmllint with distribution identity plus licensing plus platform
# artifacts pending, delivered under #799.
if grep -q -F -e 'Decided route (Qt last)' "$acquisition" &&
  grep -q -F -e 'qmlformat and qmllint take the authoritative-toolchain route' "$acquisition" &&
  grep -q -F -e 'exact Qt distribution' "$acquisition" &&
  grep -q -F -e 'with `qml` claimed via' "$acquisition" &&
  grep -q -F -e '(delivered under #799' "$acquisition" &&
  grep -q -F -e 'Qt closed that order' "$acquisition"; then
  ok
else
  bad "tool-acquisition lost its delivered Qt route under #799"
fi

# Tool acquisition keeps initial artifact research rows for the cohort
# with byte-identity risk explicit (digests stay observations, not pins).
cohort_research=""
for tool in '| buf |' '| qmlformat |' '| qmllint |'; do
  grep -q -F -e "$tool" "$acquisition" || cohort_research="$cohort_research $tool:missing"
done
if [[ -z "$cohort_research" ]] &&
  grep -q -F -e 'maintainer acquisition must establish and record byte identity' "$acquisition"; then
  ok
else
  bad "tool-acquisition lost a structured research row:$cohort_research"
fi

# Tool integrations keep the delivered structured adapter notes:
# buf JSONL as the faithful shape (no SARIF) with STANDARD plus
# module-root-sensitive scoping, qmlformat check plus `-i` with ini
# settings, qmllint `--json -` with ini plus comment scoping,
# whole-file rewrite versus check-only fix modes, Qt-last ordering,
# versions qualified under #488 with digests as observations, adapters
# qualified under #799.
if grep -q -F -e '**Structured cohort (#799' "$integrations" &&
  grep -q -F -e 'adapters `buf` (format plus lint `protobuf`)' "$integrations" &&
  grep -q -F -e 'no SARIF in 1.71.0' "$integrations" &&
  grep -q -F -e 'PACKAGE_DIRECTORY_MATCH' "$integrations" &&
  grep -q -F -e 'itemized here, not silently dropped' "$integrations" &&
  grep -q -F -e '.qmlformat.ini' "$integrations" &&
  grep -q -F -e '--json -' "$integrations" &&
  grep -q -F -e 'qualified seed-only under issue #488' "$integrations" &&
  grep -q -F -e 'adapters qualified seed-only under #799' "$integrations"; then
  ok
else
  bad "tool-integrations lost its delivered structured notes under #799"
fi

# Support matrix keeps the file-family record with protobuf/qml delivered
# under #799, never double-claimed.
# Tool baseline keeps the Protocol Buffer/QML coverage rows (integration inventory,
# not a support claim).
if grep -q -F -e '| Protocol Buffer | buf format | buf lint |' "$baseline" &&
  grep -q -F -e '| QML | qmlformat | qmllint |' "$baseline"; then
  ok
else
  bad "tool-baseline lost its Protocol Buffer/QML coverage rows"
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

dx_test_summary "Structured-cohort qualification harness"
