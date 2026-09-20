#!/usr/bin/env bash
# JVM-cohort qualification harness.
#
# Qualifies the as-built JVM quality-cohort record with fixture evidence
# and owned gaps, without claiming Supported and without a false adapter
# claim:
# - delivered: decided complete-upstream-artifact plus shared-JDK route
#   (google-java-format and Checkstyle all-deps JARs, PMD and SpotBugs
#   binary distributions, ktfmt with-deps JAR, ktlint executable JAR over
#   one managed JDK cohort; no Maven-module reconstruction, no
#   installer/solver/compiler on the consumer path), initial artifact
#   research rows as observations for digests (versions qualified seed-only
# , provisional SARIF-native
#   adapter-input notes with Error Prone javac-diagnostic parsing itemized
#   as open work (never silently dropped), native-config defaults qualified
# seed-only (SpotBugs default effort, PMD default ruleset,
#   Error Prone default severities, detekt buildUponDefaultConfig, ktlint
#   standard as upstream built-in defaults with no hidden preset),
#   parity-deferred java/kotlin
#   with owner plus frozen route, classification-only taxonomy with no
#   curated defaults and no native-config binding;
# - open under with honest records: exact artifact digests
#   plus shared-JDK cohort qualification, SARIF parser plus runner-matrix
#   pass/fail plus fix/format evidence per adapter-backed class,
#   native-config qualification against the native-config contract,
#   platform plus consumer plus release evidence. REAL_ADAPTERS claims
#   java/kotlin only when green.
#
# Versioned here, run by CI via `bazel run //tools/ci:jvm_cohort_qualification`,
# following //tools/ci:file_family_qualification.
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

# No false adapter claim for the JVM cohort: none of the cohort tool IDs
# appear in REAL_ADAPTERS. Classification exists in
# REAL_CLASS_TO_FAMILY; adapter claim does not.
jvm_claim=""
for tool in google-java-format checkstyle pmd spotbugs ktfmt ktlint detekt error-prone error_prone; do
  if grep -q -F -e "\"$tool\":" "$adapters"; then
    jvm_claim="$jvm_claim $tool:claimed"
  fi
done
if [[ -z "$jvm_claim" ]]; then
  ok
else
  bad "false adapter claim for JVM cohort:$jvm_claim"
fi

# Parity deferrals own java/kotlin with owner plus frozen route plus the
# live-successor record (closed owns nothing here).
if grep -q -F -e '"java": ["ADR 0019"' "$parity" &&
  grep -q -F -e '"kotlin": ["ADR 0019"' "$parity" &&
  grep -q -F -e 'complete upstream artifact plus shared JDK (google-java-format, Checkstyle, PMD, SpotBugs)' "$parity" &&
  grep -q -F -e 'complete upstream artifact plus shared JDK (ktfmt, ktlint); detekt pending' "$parity" &&
  grep -q -F -e 'issue #416' "$parity"; then
  ok
else
  bad "parity deferrals lost the JVM java/kotlin owner plus frozen route plus #416 record"
fi

# Every JVM class stays classified in the frozen taxonomy, one family each.
if grep -q -F -e '"java": "java"' "$adapters" &&
  grep -q -F -e '"kotlin": "kotlin"' "$adapters"; then
  ok
else
  bad "frozen taxonomy lost the java/kotlin classification"
fi

# Classification-only today: java/kotlin families carry no curated defaults
# (curated membership unchanged; opt-ins stay opt-ins).
jvm_curated=""
for family in '"java": {' '"kotlin": {'; do
  if grep -q -F -e "$family" "$curated"; then
    jvm_curated="$jvm_curated $family:claimed"
  fi
done
if [[ -z "$jvm_curated" ]]; then
  ok
else
  bad "curated defaults claim a JVM family before adapters land:$jvm_curated"
fi

# No hidden JVM native-config preset: no JVM binding exists in the typed
# native-config rules (adapters run pinned upstream defaults until 
# qualifies checked-in policy against the native-config contract).
jvm_config=""
for tool in google-java-format checkstyle pmd spotbugs ktfmt ktlint detekt; do
  if grep -q -F -e "${tool}_config" "$native"; then
    jvm_config="$jvm_config $tool:preset"
  fi
done
if [[ -z "$jvm_config" ]]; then
  ok
else
  bad "native-config carries a hidden JVM preset:$jvm_config"
fi

# No false green claim: the runner matrix carries no JVM cells yet, so
# REAL_ADAPTERS cannot claim java/kotlin (claims land only with green
# pass/fail plus fix/format evidence per adapter-backed class). The
# trailing underscore keeps `matrix_javascript_*` from matching a future
# `matrix_java_*` cell.
if ! grep -q -F -e 'matrix_java_' "$matrix" &&
  ! grep -q -F -e 'matrix_kotlin_' "$matrix"; then
  ok
else
  bad "runner matrix claims a JVM cell without adapter qualification"
fi

# Tool acquisition keeps the decided complete-upstream-artifact plus
# shared-JDK route with no Maven reconstruction and no false claim,
# owned by (live successor to closed for this cohort).
if grep -q -F -e 'Decided route: google-java-format, Checkstyle,' "$acquisition" &&
  grep -q -F -e 'ktlint executable JAR) sharing the one managed JDK cohort runtime' "$acquisition" &&
  grep -q -F -e 'no adapter claims `java` or `kotlin` yet' "$acquisition" &&
  grep -q -F -e '(open under issue #416)' "$acquisition" &&
  grep -q -F -e 'no tool' "$acquisition"; then
  ok
else
  bad "tool-acquisition lost its decided JVM route or #416 ownership or no-claim honesty"
fi

# Tool acquisition keeps initial artifact research rows for the cohort as
# observations, not pins, with byte-identity risk explicit.
jvm_research=""
for tool in '| google-java-format |' '| Checkstyle |' '| PMD |' '| SpotBugs |' '| ktfmt |' '| ktlint |' '| detekt |' '| Error Prone |'; do
  grep -q -F -e "$tool" "$acquisition" || jvm_research="$jvm_research $tool:missing"
done
if [[ -z "$jvm_research" ]] &&
  grep -q -F -e 'owned by issue #416' "$acquisition" &&
  grep -q -F -e 'observations, not pins' "$acquisition" &&
  grep -q -F -e 'maintainer acquisition must establish and record byte identity' "$acquisition"; then
  ok
else
  bad "tool-acquisition lost a JVM research row or its observations-not-pins honesty:$jvm_research"
fi

# Tool integrations keep the JVM adapter-input notes: SARIF
# shapes as unproven mappings, Error Prone javac-diagnostic parsing
# itemized as open work (never silently dropped), versions qualified under
# with digests as observations, no adapter claim.
if grep -q -F -e '**JVM cohort (issue #416' "$integrations" &&
  grep -q -F -e 'unproven mappings' "$integrations" &&
  grep -q -F -e 'observations, not pins' "$integrations" &&
  grep -q -F -e 'no adapter claims `java` or `kotlin` yet' "$integrations" &&
  grep -q -F -e 'Error Prone has no' "$integrations" &&
  grep -q -F -e 'itemized here, not silently dropped' "$integrations"; then
  ok
else
  bad "tool-integrations lost its JVM adapter-input notes or Error Prone open-work honesty"
fi

# Support matrix keeps the JVM route plus qualified native-config defaults
# plus SARIF notes plus cohort tracking, all citing for
# adapters/digests without approving hidden presets or claiming support.
if grep -q -F -e 'take the complete-upstream-artifact plus shared-JDK route (issue #416' "$support" &&
  grep -q -F -e 'qualified seed-only under issue #485' "$support" &&
  grep -q -F -e 'no auto-supplied Google checks' "$support" &&
  grep -q -F -e 'upstream built-in defaults' "$support" &&
  grep -q -F -e 'owned by issue #416.' "$support" &&
  grep -q -F -e 'itemized under issue #416' "$support" &&
  grep -q -F -e '(issue #416)' "$support" &&
  grep -q -F -e 'to issue #416;' "$support"; then
  ok
else
  bad "support-matrix lost its JVM route, qualified defaults, adapter notes, or #416 cohort tracking"
fi

# Tool baseline keeps the Java/Kotlin coverage rows (integration inventory,
# not a support claim).
if grep -q -F -e '| Java | google-java-format | PMD, Checkstyle, SpotBugs |' "$baseline" &&
  grep -q -F -e '| Kotlin | ktfmt | ktlint |' "$baseline"; then
  ok
else
  bad "tool-baseline lost its Java/Kotlin coverage rows"
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

dx_test_summary "JVM-cohort qualification harness"
