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
build="tools/ci/BUILD.bazel"
ci=".github/workflows/ci.yml"
verify="docs/testing/verification-matrix.md"

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

# No hidden structured native-config preset: no cohort binding exists in
# the typed native-config rules (adapters run pinned upstream defaults until
# checked-in policy qualifies against the native-config contract).
cohort_config=""
for tool in buf qmlformat qmllint qml protobuf; do
  if grep -q -F -e "${tool}_config" "$native"; then
    cohort_config="$cohort_config $tool:preset"
  fi
done
if [[ -z "$cohort_config" ]]; then
  ok
else
  bad "native-config carries a hidden structured preset:$cohort_config"
fi

# No false adapter claim for the structured cohort: none of the cohort
# tool IDs appear in REAL_ADAPTERS. Classification exists; adapter claim
# does not (owned).
cohort_claim=""
for tool in buf qmlformat qmllint; do
  if grep -q -F -e "\"$tool\":" "$adapters"; then
    cohort_claim="$cohort_claim $tool:claimed"
  fi
done
if [[ -z "$cohort_claim" ]]; then
  ok
else
  bad "false adapter claim for structured cohort:$cohort_claim"
fi

# Classification-only today: protobuf/qml families carry no curated defaults,
# no runner-matrix cells (claims land only with green adapter evidence
#
cohort_curated=""
for family in '"protobuf": {' '"qml": {'; do
  if grep -q -F -e "$family" "$curated"; then
    cohort_curated="$cohort_curated $family:claimed"
  fi
done
if [[ -z "$cohort_curated" ]] &&
  ! grep -q -F -e 'matrix_protobuf_' "$matrix" &&
  ! grep -q -F -e 'matrix_qml_' "$matrix"; then
  ok
else
  bad "curated or matrix claims a structured family before adapters land:$cohort_curated"
fi

# Support matrix keeps the qualified structured versions plus rule-sets
# with fixtures and harness; digests plus adapters stay under.
if grep -q -F -e 'qualified seed-only under issue #488' "$support" &&
  grep -q -F -e 'structured_defaults_qualification' "$support" &&
  grep -q -F -e 'quality/tests/fixtures/structured_quality/pins.bzl' "$support" &&
  grep -q -F -e 'no hidden preset' "$support" &&
  grep -q -F -e 'digests plus adapter mappings stay owned under issue #419' "$support"; then
  ok
else
  bad "support-matrix lost its #488 qualified structured versions plus rule-sets record with fixtures"
fi

# Support matrix resolves the buf STANDARD plus qml ini conflicts as
# upstream built-in defaults (no auto-supplied preset).
if grep -q -F -e 'STANDARD' "$support" &&
  grep -q -F -e 'upstream built-in default' "$support" &&
  grep -q -F -e 'no auto-supplied' "$support" &&
  grep -q -F -e 'qualified seed-only under issue #488' "$support"; then
  ok
else
  bad "support-matrix lost its #488 buf STANDARD plus qml ini conflict resolution"
fi

# Tool acquisition keeps the decided checksummed buf route with no
# target-compiler context plus execution-platform laziness and no false
# claim; versions qualified under, digests plus adapters stay pending
# under.
if grep -q -F -e 'Decided route: `buf` takes the checksummed' "$acquisition" &&
  grep -q -F -e 'needs no target compiler context' "$acquisition" &&
  grep -q -F -e 'no adapter claims `protobuf` yet' "$acquisition" &&
  grep -q -F -e 'qualified seed-only under issue #488' "$acquisition" &&
  grep -q -F -e 'digests plus adapter mappings stay owned under issue #419' "$acquisition"; then
  ok
else
  bad "tool-acquisition lost its decided buf route with #488 versions plus #419 digests/adapters split"
fi

# Tool acquisition keeps the decided Qt-last authoritative-toolchain route
# for qmlformat/qmllint with distribution identity plus licensing plus
# platform artifacts pending and no false claim; versions qualified under
# , digests plus adapters stay pending under.
if grep -q -F -e 'Decided route (Qt last)' "$acquisition" &&
  grep -q -F -e 'qmlformat and qmllint take the authoritative-toolchain route' "$acquisition" &&
  grep -q -F -e 'exact Qt distribution identity, licensing, and platform' "$acquisition" &&
  grep -q -F -e 'no adapter claiming `qml` yet' "$acquisition" &&
  grep -q -F -e 'qualified seed-only under issue #488' "$acquisition" &&
  grep -q -F -e 'stay owned under issue #419' "$acquisition"; then
  ok
else
  bad "tool-acquisition lost its decided Qt route with #488 versions plus #419 digests/adapters split"
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

# Tool integrations keep the structured adapter-input notes with open
# parser work plus pinned versions (adapters still open under).
if grep -q -F -e '**Structured cohort (issue #419' "$integrations" &&
  grep -q -F -e 'no adapter claims `protobuf` or `qml` yet' "$integrations" &&
  grep -q -F -e 'qualified seed-only under issue #488' "$integrations" &&
  grep -q -F -e 'adapters stay owned under issue #419' "$integrations"; then
  ok
else
  bad "tool-integrations lost its structured notes with #488 versions plus #419 adapters split"
fi

# Native-configuration sole policy stands (no hidden presets authorized).
if grep -q -F -e 'defines no hidden rule' "$native_doc" &&
  grep -q -F -e 'sole behavioral policy' "$native_doc"; then
  ok
else
  bad "native-configuration lost its sole-policy plus no-hidden-preset honesty"
fi

# Verification matrix owns the qualified seed-only record under.
if grep -q -F -e 'structured_defaults_qualification' "$verify" &&
  grep -q -F -e 'qualified seed-only under #488' "$verify" &&
  grep -q -F -e 'bazel run //tools/ci:structured_defaults_qualification' "$verify" &&
  grep -q -F -e '`structured_defaults_qualification` 17/17' "$verify"; then
  ok
else
  bad "verification-matrix lost its #488 structured quality qualified record"
fi

# BUILD owns the harness target plus CI wires it in dogfood-freshness.
if grep -q -F -e 'name = "structured_defaults_qualification"' "$build" &&
  grep -q -F -e 'bazel run --noshow_progress //tools/ci:structured_defaults_qualification' "$ci"; then
  ok
else
  bad "tools/ci/BUILD.bazel or ci.yml lost the structured_defaults_qualification wiring (want target plus dogfood-freshness)"
fi

# Live proof: the structured fixture loads on the seed host (no
# protobuf/qml hello bazel test exists; foundations not admitted).
if bazel build //quality/tests/fixtures/structured_quality:corpus_starlark --noshow_progress >/dev/null 2>&1; then
  ok
else
  bad "structured quality live proof failed (want fixture corpus_starlark build green)"
fi

dx_test_summary "structured defaults qualification harness"
