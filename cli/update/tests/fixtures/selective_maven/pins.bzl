"""Selective Maven `dx update` wont-fix fixture (issue #634).

Contract: `docs/decisions/0024-selective-update.md`,
`docs/cli/commands/audit-update-bazel.md#dx-update`.
Fixture: `cli/update/tests/fixtures/selective_maven/` via
`bazel run //tools/ci:selective_maven_qualification`.

Maven per-artifact selective is wont-fix in V1: `rules_jvm_external`
offers no per-artifact pin target, exact pins in `MODULE.bazel` stay
constraints, and the whole-lock `REPIN=1 bazel run @maven//:pin`
refresh is the only approved updater. `maven:group:artifact` parses
but fails closed as `unsupported` with the `dx update maven` hint,
never silently substituting a full update. Update-only; no lock
format change. Seed only: platform plus consumer plus release
evidence stays owned gap; no Supported claim.
"""

# Disposition for this set (wont-fix in V1).
SELECTIVE_MAVEN = "wont-fix"

# Approved full-set operation (argv owned by `dx_update::backend`).
SELECTIVE_MAVEN_FULL = "REPIN=1 bazel run @maven//:pin"

# Fail-closed selective hint (never widens to full silently).
SELECTIVE_MAVEN_HINT = "use `dx update maven` for the set"

# Per-artifact identities that parse yet stay unsupported (seed line
# plus Jupiter line from MODULE.bazel).
SELECTIVE_MAVEN_SEED = "maven:junit:junit"
SELECTIVE_MAVEN_JUPITER = "maven:org.junit.jupiter:junit-jupiter-api"

# Invalid shapes still fail closed at parse time (no group:artifact).
SELECTIVE_MAVEN_BARE_INVALID = "maven:junit"
SELECTIVE_MAVEN_EMPTY_INVALID = "maven::artifact"

# Lock authority (pins stay exact, lock is generated).
SELECTIVE_MAVEN_LOCK = "third_party/jvm/maven_install.json"
SELECTIVE_MAVEN_MANIFEST = "MODULE.bazel"

# Rejected and honesty lines (never pinned as supported here).
REJECTED_SILENT_FULL_SUBSTITUTION = "silent full-update substitution rejected"
REJECTED_PRIVATE_EMULATION = "private per-artifact emulation rejected"
REJECTED_HAND_EDITED_LOCK = "hand-edited maven_install.json rejected"
NO_SUPPORTED = "no Supported claim"
SEED_ONLY = "qualified seed-only under issue #634"
