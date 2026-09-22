#!/usr/bin/env bash
# JVM quality defaults qualification harness.
#
# Qualifies the delivered JVM format plus lint defaults against the
# native-configuration contract with no hidden presets. Covers
# versions plus rule-sets (adapters plus digests delivered under #796):
# this harness owns versions plus rule-sets.
# - pinned: google-java-format 1.35.0, Checkstyle 14.1.0, PMD 7.27.0,
#   SpotBugs 4.10.4, ktfmt 0.63, ktlint 1.8.0, detekt 1.23.8, Error Prone
#   2.50.0 in `java/tests/fixtures/jvm_quality/pins.bzl` (v1.36.x plus v0.64
#   head plus 2.0.0 alphas rejected; Error Prone follows the qualified JDK
#   baseline); complete-artifact plus shared-JDK identities recorded, digests
# stay owned.
# - rule-sets: native-configuration sole policy, no hidden presets. Without
#   an applicable checked-in native config the pinned tool uses upstream
#   built-in defaults; with a config it interprets natively; adapters add
#   only transport/hermetic settings. Checkstyle Google checks never
#   auto-supplied; SpotBugs default effort, PMD default ruleset, Error Prone
#   default severities, detekt buildUponDefaultConfig (not allRules), ktlint
#   standard are upstream built-in defaults. Beyond-default switches
#   (detekt allRules, experimental Error Prone, --enable=all maxima) rejected.
# - fixtures: `java/tests/fixtures/jvm_quality/` pins plus BUILD; Checkstyle
#   native-config preset plus adapter claims plus curated defaults plus matrix
#   cells delivered under #796; java/kotlin hello fixtures stay green.
# - open owned gaps: detekt plus Error Prone adapters, platform plus consumer
#   plus release evidence, no `Supported` claim. Compatibility is defaults only.
#
# Versioned here, run by CI via `bazel run //tools/ci:jvm_quality_qualification`,
# following //tools/ci:cc_hermetic_qualification.
set -euo pipefail

# Shared workspace + runfiles helpers.
# Bootstrap via tools/sh/bootstrap.sh (issue #654): runfiles forest first, then source tree.
source "${RUNFILES_DIR:-/dev/null}/_main/tools/sh/bootstrap.sh" 2>/dev/null || source "${TEST_SRCDIR:-/dev/null}/_main/tools/sh/bootstrap.sh" 2>/dev/null || source "$0.runfiles/_main/tools/sh/bootstrap.sh" 2>/dev/null || source "${BASH_SOURCE[0]}.runfiles/_main/tools/sh/bootstrap.sh" 2>/dev/null || source "$(git rev-parse --show-toplevel 2>/dev/null)/tools/sh/bootstrap.sh"
dx_bootstrap "tools/sh/lib.sh"

dx_cd_workspace

dx_test_init

dx_bash_pin

pins="java/tests/fixtures/jvm_quality/pins.bzl"
pins_build="java/tests/fixtures/jvm_quality/BUILD.bazel"
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
  bad "jvm quality fixture missing (want $pins plus $pins_build)"
fi

# Pins record the eight qualified upstream versions.
if grep -q -F -e 'GOOGLE_JAVA_FORMAT_VERSION = "1.35.0"' "$pins" &&
  grep -q -F -e 'CHECKSTYLE_VERSION = "14.1.0"' "$pins" &&
  grep -q -F -e 'PMD_VERSION = "7.27.0"' "$pins" &&
  grep -q -F -e 'SPOTBUGS_VERSION = "4.10.4"' "$pins" &&
  grep -q -F -e 'KTFMT_VERSION = "0.63"' "$pins" &&
  grep -q -F -e 'KTLINT_VERSION = "1.8.0"' "$pins" &&
  grep -q -F -e 'DETEKT_VERSION = "1.23.8"' "$pins" &&
  grep -q -F -e 'ERROR_PRONE_VERSION = "2.50.0"' "$pins"; then
  ok
else
  bad "pins.bzl lost its eight qualified JVM versions under issue #485"
fi

# Pins record the artifact identities plus rejected head/alpha lines.
if grep -q -F -e 'GOOGLE_JAVA_FORMAT_ARTIFACT = "google-java-format-all-deps.jar"' "$pins" &&
  grep -q -F -e 'CHECKSTYLE_ARTIFACT = "checkstyle-all.jar"' "$pins" &&
  grep -q -F -e 'PMD_ARTIFACT = "pmd-dist-bin.zip"' "$pins" &&
  grep -q -F -e 'SPOTBUGS_ARTIFACT = "spotbugs-dist.zip"' "$pins" &&
  grep -q -F -e 'KTFMT_ARTIFACT = "ktfmt-with-dependencies.jar"' "$pins" &&
  grep -q -F -e 'KTLINT_ARTIFACT = "ktlint-executable.jar"' "$pins" &&
  grep -q -F -e 'DETEKT_ARTIFACT = "detekt-cli-all.jar"' "$pins" &&
  grep -q -F -e 'living at head rejected' "$pins" &&
  grep -q -F -e '2.0.0 alphas' "$pins"; then
  ok
else
  bad "pins.bzl lost its artifact identities plus rejected head/alpha lines under issue #485"
fi

# Pins record the sole-policy plus qualified rule-set resolutions plus rejections.
if grep -q -F -e 'NATIVE_CONFIG_POLICY = "native-configuration sole policy: no hidden presets"' "$pins" &&
  grep -q -F -e 'no auto-supplied Google checks' "$pins" &&
  grep -q -F -e 'default effort is the upstream built-in default effort' "$pins" &&
  grep -q -F -e 'default ruleset is the upstream built-in default ruleset' "$pins" &&
  grep -q -F -e 'default severities are the upstream default severities' "$pins" &&
  grep -q -F -e 'buildUponDefaultConfig full default set, not allRules' "$pins" &&
  grep -q -F -e 'standard rules are the upstream built-in standard set' "$pins" &&
  grep -q -F -e 'beyond-default switches rejected' "$pins" &&
  grep -q -F -e 'hidden presets rejected' "$pins"; then
  ok
else
  bad "pins.bzl lost its sole-policy plus rule-set resolutions plus rejections under issue #485"
fi

# Delivered native-config binding: Checkstyle carries the typed XML
# binding (config-required); the rest run upstream defaults.
if grep -q -F -e 'checkstyle_config' "$native"; then
  ok
else
  bad "native-config lost delivered checkstyle binding under #796"
fi

# Delivered adapter claims: six cohort tools in REAL_ADAPTERS;
# detekt plus Error Prone stay unclaimed.
jvm_missing=""
for tool in google_java_format checkstyle pmd spotbugs ktfmt ktlint; do
  grep -q -F -e "\"$tool\":" "$adapters" || jvm_missing="$jvm_missing $tool:missing"
done
if [[ -z "$jvm_missing" ]] &&
  ! grep -q -F -e '"detekt":' "$adapters" &&
  ! grep -q -F -e '"error_prone":' "$adapters"; then
  ok
else
  bad "JVM adapter claims drifted (missing:$jvm_missing)"
fi

# Delivered curated plus matrix: java/kotlin families plus cells.
jvm_matrix="quality/testdata/runner_matrix_jvm.bzl"
if grep -q -F -e '"java": {' "$curated" &&
  grep -q -F -e '"kotlin": {' "$curated" &&
  grep -q -F -e 'matrix_java_format_pass' "$jvm_matrix" &&
  grep -q -F -e 'matrix_kotlin_lint_pass' "$jvm_matrix"; then
  ok
else
  bad "curated or matrix lost delivered JVM families under #796"
fi

# Tool acquisition keeps the decided shared-JDK route with no Maven
# reconstruction and no false claim; versions qualified under, digests
# plus adapters stay pending under.
if grep -q -F -e 'Decided route: google-java-format, Checkstyle,' "$acquisition" &&
  grep -q -F -e 'sharing the one managed JDK cohort runtime' "$acquisition" &&
  grep -q -F -e 'with `java` plus `kotlin` claimed' "$acquisition" &&
  grep -q -F -e '(delivered under #796' "$acquisition"; then
  ok
else
  bad "tool-acquisition lost its delivered JVM route under #796"
fi

# Tool acquisition keeps the eight research rows with byte-identity risk.
jvm_research=""
for tool in '| google-java-format |' '| Checkstyle |' '| PMD |' '| SpotBugs |' '| ktfmt |' '| ktlint |' '| detekt |' '| Error Prone |'; do
  grep -q -F -e "$tool" "$acquisition" || jvm_research="$jvm_research $tool:missing"
done
if [[ -z "$jvm_research" ]] &&
  grep -q -F -e 'maintainer acquisition must establish and record byte identity' "$acquisition"; then
  ok
else
  bad "tool-acquisition lost a JVM research row or byte-identity honesty:$jvm_research"
fi

# Tool integrations keep the delivered JVM notes under #796.
if grep -q -F -e '**JVM cohort (#796' "$integrations" &&
  grep -q -F -e 'digests pinned in `MODULE.bazel`' "$integrations" &&
  grep -q -F -e 'adapters delivered under #796' "$integrations" &&
  grep -q -F -e 'Error Prone has no' "$integrations"; then
  ok
else
  bad "tool-integrations lost its delivered JVM notes under #796"
fi

# Native-configuration sole policy stands (no hidden presets authorized).
if grep -q -F -e 'defines no hidden rule' "$native_doc" &&
  grep -q -F -e 'sole behavioral policy' "$native_doc"; then
  ok
else
  bad "native-configuration lost its sole-policy plus no-hidden-preset honesty"
fi

# CI targets own the harness (split targets file, no behavior change).
ci_targets="tools/ci/ci_targets_d.bzl"
if grep -q -F -e 'name = "jvm_quality_qualification"' "$ci_targets"; then
  ok
else
  bad "tools/ci/ci_targets_d.bzl lost the jvm_quality_qualification target"
fi

# Live proof: JVM foundation fixtures stay green on the seed host
# (adapters delivered; hello stays green alongside).
if bazel test //java/tests/fixtures/hello:hello_test //kotlin/tests/fixtures/hello:hello_test --noshow_progress >/dev/null 2>&1; then
  ok
else
  bad "jvm quality live proof failed (want java plus kotlin hello green)"
fi

dx_test_summary "jvm quality qualification harness"
