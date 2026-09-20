"""JVM quality defaults pins (issue #485).

Contract: `docs/product/support-matrix.md#provisional-default-quality-tools`,
`docs/quality/native-configuration.md#authority`.

Qualifies the provisional JVM format plus lint defaults against the
native-configuration contract with no hidden presets. Versions below are the
pinned upstream releases from the initial artifact research; rule-sets are
upstream built-in defaults unless an applicable checked-in native config
supplies policy. Adapters add only transport/hermetic settings. Exact
artifact digests plus adapter mappings stay owned under issue #416.
"""

# Pinned upstream versions (initial artifact research observations, now
# qualified seed-only under #485; living at head plus alphas rejected).
GOOGLE_JAVA_FORMAT_VERSION = "1.35.0"
CHECKSTYLE_VERSION = "14.1.0"
PMD_VERSION = "7.27.0"
SPOTBUGS_VERSION = "4.10.4"
KTFMT_VERSION = "0.63"
KTLINT_VERSION = "1.8.0"
DETEKT_VERSION = "1.23.8"
ERROR_PRONE_VERSION = "2.50.0"

# Upstream distribution identities (complete-artifact plus shared-JDK route;
# digests stay owned under issue #416, never reconstructed from modules).
GOOGLE_JAVA_FORMAT_ARTIFACT = "google-java-format-all-deps.jar"
CHECKSTYLE_ARTIFACT = "checkstyle-all.jar"
PMD_ARTIFACT = "pmd-dist-bin.zip"
SPOTBUGS_ARTIFACT = "spotbugs-dist.zip"
KTFMT_ARTIFACT = "ktfmt-with-dependencies.jar"
KTLINT_ARTIFACT = "ktlint-executable.jar"
DETEKT_ARTIFACT = "detekt-cli-all.jar"
ERROR_PRONE_COUPLING = "javac plugin following the qualified JDK baseline"

# Rejected lines (never pinned here).
REJECTED_GOOGLE_JAVA_FORMAT_HEAD = "v1.36.x observed, not pinned; living at head rejected"
REJECTED_KTFMT_HEAD = "v0.64 observed, not pinned; living at head rejected"
REJECTED_KTLINT_ALPHA = "2.0.0 alphas are not stable; rejected"
REJECTED_DETEKT_ALPHA = "2.0.0 alphas target JDK 25, not stable; rejected"

# Native-configuration sole policy (no hidden presets). Without an applicable
# checked-in native config the pinned tool uses its upstream built-in
# behavioral defaults; with a config the tool interprets it natively.
NATIVE_CONFIG_POLICY = "native-configuration sole policy: no hidden presets"

# Qualified rule-set resolutions (upstream built-in defaults, not rules_dx presets).
CHECKSTYLE_POLICY = "no auto-supplied Google checks; upstream built-in defaults without config, native interpretation with config"
SPOTBUGS_POLICY = "default effort is the upstream built-in default effort, not a rules_dx preset"
PMD_POLICY = "default ruleset is the upstream built-in default ruleset, not a rules_dx preset"
ERROR_PRONE_POLICY = "default severities are the upstream default severities; experimental checks never enabled"
DETEKT_POLICY = "buildUponDefaultConfig full default set, not allRules; allRules never enabled"
KTLINT_POLICY = "standard rules are the upstream built-in standard set"
BEYOND_DEFAULT_REJECTED = "beyond-default switches rejected: detekt allRules, experimental Error Prone checks, --enable=all maxima"

# Live proof labels (foundation consumers stay green; quality adapters claim
# nothing yet under issue #416).
JVM_FIXTURE_JAVA_HELLO = "//java/tests/fixtures/hello:hello_test"
JVM_FIXTURE_KOTLIN_HELLO = "//kotlin/tests/fixtures/hello:hello_test"

# Rejected: hidden presets plus unpinned versions.
JVM_REJECTED = "hidden presets rejected: no auto-supplied Checkstyle Google checks, no preset effort/ruleset/severities; unpinned versions rejected: no floating version or head"
