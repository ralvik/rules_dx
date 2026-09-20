"""Native quality defaults pins.

Contract: `docs/product/support-matrix.md#provisional-default-quality-tools`,
`docs/quality/native-configuration.md#authority`.
"""

# Pinned upstream versions (initial artifact research observations, now
# qualified seed-only under; living at head rejected).
CLANG_TOOLCHAIN_VERSION = "hermetic-llvm v0.8.19"
LLVM_VERSION = "23.1.0"
CPPCHECK_VERSION = "2.21.0"
GOFUMPT_VERSION = "v0.11.0"
STATICCHECK_VERSION = "2026.2"
GOVET_TOOLCHAIN_VERSION = "1.26.6"
ERRCHECK_VERSION = "v1.20.0"

# Upstream distribution identities (split native route; digests stay owned
# , never reconstructed or ambient-resolved).
CLANG_TOOLCHAIN_ROUTE = "authoritative hermetic-llvm LLVM tool targets (clang-format, clang-tidy), no separate acquisition"
CPPCHECK_ARTIFACT = "standalone checksummed release artifact; --xml --xml-version=2 on stderr"
GOFUMPT_ARTIFACT = "standalone checksummed release artifact; strict superset of gofmt"
STATICCHECK_ARTIFACT = "standalone checksummed release artifact; SARIF via -f sarif, JSON via -f json"
GOVET_COUPLING = "ships with the qualified Go toolchain (rules_go 0.63.0 plus Go SDK 1.26.6), no separate acquisition"
ERRCHECK_ARTIFACT = "standalone checksummed release artifact; complementary for unhandled errors"

# Rejected lines (never pinned here).
REJECTED_LLVM_PRIOR = "hermetic-llvm v0.8.18 plus LLVM 22.1.8 prior line rejected; living at head rejected"
REJECTED_GOVET_OBSERVED = "Go 1.27.1 observed, not pinned; govet follows the qualified Go SDK 1.26.6 pin"
REJECTED_STATICCHECK_BINARY = "v0.8.1 observed binary identity, not the pinned release line; unpinned versions rejected"

# Native-configuration sole policy (no hidden presets). Without an applicable
# checked-in native config the pinned tool uses its upstream built-in
# behavioral defaults; with a config the tool interprets it natively.
NATIVE_CONFIG_POLICY = "native-configuration sole policy: no hidden presets"

# Qualified rule-set resolutions (upstream built-in defaults, not rules_dx presets).
STATICCHECK_POLICY = "default checks are the upstream built-in default checks, not an SA-only preset; SA-only shortcut rejected without qualification"
GOVET_POLICY = "default analyzers are the upstream built-in default analyzers; all-analyzer and vettool maxima never enabled"
ERRCHECK_POLICY = "complementary for unhandled errors, not a default selection; check-only"
CLANG_TIDY_POLICY = "default checks are the upstream built-in default checks; --checks=* maxima never enabled"
CPPCHECK_POLICY = "default enablement is the upstream built-in default enablement; --enable=all maxima never enabled"
GOFUMPT_POLICY = "strict superset of gofmt; whole-file rewrite with check/diff mode, no rule-set selection"
CLANG_FORMAT_POLICY = "authoritative-toolchain formatter; deterministic format, no rule-set selection"
BEYOND_DEFAULT_REJECTED = "beyond-default switches rejected: staticcheck SA-only preset plus -all, govet all analyzers, clang-tidy --checks=*, cppcheck --enable=all"

# Live proof labels (foundation consumers stay green; quality adapters claim
# nothing yet).
NATIVE_FIXTURE_CC_HELLO = "//cc/tests/fixtures/hello:hello_test"
NATIVE_FIXTURE_GO_HELLO = "//go/tests/fixtures/hello:hello_test"

# Rejected: hidden presets plus unpinned versions plus SA-only shortcut.
NATIVE_REJECTED = "hidden presets rejected: no SA-only preset, no preset checks/enablement/analyzers; unpinned versions rejected: no floating version or head; --enable=all-style maxima rejected"
