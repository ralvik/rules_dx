#!/usr/bin/env bash
# Structured quality defaults qualification harness.
#
# Qualifies the provisional structured format plus lint defaults against the
# native-configuration contract with no hidden presets. covers
# adapters (plus digests), not versions: this harness owns versions plus
# rule-sets.
# - pinned: buf 1.72.0 in `quality/tests/fixtures/structured_quality/pins.bzl`
#   (living at head rejected); qmlformat (Qt 6.11.2 observed) plus qmllint
#   (Qt 6.11.1 observed) follow the qualified Qt distribution pin
#   (authoritative-toolchain class, no separate acquisition; Qt-last ordering
#   decided; exact Qt distribution identity, licensing, and platform artifact
# qualification stay owned); checksummed
#   native/self-contained buf identity recorded, digests stay owned under
#
# - rule-sets: native-configuration sole policy, no hidden presets. Without
#   an applicable checked-in native config the pinned tool uses upstream
#   built-in defaults; with a config it interprets natively; adapters add
#   only transport/hermetic settings. buf STANDARD is the upstream built-in
#   default lint set, not a rules_dx preset (no auto-supplied buf.yaml);
#   qmlformat/qmllint use upstream built-in defaults without a checked-in
#   ini and interpret `.qmlformat.ini`/`.qmllint.ini` natively with one
#   (no auto-supplied ini preset). Beyond-default COMMENTS plus UNARY_RPC
#   opt-in maxima rejected.
# - fixtures: `quality/tests/fixtures/structured_quality/` pins plus BUILD;
#   no structured native-config preset, no adapter claim, no curated
#   defaults, no matrix cells; protobuf/qml foundations are not admitted as
#   build/test targets, so the fixture pair plus `bazel build` of the fixture
#   is the live proof (no hello bazel test exists here).
# - open owned gaps: digests (including Qt distribution identity, licensing,
# platform artifacts) plus adapters under, platform plus consumer plus
#   release evidence, no `Supported` claim. Compatibility is defaults only.
#
# Versioned here, run by CI via `bazel run //tools/ci:structured_defaults_qualification`,
# following //tools/ci:scala_dotnet_defaults_qualification.
set -euo pipefail

# Shared workspace + runfiles helpers.
# Bootstrap via tools/sh/bootstrap.sh (issue #654): runfiles forest first, then source tree.
source "${RUNFILES_DIR:-/dev/null}/_main/tools/sh/bootstrap.sh" 2>/dev/null || source "${TEST_SRCDIR:-/dev/null}/_main/tools/sh/bootstrap.sh" 2>/dev/null || source "$0.runfiles/_main/tools/sh/bootstrap.sh" 2>/dev/null || source "${BASH_SOURCE[0]}.runfiles/_main/tools/sh/bootstrap.sh" 2>/dev/null || source "$(git rev-parse --show-toplevel 2>/dev/null)/tools/sh/bootstrap.sh"
dx_bootstrap "tools/sh/lib.sh"

dx_cd_workspace

dx_test_init

pins="quality/tests/fixtures/structured_quality/pins.bzl"
pins_build="quality/tests/fixtures/structured_quality/BUILD.bazel"
adapters="quality/adapters.bzl"
parity="quality/parity_tests.bzl"
curated="quality/curated_defaults.bzl"
native="quality/native_config.bzl"
matrix="quality/testdata/runner_matrix_cases.bzl"
support="docs/product/support-matrix.md"
baseline="docs/tools/tool-baseline.md"
acquisition="docs/tools/tool-acquisition.md"
integrations="docs/quality/tool-integrations.md"
native_doc="docs/quality/native-configuration.md"
build="tools/ci/ci_targets_d.bzl"
ci="tools/ci/dogfood_freshness.sh"

# Fixture pair plus pins stay present.
if [[ -f "$pins" && -f "$pins_build" ]]; then
  ok
else
  bad "structured quality fixture missing (want $pins plus $pins_build)"
fi

# Pins record the qualified buf version plus the Qt observations with the
# authoritative-distribution coupling (qml tools follow the Qt pin).
if grep -q -F -e 'BUF_VERSION = "1.72.0"' "$pins" &&
  grep -q -F -e 'QMLFORMAT_QT_OBSERVED = "Qt 6.11.2"' "$pins" &&
  grep -q -F -e 'QMLLINT_QT_OBSERVED = "Qt 6.11.1"' "$pins" &&
  grep -q -F -e 'QT_COUPLING = "Authoritative Qt distribution' "$pins"; then
  ok
else
  bad "pins.bzl lost its qualified buf version plus Qt observations plus distribution coupling under issue #488"
fi

# Pins record the route identities plus the rejected head line.
if grep -q -F -e 'self-contained per-platform binaries with published checksums' "$pins" &&
  grep -q -F -e 'qualified Qt distribution tool targets' "$pins" &&
  grep -q -F -e 'no separate acquisition' "$pins" &&
  grep -q -F -e 'living at head rejected' "$pins"; then
  ok
else
  bad "pins.bzl lost its route identities plus rejected head line under issue #488"
fi

# Pins record the sole-policy plus qualified rule-set resolutions plus rejections.
if grep -q -F -e 'NATIVE_CONFIG_POLICY = "native-configuration sole policy: no hidden presets"' "$pins" &&
  grep -q -F -e 'STANDARD is the upstream built-in default lint set' "$pins" &&
  grep -q -F -e 'no auto-supplied ini preset' "$pins" &&
  grep -q -F -e 'native interpretation with' "$pins" &&
  grep -q -F -e 'beyond-default switches rejected' "$pins" &&
  grep -q -F -e 'hidden presets rejected' "$pins"; then
  ok
else
  bad "pins.bzl lost its sole-policy plus rule-set resolutions plus rejections under issue #488"
fi

# Structured native-config bindings delivered under #799 (checked-in
# policy qualifies against the native-config contract; no hidden preset).
cohort_config=""
for tool in buf qmlformat qmllint; do
  grep -q -F -e "${tool}_config" "$native" || cohort_config="$cohort_config $tool:missing"
done
if [[ -z "$cohort_config" ]]; then
  ok
else
  bad "native-config lost structured bindings:$cohort_config (want all three under #799)"
fi

# Adapters delivered under #799: all three cohort tools appear in REAL_ADAPTERS.
cohort_claim=""
for tool in buf qmlformat qmllint; do
  grep -q -F -e "\"$tool\":" "$adapters" || cohort_claim="$cohort_claim $tool:missing"
done
if [[ -z "$cohort_claim" ]]; then
  ok
else
  bad "structured adapter delivery missing:$cohort_claim (want all three under #799)"
fi

# Delivered: protobuf/qml families carry no curated defaults (claims
# land only with green adapter evidence, opt-in like Scala/.NET) but do
# carry runner-matrix cells under #799.
cohort_curated=""
for family in '"protobuf": {' '"qml": {'; do
  if grep -q -F -e "$family" "$curated"; then
    cohort_curated="$cohort_curated $family:claimed"
  fi
done
if [[ -z "$cohort_curated" ]] &&
  grep -q -F -e 'STRUCTURED_CASES' "$matrix"; then
  ok
else
  bad "curated claims a structured family or matrix lost cohort cells:$cohort_curated (want no curated, STRUCTURED_CASES under #799)"
fi

# Support matrix keeps the qualified structured versions plus rule-sets
# with fixtures and harness; adapters delivered under #799.
# Support matrix records protobuf/qml delivered under #799, never
# double-claimed.
# Tool acquisition keeps the decided checksummed buf route with no
# target-compiler context plus execution-platform laziness, delivered
# under #799; versions qualified under #488.
if grep -q -F -e 'Decided route: `buf` takes the checksummed' "$acquisition" &&
  grep -q -F -e 'needs no target compiler context' "$acquisition" &&
  grep -q -F -e 'with `protobuf` claimed via `buf`' "$acquisition" &&
  grep -q -F -e 'qualified seed-only under issue #488' "$acquisition" &&
  grep -q -F -e '(delivered under #799' "$acquisition"; then
  ok
else
  bad "tool-acquisition lost its delivered buf route with #488 versions plus #799 delivery"
fi

# Tool acquisition keeps the decided Qt-last authoritative-toolchain route
# for qmlformat/qmllint, delivered under #799; versions qualified under
# #488.
if grep -q -F -e 'Decided route (Qt last)' "$acquisition" &&
  grep -q -F -e 'qmlformat and qmllint take the authoritative-toolchain route' "$acquisition" &&
  grep -q -F -e 'with `qml` claimed via' "$acquisition" &&
  grep -q -F -e 'qualified seed-only under issue #488' "$acquisition" &&
  grep -q -F -e '(delivered under #799' "$acquisition"; then
  ok
else
  bad "tool-acquisition lost its delivered Qt route with #488 versions plus #799 delivery"
fi

# Tool acquisition keeps the three research rows with byte-identity risk.
cohort_research=""
for tool in '| buf |' '| qmlformat |' '| qmllint |'; do
  grep -q -F -e "$tool" "$acquisition" || cohort_research="$cohort_research $tool:missing"
done
if [[ -z "$cohort_research" ]] &&
  grep -q -F -e 'maintainer acquisition must establish and record byte identity' "$acquisition"; then
  ok
else
  bad "tool-acquisition lost a structured research row or byte-identity honesty:$cohort_research"
fi

# Tool integrations keep the delivered structured notes with pinned
# versions (adapters delivered under #799).
if grep -q -F -e '**Structured cohort (#799' "$integrations" &&
  grep -q -F -e 'adapters `buf` (format plus lint `protobuf`)' "$integrations" &&
  grep -q -F -e 'qualified seed-only under issue #488' "$integrations" &&
  grep -q -F -e 'adapters qualified seed-only under #799' "$integrations"; then
  ok
else
  bad "tool-integrations lost its structured notes with #488 versions plus #799 delivery"
fi

# Native-configuration sole policy stands (no hidden presets authorized).
if grep -q -F -e 'defines no hidden rule' "$native_doc" &&
  grep -q -F -e 'sole behavioral policy' "$native_doc"; then
  ok
else
  bad "native-configuration lost its sole-policy plus no-hidden-preset honesty"
fi

# BUILD owns the harness target plus CI wires it in dogfood-freshness.
if grep -q -F -e 'name = "structured_defaults_qualification"' "$build" &&
  grep -q -F -e 'bazel run --noshow_progress //tools/ci:structured_defaults_qualification' "$ci"; then
  ok
else
  bad "ci_targets or dogfood lost the structured_defaults_qualification wiring (want target plus dogfood-freshness)"
fi

# Live proof: the structured fixture loads on the seed host (no
# protobuf/qml hello bazel test exists; foundations not admitted).
if bazel build //quality/tests/fixtures/structured_quality:corpus_starlark --noshow_progress >/dev/null 2>&1; then
  ok
else
  bad "structured quality live proof failed (want fixture corpus_starlark build green)"
fi

dx_test_summary "structured defaults qualification harness"
