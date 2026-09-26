"""Cross routes pins."""

LINUX_X86_64_EXEC = "Linux x86_64"
LINUX_ARM64_EXEC = "Linux arm64"
FIRST_COHORT_TARGETS = [
    "Linux x86_64 glibc",
    "Linux arm64 glibc",
]
FIRST_COHORT_NOTE = "First Linux cross-build cohort"
ALL_CROSS_NOTE = "not a mandate to build every target from every host"

NATIVE_ROWS = [
    "Native x86_64 glibc qualified on ubuntu-latest seed",
    "Native arm64 glibc qualified on ubuntu-24.04-arm",
    "Native macOS arm64 qualified on macos-14",
    "Native Windows x86_64 qualified on windows-latest",
]
NATIVE_BACKEND_NOTE = "pinned upstream toolchains with provisional backends"

CROSS_ARCH_NOTE = "arm64-to-x86_64 cross stays in the first Linux cross-build cohort"
CROSS_ARCH_TARGETS = [
    "Linux x86_64-to-arm64 glibc",
    "Linux arm64-to-x86_64 glibc",
]
CROSS_ARCH_GATE = "matching native target execution with separate cache and remote evidence"

OPTIONAL_EXPANSION_EXEC = "macOS arm64"
OPTIONAL_EXPANSION_TARGETS = [
    "Linux x86_64 and arm64 profiles",
]
OPTIONAL_EXPANSION_NOTE = "Optional first expansion if bounded upstream configuration suffices"
EXPANSION_GATE = "Expand only after the initial cohort passes"

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

EXECUTION_PLATFORM_NOTE = "Record the actual action execution platform, not only the Bazel client or CI runner OS"
TARGET_EXECUTION_NOTE = "Run resulting artifacts on matching native target workers; cross-building alone is insufficient"
DISTINCT_EVIDENCE_NOTE = "Emulation, Rosetta, remote execution and deployment-floor testing are distinct evidence"

LLVM_CI_NOTE = "hermetic-llvm CI uses remote execution in Linux jobs and narrower macOS smoke and coverage tests"
RULES_RS_CI_NOTE = "rules_rs Windows-labelled lane builds remote GNULVM targets, not native MSVC tests or coverage"

CACHE_SHARED = "BuildBuddy shared remote cache, no per-host scopes"
CACHE_SCOPE_NOTE = "shared cache evidence for every claimed row"
REMOTE_EVIDENCE_NOTE = "separate cache and remote evidence for every claimed row"
NATIVE_WEAKENING_NOTE = "Do not weaken required native workflows to obtain a larger cross-build table"

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

NATIVE_ONLY_NOTE = "Native only"
NATIVE_ONLY_PROFILES = "glibc on Linux x86_64 and arm64"
NO_CROSS_HOST_CLAIM = "no Windows or macOS cross-host claim"

CROSS_ROUTES_FIXTURE_CORPUS = "//cc/tests/fixtures/cross_routes:corpus_starlark"
CROSS_ROUTES_SEED_HELLO = "//cc/tests/fixtures/hello:hello"
CROSS_ROUTES_LINUX_CORPUS = "//cc/tests/fixtures/linux_corpus:corpus"
