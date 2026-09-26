"""JUnit 6.1.3 plus 5.14.x fallback version pins.

Contract: `docs/product/support-matrix.md#provisional-default-test-runners`,
`docs/generation/README.md#language-mapping-qualification`.
"""

# Primary line: JUnit 6.1.3 (single BOM version for Platform/Jupiter/Vintage).
JUNIT6_VERSION = "6.1.3"

# Fallback line: Jupiter 5.14.4 plus Platform 1.14.4 (latest 5.14.x).
JUNIT5_JUPITER_VERSION = "5.14.4"
JUNIT5_PLATFORM_VERSION = "1.14.4"

# Seed line: JUnit 4.13.2 (latest 4.x; hamcrest-core 1.3 transitively).
JUNIT4_VERSION = "4.13.2"

# Shared leaves (identical on both lines per the resolved POMs).
OPENTEST4J_VERSION = "1.3.0"
APIGUARDIAN_VERSION = "1.1.2"

# JUnit 6-only leaf (JSpecify nullability, absent from the 5.14.x line).
JSPECIFY_VERSION = "1.0.0"

# Primary Maven coordinates pinned in MODULE.bazel plus maven_install.json.
JUNIT6_ARTIFACTS = [
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

# Fallback coordinates for JDK <17 (swap, never float; not in the lock).
JUNIT5_FALLBACK_ARTIFACTS = [
    "org.junit.jupiter:junit-jupiter-api:5.14.4",
    "org.junit.jupiter:junit-jupiter-engine:5.14.4",
    "org.junit.jupiter:junit-jupiter-params:5.14.4",
    "org.junit.platform:junit-platform-commons:1.14.4",
    "org.junit.platform:junit-platform-engine:1.14.4",
    "org.junit.platform:junit-platform-launcher:1.14.4",
    "org.junit.platform:junit-platform-console:1.14.4",
]

# Runner mapping: default Bazel runner over the JUnit 4 seed today;
JUNIT_RUNNER_MAIN_CLASS = "org.junit.platform.console.ConsoleLauncher"
JUNIT_RUNNER_USE_TESTRUNNER = False

# JDK floor mapping: 17+ selects 6.1.3, below selects 5.14.x fallback.
JUNIT_JDK_FLOOR = "17"

# Rejected: unpinned runner (floating version, living at head, implicit).
JUNIT_REJECTED = "unpinned runner rejected: no floating version or head"

JUNIT_FIXTURE_JAVA_CLASS = "hello.HelloJupiterTest"
JUNIT_FIXTURE_KOTLIN_CLASS = "hello.HelloJupiterTest"
