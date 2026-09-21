"""Go cgo scope fixture pins.

Contract: `docs/environments/environment.md#ownership-and-refresh`.
Fixture: `go/tests/fixtures/cgo/` via
`bazel run //tools/ci:env_plugins_cgo_qualification`.
"""

# Handwritten cgo scope: generation never emits cgo sources.
CGO_LIB = "//go/tests/fixtures/cgo:cgo"
CGO_TEST = "//go/tests/fixtures/cgo:cgo_test"
CGO_ATTR = "cgo = True"

# Upstream ruleset pin (MODULE.bazel plus MODULE.bazel.lock).
RULES_GO_VERSION = "0.63.0"

# Race scope rides the cgo toolchain: race requires pure off.
RACE_TEST_ATTRS = "race = on plus pure = off"
RACE_REQUIRES_CGO = "race requires cgo"

# Rejected: generated cgo rules plus generic IDE parity claim.
CGO_REJECTED = "generated cgo plus generic parity rejected: cgo stays handwritten"
