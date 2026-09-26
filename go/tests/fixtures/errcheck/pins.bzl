"""Errcheck check-only wiring, complementary to govet.

"""

ERRCHECK_VERSION = "v1.20.0"

ERRCHECK_ARTIFACT = "standalone checksummed release artifact; complementary for unhandled errors"

ERRCHECK_CHECK = "errcheck text diagnostics file:line:col: message on stdout (exit 0 clean, exit 1 with findings)"
ERRCHECK_FIX = "check-only with the provisional sandbox-apply-and-diff fix flow"
ERRCHECK_CONFIG_POLICY = "complementary for unhandled errors, not a default selection; check-only"

GO_FIXTURE_HELLO = "//go/tests/fixtures/hello:hello_test"

ERRCHECK_REJECTED = "default-selection claim rejected; exit-code-only classification rejected"
