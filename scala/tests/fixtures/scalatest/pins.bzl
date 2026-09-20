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

# Managed-route Maven coordinates resolved by rules_scala Coursier for 2.13
# (single version per the ScalaTest 3.2.20 release; covers 2.10-2.13 and 3.x
# per the release notes).
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

# Live proof target (wrapper consumer over the managed route).
SCALATEST_FIXTURE_LABEL = "//scala/tests/fixtures/hello:hello_test"
SCALATEST_FIXTURE_SUITE = "hello.HelloTest"

# Rejected: unpinned runner (floating version, living at head, implicit).
SCALATEST_REJECTED = "unpinned runner rejected: no floating version or head"
