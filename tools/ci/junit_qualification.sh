#!/usr/bin/env bash
# JUnit 6.1.3 plus 5.14.x fallback qualification harness (issue #476).
#
# Qualifies the owned gap from closed #304: provisional JUnit 6.1.3 with
# 5.14.x fallback, closed #304 owner only. #416 covers adapters not runners.
# - pinned: primary JUnit 6.1.3 (Platform 6.1.3 plus Jupiter 6.1.3 plus
#   Vintage 6.1.3, single BOM version; JDK 17+ baseline, remotejdk_21 on the
#   seed host) plus fallback Jupiter 5.14.4 plus Platform 1.14.4 (latest
#   5.14.x, JDK 8 baseline for hosts below the JUnit 6 floor) in
#   `java/tests/fixtures/junit/pins.bzl`; MODULE.bazel plus
#   `maven_install.json` pin the 6.1.3 line with fail-closed repin; the
#   fallback is selected by swapping the same coordinates, never by floating.
# - runner: default Bazel runner stays the JUnit 4.13.2 seed; the qualified
#   Jupiter upgrade is `use_testrunner = False` plus ConsoleLauncher
#   `execute --select-class` with pinned Jupiter/Platform deps (Java plus
#   Kotlin fixtures); Vintage runs JUnit 4 on the Platform (deprecated,
#   temporary migration only). Unpinned runner (floating, head, implicit)
#   stays rejected. Kotlin `suspend` stays documented capability.
# - fixtures: `java/tests/fixtures/junit/` (pins plus HelloJupiterTest plus
#   console-launcher `hello_jupiter_test`) plus
#   `kotlin/tests/fixtures/junit/` (HelloJupiterTest plus console-launcher
#   `hello_jupiter_test`); JUnit 4 seeds stay in `hello/` fixtures.
# - open owned gaps: platform plus consumer plus release evidence, no
#   `Supported` claim. Compatibility is test mapping only.
#
# Versioned here, run by CI via `bazel run //tools/ci:junit_qualification`,
# following //tools/ci:exact_target_qualification.
set -euo pipefail

# Shared workspace + runfiles helpers (issue #319).
# Bootstrap: Bazel runfiles forest first (`data = ["//tools/sh:lib"]`), then source tree.
source "${RUNFILES_DIR:-/dev/null}/_main/tools/sh/lib.sh" 2>/dev/null || source "${TEST_SRCDIR:-/dev/null}/_main/tools/sh/lib.sh" 2>/dev/null || source "$0.runfiles/_main/tools/sh/lib.sh" 2>/dev/null || source "${BASH_SOURCE[0]}.runfiles/_main/tools/sh/lib.sh" 2>/dev/null || source "$(dirname "${BASH_SOURCE[0]}")/../sh/lib.sh"

dx_cd_workspace

dx_test_init

pins="java/tests/fixtures/junit/pins.bzl"
java_build="java/tests/fixtures/junit/BUILD.bazel"
java_test="java/tests/fixtures/junit/HelloJupiterTest.java"
kotlin_build="kotlin/tests/fixtures/junit/BUILD.bazel"
kotlin_test="kotlin/tests/fixtures/junit/HelloJupiterTest.kt"
module="MODULE.bazel"
lock="third_party/jvm/maven_install.json"
matrix="docs/product/support-matrix.md"
gen_readme="docs/generation/README.md"
build="tools/ci/BUILD.bazel"
ci=".github/workflows/ci.yml"
verify="docs/testing/verification-matrix.md"

# Fixture pair plus pins stay present (issue #476).
if [[ -f "$pins" && -f "$java_build" && -f "$java_test" && -f "$kotlin_build" && -f "$kotlin_test" ]]; then
  ok
else
  bad "junit fixture missing (want $pins plus $java_build plus $java_test plus $kotlin_build plus $kotlin_test)"
fi

# Pins record the primary plus fallback plus seed identities.
if grep -q -F -e 'JUNIT6_VERSION = "6.1.3"' "$pins" &&
  grep -q -F -e 'JUNIT5_JUPITER_VERSION = "5.14.4"' "$pins" &&
  grep -q -F -e 'JUNIT5_PLATFORM_VERSION = "1.14.4"' "$pins" &&
  grep -q -F -e 'JUNIT4_VERSION = "4.13.2"' "$pins"; then
  ok
else
  bad "pins.bzl lost its JUnit 6.1.3 plus 5.14.x fallback plus 4.13.2 seed identities under issue #476"
fi

# Pins record the shared plus 6-only leaves plus rejected runner.
if grep -q -F -e 'OPENTEST4J_VERSION = "1.3.0"' "$pins" &&
  grep -q -F -e 'APIGUARDIAN_VERSION = "1.1.2"' "$pins" &&
  grep -q -F -e 'JSPECIFY_VERSION = "1.0.0"' "$pins" &&
  grep -q -F -e 'unpinned runner rejected' "$pins"; then
  ok
else
  bad "pins.bzl lost its shared leaves plus rejected unpinned runner under issue #476"
fi

# Pins record the artifact lists plus runner plus JDK floor.
if grep -q -F -e 'org.junit.jupiter:junit-jupiter-api:6.1.3' "$pins" &&
  grep -q -F -e 'org.junit.platform:junit-platform-console:6.1.3' "$pins" &&
  grep -q -F -e 'org.junit.vintage:junit-vintage-engine:6.1.3' "$pins" &&
  grep -q -F -e 'org.junit.jupiter:junit-jupiter-api:5.14.4' "$pins" &&
  grep -q -F -e 'org.junit.platform:junit-platform-console:1.14.4' "$pins" &&
  grep -q -F -e 'org.junit.platform.console.ConsoleLauncher' "$pins" &&
  grep -q -F -e 'JUNIT_JDK_FLOOR = "17"' "$pins"; then
  ok
else
  bad "pins.bzl lost its 6.1.3 plus fallback artifact lists plus ConsoleLauncher plus JDK floor under issue #476"
fi

# MODULE pins the 6.1.3 primary line with fail-closed repin.
if grep -q -F -e 'org.junit.jupiter:junit-jupiter-api:6.1.3' "$module" &&
  grep -q -F -e 'org.junit.jupiter:junit-jupiter-engine:6.1.3' "$module" &&
  grep -q -F -e 'org.junit.jupiter:junit-jupiter-params:6.1.3' "$module" &&
  grep -q -F -e 'org.junit.platform:junit-platform-console:6.1.3' "$module" &&
  grep -q -F -e 'org.junit.vintage:junit-vintage-engine:6.1.3' "$module" &&
  grep -q -F -e 'junit:junit:4.13.2' "$module" &&
  grep -q -F -e 'fail_if_repin_required' "$module"; then
  ok
else
  bad "MODULE.bazel lost its JUnit 6.1.3 primary pins plus fail-closed under issue #476"
fi

# Lock records the resolved 6.1.3 versions plus leaves.
if grep -q -F -e '"org.junit.jupiter:junit-jupiter-api"' "$lock" &&
  grep -q -F -e '"version": "6.1.3"' "$lock" &&
  grep -q -F -e '"org.junit.vintage:junit-vintage-engine"' "$lock" &&
  grep -q -F -e '"org.jspecify:jspecify"' "$lock" &&
  grep -q -F -e '"org.opentest4j:opentest4j"' "$lock"; then
  ok
else
  bad "maven_install.json lost its resolved JUnit 6.1.3 plus leaves under issue #476"
fi

# Java fixture keeps the console-launcher runner shape.
if grep -q -F -e 'use_testrunner = False' "$java_build" &&
  grep -q -F -e 'org.junit.platform.console.ConsoleLauncher' "$java_build" &&
  grep -q -F -e '"execute"' "$java_build" &&
  grep -q -F -e '"--select-class"' "$java_build" &&
  grep -q -F -e 'hello.HelloJupiterTest' "$java_build" &&
  grep -q -F -e 'org_junit_jupiter_junit_jupiter_api' "$java_build" &&
  grep -q -F -e 'org_junit_platform_junit_platform_console' "$java_build" &&
  grep -q -F -e 'org.junit.jupiter.api.Test' "$java_test"; then
  ok
else
  bad "java/tests/fixtures/junit lost its ConsoleLauncher execute --select-class runner shape under issue #476"
fi

# Kotlin fixture keeps the console-launcher runner shape with suspend noted.
if grep -q -F -e 'org.junit.platform.console.ConsoleLauncher' "$kotlin_build" &&
  grep -q -F -e '"execute"' "$kotlin_build" &&
  grep -q -F -e '"--select-class"' "$kotlin_build" &&
  grep -q -F -e 'hello.HelloJupiterTest' "$kotlin_build" &&
  grep -q -F -e 'org_junit_jupiter_junit_jupiter_api' "$kotlin_build" &&
  grep -q -F -e 'org_junit_platform_junit_platform_console' "$kotlin_build" &&
  grep -q -F -e 'org.junit.jupiter.api.Test' "$kotlin_test"; then
  ok
else
  bad "kotlin/tests/fixtures/junit lost its ConsoleLauncher execute --select-class runner shape under issue #476"
fi

# JUnit 4 seed stays green via the default runner (hello fixtures).
if grep -q -F -e 'test_class' java/tests/fixtures/hello/BUILD.bazel &&
  grep -q -F -e 'org.junit.Test' java/tests/fixtures/hello/HelloTest.java &&
  grep -q -F -e 'test_class' kotlin/tests/fixtures/hello/BUILD.bazel &&
  grep -q -F -e 'org.junit.Test' kotlin/tests/fixtures/hello/HelloTest.kt &&
  grep -q -F -e 'junit:junit:4.13.2' "$module"; then
  ok
else
  bad "JUnit 4 seed lost its default-runner hello mapping under issue #476"
fi

# Unpinned runner stays rejected: no floating version or head dep lands.
if ! grep -q -F -e 'junit:junit:4.+' "$module" &&
  ! grep -R --include='*.bzl' --include='BUILD.bazel' --exclude='junit_qualification.sh' --exclude='pins.bzl' -F -e 'living at head' -- java kotlin third_party 2>/dev/null | grep -q .; then
  ok
else
  bad "unpinned JUnit runner detected (floating version or head; want pins.bzl pins only)"
fi

# Support matrix keeps the qualified JUnit gap wording.
if grep -q -F -e 'JUnit 6.1.3 plus 5.14.x fallback qualified seed-only under issue #476' "$matrix" &&
  grep -q -F -e 'junit_qualification' "$matrix" &&
  grep -q -F -e 'JUnit 6.1.3 primary' "$matrix" &&
  grep -q -F -e '#476' "$matrix"; then
  ok
else
  bad "docs/product/support-matrix.md lost its qualified JUnit gap wording under issue #476"
fi

# Generation README pins the qualified runner alongside the other gaps.
if grep -q -F -e 'qualified JUnit 6.1.3 Jupiter' "$gen_readme" &&
  grep -q -F -e 'junit_qualification' "$gen_readme" &&
  grep -q -F -e 'issue #476' "$gen_readme"; then
  ok
else
  bad "docs/generation/README.md lost its qualified JUnit record under issue #476"
fi

# Verification matrix owns the qualified seed-only record under #476.
if grep -q -F -e 'junit_qualification' "$verify" &&
  grep -q -F -e 'qualified seed-only under #476' "$verify" &&
  grep -q -F -e 'bazel run //tools/ci:junit_qualification' "$verify" &&
  grep -q -F -e '`junit_qualification` 16/16' "$verify"; then
  ok
else
  bad "verification-matrix lost its #476 JUnit qualified record"
fi

# BUILD owns the harness target.
if grep -q -F -e 'name = "junit_qualification"' "$build"; then
  ok
else
  bad "tools/ci/BUILD.bazel lost the junit_qualification target"
fi

# CI wires the harness in dogfood-freshness.
if grep -q -F -e 'bazel run --noshow_progress //tools/ci:junit_qualification' "$ci"; then
  ok
else
  bad "ci.yml lost the junit_qualification step (want dogfood-freshness)"
fi

# Live proof: Jupiter plus seed mappings execute green on the seed host.
if bazel test //java/tests/fixtures/junit:hello_jupiter_test //kotlin/tests/fixtures/junit:hello_jupiter_test //java/tests/fixtures/hello:hello_test //kotlin/tests/fixtures/hello:hello_test --noshow_progress >/dev/null 2>&1; then
  ok
else
  bad "junit live proof failed (want Jupiter plus JUnit 4 seed green)"
fi

dx_test_summary "junit qualification harness"
