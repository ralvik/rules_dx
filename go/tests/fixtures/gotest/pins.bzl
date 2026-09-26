"""Go test runner pins.

"""

RULES_GO_VERSION = "0.63.0"

GO_SDK_VERSION = "1.26.6"

GOTEST_KIND = "go_test"
GOTEST_RUNNER = "go test"
GOTEST_EMBED_ATTR = "embed"

GOTEST_FIXTURE_LABEL = "//go/tests/fixtures/hello:hello_test"

GOTEST_REJECTED = "implicit runner rejected: no bare go test without go_test plus embed"
