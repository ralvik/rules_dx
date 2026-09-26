"""Staticcheck check-only wiring.

Contract: `docs/quality/tool-integrations.md#initial-adapter-qualification`,
`docs/tools/tool-acquisition.md#initial-artifact-research`.
"""

STATICCHECK_VERSION = "2026.2"

# Distribution identity (standalone checksummed release artifact; digests
STATICCHECK_ARTIFACT = "standalone checksummed release artifact"

# Invocation shapes (check-only with the provisional sandbox-apply-and-diff
# fix flow).
STATICCHECK_CHECK = "staticcheck -f json with a JSON array on stdout (exit 0 clean with [], exit 1 with finding objects when dirty)"
STATICCHECK_FIX = "check-only with the provisional sandbox-apply-and-diff fix flow"
STATICCHECK_CONFIG_POLICY = "default checks are the upstream built-in default checks, not an SA-only preset; SA-only shortcut rejected without qualification"

# Live proof labels (foundation consumers stay green; adapter dispatch
GO_FIXTURE_HELLO = "//go/tests/fixtures/hello:hello_test"

# Rejected: SA-only shortcut preset, -all maxima, SARIF ingestion.
STATICCHECK_REJECTED = "SA-only shortcut rejected without qualification; -all maxima rejected; SARIF ingestion rejected (native JSON is the faithful shape)"
