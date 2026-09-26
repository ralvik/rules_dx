"""Release evidence pins for Windows x86_64 MSVC-compatible.
Contract: `docs/deploy/release-runbook.md`.
Fixture: `tools/ci/tests/fixtures/release_windows/` via
`bazel run //tools/ci:release_windows_qualification`.
SPDX-2.3 plus SLSA v1 via sbom_demo on the windows native runner with no CI
upload; Platform-qualified, no Supported claim.
"""

# SBOM wire profile: SPDX-2.3 plus SLSA v1 via the managed toolchain, subject binds artifact.
SBOM_SPDX = "SPDX-2.3 via //deploy/release:sbom_demo"
SBOM_PREDICATE = "https://slsa.dev/provenance/v1 via //deploy/release:sbom_demo"
SBOM_STATEMENT = "https://in-toto.io/Statement/v1 with subject digest equal to artifact sha256"
SBOM_HERMETIC = "managed Rust toolchain only, no host sha256sum/shasum/python3"
SBOM_FIXTURE = "//deploy/rules:release_demo_archive as the SBOM subject fixture"
SBOM_VERIFY = "bazel test //deploy/release:dx_release_tools_test binds SPDX plus in-toto plus SLSA"

# CI surface removed: no sbom-windows-x86_64 job, no upload, no stage, no cache scope.
CI_NO_JOB = "ci.yml carries no sbom-windows-x86_64 job; SBOM stays a local target under #807"
CI_NO_CACHE_SCOPE = "no per-host cache scope; disk cache deleted, BuildBuddy remote cache only"
CI_NO_UPLOAD = "no upload-artifact, no RUNNER_TEMP stage; CI publishes nothing under #807"
CI_PERMISSIONS = "contents: read only, no id-token, persist-credentials false, publishes nothing"
CI_SEED_REMOVED = "seed sbom job removed with the sbom job deletion, no regression"
CI_ARM64_REMOVED = "arm64 sbom-arm64 job removed with the sbom job deletion, no regression"
CI_MUSL_REMOVED = "musl sbom-musl jobs removed with the sbom job deletion, no regression"

# Promotion-checklist cells for this host: platform plus consumer plus coverage plus release.
CELL_PLATFORM = "Platform-qualified Windows x86_64 MSVC-compatible native under issue #414"
CELL_CONSUMER = "dogfood self-call covers windows_x86_64 with full dx test plus dx coverage"
CELL_COVERAGE = "windows x86_64 coverage cell with no union"
CELL_EULA = "explicit EULA acceptance required never automatic, installed Build Tools fallback never approved; EULA acknowledgement UX qualified seed-only under issue #818 (cc/tests/fixtures/windows_eula/pins.bzl via bazel run //tools/ci:windows_eula_qualification)"
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
