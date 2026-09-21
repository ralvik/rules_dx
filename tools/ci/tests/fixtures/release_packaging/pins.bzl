"""Release packaging pins for artifact-into-releases.
Contract: `docs/deploy/release-runbook.md#packaging`.
Fixture: `tools/ci/tests/fixtures/release_packaging/` via
`bazel run //tools/ci:release_packaging_qualification`.
Seed releasable unit plus SBOM/provenance linkage plus NOTICE signed
alongside the SBOM pair; seed-only, no Supported claim.
"""

# Packaged releasable unit: one build proves the publishing set travels together.
PACKAGING_TARGET = "//deploy/release:release_artifacts builds curator plus binary plus man page plus NOTICE plus SBOM/provenance"
PACKAGING_CURATOR = "//:audit_curator declares licenses.toml bytes into the release"
PACKAGING_BINARY = "//cli/cli:dx seed binary in the releasable unit"
PACKAGING_MAN = "//cli/cli:man_pages ships man/dx.1 in the draft"
PACKAGING_NOTICE = ":notice_demo hermetic notice_bundle with byte-identical rebuilds"
PACKAGING_SBOM = ":sbom_demo SPDX-2.3 plus SLSA v1 with subject digest equal to artifact sha256"

# SBOM/provenance linkage: subject binds exact artifact bytes, verified in dry-run.
SBOM_SPDX = "SPDX-2.3 via //deploy/release:sbom_demo"
SBOM_PREDICATE = "https://slsa.dev/provenance/v1 via //deploy/release:sbom_demo"
SBOM_STATEMENT = "https://in-toto.io/Statement/v1 with subject digest equal to artifact sha256"
SBOM_LINKAGE = "provenance carries the SPDX subject digest (dry-run grep binds both)"
SBOM_HERMETIC = "hermetic Rust toolchain only, no host sha256sum/shasum/python3"
SBOM_VERIFY = "bazel test //deploy/release:dx_release_tools_test binds SPDX plus in-toto plus SLSA"

# NOTICE packaging: validated audit inputs to hermetic bundle, signed with the SBOM pair.
NOTICE_BUNDLE = "hermetic notice_bundle in deploy/release/notice.bzl over audited inventory"
NOTICE_DETERMINISTIC = "deterministic bytes with byte-identical rebuilds"
NOTICE_MISSING = "missing-notice-text fails the action with an actionable diagnostic"
NOTICE_VERIFY = "notice_verify_files plus dx_verify --notice before install"
NOTICE_SIGNED = "signed alongside the SBOM pair via //deploy/release:signing_demo"

# Pipeline wiring: publish dry-run builds plus stages plus verifies the unit, publishes nothing.
PIPELINE_BUILD = "publish-dry-run.yml builds //deploy/release:release_artifacts"
PIPELINE_STAGE = "stages SPDX plus provenance plus NOTICE under RUNNER_TEMP/publish-dry-run"
PIPELINE_SIGNING = "signing dry-run log covers sbom_demo.spdx.json plus sbom_demo.provenance.json plus notice_demo.NOTICE"
PIPELINE_REPORT = "dry-run-report.json records packaging plus sbom plus notice with published False"
PIPELINE_SUMMARY = "step summary records packaging plus SBOM linkage plus NOTICE"
PIPELINE_CLEAN = "dist/release stay git-ignored with staging under RUNNER_TEMP and clean-checkout proof"

# Human-run driver names the packaging order before signing plus draft.
DRIVER_PACKAGING = "release_driver dry run names release_artifacts before notice_demo before signing_demo before github_draft"

# Attestation stays owner-gated human-run (fork-safe, no CI signing).
ATTESTATION_OWNER_GATED = "Sigstore keyless cosign sign-blob --bundle plus gh attestation create via //deploy/release:signing_demo"
ATTESTATION_NO_CI_SIGN = "CI never signs PR code, fork-safe, publishes nothing"
ATTESTATION_TRUST_ROOT = "tuf-repo-cdn.sigstore.dev plus https://token.actions.githubusercontent.com"

# Rejected substitutes.
REJECTED_UNTRACKED = "untracked workspace read rejected, declared Bazel inputs only"
REJECTED_CHECKSUM_ONLY = "checksum-only verification is not publisher-identity proof"
REJECTED_UNSIGNED = "unsigned draft or publish rejected, signing-first"
COMPAT_RELEASE_ONLY = "Compatibility: Release only"

NO_SUPPORTED = "no Supported claim"
SEED_ONLY = "qualified seed-only under issue #813"
OWNED_GAP = "wider matrix plus tag cut stays owned gap under process #808"
