#!/usr/bin/env bash
# JVM quality defaults qualification harness (issue #485).
#
# Qualifies the provisional JVM format plus lint defaults against the
# native-configuration contract with no hidden presets. Issue #416 covers
# adapters (plus digests), not versions: this harness owns versions plus
# rule-sets.
# - pinned: google-java-format 1.35.0, Checkstyle 14.1.0, PMD 7.27.0,
#   SpotBugs 4.10.4, ktfmt 0.63, ktlint 1.8.0, detekt 1.23.8, Error Prone
#   2.50.0 in `java/tests/fixtures/jvm_quality/pins.bzl` (v1.36.x plus v0.64
#   head plus 2.0.0 alphas rejected; Error Prone follows the qualified JDK
#   baseline); complete-artifact plus shared-JDK identities recorded, digests
#   stay owned under issue #416.
# - rule-sets: native-configuration sole policy, no hidden presets. Without
#   an applicable checked-in native config the pinned tool uses upstream
#   built-in defaults; with a config it interprets natively; adapters add
#   only transport/hermetic settings. Checkstyle Google checks never
#   auto-supplied; SpotBugs default effort, PMD default ruleset, Error Prone
#   default severities, detekt buildUponDefaultConfig (not allRules), ktlint
#   standard are upstream built-in defaults. Beyond-default switches
#   (detekt allRules, experimental Error Prone, --enable=all maxima) rejected.
# - fixtures: `java/tests/fixtures/jvm_quality/` pins plus BUILD; no JVM
#   native-config preset, no adapter claim, no curated defaults, no matrix
#   cells; java/kotlin hello fixtures stay green.
# - open owned gaps: digests plus adapters under #416, platform plus consumer
#   plus release evidence, no `Supported` claim. Compatibility is defaults only.
#
# Versioned here, run by CI via `bazel run //tools/ci:jvm_quality_qualification`,
# following //tools/ci:cc_hermetic_qualification.
set -euo pipefail

# Shared workspace + runfiles helpers (issue #319).
# Bootstrap: Bazel runfiles forest first (`data = ["//tools/sh:lib"]`), then source tree.
source "${RUNFILES_DIR:-/dev/null}/_main/tools/sh/lib.sh" 2>/dev/null || source "${TEST_SRCDIR:-/dev/null}/_main/tools/sh/lib.sh" 2>/dev/null || source "$0.runfiles/_main/tools/sh/lib.sh" 2>/dev/null || source "${BASH_SOURCE[0]}.runfiles/_main/tools/sh/lib.sh" 2>/dev/null || source "$(dirname "${BASH_SOURCE[0]}")/../sh/lib.sh"

dx_cd_workspace

dx_test_init

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
verify="docs/testing/verification-matrix.md"

# Fixture pair plus pins stay present (issue #485).
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

# No hidden JVM native-config preset: no JVM binding exists in the typed
# native-config rules (adapters run pinned upstream defaults until checked-in
# policy qualifies against the native-config contract).
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

# No false adapter claim for the JVM cohort: none of the cohort tool IDs
# appear in REAL_ADAPTERS. Classification exists; adapter claim does not
# (owned under issue #416).
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

# Classification-only today: java/kotlin families carry no curated defaults,
# no runner-matrix cells (claims land only with green adapter evidence
# under issue #416).
jvm_curated=""
for family in '"java": {' '"kotlin": {'; do
  if grep -q -F -e "$family" "$curated"; then
    jvm_curated="$jvm_curated $family:claimed"
  fi
done
if [[ -z "$jvm_curated" ]] &&
  ! grep -q -F -e 'matrix_java_' "$matrix" &&
  ! grep -q -F -e 'matrix_kotlin_' "$matrix"; then
  ok
else
  bad "curated or matrix claims a JVM family before adapters land:$jvm_curated"
fi

# Support matrix keeps the qualified JVM versions plus rule-sets with
# fixtures and harness (issue #485); digests plus adapters stay under #416.
if grep -q -F -e 'qualified seed-only under issue #485' "$support" &&
  grep -q -F -e 'jvm_quality_qualification' "$support" &&
  grep -q -F -e 'java/tests/fixtures/jvm_quality/pins.bzl' "$support" &&
  grep -q -F -e 'no hidden preset' "$support" &&
  grep -q -F -e 'digests plus adapter mappings stay owned under issue #416' "$support"; then
  ok
else
  bad "support-matrix lost its #485 qualified JVM versions plus rule-sets record with fixtures"
fi

# Support matrix resolves the Checkstyle/Google plus default-effort/ruleset
# conflicts as upstream built-in defaults (no auto-supplied preset).
if grep -q -F -e 'no auto-supplied Google checks' "$support" &&
  grep -q -F -e 'upstream built-in defaults' "$support" &&
  grep -q -F -e 'qualified seed-only under issue #485' "$support"; then
  ok
else
  bad "support-matrix lost its #485 Checkstyle plus defaults conflict resolution"
fi

# Tool acquisition keeps the decided shared-JDK route with no Maven
# reconstruction and no false claim; versions qualified under #485, digests
# plus adapters stay pending under #416.
if grep -q -F -e 'Decided route: google-java-format, Checkstyle,' "$acquisition" &&
  grep -q -F -e 'sharing the one managed JDK cohort runtime' "$acquisition" &&
  grep -q -F -e 'no adapter claims `java` or `kotlin` yet' "$acquisition" &&
  grep -q -F -e 'qualified seed-only under issue #485' "$acquisition" &&
  grep -q -F -e 'digests plus adapter mappings stay owned under issue #416' "$acquisition"; then
  ok
else
  bad "tool-acquisition lost its decided JVM route with #485 versions plus #416 digests/adapters split"
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

# Tool integrations keep the JVM adapter-input notes with Error Prone open
# work plus #485 pinned versions (adapters still open under #416).
if grep -q -F -e 'JVM cohort' "$integrations" &&
  grep -q -F -e 'no adapter claims `java` or `kotlin` yet' "$integrations" &&
  grep -q -F -e 'Error Prone has no' "$integrations" &&
  grep -q -F -e 'qualified seed-only under issue #485' "$integrations" &&
  grep -q -F -e 'adapters stay owned under issue #416' "$integrations"; then
  ok
else
  bad "tool-integrations lost its JVM notes with #485 versions plus #416 adapters split"
fi

# Native-configuration sole policy stands (no hidden presets authorized).
if grep -q -F -e 'defines no hidden rule' "$native_doc" &&
  grep -q -F -e 'sole behavioral policy' "$native_doc"; then
  ok
else
  bad "native-configuration lost its sole-policy plus no-hidden-preset honesty"
fi

# Verification matrix owns the qualified seed-only record under #485.
if grep -q -F -e 'jvm_quality_qualification' "$verify" &&
  grep -q -F -e 'qualified seed-only under #485' "$verify" &&
  grep -q -F -e 'bazel run //tools/ci:jvm_quality_qualification' "$verify" &&
  grep -q -F -e '`jvm_quality_qualification` 16/16' "$verify"; then
  ok
else
  bad "verification-matrix lost its #485 JVM quality qualified record"
fi

# BUILD owns the harness target plus CI wires it in dogfood-freshness.
if grep -q -F -e 'name = "jvm_quality_qualification"' "$build" &&
  grep -q -F -e 'bazel run --noshow_progress //tools/ci:jvm_quality_qualification' "$ci"; then
  ok
else
  bad "tools/ci/BUILD.bazel or ci.yml lost the jvm_quality_qualification wiring (want target plus dogfood-freshness)"
fi

# Live proof: JVM foundation fixtures stay green on the seed host
# (defaults change only; no adapter behavior yet).
if bazel test //java/tests/fixtures/hello:hello_test //kotlin/tests/fixtures/hello:hello_test --noshow_progress >/dev/null 2>&1; then
  ok
else
  bad "jvm quality live proof failed (want java plus kotlin hello green)"
fi

dx_test_summary "jvm quality qualification harness"
