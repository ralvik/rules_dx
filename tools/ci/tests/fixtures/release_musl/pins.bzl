"""Release evidence pins for Linux static-musl profiles.
Contract: `docs/deploy/release-runbook.md`.
Fixture: `tools/ci/tests/fixtures/release_musl/` via
`bazel run //tools/ci:release_musl_qualification`.
SPDX-2.3 plus SLSA v1 via sbom_demo on both musl profile runners with per-profile uploads; Platform-qualified, no Supported claim.
"""

# SBOM wire profile: SPDX-2.3 plus SLSA v1 via the managed toolchain, subject binds artifact.
SBOM_SPDX = "SPDX-2.3 via //deploy/release:sbom_demo"
SBOM_PREDICATE = "https://slsa.dev/provenance/v1 via //deploy/release:sbom_demo"
SBOM_STATEMENT = "https://in-toto.io/Statement/v1 with subject digest equal to artifact sha256"
SBOM_HERMETIC = "managed Rust toolchain only, no host sha256sum/shasum/python3"
SBOM_FIXTURE = "//deploy/rules:release_demo_archive as the SBOM subject fixture"
SBOM_VERIFY = "bazel test //deploy/release:dx_release_tools_test binds SPDX plus in-toto plus SLSA"

# CI per-profile uploads: sbom-musl-x86_64 plus sbom-musl-arm64 jobs build plus verify plus stage plus upload.
CI_JOB_MUSL_X86_64 = "ci.yml sbom-musl-x86_64 job builds //deploy/release:sbom_demo plus verifies //deploy/release:dx_release_tools_test on ubuntu-latest"
CI_CACHE_MUSL_X86_64 = "bazel-musl-x86_64- cache scope with needs build-musl-x86_64, local-only"
CI_STAGE_MUSL_X86_64 = "stages SPDX-2.3 plus SLSA v1 under RUNNER_TEMP/sbom-musl-x86_64"
CI_UPLOAD_MUSL_X86_64 = "uploads sbom-provenance-linux_x86_64_musl via actions/upload-artifact pinned SHA plus tag"
CI_JOB_MUSL_ARM64 = "ci.yml sbom-musl-arm64 job builds //deploy/release:sbom_demo plus verifies //deploy/release:dx_release_tools_test on ubuntu-24.04-arm"
CI_CACHE_MUSL_ARM64 = "bazel-musl-arm64- cache scope with needs build-musl-arm64, local-only"
CI_STAGE_MUSL_ARM64 = "stages SPDX-2.3 plus SLSA v1 under RUNNER_TEMP/sbom-musl-arm64"
CI_UPLOAD_MUSL_ARM64 = "uploads sbom-provenance-linux_arm64_musl via actions/upload-artifact pinned SHA plus tag"
CI_PERMISSIONS = "contents: read only, no id-token, persist-credentials false, publishes nothing"
CI_SUMMARY_MUSL_X86_64 = "sbom-musl-x86_64 summary with build plus verify plus upload commands"
CI_SUMMARY_MUSL_ARM64 = "sbom-musl-arm64 summary with build plus verify plus upload commands"
CI_SEED_KEPT = "seed sbom job kept with sbom-provenance plus bazel-seed-, no regression"
CI_ARM64_KEPT = "arm64 sbom-arm64 job kept with sbom-provenance-linux_arm64 plus bazel-arm64-, no regression"

# Promotion-checklist cells per profile: platform plus consumer plus coverage plus release.
CELL_PLATFORM = "Platform-qualified static musl under issue #411"
CELL_CONSUMER = "dogfood consumer self-call test-disabled plus musl jobs under issue #408"
CELL_COVERAGE = "musl x86_64 plus musl arm64 coverage cells with no union"
CELL_CLOSURE = "static native closure only, dynamic musl explicitly out of scope"
CELL_TAG = 'module version = "0.0.0" with no v* tags, callers pin reviewed commits'
CELL_VERSION = "--verify-tag everywhere, never creates or pushes tags"
CELL_RELEASE_X86_64 = "linux_x86_64_musl sbom-provenance delivered under issue #804"
CELL_RELEASE_ARM64 = "linux_arm64_musl sbom-provenance delivered under issue #804"

# Attestation stays owner-gated human-run (fork-safe, no CI signing).
ATTESTATION_OWNER_GATED = "Sigstore keyless cosign sign-blob --bundle plus gh attestation create via //deploy/release:signing_demo"
ATTESTATION_NO_CI_SIGN = "CI never signs PR code, fork-safe, publishes nothing"
ATTESTATION_TRUST_ROOT = "tuf-repo-cdn.sigstore.dev plus https://token.actions.githubusercontent.com"

# Rejected substitutes.
REJECTED_DISPATCH_ONLY = "Dry-run only forever is rejected, blocks Supported"
REJECTED_CHECKSUM_ONLY = "checksum-only verification is not publisher-identity proof"
COMPAT_RELEASE_ONLY = "Compatibility: Release only"

NO_SUPPORTED = "no Supported claim"
SEED_ONLY = "qualified under issue #804"
OWNED_GAP = "remaining hosts plus tag cut stay owned gap under issues #805-#807 plus process #808"
