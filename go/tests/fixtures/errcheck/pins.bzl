"""Errcheck check-only wiring, complementary to govet.

Contract: `docs/quality/tool-integrations.md#initial-adapter-qualification`,
`docs/tools/tool-acquisition.md#initial-artifact-research`.
"""

ERRCHECK_VERSION = "v1.20.0"

# Distribution identity (standalone checksummed release artifact;
# complementary for unhandled errors, not a default selection; digests
ERRCHECK_ARTIFACT = "standalone checksummed release artifact; complementary for unhandled errors"

# Invocation shapes (check-only with the provisional sandbox-apply-and-diff
# fix flow).
ERRCHECK_CHECK = "errcheck text diagnostics file:line:col: message on stdout (exit 0 clean, exit 1 with findings)"
ERRCHECK_FIX = "check-only with the provisional sandbox-apply-and-diff fix flow"
ERRCHECK_CONFIG_POLICY = "complementary for unhandled errors, not a default selection; check-only"

# Live proof labels (foundation consumers stay green; adapter dispatch
GO_FIXTURE_HELLO = "//go/tests/fixtures/hello:hello_test"

# Rejected: default-selection claim, exit-code-only classification.
ERRCHECK_REJECTED = "default-selection claim rejected; exit-code-only classification rejected"
