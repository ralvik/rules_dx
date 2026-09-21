"""Release matrix pins for the four follow-up cells.
Contract: `docs/deploy/release-runbook.md`.
Fixture: `tools/ci/tests/fixtures/release_matrix/` via
`bazel run //tools/ci:release_matrix_qualification`.
Five cells frozen with the seed plus four follow-ups qualified with per-host evidence; draft-only ceiling plus owner approval unchanged; no Supported claim.
"""

# Matrix shape: five cells frozen in deploy/release/matrix.bzl, seed first.
MATRIX_SEED = "dx-linux-x86_64 qualified-seed-built-here via publish dry-run"
MATRIX_ARM64 = "dx-linux-arm64 qualified-host-evidence with Platform-qualified plus sbom-provenance-linux_arm64"
MATRIX_MACOS_ARM64 = "dx-macos-arm64 qualified-host-evidence with Platform-qualified plus sbom-provenance-macos_arm64"
MATRIX_MACOS_X86_64 = "dx-macos-x86_64 qualified-host-evidence best-effort with Platform-qualified, release evidence exempt non-blocking"
MATRIX_WINDOWS = "dx-windows-x86_64 qualified-host-evidence with Platform-qualified plus sbom-provenance-windows_x86_64"
MATRIX_UNQUALIFIED_EMPTY = "release_matrix_unqualified() empty via qualified- prefix, no unqualified-per-issue-311 remains"

# Per-host platform evidence: Platform-qualified per the support matrix.
CELL_PLATFORM_ARM64 = "Platform-qualified Linux arm64 native under issue #410"
CELL_PLATFORM_MUSL = "Platform-qualified static musl profiles under issue #411"
CELL_PLATFORM_MACOS_ARM64 = "Platform-qualified macOS arm64 native under issue #412"
CELL_PLATFORM_MACOS_X86_64 = "Platform-qualified macOS x86_64 best-effort native under issue #413"
CELL_PLATFORM_WINDOWS = "Platform-qualified Windows x86_64 MSVC-compatible native under issue #414"

# Per-host release evidence: sbom-provenance per host plus best-effort exempt.
CELL_RELEASE_ARM64 = "linux_arm64 sbom-provenance delivered under issue #803"
CELL_RELEASE_MUSL = "linux_x86_64_musl plus linux_arm64_musl sbom-provenance delivered under issue #804"
CELL_RELEASE_MACOS_ARM64 = "macos_arm64 sbom-provenance delivered under issue #805"
CELL_RELEASE_MACOS_X86_64_EXEMPT = "macos x86_64 release evidence exempt as best-effort non-blocking under closed #806 moot"
CELL_RELEASE_WINDOWS = "windows_x86_64 sbom-provenance delivered under issue #807"
CELL_RELEASE_ALL = "every required host landed under closed #803 plus #804 plus #805 plus #807 with best-effort exempt under closed #806 moot, process #808"

# Workflow parity: publish-dry-run carries the same qualified matrix, seed-only build.
WORKFLOW_MATRIX = "publish-dry-run.yml release_matrix carries qualified-seed-built-here plus qualified-host-evidence"
WORKFLOW_SEED_ONLY = "seed binary plus dx_standalone plus release_artifacts built on the seed host, non-seed cells human-run only"
WORKFLOW_NEVER_PUBLISHES = "published False plus submitted False everywhere, RUNNER_TEMP staging plus clean-checkout proof"

# Docs parity: authoring plus runbook plus CLI build record the qualified matrix.
DOCS_AUTHORING = "docs/deploy/authoring.md qualified with per-host evidence under issue #815"
DOCS_RUNBOOK = "docs/deploy/release-runbook.md qualified with per-host evidence under issue #815, no Supported claim"
DOCS_CLI_BUILD = "cli/cli/BUILD.bazel wider matrix is qualified with per-host evidence"

# Ceiling unchanged: draft-only plus owner approval plus unpublishable module.
CEILING_DRAFT = "Draft-only ceiling enforced: draft defaults True, no draft=False, --draft --verify-tag, v0.0.0-dryrun placeholder"
CEILING_APPROVAL = "Owner approval required: approve default false, RELEASE_APPROVE=1 plus pre-pushed tag, never creates tags"
CEILING_UNPUBLISHABLE = 'module version = "0.0.0" with no v* tags, callers pin reviewed commits'

# Rejected substitutes.
REJECTED_UNQUALIFIED = "unqualified-per-issue-311 is rejected, blocks the qualified claim"
REJECTED_CHECKSUM_ONLY = "checksum-only verification is not publisher-identity proof"
COMPAT_RELEASE_ONLY = "Compatibility: Release only"

NO_SUPPORTED = "no Supported claim"
SEED_ONLY = "qualified under issue #815"
OWNED_GAP = "tag cut stays owned gap under process #808"
