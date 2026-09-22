#!/usr/bin/env bash
# ScalaTest 3.2.20 qualification harness.
#
# Qualifies the owned gap from closed: provisional ScalaTest 3.2.20,
# closed owner only. covers adapters not the runner.
# - pinned: rules_scala 7.3.0 plus Scala 2.13.18 plus ScalaTest 3.2.20
#   (Scalactic 3.2.20 companion) in `scala/tests/fixtures/scalatest/pins.bzl`;
#   MODULE.bazel pins the ruleset plus toolchain plus `scala_deps.scalatest()`
#   runner classpath, with the hello closure's ScalaTest deps declared via
#   the shared Maven lock `//third_party/jvm:maven_install.json` (fail-closed,
#   issue #1080; no separate ecosystem lock).
# - runner: `scala_test` runs suites written using the `scalatest` library
#   (rule implementation is ScalaTest-wired); test sources are the test's
#   direct sources for QualitySourcesInfo, the library stays its ordinary
#   owner via `deps`. The `scala_test` wrapper in `scala/rules/defs.bzl`
#   preserves the upstream providers plus QualitySourcesInfo. Unpinned runner
#   (floating version, living at head, implicit) stays rejected.
#   `scala_junit_test` plus `scala_specs2_junit_test` stay rules-supported
#   choices, not defaults. The live proof is
#   `//scala/tests/fixtures/hello:hello_test` (`AnyFlatSpec`).
# - open owned gaps: platform plus consumer plus release evidence, no
#   `Supported` claim. Compatibility is test mapping only.
#
# Versioned here, run by CI via `bazel run //tools/ci:scalatest_qualification`,
# following //tools/ci:gotest_qualification.
set -euo pipefail

# Shared workspace + runfiles helpers.
# Bootstrap via tools/sh/bootstrap.sh (issue #654): runfiles forest first, then source tree.
source "${RUNFILES_DIR:-/dev/null}/_main/tools/sh/bootstrap.sh" 2>/dev/null || source "${TEST_SRCDIR:-/dev/null}/_main/tools/sh/bootstrap.sh" 2>/dev/null || source "$0.runfiles/_main/tools/sh/bootstrap.sh" 2>/dev/null || source "${BASH_SOURCE[0]}.runfiles/_main/tools/sh/bootstrap.sh" 2>/dev/null || source "$(git rev-parse --show-toplevel 2>/dev/null)/tools/sh/bootstrap.sh"
dx_bootstrap "tools/sh/lib.sh"

dx_cd_workspace

dx_test_init

dx_bash_pin

pins="scala/tests/fixtures/scalatest/pins.bzl"
scalatest_build="scala/tests/fixtures/scalatest/BUILD.bazel"
hello_build="scala/tests/fixtures/hello/BUILD.bazel"
hello_test="scala/tests/fixtures/hello/HelloTest.scala"
wrapper="scala/rules/defs.bzl"
module="MODULE.bazel"
jvm_pins="third_party/jvm/pins.bzl"
jvm_lock="third_party/jvm/maven_install.json"
matrix="docs/product/support-matrix.md"
gen_readme="docs/generation/foundation-qualification.md"
build="tools/ci/BUILD.bazel"
ci=".github/workflows/ci.yml"

# Fixture pins plus hello consumer stay present.
if [[ -f "$pins" && -f "$scalatest_build" && -f "$hello_build" && -f "$hello_test" ]]; then
  ok
else
  bad "scalatest fixture missing (want $pins plus $scalatest_build plus $hello_build plus $hello_test)"
fi

# Pins record the ruleset plus toolchain plus runner identities.
if grep -q -F -e 'RULES_SCALA_VERSION = "7.3.0"' "$pins" &&
  grep -q -F -e 'SCALA_VERSION = "2.13.18"' "$pins" &&
  grep -q -F -e 'SCALATEST_VERSION = "3.2.20"' "$pins" &&
  grep -q -F -e 'SCALACTIC_VERSION = "3.2.20"' "$pins"; then
  ok
else
  bad "pins.bzl lost its rules_scala plus Scala plus ScalaTest 3.2.20 identities under issue #480"
fi

# Pins record the managed-route coordinates plus runner mapping.
if grep -q -F -e 'org.scalatest:scalatest_2.13:3.2.20' "$pins" &&
  grep -q -F -e 'org.scalactic:scalactic_2.13:3.2.20' "$pins" &&
  grep -q -F -e 'SCALATEST_KIND = "scala_test"' "$pins" &&
  grep -q -F -e 'SCALATEST_RUNNER = "ScalaTest"' "$pins" &&
  grep -q -F -e 'AnyFlatSpec' "$pins"; then
  ok
else
  bad "pins.bzl lost its 3.2.20 coordinates plus scala_test mapping under issue #480"
fi

# Pins record the Maven lock authority for the hello closure (issue #1080).
if grep -q -F -e 'SCALATEST_MAVEN_LABEL = "@maven//:org_scalatest_scalatest_2_13"' "$pins" &&
  grep -q -F -e 'SCALATEST_MAVEN_LOCK = "//third_party/jvm:maven_install.json"' "$pins"; then
  ok
else
  bad "pins.bzl lost its Maven lock authority wiring under issue #1080"
fi

# Pins record the live fixture label plus rejected unpinned runner.
if grep -q -F -e '//scala/tests/fixtures/hello:hello_test' "$pins" &&
  grep -q -F -e 'unpinned runner rejected' "$pins"; then
  ok
else
  bad "pins.bzl lost its hello fixture label plus rejected unpinned runner under issue #480"
fi

# MODULE pins the ruleset plus toolchain plus managed scalatest route.
if grep -q -F -e 'bazel_dep(name = "rules_scala", version = "7.3.0")' "$module" &&
  grep -q -F -e 'scala_version = "2.13.18"' "$module" &&
  grep -q -F -e 'scala_deps.scalatest()' "$module"; then
  ok
else
  bad "MODULE.bazel lost its rules_scala 7.3.0 plus Scala 2.13.18 plus scalatest route under issue #480"
fi

# MODULE declares ScalaTest via the shared Maven lock (issue #1080).
if grep -q -F -e 'org.scalatest:scalatest_2.13:3.2.20' "$module" &&
  grep -q -F -e 'lock_file = "//third_party/jvm:maven_install.json"' "$module" &&
  grep -q -F -e 'fail_if_repin_required = True' "$module"; then
  ok
else
  bad "MODULE.bazel lost its ScalaTest Maven lock declaration under issue #1080"
fi

# Shared lock records ScalaTest plus hello proves the fail-closed consumer.
if grep -q -F -e '"org.scalatest:scalatest_2.13"' "$jvm_lock" &&
  grep -q -F -e '"version": "3.2.20"' "$jvm_lock" &&
  grep -q -F -e 'org.scalatest:scalatest_2.13:3.2.20' "$jvm_pins" &&
  grep -q -F -e '@maven//:org_scalatest_scalatest_2_13' "$hello_build"; then
  ok
else
  bad "shared Maven lock lost its ScalaTest authority plus hello consumer under issue #1080"
fi

# Wrapper preserves upstream providers plus QualitySourcesInfo with a scala_test def.
if grep -q -F -e 'JavaInfo' "$wrapper" &&
  grep -q -F -e 'QualitySourcesInfo' "$wrapper" &&
  grep -q -F -e 'def scala_test' "$wrapper" &&
  grep -q -F -e 'rules_scala 7.3.0' "$wrapper" &&
  grep -q -F -e 'ScalaTest 3.2.20' "$wrapper"; then
  ok
else
  bad "scala/rules/defs.bzl lost its scala_test provider mapping under issue #480"
fi

# Wrapper keeps the forwarder shape (private upstream plus test forwarder).
if grep -q -F -e '_scala_forward_test' "$wrapper" &&
  grep -q -F -e '_upstream' "$wrapper" &&
  grep -q -F -e 'testonly = True' "$wrapper"; then
  ok
else
  bad "scala/rules/defs.bzl lost its forwarder shape (want _scala_forward_test plus private upstream) under issue #480"
fi

# Hello fixture keeps the wrapper consumer shape (scala_test plus deps).
if grep -q -F -e 'scala/rules:defs.bzl' "$hello_build" &&
  grep -q -F -e 'scala_test' "$hello_build" &&
  grep -q -F -e 'HelloTest.scala' "$hello_build" &&
  grep -q -F -e ':hello_lib' "$hello_build" &&
  grep -q -F -e '@maven//:org_scalatest_scalatest_2_13' "$hello_build"; then
  ok
else
  bad "scala/tests/fixtures/hello lost its wrapper scala_test consumer shape under issue #480"
fi

# Hello test source stays a ScalaTest AnyFlatSpec suite.
if grep -q -F -e 'org.scalatest.flatspec.AnyFlatSpec' "$hello_test" &&
  grep -q -F -e 'class HelloTest' "$hello_test" &&
  grep -q -F -e 'Hello.hello' "$hello_test"; then
  ok
else
  bad "scala/tests/fixtures/hello/HelloTest.scala lost its AnyFlatSpec shape under issue #480"
fi

# Implicit runner stays rejected: fixtures never load upstream scala_test directly
# (hermetic tree search: BSD grep lacks --include, issue #1006).
if dx_tree_absent --include='BUILD.bazel' '@rules_scala//scala:scala.bzl' -- scala/tests/fixtures; then
  ok
else
  bad "implicit Scala runner detected (want wrapper scala_test only, no direct @rules_scala load in fixtures)"
fi

# Generation README pins the qualified runner alongside the other gaps.
if grep -q -F -e 'qualified seed-only under issue #480' "$gen_readme" &&
  grep -q -F -e 'scalatest_qualification' "$gen_readme" &&
  grep -q -F -e 'scala/tests/fixtures/scalatest/pins.bzl' "$gen_readme" &&
  grep -q -F -e 'issue #480' "$gen_readme"; then
  ok
else
  bad "docs/generation/foundation-qualification.md lost its qualified ScalaTest record under issue #480"
fi

# BUILD owns the harness target plus CI wires it in dogfood-freshness.
if grep -q -F -e 'name = "scalatest_qualification"' "$build" &&
  grep -q -F -e 'bazel run --noshow_progress //tools/ci:scalatest_qualification' "$ci"; then
  ok
else
  bad "tools/ci/BUILD.bazel or ci.yml lost the scalatest_qualification wiring (want target plus dogfood-freshness)"
fi

# Live proof: the wrapper scala_test executes green on the seed host.
if bazel test //scala/tests/fixtures/hello:hello_test --noshow_progress >/dev/null 2>&1; then
  ok
else
  bad "scalatest live proof failed (want //scala/tests/fixtures/hello:hello_test green)"
fi

dx_test_summary "scalatest qualification harness"
