"""Correlation plus committed-change manifest pins.

Contract: `docs/cli/output-protocol.md#ndjson-envelope`,
`docs/cli/output-protocol.md#mutation`,
`docs/cli/commands/audit-update-bazel.md#dx-update`.
Fixture: `cli/update/tests/fixtures/correlation_manifest/` via
`bazel run //tools/ci:correlation_manifest_qualification`.
"""

# Schema is minor 1.1: correlation plus manifest, forward-compatible.
SCHEMA_MAJOR = 1
SCHEMA_MINOR = 1

# Correlation shape: 1-128 ASCII chars from [A-Za-z0-9/_:.-].
CORRELATION_RUN = "run://app:bin"
CORRELATION_UPDATE_CARGO = "update:cargo"
CORRELATION_UPDATE_NPM = "update:npm"
CORRELATION_UMBRELLA = "check/format"

# Run multirun operations carry run:<target> in execution order.
RUN_CORRELATION_PREFIX = "run:"
RUN_LINE_ORDER_AUTHORITATIVE = True

# Update per-set reports carry update:<set>; file pairs share it.
UPDATE_CORRELATION_PREFIX = "update:"
UPDATE_FILE_ORDER_PATH_BYTES = True

# Manifest validation: owning set known, paths are normalized declared
# file locks, unique in path-byte order, modify carries a 64-char
# lowercase hex digest with pre-commit length, create omits digest with
# zero length, directory hubs never validate.
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

# Projection: modify is one 0..old_len spanning replacement, create is
# one 0..0 insertion, empty or absent manifests emit nothing.
PROJECTION_MODIFY_SPAN = "0..old_len"
PROJECTION_CREATE_SPAN = "0..0"
PROJECTION_EMPTY_EMITS_NOTHING = True

# Version compat: v1.0 consumers ignore correlation, omitted correlation
# preserves v1.0 wire shape, 1.0 witnesses decode under 1.1 by major-only
# enforcement, empty manifests preserve the v1.0 per-set contract.
COMPAT_V10_IGNORES_CORRELATION = True
COMPAT_OMITTED_PRESERVES_V10 = True
COMPAT_MAJOR_ONLY_WITNESS = True

# Rejected inference routes (protocol already forbids them).
REJECTED_GIT_SCAN = "git scan rejected"
REJECTED_BUILD_PARSE = "BUILD parse rejected"
REJECTED_RERUN = "rerun rejected"

# Honesty lines (never pinned as supported here).
NO_SUPPORTED = "no Supported claim"
SEED_ONLY = "qualified seed-only under issue #811"
