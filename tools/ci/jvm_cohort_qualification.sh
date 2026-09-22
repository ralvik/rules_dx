#!/usr/bin/env bash
# JVM-cohort qualification harness.
#
# Qualifies the as-built JVM quality-cohort record with fixture evidence
# and owned gaps, without claiming Supported:
# - delivered under #796 (successor to closed #416): decided
#   complete-upstream-artifact plus shared-JDK route
#   (google-java-format and Checkstyle all-deps JARs, PMD and SpotBugs
#   binary distributions, ktfmt with-deps JAR, ktlint CLI all JAR over
#   one managed JDK cohort; no Maven-module reconstruction, no
#   installer/solver/compiler on the consumer path), digests pinned in
#   MODULE.bazel plus java_binary wrappers in quality/tools/jvm/,
#   REAL_ADAPTERS claims java/kotlin via google_java_format, checkstyle,
#   pmd, spotbugs, ktfmt, ktlint (detekt pending plus Error Prone
#   itemized open work, never silently dropped), SARIF plus
#   dry-run parsers with runner-matrix pass/fail plus fix/format
#   evidence per adapter-backed class, Checkstyle native-config binding
#   against the native-config contract (others run upstream defaults);
# - open under #796 with honest records: detekt plus Error Prone
#   adapters, platform plus consumer plus release evidence.
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

# Delivered adapter claims for the JVM cohort: the six cohort tool IDs
# appear in REAL_ADAPTERS with java/kotlin classes; detekt plus Error Prone
# stay unclaimed (pending plus itemized open work, never silently dropped).
jvm_missing=""
for tool in google_java_format checkstyle pmd spotbugs ktfmt ktlint; do
  grep -q -F -e "\"$tool\":" "$adapters" || jvm_missing="$jvm_missing $tool:missing"
done
jvm_pending=""
for tool in detekt error_prone; do
  if grep -q -F -e "\"$tool\":" "$adapters"; then
    jvm_pending="$jvm_pending $tool:claimed"
  fi
done
if [[ -z "$jvm_missing" && -z "$jvm_pending" ]]; then
  ok
else
  bad "JVM adapter claims drifted (missing:$jvm_missing pending:$jvm_pending)"
fi

# Parity deferrals no longer own java/kotlin (delivered under #796):
# adapter-backed classes leave PARITY_DEFERRED; detekt/error_prone own
# no class (they are tool-level pending, never a separate class).
if ! grep -q -F -e '"java":' "$parity" &&
  ! grep -q -F -e '"kotlin":' "$parity" &&
  grep -q -F -e 'PARITY_DEFERRED = {' "$parity"; then
  ok
else
  bad "parity deferrals still claim delivered java/kotlin (want removed under #796)"
fi

# Every JVM class stays classified in the frozen taxonomy, one family each.
if grep -q -F -e '"java": "java"' "$adapters" &&
  grep -q -F -e '"kotlin": "kotlin"' "$adapters"; then
  ok
else
  bad "frozen taxonomy lost the java/kotlin classification"
fi

# Delivered curated defaults: java/kotlin families carry the baseline
# formatter plus linters (google-java-format plus checkstyle/pmd/spotbugs;
# ktfmt plus ktlint).
if grep -q -F -e '"java": {' "$curated" &&
  grep -q -F -e '"kotlin": {' "$curated" &&
  grep -q -F -e '"google_java_format"' "$curated" &&
  grep -q -F -e '"ktfmt"' "$curated" &&
  grep -q -F -e '"ktlint"' "$curated"; then
  ok
else
  bad "curated defaults lost delivered java/kotlin families under #796"
fi

# Delivered native-config binding: Checkstyle carries the typed XML
# binding (config-required, no usable upstream default); the remaining
# cohort tools run pinned upstream defaults with no hidden preset.
if grep -q -F -e 'checkstyle_config' "$native" &&
  grep -q -F -e '"checkstyle": ".xml"' "$native"; then
  ok
else
  bad "native-config lost delivered checkstyle binding under #796"
fi

# Delivered green claim: the runner matrix carries JVM cells with
# pass/fail plus fix/format evidence per adapter-backed class (SpotBugs
# stays target-coupled with no matrix cell, like tsc). Cells live in
# runner_matrix_jvm.bzl (split from the cases file, no behavior change).
jvm_matrix="quality/testdata/runner_matrix_jvm.bzl"
if grep -q -F -e 'matrix_java_format_pass' "$jvm_matrix" &&
  grep -q -F -e 'matrix_java_format_fail' "$jvm_matrix" &&
  grep -q -F -e 'matrix_java_checkstyle_pass' "$jvm_matrix" &&
  grep -q -F -e 'matrix_java_checkstyle_fail' "$jvm_matrix" &&
  grep -q -F -e 'matrix_java_pmd_pass' "$jvm_matrix" &&
  grep -q -F -e 'matrix_java_pmd_fail' "$jvm_matrix" &&
  grep -q -F -e 'matrix_kotlin_format_pass' "$jvm_matrix" &&
  grep -q -F -e 'matrix_kotlin_format_fail' "$jvm_matrix" &&
  grep -q -F -e 'matrix_kotlin_lint_pass' "$jvm_matrix" &&
  grep -q -F -e 'matrix_kotlin_lint_fail' "$jvm_matrix"; then
  ok
else
  bad "runner matrix lost delivered JVM cells under #796"
fi

# Tool acquisition keeps the decided complete-upstream-artifact plus
# shared-JDK route with digests pinned plus adapters delivered under #796.
if grep -q -F -e 'Decided route: google-java-format, Checkstyle,' "$acquisition" &&
  grep -q -F -e 'sharing the one managed JDK cohort runtime' "$acquisition" &&
  grep -q -F -e 'with `java` plus `kotlin` claimed' "$acquisition" &&
  grep -q -F -e '(delivered under #796' "$acquisition"; then
  ok
else
  bad "tool-acquisition lost its delivered JVM route under #796"
fi

# Tool acquisition keeps initial artifact research rows for the cohort
# with byte-identity risk explicit (digests now pinned in MODULE.bazel).
jvm_research=""
for tool in '| google-java-format |' '| Checkstyle |' '| PMD |' '| SpotBugs |' '| ktfmt |' '| ktlint |' '| detekt |' '| Error Prone |'; do
  grep -q -F -e "$tool" "$acquisition" || jvm_research="$jvm_research $tool:missing"
done
if [[ -z "$jvm_research" ]] &&
  grep -q -F -e 'maintainer acquisition must establish and record byte identity' "$acquisition"; then
  ok
else
  bad "tool-acquisition lost a JVM research row:$jvm_research"
fi

# Tool integrations keep the delivered JVM adapter notes: SARIF
# shapes plus dry-run grammars qualified with fixtures, Error Prone
# javac-diagnostic parsing itemized as open work (never silently
# dropped), versions qualified under #485 with digests pinned.
if grep -q -F -e '**JVM cohort (#796' "$integrations" &&
  grep -q -F -e 'digests pinned in `MODULE.bazel`' "$integrations" &&
  grep -q -F -e 'adapters delivered under #796' "$integrations" &&
  grep -q -F -e 'Error Prone has no' "$integrations" &&
  grep -q -F -e 'itemized here, not silently dropped' "$integrations"; then
  ok
else
  bad "tool-integrations lost its delivered JVM notes under #796"
fi

# Support matrix keeps the JVM rows plus the delivered Layer-2 mapping
# (Java/Kotlin stay Planned per the lifecycle; verification Delivered is
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
