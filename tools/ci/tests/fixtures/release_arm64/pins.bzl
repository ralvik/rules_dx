"""Release evidence pins for Linux arm64.
Contract: `docs/deploy/release-runbook.md`.
Fixture: `tools/ci/tests/fixtures/release_arm64/` via
`bazel run //tools/ci:release_arm64_qualification`.
SPDX-2.3 plus SLSA v1 via sbom_demo on the arm64 native runner with per-host upload; Platform-qualified, no Supported claim.
"""

# SBOM wire profile: SPDX-2.3 plus SLSA v1 via the managed toolchain, subject binds artifact.
SBOM_SPDX = "SPDX-2.3 via //deploy/release:sbom_demo"
SBOM_PREDICATE = "https://slsa.dev/provenance/v1 via //deploy/release:sbom_demo"
SBOM_STATEMENT = "https://in-toto.io/Statement/v1 with subject digest equal to artifact sha256"
SBOM_HERMETIC = "managed Rust toolchain only, no host sha256sum/shasum/python3"
SBOM_FIXTURE = "//deploy/rules:release_demo_archive as the SBOM subject fixture"
SBOM_VERIFY = "bazel test //deploy/release:dx_release_tools_test binds SPDX plus in-toto plus SLSA"

# CI per-host upload: sbom-arm64 job builds plus verifies plus stages plus uploads.
CI_JOB_ARM64 = "ci.yml sbom-arm64 job builds //deploy/release:sbom_demo plus verifies //deploy/release:dx_release_tools_test on ubuntu-24.04-arm"
CI_CACHE_ARM64 = "bazel-arm64- cache scope with needs build-arm64, local-only"
CI_STAGE_ARM64 = "stages SPDX-2.3 plus SLSA v1 under RUNNER_TEMP/sbom-arm64"
CI_UPLOAD_ARM64 = "uploads sbom-provenance-linux_arm64 via actions/upload-artifact pinned SHA plus tag"
CI_PERMISSIONS = "contents: read only, no id-token, persist-credentials false, publishes nothing"
CI_SUMMARY_ARM64 = "sbom-arm64 summary with build plus verify plus upload commands"
CI_SEED_KEPT = "seed sbom job kept with sbom-provenance plus bazel-seed-, no regression"

# Promotion-checklist cells for this host: platform plus consumer plus release.
CELL_PLATFORM = "Platform-qualified Linux arm64 native under issue #410"
CELL_CONSUMER = "dogfood consumer self-call covers linux_arm64 with test disabled under issue #408"
CELL_TAG = 'module version = "0.0.0" with no v* tags, callers pin reviewed commits'
CELL_VERSION = "--verify-tag everywhere, never creates or pushes tags"
CELL_RELEASE = "linux_arm64 sbom-provenance delivered under issue #803"

# Attestation stays owner-gated human-run (fork-safe, no CI signing).
ATTESTATION_OWNER_GATED = "Sigstore keyless cosign sign-blob --bundle plus gh attestation create via //deploy/release:signing_demo"
ATTESTATION_NO_CI_SIGN = "CI never signs PR code, fork-safe, publishes nothing"
ATTESTATION_TRUST_ROOT = "tuf-repo-cdn.sigstore.dev plus https://token.actions.githubusercontent.com"

# Rejected substitutes.
REJECTED_DISPATCH_ONLY = "Dry-run only forever is rejected, blocks Supported"
REJECTED_CHECKSUM_ONLY = "checksum-only verification is not publisher-identity proof"
COMPAT_RELEASE_ONLY = "Compatibility: Release only"

NO_SUPPORTED = "no Supported claim"
SEED_ONLY = "qualified under issue #803"
OWNED_GAP = "remaining hosts plus tag cut stay owned gap under issues #804-#807 plus process #808"
