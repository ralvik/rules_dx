"""Selective Maven `dx update` wont-fix fixture.

"""

SELECTIVE_MAVEN = "wont-fix"

SELECTIVE_MAVEN_FULL = "REPIN=1 bazel run @maven//:pin"

SELECTIVE_MAVEN_HINT = "use `dx update maven` for the set"

SELECTIVE_MAVEN_SEED = "maven:junit:junit"
SELECTIVE_MAVEN_JUPITER = "maven:org.junit.jupiter:junit-jupiter-api"

SELECTIVE_MAVEN_BARE_INVALID = "maven:junit"
SELECTIVE_MAVEN_EMPTY_INVALID = "maven::artifact"

SELECTIVE_MAVEN_LOCK = "third_party/jvm/maven_install.json"
SELECTIVE_MAVEN_MANIFEST = "MODULE.bazel"

REJECTED_SILENT_FULL_SUBSTITUTION = "silent full-update substitution rejected"
REJECTED_PRIVATE_EMULATION = "private per-artifact emulation rejected"
REJECTED_HAND_EDITED_LOCK = "hand-edited maven_install.json rejected"
NO_SUPPORTED = "no Supported claim"
SEED_ONLY = "qualified seed-only under issue #634"
