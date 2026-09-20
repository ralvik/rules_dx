"""Go test runner pins.

Contract: `docs/product/support-matrix.md#provisional-default-test-runners`.
"""

# Upstream ruleset pin (MODULE.bazel plus MODULE.bazel.lock).
RULES_GO_VERSION = "0.63.0"

# Toolchain pin (go_sdk.download in MODULE.bazel).
GO_SDK_VERSION = "1.26.6"

# Runner mapping: one package-level go_test per directory via embed.
GOTEST_KIND = "go_test"
GOTEST_RUNNER = "go test"
GOTEST_EMBED_ATTR = "embed"

# Live proof target (wrapper consumer, stdlib-only, no ecosystem lock).
GOTEST_FIXTURE_LABEL = "//go/tests/fixtures/hello:hello_test"

# Rejected: implicit runner (bare go test without the wrapper mapping).
GOTEST_REJECTED = "implicit runner rejected: no bare go test without go_test plus embed"
