CGO_LIB = "//go/tests/fixtures/cgo:cgo"
CGO_TEST = "//go/tests/fixtures/cgo:cgo_test"
CGO_ATTR = "cgo = True"

RULES_GO_VERSION = "0.63.0"

RACE_TEST_ATTRS = "race = on plus pure = off"
RACE_REQUIRES_CGO = "race requires cgo"

CGO_REJECTED = "generated cgo plus generic parity rejected: cgo stays handwritten"
