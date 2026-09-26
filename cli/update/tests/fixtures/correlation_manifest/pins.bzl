SCHEMA_MAJOR = 1
SCHEMA_MINOR = 1

CORRELATION_RUN = "run://app:bin"
CORRELATION_UPDATE_CARGO = "update:cargo"
CORRELATION_UPDATE_NPM = "update:npm"
CORRELATION_UMBRELLA = "check/format"

RUN_CORRELATION_PREFIX = "run:"
RUN_LINE_ORDER_AUTHORITATIVE = True

UPDATE_CORRELATION_PREFIX = "update:"
UPDATE_FILE_ORDER_PATH_BYTES = True

MANIFEST_SETS = [
    "cargo",
    "go",
    "maven",
    "npm",
    "nuget",
]
MANIFEST_MODIFY_LOCKS = [
    "cargo-bazel-lock.json",
    "rust/tests/fixtures/hello/Cargo.lock",
    "pnpm-lock.yaml",
    "third_party/jvm/maven_install.json",
    "third_party/dotnet/paket.lock",
]
MANIFEST_REJECTS_DIR_HUB = "third_party/dotnet/deps"

PROJECTION_MODIFY_SPAN = "0..old_len"
PROJECTION_CREATE_SPAN = "0..0"
PROJECTION_EMPTY_EMITS_NOTHING = True

COMPAT_V10_IGNORES_CORRELATION = True
COMPAT_OMITTED_PRESERVES_V10 = True
COMPAT_MAJOR_ONLY_WITNESS = True

REJECTED_GIT_SCAN = "git scan rejected"
REJECTED_BUILD_PARSE = "BUILD parse rejected"
REJECTED_RERUN = "rerun rejected"

NO_SUPPORTED = "no Supported claim"
SEED_ONLY = "qualified seed-only"
