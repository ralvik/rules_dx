"""JVM foundation pins (Java, Kotlin, Scala managed route). Contract: docs/decisions/0019-first-release-additional-foundations.md."""

# Pinned JVM foundation (see MODULE.bazel; per-split pin_consistency in tools/ci/pin_consistency.sh).
RULES_JAVA_VERSION = "9.7.0"
RULES_KOTLIN_VERSION = "2.4.10"
RULES_SCALA_VERSION = "7.3.0"
SCALA_VERSION = "2.13.18"
RULES_JVM_EXTERNAL_VERSION = "7.1"

# Maven lock authority (manifest lives in //third_party/jvm:pins.bzl; see MODULE.bazel maven.install).
MAVEN_LOCK = "//third_party/jvm:maven_install.json"
MAVEN_REPIN = "REPIN=1 bazel run @maven//:pin"

# JVM quality-tool acquisition pins (see //quality/tools/jvm:repos.bzl plus
# //quality/tools/jvm:extension.bzl; MODULE.bazel `jvm_tools` use_repo must
# match `JVM_TOOL_REPOS`; `//tools/ci:pin_consistency_test` fails on drift).
JVM_TOOL_VERSIONS = {
    "google-java-format": "1.35.0",
    "checkstyle": "14.1.0",
    "pmd": "7.27.0",
    "spotbugs": "4.10.4",
    "ktfmt": "0.63",
    "ktlint": "1.8.0",
}

# Hermetic Java runtime floor (pinned in .bazelrc; baseline stays open under the support matrix).
JAVA_RUNTIME_VERSION = "remotejdk_21"
