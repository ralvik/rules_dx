"""JUnit 6.1.3 plus 5.14.x fallback version pins."""

JUNIT6_VERSION = "6.1.3"

JUNIT5_JUPITER_VERSION = "5.14.4"
JUNIT5_PLATFORM_VERSION = "1.14.4"

JUNIT4_VERSION = "4.13.2"

OPENTEST4J_VERSION = "1.3.0"
APIGUARDIAN_VERSION = "1.1.2"

JSPECIFY_VERSION = "1.0.0"

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

JUNIT5_FALLBACK_ARTIFACTS = [
    "org.junit.jupiter:junit-jupiter-api:5.14.4",
    "org.junit.jupiter:junit-jupiter-engine:5.14.4",
    "org.junit.jupiter:junit-jupiter-params:5.14.4",
    "org.junit.platform:junit-platform-commons:1.14.4",
    "org.junit.platform:junit-platform-engine:1.14.4",
    "org.junit.platform:junit-platform-launcher:1.14.4",
    "org.junit.platform:junit-platform-console:1.14.4",
]

JUNIT_RUNNER_MAIN_CLASS = "org.junit.platform.console.ConsoleLauncher"
JUNIT_RUNNER_USE_TESTRUNNER = False

JUNIT_JDK_FLOOR = "17"

JUNIT_REJECTED = "unpinned runner rejected: no floating version or head"

JUNIT_FIXTURE_JAVA_CLASS = "hello.HelloJupiterTest"
JUNIT_FIXTURE_KOTLIN_CLASS = "hello.HelloJupiterTest"
