"""Clang-tidy check-only wiring with the target-coupling decision.

Contract: `docs/quality/tool-integrations.md#initial-adapter-qualification`,
`docs/tools/tool-acquisition.md#initial-artifact-research`.
"""

CLANG_TIDY_VERSION = "hermetic-llvm v0.8.19 (LLVM 23.1.0)"

# Distribution identity (authoritative-toolchain route: qualified
# hermetic-llvm LLVM tool targets, no separate acquisition; digests stay
CLANG_TIDY_ARTIFACT = "authoritative hermetic-llvm LLVM tool targets (clang-tidy), no separate acquisition"

# Target-coupling decision (recorded here, never silent): check-only with
CLANG_TIDY_WIRING = "check-only text diagnostics with optional compile-commands context from the authoritative target"
CLANG_TIDY_CHECK = "clang-tidy text diagnostics file:line:col: warning|error: message [check] (exit 0 clean, exit 1 with findings)"
CLANG_TIDY_FIX = "check-only with the provisional sandbox-apply-and-diff fix flow; --fix plus --export-fixes unbound"
CLANG_TIDY_CONFIG_POLICY = "checked-in .clang-tidy required to change policy; upstream built-in default checks without config, --checks=* maxima never enabled"

# Live proof labels (foundation consumers stay green; adapter dispatch
CC_FIXTURE_HELLO = "//cc/tests/fixtures/hello:hello_test"

# Rejected: silent skipping without compile-commands, --checks=* maxima,
# IN_PLACE mutation outside sandbox-apply-and-diff.
CLANG_TIDY_REJECTED = "silent check-only fallback that misses findings rejected; --checks=* maxima rejected; IN_PLACE patching rejected"
