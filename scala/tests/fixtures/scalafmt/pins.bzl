"""Scalafmt check plus fix wiring.

Contract: `docs/quality/tool-integrations.md#initial-adapter-qualification`,
`docs/tools/tool-acquisition.md#initial-artifact-research`.
"""

# Pinned reference version (qualified seed-only under issue #486; living
# at head rejected; digests stay owned under issue #797).
SCALAFMT_VERSION = "3.11.4"

# Distribution identity (managed-JVM route over the shared managed JDK plus
# the Scala Maven-lock story; digests stay owned under issue #797).
SCALAFMT_ARTIFACT = "compatible JVM artifact over the shared managed JDK"

# Invocation shapes (whole-file rewrite with check/diff mode).
SCALAFMT_CHECK = "scalafmt --check plus --config when hinted (exit 0 clean, exit 1 with unified diff when dirty)"
SCALAFMT_FIX = "scalafmt in-place rewrite (re-read on exit 0, keep input otherwise)"
SCALAFMT_CONFIG_POLICY = "checked-in .scalafmt.conf required to change policy; upstream built-in defaults without config, native interpretation with config"

# Live proof labels (foundation consumers stay green; adapter dispatch
# owned under issue #797).
SCALA_FIXTURE_HELLO = "//scala/tests/fixtures/hello:hello_test"

# Rejected: IN_PLACE mutation of inputs outside sandbox-apply-and-diff,
# ambient config discovery, auto-supplied preset.
SCALAFMT_REJECTED = "IN_PLACE patching outside sandbox rejected; ambient .scalafmt.conf discovery rejected; auto-supplied preset rejected"
