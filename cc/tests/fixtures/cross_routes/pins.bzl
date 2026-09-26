"""Cross routes pins.

Contract: `docs/native-toolchains.md#profiles-and-cross-builds`,
`docs/native-toolchains.md#qualification-questions-and-delivery`,
`docs/decisions/0014-tested-platform-release-stack.md#decision`.
Fixture: `cc/tests/fixtures/cross_routes/` via
`bazel run //tools/ci:cross_routes_qualification`.
"""

# First Linux cross-build cohort: execution platforms plus targets.
# Linux x86_64 builds Linux x86_64 and arm64, each glibc and static
# musl; Linux arm64 builds Linux arm64 and x86_64, each glibc and
# static musl. This is the first Linux cross-build cohort, not an
# all-cross mandate.
LINUX_X86_64_EXEC = "Linux x86_64"
LINUX_ARM64_EXEC = "Linux arm64"
FIRST_COHORT_TARGETS = [
    "Linux x86_64 glibc",
    "Linux arm64 glibc",
    "Linux x86_64 static musl",
    "Linux arm64 static musl",
]
FIRST_COHORT_NOTE = "First Linux cross-build cohort"
ALL_CROSS_NOTE = "not a mandate to build every target from every host"

# Linux same-arch musl closures Rust musl
# std via extra_target_triples with exec-platform tools for build
# scripts and proc macros and target musl libs for apps; CI
# cross-builds from Linux runners with the shared BuildBuddy remote
# cache (per-host scopes deleted) plus per-cell coverage for both musl
# cells with no union.
MUSL_QUALIFIED_NOTE = "x86_64 static musl qualified under issue #411"
MUSL_CI_RUNNER_X86_64 = "ubuntu-latest with shared BuildBuddy cache"
MUSL_CI_RUNNER_ARM64 = "ubuntu-24.04-arm with shared BuildBuddy cache"
MUSL_EXEC_SEPARATION = "exec-platform tools for build scripts and proc macros with target musl libs for apps"

# Native rows qualified, each with its CI
# runner on the pinned upstream toolchains (shared remote cache only).
NATIVE_ROWS = [
    "Native x86_64 glibc qualified under issue #410 on ubuntu-latest seed",
    "Native arm64 glibc qualified under issue #410 on ubuntu-24.04-arm",
    "Native macOS arm64 qualified under issue #412 on macos-14",
    "Native Windows x86_64 qualified under issue #414 on windows-latest",
]
NATIVE_BACKEND_NOTE = "pinned upstream toolchains with provisional backends"

# Linux cross-arch rows stay in the first cohort: arm64-to-x86_64
# cross plus x86_64-to-arm64 cross require matching native target
# execution plus separate cache and remote evidence before any claim.
# Cross-building alone is insufficient.
CROSS_ARCH_NOTE = "arm64-to-x86_64 cross stays in the first Linux cross-build cohort"
CROSS_ARCH_TARGETS = [
    "Linux x86_64-to-arm64 glibc plus static musl",
    "Linux arm64-to-x86_64 glibc plus static musl",
]
CROSS_ARCH_GATE = "matching native target execution with separate cache and remote evidence"

# Optional first expansion: macOS arm64 builds Linux x86_64 and arm64
# profiles only if bounded upstream configuration suffices. Expand only
# after the initial cohort passes. macOS x86_64 is Not planned per #976.
OPTIONAL_EXPANSION_EXEC = "macOS arm64"
OPTIONAL_EXPANSION_TARGETS = [
    "Linux x86_64 and arm64 profiles",
]
OPTIONAL_EXPANSION_NOTE = "Optional first expansion if bounded upstream configuration suffices"
EXPANSION_GATE = "Expand only after the initial cohort passes"

# Excluded routes: outside the initial qualification cohort, not
# claims of impossibility or permanent product exclusions. Do not
# require these rows to complete this cohort.
EXCLUDED_ROUTES = [
    "Linux-to-Windows",
    "macOS-to-Windows",
    "Linux-to-macOS",
    "Windows-to-macOS",
    "Windows-to-Linux",
    "Windows arm64",
]
EXCLUDED_NOTE = "do not require Linux/macOS-to-Windows, Linux/Windows-to-macOS, Windows-to-Linux, or Windows arm64 to complete this cohort"
EXCLUSION_SCOPE = "outside the initial qualification cohort, not claims of impossibility"

# Execution evidence: record the actual action execution platform,
# not only the Bazel client or CI runner OS. Run resulting artifacts
# on matching native target workers; cross-building alone is
# insufficient. Emulation, Rosetta, remote execution and
# deployment-floor testing are distinct evidence.
EXECUTION_PLATFORM_NOTE = "Record the actual action execution platform, not only the Bazel client or CI runner OS"
TARGET_EXECUTION_NOTE = "Run resulting artifacts on matching native target workers; cross-building alone is insufficient"
DISTINCT_EVIDENCE_NOTE = "Emulation, Rosetta, remote execution and deployment-floor testing are distinct evidence"

# Inspected upstream CI distinction: hermetic-llvm Linux jobs use
# remote execution with narrower macOS smoke and coverage tests;
# rules_rs Windows-labelled lane builds remote GNULVM targets, not
# native MSVC tests or coverage.
LLVM_CI_NOTE = "hermetic-llvm CI uses remote execution in Linux jobs and narrower macOS smoke and coverage tests"
RULES_RS_CI_NOTE = "rules_rs Windows-labelled lane builds remote GNULVM targets, not native MSVC tests or coverage"

# Cache plus remote separation: every claimed row carries the shared
# BuildBuddy remote cache (the per-host disk-cache scopes are deleted)
# plus remote evidence; per-cell coverage never unions across cells.
# Required native workflows are never weakened to obtain a larger
# cross-build table.
CACHE_SHARED = "BuildBuddy shared remote cache, no per-host scopes"
CACHE_SCOPE_NOTE = "shared cache evidence for every claimed row"
REMOTE_EVIDENCE_NOTE = "separate cache and remote evidence for every claimed row"
NATIVE_WEAKENING_NOTE = "Do not weaken required native workflows to obtain a larger cross-build table"

# Rejected substitutes per the issue alternatives: all-cross mandate
# plus availability-style shortcuts plus host-state inference.
REJECTED_ALTERNATIVES = [
    "all-cross mandate",
    "every target from every host",
    "cross-building alone as execution proof",
    "emulation as native target execution",
    "Rosetta as native target execution",
    "remote execution as target execution",
    "compiler-target availability as route proof",
    "cross-host Windows inference",
    "weakened native workflows for a larger table",
]

# Native-only boundary: glibc plus static musl on Linux
# x86_64 and arm64; dynamic musl explicitly out of scope with no
# cell; no Windows or macOS cross-host claim here.
NATIVE_ONLY_NOTE = "Native only"
NATIVE_ONLY_PROFILES = "glibc plus static musl on Linux x86_64 and arm64"
DYNAMIC_MUSL_NOTE = "dynamic musl explicitly out of scope"
NO_CROSS_HOST_CLAIM = "no Windows or macOS cross-host claim"

# Live proof labels: the seed hello plus the linux corpus corpus
# prove the Linux profiles stay green; the musl toolchain query
# proves Rust musl std still resolves; the fixture corpus target
# proves this fixture is wired.
CROSS_ROUTES_FIXTURE_CORPUS = "//cc/tests/fixtures/cross_routes:corpus_starlark"
CROSS_ROUTES_SEED_HELLO = "//cc/tests/fixtures/hello:hello"
CROSS_ROUTES_LINUX_CORPUS = "//cc/tests/fixtures/linux_corpus:corpus"
CROSS_ROUTES_MUSL_TOOLCHAINS = "@rust_toolchains//..."
