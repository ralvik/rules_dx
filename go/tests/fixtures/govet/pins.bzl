"""Govet check-only wiring with errcheck complementary.

"""

GOVET_TOOLCHAIN_VERSION = "1.26.6"

GOVET_COUPLING = "ships with the qualified Go toolchain (rules_go 0.63.0 plus Go SDK 1.26.6), no separate acquisition"

GOVET_CHECK = "go vet text diagnostics file:line:col: message on stderr (exit 0 clean, exit 1 with findings)"
GOVET_FIX = "check-only with the provisional sandbox-apply-and-diff fix flow"
GOVET_CONFIG_POLICY = "default analyzers are the upstream built-in default analyzers; all-analyzer and vettool maxima never enabled"

GO_FIXTURE_HELLO = "//go/tests/fixtures/hello:hello_test"

GOVET_REJECTED = "all-analyzer maxima rejected; exit-code-only classification rejected"
