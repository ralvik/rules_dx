"""Stable-stack compose pins (issue #494).

Contract: `docs/native-toolchains.md#selected-qualification-stack`,
`docs/native-toolchains.md#inspected-identities`,
`docs/native-toolchains.md#qualification-questions-and-delivery`.

Freezes the exact current stable stack that composes on the seed host,
with fixture evidence recorded explicitly here and in the owning docs,
never silently composed. Ad-hoc compose is rejected per the issue
alternatives.

As-built stack (frozen resolved Bzlmod identities from `.bazelversion`,
`MODULE.bazel`, `MODULE.bazel.lock`, and `libs/testing/tested_stack.bzl`):
Bazel 9.2.0 with rules_rust 0.74.0 plus rules_cc 0.2.22 plus Rust 1.98.0
(edition 2021, rustfmt coupled 1.98.0). The committed `MODULE.bazel.lock`
carries the BCR `registryFileHashes` integrity for the pinned modules.
Living at head plus floating versions stay rejected.

Candidate comparison (upgrading is a real stack change, never silent):
rules_rs v0.0.109 (commit `b55b132af0c9951807c926768e40222330348632`)
declares LLVM rules `0.8.18`/LLVM `22.1.8` as its baseline, not the newer
candidate hermetic-llvm v0.8.19 (commit
`6314688712edf3a95f78642d80393868256b4ef2`, LLVM `23.1.0`). Its pinned
patched rules_rust (commit `e9dd49f22cfa43c75ba30cd9d9bb7d8bdc459dde`,
module `0.74.0`) does not make it stock bazelbuild/rules_rust `0.74.0`.
C/C++ rules stay rules_cc 0.2.22 (record the resolved module graph:
upstream direct declarations differ). Windows backend stays
toolchains_msvc at `8e2aa4624bbb5a53a94f135e90995f307875d1ad`
(declared module `0.0.0` is not a release). Reference v0.8.18 uses LLVM
22.1.8; upgrading to v0.8.19/LLVM 23.1.0 is a real stack change.

Checksums plus source patches plus compiler/profile compatibility:
freeze compiler archives, runtime sources, upstream patches, SDK manifests
and package hashes separately from ruleset source hashes. For LLVM, the
pinned compiler archive index
(`extensions/llvm_toolchain_minimal_index.json`) and source acquisition
(`extensions/llvm.bzl`) are the starting points, not an independently
maintained duplicate inventory. Bazel 9 is a provisional initial coverage
baseline because the inspected hermetic-llvm coverage fixture requires it,
not a release pin: the release default follows ADR 0008 and the exact seed
pin is tracked here. Do not silently inherit rules_rs's older compiler
default or turn a research version into a release pin. Do not compose a
second independent Rust rules graph (no `cxx.rs` second graph, single
`rust_toolchains` repo, no `llvm.version(...)` override, no `bazel_dep`
`llvm` module, no `bazel_binaries.download` second Bazel).
"""

# As-built freeze (resolved Bzlmod identities; living at head rejected).
BAZEL_VERSION = "9.2.0"
RULES_RUST_VERSION = "0.74.0"
RULES_CC_VERSION = "0.2.22"
RUST_VERSION = "1.98.0"
RUST_EDITION = "2021"
RUSTFMT_VERSION = "1.98.0"

# Lock authority (frozen resolved graph; fail-closed committed lock).
STABLE_LOCK_FILE = "//:MODULE.bazel.lock"
STABLE_LOCK_INTEGRITY = "registryFileHashes"

# Candidate comparison (baseline vs target; upgrading is a real stack change).
RULES_RS_VERSION = "v0.0.109"
RULES_RS_COMMIT = "b55b132af0c9951807c926768e40222330348632"
PATCHED_RULES_RUST_COMMIT = "e9dd49f22cfa43c75ba30cd9d9bb7d8bdc459dde"
PATCHED_RULES_RUST_MODULE = "0.74.0"
LLVM_BASELINE_MODULE = "0.8.18"
LLVM_BASELINE_LLVM = "22.1.8"
LLVM_TARGET_MODULE = "0.8.19"
LLVM_TARGET_LLVM = "23.1.0"
LLVM_TARGET_COMMIT = "6314688712edf3a95f78642d80393868256b4ef2"
TOOLCHAINS_MSVC_COMMIT = "8e2aa4624bbb5a53a94f135e90995f307875d1ad"

# Checksums plus source patches (freeze separately from ruleset hashes).
COMPILER_ARCHIVE_INDEX = "extensions/llvm_toolchain_minimal_index.json"
SOURCE_ACQUISITION = "extensions/llvm.bzl"
FREEZE_NOTE = "Freeze compiler archives, runtime sources, upstream patches, SDK manifests and package hashes separately from ruleset source hashes"

# Compiler/profile compatibility (provisional baseline, not a release pin).
BAZEL_BASELINE_NOTE = "Bazel 9 is a provisional initial coverage baseline because the inspected hermetic-llvm coverage fixture requires it, not a release pin"
RELEASE_DEFAULT_NOTE = "the release default follows ADR 0008 and the exact seed pin is tracked here"
NO_SILENT_INHERIT = "Do not silently inherit rules_rs's older compiler default or turn a research version into a release pin"
NO_SECOND_GRAPH = "do not compose a second independent Rust rules graph"

# Live proof labels (as-built stack composes on the seed host; candidate
# backend plus corpus wiring stay provisional under issue #499).
STABLE_FIXTURE_RUST_HELLO = "//rust/tests/fixtures/hello:hello"
STABLE_FIXTURE_RUST_TEST = "//rust/tests/fixtures/hello:hello_test"
STABLE_FIXTURE_CC_HELLO = "//cc/tests/fixtures/hello:hello"
STABLE_FIXTURE_CC_TEST = "//cc/tests/fixtures/hello:hello_test"

# Rejected: ad-hoc compose (floating, silent inherit, research as release
# pin, second graph, host discovery, second Bazel).
STABLE_REJECTED = "ad-hoc compose rejected: floating llvm.version override plus MODULE llvm dep plus cxx.rs second graph plus silent older-compiler inherit plus research version as release pin plus host discovery plus bazel_binaries.download second Bazel rejected"
