"""SBOM plus provenance upload pins.
Contract: `docs/deploy/release-runbook.md`.
Fixture: `tools/ci/tests/fixtures/sbom_upload/` via
`bazel run //tools/ci:sbom_upload_qualification`.
SPDX-2.3 plus SLSA v1 via sbom_demo on CI with upload; seed-only, no Supported claim.
"""

# SBOM wire profile: SPDX-2.3 plus SLSA v1 via the managed toolchain, subject binds artifact.
SBOM_SPDX = "SPDX-2.3 via //deploy/release:sbom_demo"
SBOM_PREDICATE = "https://slsa.dev/provenance/v1 via //deploy/release:sbom_demo"
SBOM_STATEMENT = "https://in-toto.io/Statement/v1 with subject digest equal to artifact sha256"
SBOM_HERMETIC = "managed Python toolchain only, no host sha256sum/shasum/python3"
SBOM_FIXTURE = "//deploy/rules:release_demo_archive as the SBOM subject fixture"
SBOM_VERIFY = "bazel test //deploy/release:sbom_demo_verify binds SPDX plus in-toto plus SLSA"

# CI upload: seed-host sbom job builds plus verifies plus stages plus uploads.
CI_JOB = "ci.yml sbom job builds //deploy/release:sbom_demo plus verifies //deploy/release:sbom_demo_verify"
CI_STAGE = "stages SPDX-2.3 plus SLSA v1 under RUNNER_TEMP/sbom"
CI_UPLOAD = "uploads sbom-provenance via actions/upload-artifact pinned SHA plus tag"
CI_PERMISSIONS = "contents: read only, no id-token, persist-credentials false, publishes nothing"
CI_SUMMARY = "sbom summary with build plus verify plus upload commands"

# Attestation stays owner-gated human-run (fork-safe, no CI signing).
ATTESTATION_OWNER_GATED = "Sigstore keyless cosign sign-blob --bundle plus gh attestation create via //deploy/release:signing_demo"
ATTESTATION_NO_CI_SIGN = "CI never signs PR code, fork-safe, publishes nothing"
ATTESTATION_TRUST_ROOT = "tuf-repo-cdn.sigstore.dev plus https://token.actions.githubusercontent.com"

# Rejected substitutes.
REJECTED_DISPATCH_ONLY = "Dry-run only forever is rejected, blocks Supported"
REJECTED_CHECKSUM_ONLY = "checksum-only verification is not publisher-identity proof"
COMPAT_RELEASE_ONLY = "Compatibility: Release only"

NO_SUPPORTED = "no Supported claim"
SEED_ONLY = "qualified seed-only under issue #612"
OWNED_GAP = "attestation on CI stays owned gap until owner-gated human-run qualifies"
