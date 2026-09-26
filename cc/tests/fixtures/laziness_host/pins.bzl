BZLMOND_BOUNDARY = [
    "version-resolution cost",
    "MODULE.bazel.lock size is version-resolution metadata",
    "extension Starlark that reads checked-in locks and manifests",
    "fixed-manifest plus package-index inputs",
    "deferred EULA failure never proves laziness",
    "extension evaluation already fetches manifests",
]

CONSUMER_HOSTS = (
    "linux_x86_64",
    "linux_arm64",
    "macos_arm64",
    "windows_x86_64",
)
CONSUMER_ADOPT = (
    "examples/adopt-rust",
    "examples/adopt-python",
    "examples/adopt-js-ts",
    "examples/adopt-go",
    "examples/adopt-cpp",
    "examples/adopt-java",
    "examples/adopt-kotlin",
    "examples/adopt-scala",
    "examples/adopt-csharp",
    "examples/adopt-fsharp",
)
NO_FETCH_CONTRACT = [
    "adding unused foundation pulls no EULA payload",
    "adding unused foundation pulls no SDK payload",
    "unrelated workflows fetch no Windows payloads",
    "missing acceptance leaves unrelated workflows green",
]

OVERRIDE_MATRIX = [
    "default graph stays lazy",
    "root-override graph stays lazy",
    "no special CLI policy for overrides",
    "no warning on graphs that differ from release defaults",
]

SYMLINK_PROBE = "probe_symlink"
SYMLINK_ERROR = "SymlinkUnsupported"
SYMLINK_GUIDANCE = "enable Developer Mode or grant SeBackupPrivilege"
SYMLINK_FAIL_BEFORE_MUTATION = "failure before mutation"
SYMLINK_ENTERPRISE_UNSUPPORTED = "hosts that cannot grant it are unsupported"
SYMLINK_NO_FALLBACK = "no junction or copy fallback"

FALLBACK_REJECTED = [
    "junction fallback rejected",
    "launcher fallback rejected",
    "copy fallback rejected",
    "fallback breaks atomic replacement and ownership validation",
]

PROVISIONAL_BACKENDS = [
    "hermetic-llvm Apple-SDK provisional",
    "toolchains_msvc clang-cl/Microsoft-STL provisional",
]
BACKEND_QUALIFIER = "provisional-backend exception"
BACKEND_NO_SUPPORTED = "no Supported claim"
BACKEND_OWNED_GAPS = "issues pinned plus release evidence pinned"

LAZINESS_HOST_FIXTURE_CORPUS = "//cc/tests/fixtures/laziness_host:corpus_starlark"
LAZINESS_HOST_LIVE_HELLO = "//cc/tests/fixtures/hello:hello"
LAZINESS_HOST_SLICES = [
    "examples_laziness_query",
    "examples_laziness_aquery",
    "examples_laziness_runtime",
]
