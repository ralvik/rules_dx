"""Linux corpus pins.

Contract: `docs/native-toolchains.md#profiles-and-cross-builds`,
`docs/native-toolchains.md#rust-and-native-integration`,
`docs/native-toolchains.md#qualification-questions-and-delivery`.
Fixture: `cc/tests/fixtures/linux_corpus/` via
`bazel run //tools/ci:linux_corpus_qualification`.
"""

# Linux profiles proved complete by this corpus (native only).
# Glibc on Linux x86_64/arm64; hosts stay owned.
LINUX_GLIBC_PROFILE = "linux_x86_64/arm64 glibc"
LINUX_NATIVE_ONLY = "Native only"

# Corpus members: pure-Rust plus cc-rs C plus SQLite plus OpenSSL with
# declared tools plus ring-style C/assembly plus bindgen plus CXX plus
# native proc-macro dependencies. Application-locked crate versions and
# feature sets define tested workflows; crate names alone do not.
CORPUS_MEMBERS = [
    "pure-Rust",
    "cc-rs C",
    "SQLite source-built",
    "OpenSSL with declared build tools",
    "ring-style C/assembly with target libs",
    "bindgen standalone plus build-script routes",
    "CXX single-graph execution",
    "native proc-macro exec/target separation",
]

# SQLite shape: source-built C amalgamation through declared Bazel
# native inputs, never a prebuilt glibc binary.
SQLITE_SHAPE = "source-built SQLite"
SQLITE_NOTE = "SQLite source-built C amalgamation with declared Bazel native inputs"

# OpenSSL shape: explicitly declared build tools or Bazel library
# inputs. The canonical upstream build needs perl plus declared tools;
# ambient host-tool discovery stays rejected.
OPENSSL_SHAPE = "OpenSSL with declared build tools"
OPENSSL_DECLARED_TOOLS = ["perl", "declared build tools or Bazel library inputs"]
OPENSSL_NOTE = "OpenSSL with explicitly declared build tools or Bazel library inputs, never ambient discovery"

# ring shape: C plus assembly with target-platform libraries.
# Build scripts and proc macros keep execution-platform tools while
# applications link target libraries.
RING_SHAPE = "ring-style C/assembly with target libs"
RING_NOTE = "ring-style C plus assembly with target-platform libraries, exec-platform tools for scripts"

# Bindgen shapes: standalone rust_bindgen plus build-script routes.
BINDGEN_BASELINE_LLVM = "22.1.8"
BINDGEN_TARGET_LLVM = "23.1.0"
BINDGEN_FLAGS = ["--no-include-path-detection", "--formatter=none"]
BINDGEN_EXEC_CLOSURE = "explicit execution-platform libclang closure"
BINDGEN_TARGET_FLAGS = "target parsing flags"
BINDGEN_ROUTES = ["standalone rust_bindgen", "build-script route"]
BINDGEN_NOTE = "bindgen standalone plus build-script routes with execution libclang closure and target parsing flags"

# CXX shape: decided single-graph identity plus corpus execution.
# cxx plus cxxbridge-cmd at 1.0.200 from the single crate_universe
# crates graph with generator tool @crates//:cxxbridge-cmd, never a
# cxx.rs second graph (decided).
CXX_IDENTITY_VERSION = "1.0.200"
CXXBRIDGE_CMD_LABEL = "@crates//:cxxbridge-cmd"
CXX_NOTE = "CXX single-graph execution with @crates//:cxxbridge-cmd"

# Rejected substitutes per the issue alternatives: single-crate proof,
# plus availability/ancestry-style shortcuts carried from.
REJECTED_ALTERNATIVES = [
    "single-crate proof",
    "ambient host-tool discovery",
]

# Live proof labels: the corpus lib plus test below prove the C
# SQLite/OpenSSL/ring shapes on the seed host; bindgen headers plus
# cxx_identity bridge plus hellos prove the Rust/C++ composition.
LINUX_CORPUS_FIXTURE_LIB = "//cc/tests/fixtures/linux_corpus:corpus"
LINUX_CORPUS_FIXTURE_TEST = "//cc/tests/fixtures/linux_corpus:corpus_test"
LINUX_CORPUS_BINDGEN_HEADERS = "//rust/tests/fixtures/bindgen:bindgen_headers"
LINUX_CORPUS_MIXED_CXX = "//rust/tests/fixtures/cxx_identity:bridge"
LINUX_CORPUS_MIXED_RUST = "//rust/tests/fixtures/cxx_identity:cxx_identity"
LINUX_CORPUS_STABLE_CC = "//cc/tests/fixtures/hello:hello"
LINUX_CORPUS_STABLE_RUST = "//rust/tests/fixtures/hello:hello"
LINUX_CORPUS_FIXTURE_CORPUS = "//cc/tests/fixtures/linux_corpus:corpus_starlark"
