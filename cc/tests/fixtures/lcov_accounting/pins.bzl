"""LCOV accounting pins (issue #501).

Contract: `docs/native-toolchains.md#coverage-generation-and-ide-gaps`,
`docs/native-toolchains.md#qualification-questions-and-delivery`,
`docs/testing/README.md#coverage`.
Fixture: `cc/tests/fixtures/lcov_accounting/` via
`bazel run //tools/ci:lcov_accounting_qualification`.

Decides the accounting slice of the native baseline: every executable
first-party line is accounted as covered or validly ignored with fixture
evidence recorded here and in the owning docs. Unaccounted lines stay
rejected per the issue alternatives. Collection failures are never
ignored: missing reports plus incomplete instrumentation plus absent
eligible sources fail the gate. Backends stay provisional; floors stay
owned under issue #500, never double-claimed here.
"""

# Accounted shapes: Rust-only plus C/C++-only plus mixed/DLL LCOV.
# Rust-only is the pinned rules_rust llvm-cov integration over
# `rust/tests/fixtures/hello` (DA union with maximum hits winning).
# C/C++-only is the pinned Bazel LLVM source coverage over the
# accounting lib plus test below (DA records for `.c`/`.cc`/`.cpp`/
# `.cxx`/`.h`/`.hh`/`.hpp`/`.hxx`, blank plus comment-only lines never
# executable). Mixed/DLL unions hits across the cell's tests by authored
# source, proven by the cxx_identity bridge plus linux_corpus corpus.
RUST_ONLY_SHAPE = "Rust-only LCOV via pinned rules_rust llvm-cov"
RUST_ONLY_FIXTURE = "//rust/tests/fixtures/hello:hello_test"
CC_ONLY_SHAPE = "C/C++-only LCOV via pinned Bazel LLVM source coverage"
CC_ONLY_FIXTURE_LIB = "//cc/tests/fixtures/lcov_accounting:accounting"
CC_ONLY_FIXTURE_TEST = "//cc/tests/fixtures/lcov_accounting:accounting_test"
MIXED_DLL_SHAPE = "mixed plus DLL LCOV unioned by authored source"
MIXED_DLL_FIXTURES = [
    "//rust/tests/fixtures/cxx_identity:bridge",
    "//rust/tests/fixtures/cxx_identity:cxx_identity",
    "//cc/tests/fixtures/linux_corpus:corpus_test",
]
MIXED_DLL_NOTE = "Aggregation deduplicates by authored source and unions hits across the cell's tests"
CXXBRIDGE_CMD_LABEL = "@crates//:cxxbridge-cmd"

# Missed-line tests: a zero-hit eligible line fails with its location,
# never just a rate dip. Proven by the synthetic uncovered LCOV in the
# harness plus the deliberately missed-line shape below.
MISSED_LINE_SHAPE = "missed-line test with uncovered location"
MISSED_LINE_NOTE = "a zero-hit eligible line fails with its location, not just a rate dip"

# Coverage-tool version pairing: Rust/Clang raw-profile compatibility is
# the upgrade gate. rules_rs selects llvm-cov plus llvm-profdata from
# `@llvm` with no override parameters; the paired identities below must
# move together. Unpinned LLVM stays rejected.
BAZEL_VERSION = "9.2.0"
RULES_RUST_VERSION = "0.74.0"
RULES_CC_VERSION = "0.2.22"
RUST_VERSION = "1.98.0"
LLVM_BASELINE_MODULE = "0.8.18"
LLVM_BASELINE_LLVM = "22.1.8"
LLVM_TARGET_MODULE = "0.8.19"
LLVM_TARGET_LLVM = "23.1.0"
LLVM_TARGET_COMMIT = "6314688712edf3a95f78642d80393868256b4ef2"
RULES_RS_VERSION = "v0.0.109"
PATCHED_RULES_RUST_COMMIT = "e9dd49f22cfa43c75ba30cd9d9bb7d8bdc459dde"
TOOL_PAIRING_NOTE = "Rust/Clang raw-profile compatibility is the upgrade gate; llvm-cov plus llvm-profdata move with LLVM"

# Native ignores plus denominator validation: `LCOV_EXCL_LINE` for one
# line plus `LCOV_EXCL_START`/`LCOV_EXCL_STOP` for a range, each with a
# nearby `reason:` on the same or previous line (see
# `docs/testing/README.md#coverage`). Valid ignores exclude their lines
# from the denominator; missing reasons plus malformed directives fail.
# A target with no executable lines is listed as no-code, never an
# implicit pass; an empty denominator is never a pass.
NATIVE_IGNORE_LINE = "LCOV_EXCL_LINE"
NATIVE_IGNORE_START = "LCOV_EXCL_START"
NATIVE_IGNORE_STOP = "LCOV_EXCL_STOP"
NATIVE_IGNORE_REASON = "reason:"
NATIVE_IGNORE_NOTE = "valid ignores exclude their executable lines from the denominator with a nearby reason"
DENOMINATOR_NOTE = "Non-ignored eligible sources absent from reports or never executed remain in the denominator"

# Rejected substitutes per the issue alternatives: unaccounted lines plus
# ignored collection failures plus averaged percentages plus cross-cell
# union plus rounding up.
REJECTED_ALTERNATIVES = [
    "unaccounted lines",
    "ignored collection failures",
    "averaged percentages",
    "cross-cell union",
    "rounding up",
]

# Live proof labels: the accounting lib plus test below prove the
# C/C++-only shape on the seed host; the Rust hello plus cxx_identity
# bridge plus linux_corpus prove the Rust-only plus mixed shapes.
LCOV_ACCOUNTING_FIXTURE_LIB = "//cc/tests/fixtures/lcov_accounting:accounting"
LCOV_ACCOUNTING_FIXTURE_TEST = "//cc/tests/fixtures/lcov_accounting:accounting_test"
LCOV_ACCOUNTING_RUST_HELLO = "//rust/tests/fixtures/hello:hello_test"
LCOV_ACCOUNTING_MIXED_CXX = "//rust/tests/fixtures/cxx_identity:bridge"
LCOV_ACCOUNTING_MIXED_RUST = "//rust/tests/fixtures/cxx_identity:cxx_identity"
LCOV_ACCOUNTING_CORPUS = "//cc/tests/fixtures/linux_corpus:corpus_test"
LCOV_ACCOUNTING_FIXTURE_CORPUS = "//cc/tests/fixtures/lcov_accounting:corpus_starlark"
