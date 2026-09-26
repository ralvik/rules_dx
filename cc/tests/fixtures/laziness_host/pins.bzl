"""Per-host strict-laziness plus symlink-privilege plus backend-adoption pins.

Contract: `docs/native-toolchains.md#qualification-questions-and-delivery`.
Fixture: `cc/tests/fixtures/laziness_host/` via
`bazel run //tools/ci:laziness_host_qualification`.
"""

# Bzlmod eager-resolution boundary: version-resolution cost, not payload.
BZLMOND_BOUNDARY = [
    "version-resolution cost",
    "MODULE.bazel.lock size is version-resolution metadata",
    "extension Starlark that reads checked-in locks and manifests",
    "fixed-manifest plus package-index inputs",
    "deferred EULA failure never proves laziness",
    "extension evaluation already fetches manifests",
]

# Per-host no-fetch in the consumer graph: adding an unused foundation
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

# Override matrix: default plus root-override graphs stay lazy. Root
# overrides need no special CLI policy or warning path; ordinary commands
# do not warn on a graph that differs from release defaults.
OVERRIDE_MATRIX = [
    "default graph stays lazy",
    "root-override graph stays lazy",
    "no special CLI policy for overrides",
    "no warning on graphs that differ from release defaults",
]

# Symlink-privilege UX: probe before mutation with an actionable error.
# Windows needs Developer Mode or SeBackupPrivilege; hosts that cannot
# grant it are unsupported, never a fallback. Enterprise refusal is that
# unsupported case, measured as a clean refusal before mutation.
SYMLINK_PROBE = "probe_symlink"
SYMLINK_ERROR = "SymlinkUnsupported"
SYMLINK_GUIDANCE = "enable Developer Mode or grant SeBackupPrivilege"
SYMLINK_FAIL_BEFORE_MUTATION = "failure before mutation"
SYMLINK_ENTERPRISE_UNSUPPORTED = "hosts that cannot grant it are unsupported"
SYMLINK_NO_FALLBACK = "no junction or copy fallback"

# Fallback evaluation: junction, launcher, and copy modes were evaluated
# and rejected for persistent managed state because fallback breaks atomic
# replacement and ownership validation. The ephemeral quality-adapter
# scratch copy is a different domain and does not apply here.
FALLBACK_REJECTED = [
    "junction fallback rejected",
    "launcher fallback rejected",
    "copy fallback rejected",
    "fallback breaks atomic replacement and ownership validation",
]

# Backend adoption: hermetic backends stay provisional with immutable lazy
PROVISIONAL_BACKENDS = [
    "hermetic-llvm Apple-SDK provisional",
    "toolchains_msvc clang-cl/Microsoft-STL provisional",
]
BACKEND_QUALIFIER = "provisional-backend exception"
BACKEND_NO_SUPPORTED = "no Supported claim"
BACKEND_OWNED_GAPS = "issues #494-#505 plus release evidence #803-#807"

# Live proof labels: seed hello builds without acceptance; the laziness
# slices plus corpus target prove the fixture is wired.
LAZINESS_HOST_FIXTURE_CORPUS = "//cc/tests/fixtures/laziness_host:corpus_starlark"
LAZINESS_HOST_LIVE_HELLO = "//cc/tests/fixtures/hello:hello"
LAZINESS_HOST_SLICES = [
    "examples_laziness_query",
    "examples_laziness_aquery",
    "examples_laziness_runtime",
]
