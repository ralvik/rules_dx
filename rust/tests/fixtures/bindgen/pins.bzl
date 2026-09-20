"""Bindgen LLVM-22-vs-23 version pins (issue #473).

Contract: `docs/native-toolchains.md`, `docs/generation/rust.md#binding-generation`.
Fixture pair: `rust/tests/fixtures/bindgen/bindgen.h` plus `bindgen.expected`.

The standalone `rust_bindgen` parser baseline is LLVM-22 (rules_rs v0.0.109
declares `llvm 0.8.18`); the qualified header/flag target is LLVM-23
(hermetic-llvm v0.8.19). Upgrading across that line is a real stack change,
so both identities stay pinned here and in the native plan. Unpinned LLVM
(floating `llvm.version(...)` or an untracked hermetic-llvm) is rejected.
"""

RULES_RS_VERSION = "v0.0.109"
RULES_RS_COMMIT = "b55b132af0c9951807c926768e40222330348632"

# Parser baseline: rules_rs v0.0.109 MODULE declares llvm 0.8.18 (LLVM 22.1.8).
LLVM_BASELINE_MODULE = "0.8.18"
LLVM_BASELINE_LLVM = "22.1.8"

# Qualified target: hermetic-llvm v0.8.19 (LLVM 23.1.0).
LLVM_TARGET_MODULE = "0.8.19"
LLVM_TARGET_LLVM = "23.1.0"
LLVM_TARGET_COMMIT = "6314688712edf3a95f78642d80393868256b4ef2"

# Self-contained bindgen executables (statically linked libclang):
# hermeticbuild/bindgen v0.0.2 ships bindgen 0.72.1 for six platforms.
BINDGEN_CRATE_VERSION = "0.72.1"
BINDGEN_PREBUILT_TAG = "v0.0.2"
BINDGEN_PREBUILT_SHA256 = {
    "darwin_amd64": "9effe0323d0441d6f541497e0590b970beb15b9104c7e034ac4557942202f869",
    "darwin_arm64": "456ab5235685c498455ddc2fafeba32eb1d93758346e585e8da5e09c19cc680c",
    "linux_amd64": "ec2b39a56443142a34dc76ec32a17cb099c6c09137c3fbac893310c623cb10ac",
    "linux_arm64": "d4da684d021d1ebf40bb3819c1521c96c3ca51d22d911ee71731a358bd768594",
    "windows_amd64": "beede8f802ab96a2cce0de84c560150dcbc89c96db94f7c17519dd164ac983fd",
    "windows_arm64": "0885047b40b119e58fcca4491a2bd00331de131f41183de2a9b563ca407f9109",
}

# Standalone route flags: disable include discovery, no formatter.
# Target compiler context comes from the cc toolchain; the Rust consumer
# links the native library separately.
BINDGEN_FLAGS = ["--no-include-path-detection", "--formatter=none"]

BINDGEN_FIXTURE_HEADER = "//rust/tests/fixtures/bindgen:bindgen.h"
BINDGEN_FIXTURE_EXPECTED = "//rust/tests/fixtures/bindgen:bindgen.expected"
