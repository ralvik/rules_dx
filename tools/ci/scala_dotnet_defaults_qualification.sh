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
# Bootstrap via tools/sh/bootstrap.sh (issue #654): runfiles forest first, then source tree.
source "${RUNFILES_DIR:-/dev/null}/_main/tools/sh/bootstrap.sh" 2>/dev/null || source "${TEST_SRCDIR:-/dev/null}/_main/tools/sh/bootstrap.sh" 2>/dev/null || source "$0.runfiles/_main/tools/sh/bootstrap.sh" 2>/dev/null || source "${BASH_SOURCE[0]}.runfiles/_main/tools/sh/bootstrap.sh" 2>/dev/null || source "$(git rev-parse --show-toplevel 2>/dev/null)/tools/sh/bootstrap.sh"
dx_bootstrap "tools/sh/lib.sh"

dx_cd_workspace

dx_test_init

dx_bash_pin

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

# Scala + .NET native-config bindings delivered under #797 (checked-in
# policy qualifies against the native-config contract; no hidden preset).
cohort_config=""
for tool in scalafmt scalafix csharpier fsharplint; do
  grep -q -F -e "${tool}_config" "$native" || cohort_config="$cohort_config $tool:missing"
done
if [[ -z "$cohort_config" ]]; then
  ok
else
  bad "native-config lost Scala + .NET bindings:$cohort_config (want all four under #797)"
fi

# Adapters delivered under #797: all six cohort tools appear in REAL_ADAPTERS.
cohort_claim=""
for tool in scalafmt scalafix csharpier fantomas fsharplint roslyn; do
  grep -q -F -e "\"$tool\":" "$adapters" || cohort_claim="$cohort_claim $tool:missing"
done
if [[ -z "$cohort_claim" ]]; then
  ok
else
  bad "Scala + .NET adapter delivery missing:$cohort_claim (want all six under #797)"
fi

# Delivered: scala/csharp/fsharp families carry no curated defaults (claims
# land only with green adapter evidence) but do carry runner-matrix cells
# under #797; StyleCop stays opt-in, never default.
cohort_curated=""
for family in '"scala": {' '"csharp": {' '"fsharp": {'; do
  if grep -q -F -e "$family" "$curated"; then
    cohort_curated="$cohort_curated $family:claimed"
  fi
done
if [[ -z "$cohort_curated" ]] &&
  grep -q -F -e 'SCALA_DOTNET_CASES' "$matrix"; then
  ok
else
  bad "curated claims a Scala + .NET family or matrix lost cohort cells:$cohort_curated (want no curated, SCALA_DOTNET_CASES under #797)"
fi

# Support matrix keeps the qualified Scala + .NET versions plus rule-sets
# with fixtures and harness; digests plus adapters stay under.
# Support matrix resolves the Scalafix/Roslyn/FSharpLint conflicts as
# upstream built-in defaults (no auto-supplied preset, StyleCop opt-in).
# Tool acquisition keeps the decided managed-JVM route for Scalafmt/Scalafix
# with no source-built route and no false claim; versions qualified under
# , digests plus adapters stay pending under.
if grep -q -F -e 'managed JVM route for Scalafmt (compatible JVM artifact over the shared' "$integrations" &&
  grep -q -F -e 'adapters qualified seed-only under #797' "$integrations" &&
  grep -q -F -e 'qualified seed-only under issue #486' "$integrations" &&
  grep -q -F -e 'successor to closed #417' "$integrations" &&
  grep -q -F -e 'cohort delivered under #797' "$acquisition"; then
  ok
else
  bad "tool-integrations lost its decided Scala route with #486 versions plus #417 digests/adapters split"
fi

# Tool acquisition keeps the decided exact-package plus shared-.NET-runtime
# route for CSharpier/Fantomas with no installer on the consumer path and
# no false claim; versions qualified under, bounds plus adapters stay
# pending under.
if grep -q -F -e 'exact-package plus shared-.NET-runtime route for CSharpier and Fantomas' "$integrations" &&
  grep -q -F -e 'no `dotnet tool install`' "$integrations" &&
  grep -q -F -e 'qualified seed-only under issue #486' "$integrations" &&
  grep -q -F -e 'successor to closed #417' "$integrations" &&
  grep -q -F -e 'Exact upstream package plus shared .NET runtime | CSharpier, Fantomas' "$acquisition"; then
  ok
else
  bad "tool-integrations lost its decided .NET route with #486 versions plus #417 bounds/adapters split"
fi

# Tool integrations keep the five research rows with byte-identity risk.
cohort_research=""
for tool in 'Scalafmt' 'Scalafix' 'CSharpier' 'Fantomas' 'FSharpLint'; do
  grep -q -F -e "$tool" "$integrations" || cohort_research="$cohort_research $tool:missing"
done
if [[ -z "$cohort_research" ]] &&
  grep -q -F -e 'actual bytes when adding each adapter' "$acquisition"; then
  ok
else
  bad "tool-integrations lost a Scala + .NET research row or byte-identity honesty:$cohort_research"
fi

# Tool integrations keep the Scala + .NET adapter-input notes with pinned
# versions (adapters delivered under #797, successor to #486 versions plus
# #417 ownership).
if grep -q -F -e 'Scala + .NET cohort' "$integrations" &&
  grep -q -F -e 'no machine-readable CLI output' "$integrations" &&
  grep -q -F -e 'qualified seed-only under issue #486' "$integrations" &&
  grep -q -F -e 'adapters qualified seed-only under #797' "$integrations"; then
  ok
else
  bad "tool-integrations lost its Scala + .NET notes with #486 versions plus #797 delivery"
fi

# Native-configuration sole policy stands (no hidden presets authorized).
if grep -q -F -e 'defines no hidden rule' "$native_doc" &&
  grep -q -F -e 'sole behavioral policy' "$native_doc"; then
  ok
else
  bad "native-configuration lost its sole-policy plus no-hidden-preset honesty"
fi

# ci_targets_d.bzl owns the harness target plus dogfood-freshness wires it.
if grep -q -F -e 'name = "scala_dotnet_defaults_qualification"' "tools/ci/ci_targets_d.bzl" &&
  grep -q -F -e 'bazel run --noshow_progress //tools/ci:scala_dotnet_defaults_qualification' tools/ci/dogfood_freshness.sh; then
  ok
else
  bad "tools/ci/ci_targets_d.bzl or dogfood_freshness.sh lost the scala_dotnet_defaults_qualification wiring (want target plus dogfood-freshness)"
fi

# Live proof: Scala + .NET foundation fixtures stay green on the seed host
# (defaults change only; no adapter behavior yet).
if bazel test //scala/tests/fixtures/hello:hello_test //csharp/tests/fixtures/hello:hello_test //fsharp/tests/fixtures/hello:hello_test --noshow_progress >/dev/null 2>&1; then
  ok
else
  bad "scala-dotnet quality live proof failed (want scala plus csharp plus fsharp hello green)"
fi

dx_test_summary "scala-dotnet defaults qualification harness"
