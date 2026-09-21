"""C++ exact-target snapshot pins.

Contract: `docs/cli/target-resolution.md#exact-target-discovery`,
`docs/native-toolchains.md#coverage-generation-and-ide-gaps`,
`docs/native-toolchains.md#qualification-questions-and-delivery`.
Fixture: `cc/tests/fixtures/cpp_snapshot/` via
`bazel run //tools/ci:cpp_snapshot_qualification`.
"""

# Inspected snapshot identity: Hedron extractor at the pinned commit is
# the reuse candidate for action-derived commands, not a conforming
# integration by itself.
HEDRON_EXTRACTOR_VERSION = "abb61a688167623088f8768cc9264798df6a9d10"
HEDRON_EXTRACTOR_COMMIT = "abb61a688167623088f8768cc9264798df6a9d10"
HEDRON_EXTRACTOR_NOTE = "uses actual action commands"
HEDRON_SOURCE = "refresh.template.py"

# Pinned native stack for the snapshot proof.
RULES_CC_VERSION = "0.2.22"
BAZEL_VERSION = "9.2.0"

# Resolver-owned exact-target expression: unconfigured `bazel query`
# only, depth-1 direct owners over //..., deterministic sorted set.
RESOLVER_EXPRESSION_SHAPE = "kind('rule', rdeps(//..., set(<file-labels>), 1))"

# Action-derived snapshot: compile commands come from Bazel action
# inspection (`bazel aquery` `CppCompile` with command line plus inputs),
# never inferred from `CcInfo`.
SNAPSHOT_SHAPE = "action-derived CppCompile commands via aquery"
SNAPSHOT_REJECTED_INFERENCE = "infer compile commands from CcInfo"

# Generated sources: headers produced by Bazel actions have no checked-in
# owner and materialize through exact mappings, never inferred producers.
GENERATED_SOURCES_NOTE = "generated-output materialization through exact mappings"
GENERATED_REJECTED = "inferred generated producer"

# Multi-context headers: one header participates in several configured
# compile actions (lib plus bin plus test); the snapshot keeps each
# target context apart instead of merging to one entry.
MULTI_CONTEXT_NOTE = "multiple header contexts kept apart per target"
MULTI_CONTEXT_EXAMPLE = "cc/tests/fixtures/hello/hello.h in hello_lib plus hello plus hello_test compiles"

# Managed host tools: host-native clangd resolves from the qualified
# toolchain, never unrestricted query-driver execution or host fallback.
MANAGED_TOOLS_NOTE = "managed host-native clangd from the qualified toolchain"
MANAGED_REJECTED = "unrestricted clangd query-driver execution"

# Bazel-9 compatibility: the exact plus action proof runs on Bazel 9.2.0
# with rules_cc 0.2.22; newer Bazel alone is not proof.
BAZEL_9_NOTE = "Bazel-9 compatibility on Bazel 9.2.0 plus rules_cc 0.2.22"

# Raw extractor gaps that stay rejected as the conforming route.
REFRESH_REJECTED = [
    "refresh runs Bazel plus preprocessors",
    "workspace writes",
    "changes extraction features",
    "continued-after-failures",
]

# Rejected substitutes per the issue alternatives.
REJECTED_ALTERNATIVES = [
    "infer compile commands from CcInfo",
    "unrestricted clangd query-driver execution",
    "refresh runs Bazel plus preprocessors",
    "workspace writes",
    "continued-after-failures",
    "package-wide snapshot widening",
]

# Exact-isolation fixture pair: hello.cc resolves to hello_lib only,
# main.cc resolves to hello only, hello.h resolves to hello_lib only.
# Package-wide widening would merge them; exact discovery keeps them apart.
CPP_SNAPSHOT_FIXTURE_LIB_OWNER = "//cc/tests/fixtures/hello:hello_lib"
CPP_SNAPSHOT_FIXTURE_BIN_OWNER = "//cc/tests/fixtures/hello:hello"
CPP_SNAPSHOT_FIXTURE_LIB_SOURCE = "cc/tests/fixtures/hello/hello.cc"
CPP_SNAPSHOT_FIXTURE_BIN_SOURCE = "cc/tests/fixtures/hello/main.cc"
CPP_SNAPSHOT_FIXTURE_HEADER = "cc/tests/fixtures/hello/hello.h"

# Live proof labels: the seed hello plus strict-generation fixtures prove
# the buildable snapshot shape on the seed host.
CPP_SNAPSHOT_LIVE_HELLO_LIB = "//cc/tests/fixtures/hello:hello_lib"
CPP_SNAPSHOT_LIVE_HELLO_BIN = "//cc/tests/fixtures/hello:hello"
CPP_SNAPSHOT_LIVE_HELLO_TEST = "//cc/tests/fixtures/hello:hello_test"
CPP_SNAPSHOT_FIXTURE_CORPUS = "//cc/tests/fixtures/cpp_snapshot:corpus_starlark"
CPP_SNAPSHOT_STRICT_LIB = "//cc/tests/fixtures/strict_generation:strict"
