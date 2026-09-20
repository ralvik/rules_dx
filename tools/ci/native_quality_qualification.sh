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
#   native-config preset, no adapter claim, no curated defaults, no matrix
#   cells; cc/go hello fixtures stay green.
# - open owned gaps: digests plus adapters under, platform plus consumer
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
build="tools/ci/BUILD.bazel"
ci=".github/workflows/ci.yml"
verify="docs/testing/verification-matrix.md"

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

# No hidden native native-config preset: no cohort binding exists in the
# typed native-config rules (adapters run pinned upstream defaults until
# checked-in policy qualifies against the native-config contract).
native_config=""
for tool in clang-format clang-tidy cppcheck gofumpt staticcheck govet errcheck; do
  if grep -q -F -e "${tool}_config" "$native"; then
    native_config="$native_config $tool:preset"
  fi
done
if [[ -z "$native_config" ]]; then
  ok
else
  bad "native-config carries a hidden native preset:$native_config"
fi

# No false adapter claim for the native cohort: none of the cohort tool IDs
# appear in REAL_ADAPTERS. Classification exists; adapter claim does not
# (owned).
native_claim=""
for tool in clang-format clang-tidy cppcheck gofmt gofumpt staticcheck govet errcheck; do
  if grep -q -F -e "\"$tool\":" "$adapters"; then
    native_claim="$native_claim $tool:claimed"
  fi
done
if [[ -z "$native_claim" ]]; then
  ok
else
  bad "false adapter claim for native cohort:$native_claim"
fi

# Classification-only today: cc/go families carry no curated defaults,
# no runner-matrix cells (claims land only with green adapter evidence
#
native_curated=""
for family in '"cc": {' '"go": {'; do
  if grep -q -F -e "$family" "$curated"; then
    native_curated="$native_curated $family:claimed"
  fi
done
if [[ -z "$native_curated" ]] &&
  ! grep -q -F -e 'matrix_c_' "$matrix" &&
  ! grep -q -F -e 'matrix_cpp_' "$matrix" &&
  ! grep -q -F -e 'matrix_go_' "$matrix"; then
  ok
else
  bad "curated or matrix claims a native family before adapters land:$native_curated"
fi

# Support matrix keeps the qualified native versions plus rule-sets with
# fixtures and harness; digests plus adapters stay under.
if grep -q -F -e 'qualified seed-only under issue #487' "$support" &&
  grep -q -F -e 'native_quality_qualification' "$support" &&
  grep -q -F -e 'cc/tests/fixtures/native_quality/pins.bzl' "$support" &&
  grep -q -F -e 'no hidden preset' "$support" &&
  grep -q -F -e 'digests plus adapter mappings stay owned under issue #418' "$support"; then
  ok
else
  bad "support-matrix lost its #487 qualified native versions plus rule-sets record with fixtures"
fi

# Support matrix resolves the SA-only versus default-checks conflict as
# upstream built-in defaults (SA-only shortcut rejected without qualification).
if grep -q -F -e 'shortcut is rejected without qualification' "$support" &&
  grep -q -F -e 'upstream built-in defaults' "$support" &&
  grep -q -F -e 'qualified seed-only under issue #487' "$support"; then
  ok
else
  bad "support-matrix lost its #487 SA-only plus defaults conflict resolution"
fi

# Tool acquisition keeps the decided split native route with no separate
# acquisition and no false claim; versions qualified under, digests
# plus adapters stay pending under.
if grep -q -F -e 'Decided route: clang-format, clang-tidy, and cppcheck take the' "$acquisition" &&
  grep -q -F -e 'no separate acquisition' "$acquisition" &&
  grep -q -F -e 'no adapter claims `c` or `cpp` yet' "$acquisition" &&
  grep -q -F -e 'qualified seed-only under issue #487' "$acquisition" &&
  grep -q -F -e 'digests plus adapter mappings stay owned under issue #418' "$acquisition"; then
  ok
else
  bad "tool-acquisition lost its decided native route with #487 versions plus #418 digests/adapters split"
fi

# Tool acquisition keeps the decided split Go route with the SA-only
# conflict resolved (neither provisional); versions qualified under.
if grep -q -F -e 'Decided route: gofumpt, staticcheck, govet, and errcheck take the' "$acquisition" &&
  grep -q -F -e 'strict superset of gofmt' "$acquisition" &&
  grep -q -F -e 'qualified seed-only under issue #487' "$acquisition" &&
  grep -q -F -e 'no adapter claims `go`' "$acquisition" &&
  grep -q -F -e 'digests plus adapter mappings stay owned under issue #418' "$acquisition"; then
  ok
else
  bad "tool-acquisition lost its decided Go route with #487 versions plus #418 digests/adapters split"
fi

# Tool acquisition keeps the seven research rows with byte-identity risk.
native_research=""
for tool in '| clang-format |' '| clang-tidy |' '| cppcheck |' '| gofumpt |' '| staticcheck |' '| govet |' '| errcheck |'; do
  grep -q -F -e "$tool" "$acquisition" || native_research="$native_research $tool:missing"
done
if [[ -z "$native_research" ]] &&
  grep -q -F -e 'maintainer acquisition must establish and record byte identity' "$acquisition"; then
  ok
else
  bad "tool-acquisition lost a native research row or byte-identity honesty:$native_research"
fi

# Tool integrations keep the native adapter-input notes with pinned
# versions (adapters still open under).
if grep -q -F -e 'Native cohort' "$integrations" &&
  grep -q -F -e 'no adapter claims `c`, `cpp`, or `go` yet' "$integrations" &&
  grep -q -F -e 'qualified seed-only under issue #487' "$integrations" &&
  grep -q -F -e 'adapters stay owned under issue #418' "$integrations"; then
  ok
else
  bad "tool-integrations lost its native notes with #487 versions plus #418 adapters split"
fi

# Native-configuration sole policy stands (no hidden presets authorized).
if grep -q -F -e 'defines no hidden rule' "$native_doc" &&
  grep -q -F -e 'sole behavioral policy' "$native_doc"; then
  ok
else
  bad "native-configuration lost its sole-policy plus no-hidden-preset honesty"
fi

# Verification matrix owns the qualified seed-only record under.
if grep -q -F -e 'native_quality_qualification' "$verify" &&
  grep -q -F -e 'qualified seed-only under #487' "$verify" &&
  grep -q -F -e 'bazel run //tools/ci:native_quality_qualification' "$verify" &&
  grep -q -F -e '`native_quality_qualification` 17/17' "$verify"; then
  ok
else
  bad "verification-matrix lost its #487 native quality qualified record"
fi

# BUILD owns the harness target plus CI wires it in dogfood-freshness.
if grep -q -F -e 'name = "native_quality_qualification"' "$build" &&
  grep -q -F -e 'bazel run --noshow_progress //tools/ci:native_quality_qualification' "$ci"; then
  ok
else
  bad "tools/ci/BUILD.bazel or ci.yml lost the native_quality_qualification wiring (want target plus dogfood-freshness)"
fi

# Live proof: native foundation fixtures stay green on the seed host
# (defaults change only; no adapter behavior yet).
if bazel test //cc/tests/fixtures/hello:hello_test //go/tests/fixtures/hello:hello_test --noshow_progress >/dev/null 2>&1; then
  ok
else
  bad "native quality live proof failed (want cc plus go hello green)"
fi

dx_test_summary "native quality qualification harness"
