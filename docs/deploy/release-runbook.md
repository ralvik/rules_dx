# Release Runbook

Owner-gated human-run release path (issue #311). No tag, GitHub
Release, registry submission, or image push happens without explicit
owner approval per the release hygiene in
[Contributing](../../CONTRIBUTING.md). The publish dry-run workflow
(`.github/workflows/publish-dry-run.yml`) exercises only dry runs and
publishes nothing either way; GHCR images ship from the separate
`.github/workflows/ghcr.yml` workflow.

## Preconditions

- Clean tree, module version bumped from `0.0.0` to SemVer in a reviewed
  PR (consumers pin reviewed commits until then).
- Tag already pushed to the remote beforehand with owner approval. No
  release program creates or pushes tags (`--verify-tag` everywhere);
  the human-run driver never creates or pushes tags itself.
- OIDC identity for Sigstore keyless signing plus `cosign` and `gh`
  CLIs on the human-run host.
- `SECURITY.md` private-reporting precondition still enabled.

## Matrix

The full matrix is frozen in `deploy/release/matrix.bzl` (seed Linux
x86_64 qualified-built-here; Linux arm64, macOS arm64, macOS x86_64,
Windows x86_64 unqualified until their hosts qualify per
[ADR 0014](../decisions/0014-tested-platform-release-stack.md#required-platforms)).
Build each qualified cell with `bazel build //cli/cli:dx
//cli/cli:dx_standalone`, then `bazel build //deploy/release:all` for
SBOM and provenance. Unqualified cells fail closed; never claim them.

## Steps

1. Dry-run everything first: `RELEASE_DRY_RUN=1
   deploy/release/release.sh <tag>` plus `GH_RELEASE_DRY_RUN=1 bazel
   run //cli/cli:github_draft`, `RELEASE_SIGN_DRY_RUN=1 bazel run
   //deploy/release:signing_demo`, and `BCR_DRY_RUN=1 bazel run
   //deploy/release:bcr_demo`.
2. Archive plus checksum: `bazel run //cli/cli:dx_standalone -- <outdir>`
   (hermetic Python archiver plus hasher, verified before copy).
3. SBOM plus provenance: `bazel build //deploy/release:sbom_demo`
   (SPDX 2.3 JSON plus SLSA v1 in-toto Statement v1, subject digest
   equals artifact sha256; Syft/CycloneDX output verifies through the
   same path when owners adopt it).
4. Signing plus attestation: `bazel run //deploy/release:signing_demo`
   without the dry-run env (Sigstore keyless `cosign sign-blob
   --bundle` on the TUF trust root plus `gh attestation create`).
5. Draft release: `bazel run //cli/cli:github_draft` with the real tag
   (`--draft --verify-tag`; publish by editing the draft on GitHub
   after approval).
6. BCR: `bazel run //deploy/release:bcr_demo` without the dry-run env
   (opens the `source.json` plus integrity plus presubmit PR manually;
   this program never pushes itself).
7. GHCR: dispatch `ghcr.yml` with `approve: true`, then `cosign sign
   <digest>` plus attestation on the same trust root; record quotas and
   update the scaffold digest reference.
8. Verify before install: `deploy/install/dx_verify.sh --binary <dx>
   --bundle <bundle> --identity <workflow-id> --issuer
   https://token.actions.githubusercontent.com [--sbom <sbom>
   --sbom-bundle <sbom-bundle>]`; checksum-only is rejected and failure
   happens before install or exec.

All release outputs stay under `RUNNER_TEMP` or the chosen outdir until
published; `dist/` and `release/` stay git-ignored and the checkout is
left clean.
