#!/usr/bin/env bash
# Scala +.NET quality defaults qualification harness.
#
# Qualifies the provisional Scala + .NET format plus lint defaults against the
# native-configuration contract with no hidden presets. covers
# adapters (plus digests), not versions: this harness owns versions plus
# rule-sets.
# - pinned: Scalafmt 3.11.4, Scalafix 0.14.7, CSharpier 1.3.0 (targets
#   .NET 8.0), Fantomas 7.x stable line (8.0.0 alphas rejected), FSharpLint
#   0.27.0 (targets .NET 8.0), Roslyn SDK-coupled (no separate version,
#   follows the qualified .NET SDK) in
#   `scala/tests/fixtures/scala_dotnet_quality/pins.bzl` (living at head
#   rejected); managed-JVM plus exact-package identities recorded, digests
# stay owned.
# - rule-sets: native-configuration sole policy, no hidden presets. Without
#   an applicable checked-in native config the pinned tool uses upstream
#   built-in defaults; with a config it interprets natively; adapters add
#   only transport/hermetic settings. Scalafix recommended built-ins plus
#   OrganizeImports plus RemoveUnused never auto-supplied (checked-in
#   .scalafix.conf required); Roslyn SDK default analysis mode is the
#   upstream built-in default; StyleCop stays opt-in, never the default;
#   FSharpLint default ruleset ships with formatting rules off (Fantomas
#   owns formatting). Auto preset plus beyond-default maxima rejected.
# - fixtures: `scala/tests/fixtures/scala_dotnet_quality/` pins plus BUILD;
#   no Scala + .NET native-config preset, no adapter claim, no curated
#   defaults, no matrix cells; scala/csharp/fsharp hello fixtures stay green.
# - open owned gaps: digests plus adapters under, platform plus consumer
#   plus release evidence, no `Supported` claim. Compatibility is defaults only.
#
# Versioned here, run by CI via `bazel run //tools/ci:scala_dotnet_defaults_qualification`,
# following //tools/ci:jvm_quality_qualification.
set -euo pipefail

# Shared workspace + runfiles helpers.
# Bootstrap: Bazel runfiles forest first (`data = ["//tools/sh:lib"]`), then source tree.
source "${RUNFILES_DIR:-/dev/null}/_main/tools/sh/lib.sh" 2>/dev/null || source "${TEST_SRCDIR:-/dev/null}/_main/tools/sh/lib.sh" 2>/dev/null || source "$0.runfiles/_main/tools/sh/lib.sh" 2>/dev/null || source "${BASH_SOURCE[0]}.runfiles/_main/tools/sh/lib.sh" 2>/dev/null || source "$(dirname "${BASH_SOURCE[0]}")/../sh/lib.sh"

dx_cd_workspace

dx_test_init

pins="scala/tests/fixtures/scala_dotnet_quality/pins.bzl"
pins_build="scala/tests/fixtures/scala_dotnet_quality/BUILD.bazel"
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
  bad "scala-dotnet quality fixture missing (want $pins plus $pins_build)"
fi

# Pins record the five qualified upstream versions plus the Fantomas line
# and the SDK-coupled Roslyn.
if grep -q -F -e 'SCALAFMT_VERSION = "3.11.4"' "$pins" &&
  grep -q -F -e 'SCALAFIX_VERSION = "0.14.7"' "$pins" &&
  grep -q -F -e 'CSHARPIER_VERSION = "1.3.0"' "$pins" &&
  grep -q -F -e 'FANTOMAS_LINE = "7.x stable"' "$pins" &&
  grep -q -F -e 'FSHARPLINT_VERSION = "0.27.0"' "$pins" &&
  grep -q -F -e 'ROSLYN_COUPLING = "SDK-built-in' "$pins"; then
  ok
else
  bad "pins.bzl lost its five qualified Scala + .NET versions plus Fantomas line plus Roslyn coupling under issue #486"
fi

# Pins record the route identities plus rejected head/alpha lines.
if grep -q -F -e 'compatible JVM artifact over the shared managed JDK' "$pins" &&
  grep -q -F -e 'semantic-rule artifacts over the shared managed JDK' "$pins" &&
  grep -q -F -e 'exact official tool package as declared DLLs' "$pins" &&
  grep -q -F -e 'living at head rejected' "$pins" &&
  grep -q -F -e '8.0.0 alphas' "$pins" &&
  grep -q -F -e 'never installed via `dotnet tool install`' "$pins"; then
  ok
else
  bad "pins.bzl lost its route identities plus rejected head/alpha lines under issue #486"
fi

# Pins record the sole-policy plus qualified rule-set resolutions plus rejections.
if grep -q -F -e 'NATIVE_CONFIG_POLICY = "native-configuration sole policy: no hidden presets"' "$pins" &&
  grep -q -F -e 'no auto-supplied OrganizeImports plus RemoveUnused preset' "$pins" &&
  grep -q -F -e 'SDK default analysis mode is the upstream built-in default mode' "$pins" &&
  grep -q -F -e 'StyleCop stays opt-in, never the default' "$pins" &&
  grep -q -F -e 'formatting rules off' "$pins" &&
  grep -q -F -e 'Fantomas owns formatting' "$pins" &&
  grep -q -F -e 'auto preset rejected' "$pins" &&
  grep -q -F -e 'hidden presets rejected' "$pins"; then
  ok
else
  bad "pins.bzl lost its sole-policy plus rule-set resolutions plus rejections under issue #486"
fi

# No hidden Scala + .NET native-config preset: no cohort binding exists in
# the typed native-config rules (adapters run pinned upstream defaults until
# checked-in policy qualifies against the native-config contract).
cohort_config=""
for tool in scalafmt scalafix csharpier fantomas fsharplint roslyn; do
  if grep -q -F -e "${tool}_config" "$native"; then
    cohort_config="$cohort_config $tool:preset"
  fi
done
if [[ -z "$cohort_config" ]]; then
  ok
else
  bad "native-config carries a hidden Scala + .NET preset:$cohort_config"
fi

# No false adapter claim for the Scala + .NET cohort: none of the cohort
# tool IDs appear in REAL_ADAPTERS. Classification exists; adapter claim
# does not (owned).
cohort_claim=""
for tool in scalafmt scalafix csharpier fantomas fsharplint roslyn; do
  if grep -q -F -e "\"$tool\":" "$adapters"; then
    cohort_claim="$cohort_claim $tool:claimed"
  fi
done
if [[ -z "$cohort_claim" ]]; then
  ok
else
  bad "false adapter claim for Scala + .NET cohort:$cohort_claim"
fi

# Classification-only today: scala/csharp/fsharp families carry no curated
# defaults, no runner-matrix cells (claims land only with green adapter
# evidence; StyleCop stays opt-in, never default).
cohort_curated=""
for family in '"scala": {' '"csharp": {' '"fsharp": {'; do
  if grep -q -F -e "$family" "$curated"; then
    cohort_curated="$cohort_curated $family:claimed"
  fi
done
if [[ -z "$cohort_curated" ]] &&
  ! grep -q -F -e 'matrix_scala_' "$matrix" &&
  ! grep -q -F -e 'matrix_csharp_' "$matrix" &&
  ! grep -q -F -e 'matrix_fsharp_' "$matrix"; then
  ok
else
  bad "curated or matrix claims a Scala + .NET family before adapters land:$cohort_curated"
fi

# Support matrix keeps the qualified Scala + .NET versions plus rule-sets
# with fixtures and harness; digests plus adapters stay under.
if grep -q -F -e 'qualified seed-only under issue #486' "$support" &&
  grep -q -F -e 'scala_dotnet_defaults_qualification' "$support" &&
  grep -q -F -e 'scala/tests/fixtures/scala_dotnet_quality/pins.bzl' "$support" &&
  grep -q -F -e 'no hidden preset' "$support" &&
  grep -q -F -e 'digests plus adapter mappings stay owned under issue #417' "$support"; then
  ok
else
  bad "support-matrix lost its #486 qualified Scala + .NET versions plus rule-sets record with fixtures"
fi

# Support matrix resolves the Scalafix/Roslyn/FSharpLint conflicts as
# upstream built-in defaults (no auto-supplied preset, StyleCop opt-in).
if grep -q -F -e 'no auto-supplied OrganizeImports' "$support" &&
  grep -q -F -e 'StyleCop stays opt-in' "$support" &&
  grep -q -F -e 'upstream built-in defaults' "$support" &&
  grep -q -F -e 'qualified seed-only under issue #486' "$support"; then
  ok
else
  bad "support-matrix lost its #486 Scalafix plus Roslyn plus FSharpLint conflict resolution"
fi

# Tool acquisition keeps the decided managed-JVM route for Scalafmt/Scalafix
# with no source-built route and no false claim; versions qualified under
# , digests plus adapters stay pending under.
if grep -q -F -e 'Decided route: Scalafmt and Scalafix take the' "$acquisition" &&
  grep -q -F -e 'same shared managed JDK and Maven-lock story' "$acquisition" &&
  grep -q -F -e 'adapter claims `scala` yet' "$acquisition" &&
  grep -q -F -e 'qualified seed-only under issue #486' "$acquisition" &&
  grep -q -F -e 'digests plus adapter mappings stay owned under issue #417' "$acquisition"; then
  ok
else
  bad "tool-acquisition lost its decided Scala route with #486 versions plus #417 digests/adapters split"
fi

# Tool acquisition keeps the decided exact-package plus shared-.NET-runtime
# route for CSharpier/Fantomas with no installer on the consumer path and
# no false claim; versions qualified under, bounds plus adapters stay
# pending under.
if grep -q -F -e 'Decided route: CSharpier and Fantomas take the' "$acquisition" &&
  grep -q -F -e 'no consumer runs `dotnet tool install`' "$acquisition" &&
  grep -q -F -e 'no adapter claims `csharp` or' "$acquisition" &&
  grep -q -F -e 'qualified seed-only under issue #486' "$acquisition" &&
  grep -q -F -e 'bounds plus adapter mappings stay owned under issue #417' "$acquisition"; then
  ok
else
  bad "tool-acquisition lost its decided .NET route with #486 versions plus #417 bounds/adapters split"
fi

# Tool acquisition keeps the five research rows with byte-identity risk.
cohort_research=""
for tool in '| Scalafmt |' '| Scalafix |' '| CSharpier |' '| Fantomas |' '| FSharpLint |'; do
  grep -q -F -e "$tool" "$acquisition" || cohort_research="$cohort_research $tool:missing"
done
if [[ -z "$cohort_research" ]] &&
  grep -q -F -e 'maintainer acquisition must establish and record byte identity' "$acquisition"; then
  ok
else
  bad "tool-acquisition lost a Scala + .NET research row or byte-identity honesty:$cohort_research"
fi

# Tool integrations keep the Scala + .NET adapter-input notes with open
# parser work plus pinned versions (adapters still open under).
if grep -q -F -e 'Scala + .NET cohort' "$integrations" &&
  grep -q -F -e 'no adapter claims `scala`, `csharp`, or' "$integrations" &&
  grep -q -F -e 'no machine-readable CLI output' "$integrations" &&
  grep -q -F -e 'qualified seed-only under issue #486' "$integrations" &&
  grep -q -F -e 'adapters stay owned under issue #417' "$integrations"; then
  ok
else
  bad "tool-integrations lost its Scala + .NET notes with #486 versions plus #417 adapters split"
fi

# Native-configuration sole policy stands (no hidden presets authorized).
if grep -q -F -e 'defines no hidden rule' "$native_doc" &&
  grep -q -F -e 'sole behavioral policy' "$native_doc"; then
  ok
else
  bad "native-configuration lost its sole-policy plus no-hidden-preset honesty"
fi

# Verification matrix owns the qualified seed-only record under.
if grep -q -F -e 'scala_dotnet_defaults_qualification' "$verify" &&
  grep -q -F -e 'qualified seed-only under #486' "$verify" &&
  grep -q -F -e 'bazel run //tools/ci:scala_dotnet_defaults_qualification' "$verify" &&
  grep -q -F -e '`scala_dotnet_defaults_qualification` 17/17' "$verify"; then
  ok
else
  bad "verification-matrix lost its #486 Scala + .NET quality qualified record"
fi

# BUILD owns the harness target plus CI wires it in dogfood-freshness.
if grep -q -F -e 'name = "scala_dotnet_defaults_qualification"' "$build" &&
  grep -q -F -e 'bazel run --noshow_progress //tools/ci:scala_dotnet_defaults_qualification' "$ci"; then
  ok
else
  bad "tools/ci/BUILD.bazel or ci.yml lost the scala_dotnet_defaults_qualification wiring (want target plus dogfood-freshness)"
fi

# Live proof: Scala + .NET foundation fixtures stay green on the seed host
# (defaults change only; no adapter behavior yet).
if bazel test //scala/tests/fixtures/hello:hello_test //csharp/tests/fixtures/hello:hello_test //fsharp/tests/fixtures/hello:hello_test --noshow_progress >/dev/null 2>&1; then
  ok
else
  bad "scala-dotnet quality live proof failed (want scala plus csharp plus fsharp hello green)"
fi

dx_test_summary "scala-dotnet defaults qualification harness"
