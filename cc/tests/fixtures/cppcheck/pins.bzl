"""Cppcheck check-only wiring.

Contract: `docs/quality/tool-integrations.md#initial-adapter-qualification`,
`docs/tools/tool-acquisition.md#initial-artifact-research`.
"""

# Pinned reference version (qualified seed-only under issue #487; living
# at head rejected; digests stay owned under issue #798).
CPPCHECK_VERSION = "2.21.0"

# Distribution identity (standalone checksummed release artifact; digests
# stay owned under issue #798).
CPPCHECK_ARTIFACT = "standalone checksummed release artifact"

# Invocation shapes (check-only with the provisional sandbox-apply-and-diff
# fix flow).
CPPCHECK_CHECK = "cppcheck --xml --xml-version=2 on stderr (exit 0 clean with empty errors, exit 1 with error elements when dirty)"
CPPCHECK_FIX = "check-only with the provisional sandbox-apply-and-diff fix flow"
CPPCHECK_CONFIG_POLICY = "upstream built-in default enablement without suppressions; --enable=all maxima never enabled"

# Live proof labels (foundation consumers stay green; adapter dispatch
# owned under issue #798).
CC_FIXTURE_HELLO = "//cc/tests/fixtures/hello:hello_test"

# Rejected: --enable=all maxima, text-parse fallback, silent empty pass.
CPPCHECK_REJECTED = "--enable=all maxima rejected; text-parse fallback rejected; unparsable XML as clean rejected"
