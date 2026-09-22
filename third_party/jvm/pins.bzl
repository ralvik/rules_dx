"""Maven lock wiring pins.

Contract: `docs/product/support-matrix.md#provisional-default-dependency-locks`,
`docs/generation/README.md#language-mapping-qualification`.
"""

# Upstream resolver pin (MODULE.bazel plus MODULE.bazel.lock).
RULES_JVM_EXTERNAL_VERSION = "7.1"

# Lock authority: committed file plus fail-closed repin flag.
MAVEN_LOCK_FILE = "//third_party/jvm:maven_install.json"
MAVEN_FAIL_IF_REPIN_REQUIRED = True

# Repin entry point (lock is generated, never edited by hand).
MAVEN_REPIN = "REPIN=1 bazel run @maven//:pin"

# Primary Maven coordinates pinned in MODULE.bazel plus maven_install.json
# (JUnit 6.1.3 line plus 4.13.2 seed; the 5.14.x fallback stays a swap,
# never a second lock; ScalaTest 3.2.20 plus Scalactic 3.2.20 for the Scala
# hello closure via the shared lock, issue #1080).
MAVEN_ARTIFACTS = [
    "junit:junit:4.13.2",
    "org.junit.jupiter:junit-jupiter-api:6.1.3",
    "org.junit.jupiter:junit-jupiter-engine:6.1.3",
    "org.junit.jupiter:junit-jupiter-params:6.1.3",
    "org.junit.platform:junit-platform-commons:6.1.3",
    "org.junit.platform:junit-platform-engine:6.1.3",
    "org.junit.platform:junit-platform-launcher:6.1.3",
    "org.junit.platform:junit-platform-console:6.1.3",
    "org.junit.vintage:junit-vintage-engine:6.1.3",
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

# Repository order: GCS Maven Central mirror first (this host's egress proxy
# serves it while repo1.maven.org answers Cloudflare challenges); the pinned
# lock hashes make the order irrelevant to artifact identity.
MAVEN_REPOSITORIES = [
    "https://maven-central.storage-download.googleapis.com/maven2",
    "https://repo1.maven.org/maven2",
]

# Live proof consumers (fail-closed fixtures over the shared @maven hub).
MAVEN_FIXTURE_JAVA_JUPITER = "//java/tests/fixtures/junit:hello_jupiter_test"
MAVEN_FIXTURE_KOTLIN_JUPITER = "//kotlin/tests/fixtures/junit:hello_jupiter_test"
MAVEN_FIXTURE_JAVA_SEED = "//java/tests/fixtures/hello:hello_test"
MAVEN_FIXTURE_KOTLIN_SEED = "//kotlin/tests/fixtures/hello:hello_test"
MAVEN_FIXTURE_SCALA_SEED = "//scala/tests/fixtures/hello:hello_test"

# Rejected: non-fail-closed lock wiring.
MAVEN_REJECTED = "non-fail-closed rejected: no lock without fail_if_repin_required, no floating coordinates, no hand-edited lock"

# Currency recheck (issue #932): coordinates below were verified current on
# this date (JUnit 4.13.2 plus 6.1.3 line plus ScalaTest 3.2.20 per ADR 0008 latest-stable).
# Refresh the date with each dependency-currency pass; pin_consistency.sh
# fails when absent or stale.
MAVEN_CURRENCY_RECHECK = "2026-09-22"
