#!/usr/bin/env bash
# Native quality defaults qualification harness.
#
# Qualifies the provisional native format plus lint defaults against the
# native-configuration contract with no hidden presets. covers
# adapters (plus digests), not selections: this harness owns versions plus
# rule-sets.
# - pinned: hermetic-llvm v0.8.19 (LLVM 23.1.0) toolchain for clang-format
#   plus clang-tidy, cppcheck 2.21.0, gofumpt v0.11.0, staticcheck 2026.2,
#   govet following the qualified Go SDK 1.26.6 pin, errcheck v1.20.0 in
#   `cc/tests/fixtures/native_quality/pins.bzl` (prior LLVM line plus Go
#   1.27.1 observation plus living at head rejected); split-native identities
# recorded, digests stay owned.
# - rule-sets: native-configuration sole policy, no hidden presets. Without
#   an applicable checked-in native config the pinned tool uses upstream
#   built-in defaults; with a config it interprets natively; adapters add
#   only transport/hermetic settings. staticcheck default checks (SA-only
#   shortcut rejected without qualification), govet default analyzers with
#   errcheck complementary for unhandled errors, clang-tidy default checks,
#   cppcheck default enablement are upstream built-in defaults.
#   Beyond-default switches (staticcheck -all/SA-only preset, govet all
#   analyzers, clang-tidy --checks=*, cppcheck --enable=all) rejected.
# - fixtures: `cc/tests/fixtures/native_quality/` pins plus BUILD; no native
#   native-config preset beyond the four bindings delivered under #798, no
#   curated defaults; cc/go hello fixtures stay green; adapters plus matrix
#   cells delivered under #798.
# - open owned gaps: digests plus toolchain bounds, platform plus consumer
#   plus release evidence, no `Supported` claim. Compatibility is defaults only.
#
# Versioned here, run by CI via `bazel run //tools/ci:native_quality_qualification`,
# following //tools/ci:jvm_quality_qualification.
set -euo pipefail

# Shared workspace + runfiles helpers.
# Bootstrap via tools/sh/bootstrap.sh (issue #654): runfiles forest first, then source tree.
source "${RUNFILES_DIR:-/dev/null}/_main/tools/sh/bootstrap.sh" 2>/dev/null || source "${TEST_SRCDIR:-/dev/null}/_main/tools/sh/bootstrap.sh" 2>/dev/null || source "$0.runfiles/_main/tools/sh/bootstrap.sh" 2>/dev/null || source "${BASH_SOURCE[0]}.runfiles/_main/tools/sh/bootstrap.sh" 2>/dev/null || source "$(git rev-parse --show-toplevel 2>/dev/null)/tools/sh/bootstrap.sh"
dx_bootstrap "tools/sh/lib.sh"

dx_cd_workspace

dx_test_init

dx_bash_pin

pins="cc/tests/fixtures/native_quality/pins.bzl"
pins_build="cc/tests/fixtures/native_quality/BUILD.bazel"
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
targets="tools/ci/ci_targets_d.bzl"
dogfood="tools/ci/dogfood_freshness.sh"

# Fixture pair plus pins stay present.
if [[ -f "$pins" && -f "$pins_build" ]]; then
  ok
else
  bad "native quality fixture missing (want $pins plus $pins_build)"
fi

# Pins record the qualified upstream versions.
if grep -q -F -e 'CLANG_TOOLCHAIN_VERSION = "hermetic-llvm v0.8.19"' "$pins" &&
  grep -q -F -e 'LLVM_VERSION = "23.1.0"' "$pins" &&
  grep -q -F -e 'CPPCHECK_VERSION = "2.21.0"' "$pins" &&
  grep -q -F -e 'GOFUMPT_VERSION = "v0.11.0"' "$pins" &&
  grep -q -F -e 'STATICCHECK_VERSION = "2026.2"' "$pins" &&
  grep -q -F -e 'GOVET_TOOLCHAIN_VERSION = "1.26.6"' "$pins" &&
  grep -q -F -e 'ERRCHECK_VERSION = "v1.20.0"' "$pins"; then
  ok
else
  bad "pins.bzl lost its qualified native versions under issue #487"
fi

# Pins record the split-native identities plus rejected prior/observed lines.
if grep -q -F -e 'CLANG_TOOLCHAIN_ROUTE = "authoritative hermetic-llvm LLVM tool targets' "$pins" &&
  grep -q -F -e 'CPPCHECK_ARTIFACT = "standalone checksummed release artifact' "$pins" &&
  grep -q -F -e 'GOFUMPT_ARTIFACT = "standalone checksummed release artifact' "$pins" &&
  grep -q -F -e 'STATICCHECK_ARTIFACT = "standalone checksummed release artifact' "$pins" &&
  grep -q -F -e 'GOVET_COUPLING = "ships with the qualified Go toolchain' "$pins" &&
  grep -q -F -e 'ERRCHECK_ARTIFACT = "standalone checksummed release artifact' "$pins" &&
  grep -q -F -e 'living at head rejected' "$pins" &&
  grep -q -F -e 'Go 1.27.1 observed, not pinned' "$pins"; then
  ok
else
  bad "pins.bzl lost its split-native identities plus rejected lines under issue #487"
fi

# Pins record the sole-policy plus qualified rule-set resolutions plus rejections.
if grep -q -F -e 'NATIVE_CONFIG_POLICY = "native-configuration sole policy: no hidden presets"' "$pins" &&
  grep -q -F -e 'default checks are the upstream built-in default checks, not an SA-only preset' "$pins" &&
  grep -q -F -e 'SA-only shortcut rejected without qualification' "$pins" &&
  grep -q -F -e 'default analyzers are the upstream built-in default analyzers' "$pins" &&
  grep -q -F -e 'complementary for unhandled errors, not a default selection' "$pins" &&
  grep -q -F -e 'default checks are the upstream built-in default checks; --checks=* maxima never enabled' "$pins" &&
  grep -q -F -e 'default enablement is the upstream built-in default enablement; --enable=all maxima never enabled' "$pins" &&
  grep -q -F -e 'beyond-default switches rejected' "$pins" &&
  grep -q -F -e 'hidden presets rejected' "$pins"; then
  ok
else
  bad "pins.bzl lost its sole-policy plus rule-set resolutions plus rejections under issue #487"
fi

# Native native-config bindings delivered under #798 (checked-in policy
# qualifies against the native-config contract; no hidden preset).
native_config=""
for tool in clang_format clang_tidy cppcheck staticcheck; do
  grep -q -F -e "${tool}_config" "$native" || native_config="$native_config $tool:missing"
done
if [[ -z "$native_config" ]]; then
  ok
else
  bad "native-config lost native bindings:$native_config (want all four under #798)"
fi

# Adapters delivered under #798: all seven cohort tools appear in
# REAL_ADAPTERS. Classification exists; adapter claim delivered.
native_claim=""
for tool in clang_format clang_tidy cppcheck gofumpt staticcheck govet errcheck; do
  grep -q -F -e "\"$tool\":" "$adapters" || native_claim="$native_claim $tool:missing"
done
if [[ -z "$native_claim" ]]; then
  ok
else
  bad "native adapter delivery missing:$native_claim (want all seven under #798)"
fi

# Delivered: cc/go families carry no curated defaults (claims land only
# with green adapter evidence) but do carry runner-matrix cells under
# #798.
native_curated=""
for family in '"cc": {' '"go": {'; do
  if grep -q -F -e "$family" "$curated"; then
    native_curated="$native_curated $family:claimed"
  fi
done
if [[ -z "$native_curated" ]] &&
  grep -q -F -e 'NATIVE_CASES' "$matrix"; then
  ok
else
  bad "curated claims a native family or matrix lost cohort cells:$native_curated (want no curated, NATIVE_CASES under #798)"
fi

# Support matrix keeps the qualified native coverage rows with the #798
# delivery record; digests stay owned.
# Support matrix resolves the SA-only versus default-checks conflict as
# upstream built-in defaults (SA-only shortcut rejected without qualification).
if grep -q -F -e 'SA-only shortcut is rejected without qualification' "$integrations" &&
  grep -q -F -e 'upstream built-in defaults with no hidden preset' "$integrations"; then
  ok
else
  bad "tool-integrations lost its SA-only plus defaults conflict resolution"
fi

# Tool integrations keep the decided split native route with no separate
# acquisition; versions qualified under #487, digests plus adapters owned
# under #798.
if grep -q -F -e 'native route for clang-format/clang-tidy via the qualified hermetic-llvm' "$integrations" &&
  grep -q -F -e 'no separate acquisition' "$integrations" &&
  grep -q -F -e 'qualified seed-only under issue #487' "$integrations" &&
  grep -q -F -e 'Native cohort (#798, successor to closed #418)' "$integrations" &&
  grep -q -F -e 'cohort delivered under #798' "$acquisition"; then
  ok
else
  bad "tool-integrations lost its decided native route with #487 versions plus #798 digests/adapters split"
fi

# Tool integrations keep the decided split Go route with the SA-only
# conflict resolved; versions qualified under #487, digests plus adapters
# owned under #798.
if grep -q -F -e 'and split Go route' "$integrations" &&
  grep -q -F -e 'strict gofmt superset' "$integrations" &&
  grep -q -F -e 'qualified seed-only under issue #487' "$integrations" &&
  grep -q -F -e 'successor to closed #418' "$integrations" &&
  grep -q -F -e 'gofumpt' "$acquisition"; then
  ok
else
  bad "tool-integrations lost its decided Go route with #487 versions plus #798 digests/adapters split"
fi

# Tool integrations keep the seven research rows with byte-identity risk.
native_research=""
for tool in 'clang-format' 'clang-tidy' 'cppcheck' 'gofumpt' 'staticcheck' 'govet' 'errcheck'; do
  grep -q -F -e "$tool" "$integrations" || native_research="$native_research $tool:missing"
done
if [[ -z "$native_research" ]] &&
  grep -q -F -e 'actual bytes when adding each adapter' "$acquisition"; then
  ok
else
  bad "tool-integrations lost a native research row or byte-identity honesty:$native_research"
fi

# Tool integrations keep the native adapter delivery record with pinned
# versions (adapters delivered under #798).
if grep -q -F -e 'Native cohort (#798' "$integrations" &&
  grep -q -F -e 'qualified seed-only under issue #487' "$integrations" &&
  grep -q -F -e 'adapters qualified seed-only' "$integrations" &&
  grep -q -F -e 'under #798' "$integrations"; then
  ok
else
  bad "tool-integrations lost its native delivery record with #487 versions plus #798 adapters"
fi

# Native-configuration sole policy stands (no hidden presets authorized).
if grep -q -F -e 'defines no hidden rule' "$native_doc" &&
  grep -q -F -e 'sole behavioral policy' "$native_doc"; then
  ok
else
  bad "native-configuration lost its sole-policy plus no-hidden-preset honesty"
fi

# Targets own the harness plus dogfood wires it.
if grep -q -F -e 'name = "native_quality_qualification"' "$targets" &&
  grep -q -F -e 'bazel run --noshow_progress //tools/ci:native_quality_qualification' "$dogfood"; then
  ok
else
  bad "ci_targets or dogfood lost the native_quality_qualification wiring (want target plus dogfood-freshness)"
fi

# Live proof: native foundation fixtures stay green on the seed host
# (defaults change only; no adapter behavior yet).
if bazel test //cc/tests/fixtures/hello:hello_test //go/tests/fixtures/hello:hello_test --noshow_progress >/dev/null 2>&1; then
  ok
else
  bad "native quality live proof failed (want cc plus go hello green)"
fi

dx_test_summary "native quality qualification harness"
