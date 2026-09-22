#!/usr/bin/env bash
# Scala +.NET-cohort qualification harness.
#
# Qualifies the as-built Scala + .NET quality-cohort record with fixture
# evidence and owned gaps, without claiming Supported and without a false
# adapter claim:
# - delivered: decided managed-JVM route for Scalafmt/Scalafix (compatible
#   JVM artifacts over the shared managed JDK plus the Scala Maven-lock
#   story, `maven_install.json` plus `fail_if_repin_required`, with
#   semanticdb plus classpath wiring for semantic rules) and decided
#   exact-package plus shared-.NET-runtime route for CSharpier/Fantomas
#   (official tool packages as declared DLLs over one managed .NET cohort;
#   no `dotnet tool install` on the consumer path), initial artifact
#   research rows as observations for digests (versions qualified seed-only
# , adapter-input notes with Roslyn per-TFM/RID SARIF
#   aggregation, FSharpLint console-parse versus library-API binding,
#   Scalafix console-output limitation recorded as a parse-vs-wire decision
#   (never silently dropped; Roslyn SDK-default mode with StyleCop opt-in),
# native-config defaults qualified seed-only (Scalafix
#   OrganizeImports plus RemoveUnused, FSharpLint default ruleset with
#   formatting off as upstream built-in defaults with no hidden preset),
#   parity-deferred scala/csharp/fsharp with owner plus frozen route,
#   classification-only taxonomy with no curated defaults and no
#   native-config binding;
# - open under with honest records: exact artifact digests
#   plus shared-JDK/.NET cohort qualification, parser plus runner-matrix
#   pass/fail plus fix/format evidence per adapter-backed class,
#   native-config qualification against the native-config contract,
#   platform plus consumer plus release evidence. REAL_ADAPTERS claims
#   scala/csharp/fsharp only when green.
#
# Versioned here, run by CI via `bazel run //tools/ci:scala_dotnet_cohort_qualification`,
# following //tools/ci:jvm_cohort_qualification.
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

# Adapters delivered under #797 (successor to closed #417): all six cohort
# tools appear in REAL_ADAPTERS.
cohort_claim=""
for tool in scalafmt scalafix csharpier fantomas fsharplint roslyn; do
  grep -q -F -e "\"$tool\":" "$adapters" || cohort_claim="$cohort_claim $tool:missing"
done
if [[ -z "$cohort_claim" ]]; then
  ok
else
  bad "Scala + .NET adapter delivery missing:$cohort_claim (want all six under #797)"
fi

# Parity no longer defers the delivered Scala/.NET classes (closed #417 owns
# nothing here; delivery under #797).
if ! grep -q -F -e '"scala":' "$parity" &&
  ! grep -q -F -e '"csharp":' "$parity" &&
  ! grep -q -F -e '"fsharp":' "$parity"; then
  ok
else
  bad "parity still defers delivered Scala/.NET classes (want none under #797)"
fi

# Every cohort class stays classified in the frozen taxonomy, one family each.
if grep -q -F -e '"scala": "scala"' "$adapters" &&
  grep -q -F -e '"csharp": "csharp"' "$adapters" &&
  grep -q -F -e '"fsharp": "fsharp"' "$adapters"; then
  ok
else
  bad "frozen taxonomy lost the scala/csharp/fsharp classification"
fi

# Classification-only today: scala/csharp/fsharp families carry no curated
# defaults (curated membership unchanged; StyleCop stays opt-in, never default).
cohort_curated=""
for family in '"scala": {' '"csharp": {' '"fsharp": {'; do
  if grep -q -F -e "$family" "$curated"; then
    cohort_curated="$cohort_curated $family:claimed"
  fi
done
if [[ -z "$cohort_curated" ]]; then
  ok
else
  bad "curated defaults claim a Scala + .NET family before adapters land:$cohort_curated"
fi

# Scala + .NET native-config bindings delivered under #797.
cohort_config=""
for tool in scalafmt scalafix csharpier fsharplint; do
  grep -q -F -e "${tool}_config" "$native" || cohort_config="$cohort_config $tool:missing"
done
if [[ -z "$cohort_config" ]]; then
  ok
else
  bad "native-config lost Scala + .NET bindings:$cohort_config (want all four under #797)"
fi

# Matrix carries Scala + .NET cells with green pass/fail plus fix/format
# evidence per adapter-backed class (delivery under #797).
if grep -q -F -e 'SCALA_DOTNET_CASES' "$matrix"; then
  ok
else
  bad "runner matrix lost Scala + .NET cells (want SCALA_DOTNET_CASES under #797)"
fi

# Tool acquisition keeps the decided managed-JVM route for Scalafmt/Scalafix
# with the Maven-lock story plus semanticdb/classpath wiring and no false
# claim, owned by (live successor to closed for this cohort).
if grep -q -F -e 'Decided route: Scalafmt and Scalafix take the' "$acquisition" &&
  grep -q -F -e 'same shared managed JDK and Maven-lock story' "$acquisition" &&
  grep -q -F -e 'adapter claims `scala` yet' "$acquisition" &&
  grep -q -F -e '(open under issue #417)' "$acquisition"; then
  ok
else
  bad "tool-acquisition lost its decided Scala managed-JVM route or #417 ownership or no-claim honesty"
fi

# Tool acquisition keeps the decided exact-package plus shared-.NET-runtime
# route for CSharpier/Fantomas with no installer on the consumer path and no
# false claim, owned by.
if grep -q -F -e 'Decided route: CSharpier and Fantomas take the' "$acquisition" &&
  grep -q -F -e 'no consumer runs `dotnet tool install`' "$acquisition" &&
  grep -q -F -e 'no adapter claims `csharp` or' "$acquisition" &&
  grep -q -F -e '(open under issue #417)' "$acquisition"; then
  ok
else
  bad "tool-acquisition lost its decided .NET route or #417 ownership or no-claim honesty"
fi

# Tool acquisition keeps initial artifact research rows for the cohort as
# observations, not pins, with byte-identity risk explicit.
cohort_research=""
for tool in '| Scalafmt |' '| Scalafix |' '| CSharpier |' '| Fantomas |' '| FSharpLint |'; do
  grep -q -F -e "$tool" "$acquisition" || cohort_research="$cohort_research $tool:missing"
done
if [[ -z "$cohort_research" ]] &&
  grep -q -F -e 'owned by issue #417' "$acquisition" &&
  grep -q -F -e 'observations, not pins' "$acquisition" &&
  grep -q -F -e 'maintainer acquisition must establish and record byte identity' "$acquisition"; then
  ok
else
  bad "tool-acquisition lost a Scala + .NET research row or its observations-not-pins honesty:$cohort_research"
fi

# Tool integrations record the Scala + .NET adapter delivery under #797
# (successor to the #417 provisional notes): decided routes plus wire
# decisions plus versions qualified under #486, adapters qualified under #797.
if grep -q -F -e 'Scala + .NET cohort' "$integrations" &&
  grep -q -F -e 'adapters qualified seed-only under #797' "$integrations" &&
  grep -q -F -e 'no machine-readable CLI output' "$integrations" &&
  grep -q -F -e 'not silent' "$integrations" &&
  grep -q -F -e 'SDK-default mode' "$integrations" &&
  grep -q -F -e 'StyleCop remains' "$integrations"; then
  ok
else
  bad "tool-integrations lost its Scala + .NET delivery record under #797"
fi

# Support matrix keeps the Scala + .NET routes plus qualified native-config
# defaults plus adapter-input notes plus cohort tracking, all
# citing for adapters/digests without approving hidden presets or
# claiming support.
# Tool baseline keeps the Scala/C#/F# coverage rows (integration inventory,
# not a support claim).
if grep -q -F -e '| Scala | scalafmt | scalafix |' "$baseline" &&
  grep -q -F -e '| C# | CSharpier |' "$baseline" &&
  grep -q -F -e '| F# | Fantomas |' "$baseline"; then
  ok
else
  bad "tool-baseline lost its Scala/C#/F# coverage rows"
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

dx_test_summary "Scala + .NET-cohort qualification harness"
