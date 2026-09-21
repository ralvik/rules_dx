"""Release evidence pins for Windows x86_64 MSVC-compatible.
Contract: `docs/deploy/release-runbook.md`.
Fixture: `tools/ci/tests/fixtures/release_windows/` via
`bazel run //tools/ci:release_windows_qualification`.
SPDX-2.3 plus SLSA v1 via sbom_demo on the windows native runner with per-host upload; Platform-qualified, no Supported claim.
"""

# SBOM wire profile: SPDX-2.3 plus SLSA v1 via the managed toolchain, subject binds artifact.
SBOM_SPDX = "SPDX-2.3 via //deploy/release:sbom_demo"
SBOM_PREDICATE = "https://slsa.dev/provenance/v1 via //deploy/release:sbom_demo"
SBOM_STATEMENT = "https://in-toto.io/Statement/v1 with subject digest equal to artifact sha256"
SBOM_HERMETIC = "managed Rust toolchain only, no host sha256sum/shasum/python3"
SBOM_FIXTURE = "//deploy/rules:release_demo_archive as the SBOM subject fixture"
SBOM_VERIFY = "bazel test //deploy/release:dx_release_tools_test binds SPDX plus in-toto plus SLSA"

# CI per-host upload: sbom-windows-x86_64 job builds plus verifies plus stages plus uploads.
CI_JOB_WINDOWS = "ci.yml sbom-windows-x86_64 job builds //deploy/release:sbom_demo plus verifies //deploy/release:dx_release_tools_test on windows-latest"
CI_CACHE_WINDOWS = "bazel-windows-x86_64- cache scope with needs build-windows-x86_64, shell bash, local-only"
CI_STAGE_WINDOWS = "stages SPDX-2.3 plus SLSA v1 under RUNNER_TEMP/sbom-windows-x86_64"
CI_UPLOAD_WINDOWS = "uploads sbom-provenance-windows_x86_64 via actions/upload-artifact pinned SHA plus tag"
CI_PERMISSIONS = "contents: read only, no id-token, persist-credentials false, publishes nothing"
CI_SUMMARY_WINDOWS = "sbom-windows-x86_64 summary with build plus verify plus upload commands"
CI_SEED_KEPT = "seed sbom job kept with sbom-provenance plus bazel-seed-, no regression"
CI_ARM64_KEPT = "arm64 sbom-arm64 job kept with sbom-provenance-linux_arm64 plus bazel-arm64-, no regression"
CI_MUSL_KEPT = "musl sbom-musl jobs kept with sbom-provenance-linux_x86_64_musl plus sbom-provenance-linux_arm64_musl, no regression"

# Promotion-checklist cells for this host: platform plus consumer plus coverage plus release.
CELL_PLATFORM = "Platform-qualified Windows x86_64 MSVC-compatible native under issue #414"
CELL_CONSUMER = "dogfood consumer self-call covers windows_x86_64 with test disabled under issue #408"
CELL_COVERAGE = "windows x86_64 coverage cell with no union"
CELL_EULA = "explicit EULA acceptance required never automatic, installed Build Tools fallback never approved"
CELL_INTEROP = "prebuilt-MSVC interop fixtures with explicit STL/CRT/linker/library combos incl mixed Rust/C/C++"
CELL_CORPUS = "linux corpus qualified seed-only under issue #499"
CELL_TAG = 'module version = "0.0.0" with no v* tags, callers pin reviewed commits'
CELL_VERSION = "--verify-tag everywhere, never creates or pushes tags"
CELL_RELEASE = "windows_x86_64 sbom-provenance delivered under issue #807"

# Attestation stays owner-gated human-run (fork-safe, no CI signing).
ATTESTATION_OWNER_GATED = "Sigstore keyless cosign sign-blob --bundle plus gh attestation create via //deploy/release:signing_demo"
ATTESTATION_NO_CI_SIGN = "CI never signs PR code, fork-safe, publishes nothing"
ATTESTATION_TRUST_ROOT = "tuf-repo-cdn.sigstore.dev plus https://token.actions.githubusercontent.com"

# Rejected substitutes.
REJECTED_DISPATCH_ONLY = "Dry-run only forever is rejected, blocks Supported"
REJECTED_CHECKSUM_ONLY = "checksum-only verification is not publisher-identity proof"
COMPAT_RELEASE_ONLY = "Compatibility: Release only"

NO_SUPPORTED = "no Supported claim"
SEED_ONLY = "qualified under issue #807"
OWNED_GAP = "tag cut stays owned gap under process #808"
