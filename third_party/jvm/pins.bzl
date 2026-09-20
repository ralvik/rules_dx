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
# never a second lock).
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

# Rejected: non-fail-closed lock wiring.
MAVEN_REJECTED = "non-fail-closed rejected: no lock without fail_if_repin_required, no floating coordinates, no hand-edited lock"
