"""ScalaTest 3.2.20 version pins.

Contract: `docs/product/support-matrix.md#provisional-default-test-runners`,
`docs/generation/README.md#language-mapping-qualification`.
"""

# Upstream ruleset pin (MODULE.bazel plus MODULE.bazel.lock).
RULES_SCALA_VERSION = "7.3.0"

# Toolchain pin (scala_config.settings in MODULE.bazel).
SCALA_VERSION = "2.13.18"

# Runner pin: ScalaTest 3.2.20 plus Scalactic 3.2.20 companion.
SCALATEST_VERSION = "3.2.20"
SCALACTIC_VERSION = "3.2.20"

# Maven-lock coordinates declared in MODULE.bazel plus maven_install.json
# (single version per the ScalaTest 3.2.20 release; covers 2.10-2.13 and 3.x
# per the release notes; the Coursier `scala_deps.scalatest()` runner
# classpath stays while the hello closure's explicit `@maven` deps are the
# fail-closed lock authority, issue #1080).
SCALATEST_ARTIFACTS = [
    "org.scalactic:scalactic_2.13:3.2.20",
    "org.scalatest:scalatest_2.13:3.2.20",
    "org.scalatest:scalatest-compatible:3.2.20",
    "org.scalatest:scalatest-core_2.13:3.2.20",
    "org.scalatest:scalatest-diagrams_2.13:3.2.20",
    "org.scalatest:scalatest-featurespec_2.13:3.2.20",
    "org.scalatest:scalatest-flatspec_2.13:3.2.20",
    "org.scalatest:scalatest-freespec_2.13:3.2.20",
    "org.scalatest:scalatest-funspec_2.13:3.2.20",
    "org.scalatest:scalatest-funsuite_2.13:3.2.20",
    "org.scalatest:scalatest-matchers-core_2.13:3.2.20",
    "org.scalatest:scalatest-mustmatchers_2.13:3.2.20",
    "org.scalatest:scalatest-propspec_2.13:3.2.20",
    "org.scalatest:scalatest-refspec_2.13:3.2.20",
    "org.scalatest:scalatest-shouldmatchers_2.13:3.2.20",
    "org.scalatest:scalatest-wordspec_2.13:3.2.20",
]

# Runner mapping: `scala_test` runs suites written using the `scalatest`
# library (rule implementation is ScalaTest-wired); test sources are the
# test's direct sources for QualitySourcesInfo, the library under test stays
# its ordinary owner via `deps`.
SCALATEST_KIND = "scala_test"
SCALATEST_RUNNER = "ScalaTest"
SCALATEST_SUITE = "org.scalatest.flatspec.AnyFlatSpec"

# Live proof target (wrapper consumer with explicit @maven deps over the lock).
SCALATEST_FIXTURE_LABEL = "//scala/tests/fixtures/hello:hello_test"
SCALATEST_FIXTURE_SUITE = "hello.HelloTest"
SCALATEST_MAVEN_LABEL = "@maven//:org_scalatest_scalatest_2_13"
SCALATEST_MAVEN_LOCK = "//third_party/jvm:maven_install.json"

# Rejected: unpinned runner (floating version, living at head, implicit).
SCALATEST_REJECTED = "unpinned runner rejected: no floating version or head"
