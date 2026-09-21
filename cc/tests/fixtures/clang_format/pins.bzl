"""Clang-format check plus fix wiring.

Contract: `docs/quality/tool-integrations.md#initial-adapter-qualification`,
`docs/tools/tool-acquisition.md#initial-artifact-research`.
"""

# Pinned reference version (qualified seed-only under issue #487; living
# at head rejected; digests stay owned under issue #798).
CLANG_FORMAT_VERSION = "hermetic-llvm v0.8.19 (LLVM 23.1.0)"

# Distribution identity (authoritative-toolchain route: qualified
# hermetic-llvm LLVM tool targets, no separate acquisition; digests stay
# owned under issue #798).
CLANG_FORMAT_ARTIFACT = "authoritative hermetic-llvm LLVM tool targets (clang-format), no separate acquisition"

# Invocation shapes (whole-file rewrite with check/diff mode).
CLANG_FORMAT_CHECK = "clang-format check with unified diff on stdout (exit 0 clean, exit 1 with diff headers when dirty)"
CLANG_FORMAT_FIX = "clang-format -i in-place rewrite (re-read on exit 0, keep input otherwise)"
CLANG_FORMAT_CONFIG_POLICY = "checked-in .clang-format required to change policy; upstream built-in defaults without config, native interpretation with config"

# Live proof labels (foundation consumers stay green; adapter dispatch
# owned under issue #798).
CC_FIXTURE_HELLO = "//cc/tests/fixtures/hello:hello_test"

# Rejected: ambient .clang-format discovery, auto-supplied style preset.
CLANG_FORMAT_REJECTED = "ambient .clang-format discovery rejected; auto-supplied --style preset rejected"
