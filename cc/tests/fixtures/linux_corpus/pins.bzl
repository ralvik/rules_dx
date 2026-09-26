LINUX_GLIBC_PROFILE = "linux_x86_64/arm64 glibc"
LINUX_NATIVE_ONLY = "Native only"

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

SQLITE_SHAPE = "source-built SQLite"
SQLITE_NOTE = "SQLite source-built C amalgamation with declared Bazel native inputs"

OPENSSL_SHAPE = "OpenSSL with declared build tools"
OPENSSL_DECLARED_TOOLS = ["perl", "declared build tools or Bazel library inputs"]
OPENSSL_NOTE = "OpenSSL with explicitly declared build tools or Bazel library inputs, never ambient discovery"

RING_SHAPE = "ring-style C/assembly with target libs"
RING_NOTE = "ring-style C plus assembly with target-platform libraries, exec-platform tools for scripts"

BINDGEN_BASELINE_LLVM = "22.1.8"
BINDGEN_TARGET_LLVM = "23.1.0"
BINDGEN_FLAGS = ["--no-include-path-detection", "--formatter=none"]
BINDGEN_EXEC_CLOSURE = "explicit execution-platform libclang closure"
BINDGEN_TARGET_FLAGS = "target parsing flags"
BINDGEN_ROUTES = ["standalone rust_bindgen", "build-script route"]
BINDGEN_NOTE = "bindgen standalone plus build-script routes with execution libclang closure and target parsing flags"

CXX_IDENTITY_VERSION = "1.0.200"
CXXBRIDGE_CMD_LABEL = "@crates//:cxxbridge-cmd"
CXX_NOTE = "CXX single-graph execution with @crates//:cxxbridge-cmd"

REJECTED_ALTERNATIVES = [
    "single-crate proof",
    "ambient host-tool discovery",
]

LINUX_CORPUS_FIXTURE_LIB = "//cc/tests/fixtures/linux_corpus:corpus"
LINUX_CORPUS_FIXTURE_TEST = "//cc/tests/fixtures/linux_corpus:corpus_test"
LINUX_CORPUS_BINDGEN_HEADERS = "//rust/tests/fixtures/bindgen:bindgen_headers"
LINUX_CORPUS_MIXED_CXX = "//rust/tests/fixtures/cxx_identity:bridge"
LINUX_CORPUS_MIXED_RUST = "//rust/tests/fixtures/cxx_identity:cxx_identity"
LINUX_CORPUS_STABLE_CC = "//cc/tests/fixtures/hello:hello"
LINUX_CORPUS_STABLE_RUST = "//rust/tests/fixtures/hello:hello"
LINUX_CORPUS_FIXTURE_CORPUS = "//cc/tests/fixtures/linux_corpus:corpus_starlark"
