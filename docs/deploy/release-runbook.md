# Release Runbook

Owner-gated human-run release path (issue #458, live successor to closed
#311 for the human-run path). No tag, GitHub
Release, registry submission, or image push happens without explicit
owner approval per the release hygiene in
[Contributing](../../CONTRIBUTING.md). The publish dry-run workflow
(`.github/workflows/publish-dry-run.yml`) exercises only dry runs and
publishes nothing either way; GHCR images ship from the separate
`.github/workflows/ghcr.yml` workflow.

## Preconditions

- Clean tree, module version bumped from `0.0.0` to SemVer in a reviewed
  PR with owner approval (issue #931; enforced by `release_policy` plus `release_hygiene`)
  (consumers pin reviewed commits until then).
- Tag already pushed to the remote beforehand with owner approval. No
  release program creates or pushes tags (`--verify-tag` everywhere);
  the human-run driver never creates or pushes tags itself.
- OIDC identity for Sigstore keyless signing plus `cosign` and `gh`
  CLIs on the human-run host.
- `SECURITY.md` private-reporting precondition still enabled.

## Matrix

The full matrix is frozen in `deploy/release/matrix.bzl` (seed Linux
x86_64 qualified-built-here; Linux arm64, macOS arm64, Windows x86_64
qualified with per-host evidence under issue #815 per
[ADR 0014](../decisions/0014-tested-platform-release-stack.md#required-platforms):
Platform-qualified plus per-host `sbom-provenance` release evidence; macOS
x86_64 Not planned per #976 with no cell).
Build each qualified cell with `bazel build //cli/cli:dx
//cli/cli:dx_standalone //cli/cli:man_pages`, then `bazel build
//deploy/release:release_artifacts` for the packaged releasable unit
(curator plus binary plus man page plus NOTICE plus SBOM/provenance).
No cell is `Supported` yet; what
evidence promotes a cell to `Supported` (tag hygiene, versioning, platform
plus consumer plus release evidence) is owned by the
[support matrix](../product/support-matrix.md#status-lifecycle), tracked
under issue #611.

## Packaging

Artifact-into-releases packaging (issue #813): validated audit inputs ride
the release as declared Bazel inputs, not untracked workspace reads. The
committed `licenses.toml` bytes ride `//:audit_curator` (per
[audit-update-bazel](../cli/commands/audit-update-bazel.md#dx-audit)), and the
per-package `[[inventory]]` words feed hermetic `notice_bundle` in
`deploy/release/notice.bzl` (deterministic bytes, byte-identical rebuilds,
`missing-notice-text` fails). SBOM plus provenance ride
`//deploy/release:sbom_demo` (SPDX 2.3 JSON plus SLSA v1 in-toto Statement
v1, subject digest equals artifact sha256). Signing via
`//deploy/release:signing_demo` binds the SBOM pair plus the NOTICE bundle
together (signed alongside the SBOM pair, Sigstore keyless plus attestation),
so nothing ships unsigned and
nothing ships without its evidence. One target proves the set travels
together: `bazel build //deploy/release:release_artifacts` (curator plus
`//cli/cli:dx` plus `//cli/cli:man_pages` plus `:notice_demo` plus
`:sbom_demo`). The publish dry-run workflow stages plus verifies this unit
(SPDX plus SLSA subject linkage, NOTICE header, signing log covers SBOM pair
plus NOTICE) and publishes nothing. Pinned by `bazel test
//deploy/release:all` plus `bazel run
//tools/ci:release_packaging_qualification`.

## Steps

Signing-first: step 5 signs plus attests before step 6 opens the draft
release, so nothing is drafted or published unsigned. Pinned by
`bazel test //deploy/release:all`.

Signing stack + distribution qualified under issue #459 (live successor
to closed #311/#26/#78 for signing + distribution; decision: keep
Sigstore keyless `cosign sign-blob --bundle` + GitHub attestations, no
stack change): cosign v2.4.1 pinned per `deploy/release/signing.bzl`
`SIGNING_COSIGN_VERSION` plus linux-amd64 sha per
`SIGNING_COSIGN_SHA256_LINUX_AMD64` (checksum-verified fetch per `ghcr.yml`),
bundle media type `application/vnd.dev.sigstore.bundle.v0.3+json` per
`SIGNING_BUNDLE_MEDIA_TYPE`, TUF trust root
`https://tuf-repo-cdn.sigstore.dev` plus GitHub OIDC issuer, SPDX 2.3
plus SLSA v1 provenance via `//deploy/release:sbom_demo` with subject
digest equal to artifact sha256, distribution to BCR (`rules_dx` module)
plus GitHub Releases (`dx` binaries) with GHCR via the separate
`ghcr.yml` route. Pinned by `bazel test //deploy/release:all` plus
`bazel run //tools/ci:signing_distribution_qualification`.

 1. Dry-run everything first: `RELEASE_DRY_RUN=1 bazel run
    //deploy/release:release_driver -- <tag>` plus `GH_RELEASE_DRY_RUN=1
    bazel run //cli/cli:github_draft`, `RELEASE_SIGN_DRY_RUN=1 bazel run
    //deploy/release:signing_demo`, and `BCR_DRY_RUN=1 bazel run
    //deploy/release:bcr_demo`.
 2. Archive plus checksum: `bazel run //cli/cli:dx_standalone -- <outdir>`
    (hermetic Rust archiver plus hasher, verified before copy).
 2b. Packaging: `bazel build //deploy/release:release_artifacts` (see
     [Packaging](#packaging): curator plus binary plus man page plus NOTICE
     plus SBOM/provenance as one releasable unit, publishes nothing).
  3. SBOM plus provenance: `bazel build //deploy/release:sbom_demo`
     (SPDX 2.3 JSON plus SLSA v1 in-toto Statement v1, subject digest
     equals artifact sha256; Syft/CycloneDX output verifies through the
     same path when owners adopt it). Builder identity is allowlisted
     (`SBOM_BUILDER_DRY_RUN` for the demo shape check,
     `SBOM_BUILDER_RELEASE` for real releases): unlisted builders fail at
     analysis time, `write_provenance` refuses them, and verification
     rejects provenance whose `builder.id` is not allowlisted, so a
     forged builder fails closed.
 4. NOTICE bundling: `bazel build //deploy/release:notice_demo`
    (aggregated NOTICE from the `dx audit license` audited inventory via
    hermetic `notice_bundle` in `deploy/release/notice.bzl`: deterministic
    bytes with byte-identical rebuilds, `missing-notice-text` fails the
    action with an actionable diagnostic; verified by `notice_verify_files`
    plus `dx_verify --notice` before install, signed alongside the SBOM
    bundle via `//deploy/release:signing_demo`).
  5. Signing plus attestation: `bazel run //deploy/release:signing_demo`
     without the dry-run env (Sigstore keyless `cosign sign-blob
     --bundle` on the TUF trust root plus `gh attestation create`; signs the
     SBOM pair plus the NOTICE bundle, so nothing ships unsigned). Live
     signing enforces the pinned cosign version before signing and runs
     `cosign verify-blob --bundle` after every sign; a version drift or a
     failed verify stops before publish.
 6. Draft release: `bazel run //cli/cli:github_draft` with the real tag
    (`--draft --verify-tag`; publish by editing the draft on GitHub
    after approval). The draft ships the `dx` binary plus `man/dx.1`
    (section 1, single page); install the page to `/usr/share/man/man1/dx.1`.
 7. BCR: `bazel run //deploy/release:bcr_demo` without the dry-run env
    (opens the `source.json` plus integrity plus presubmit PR manually;
    this program never pushes itself).
 8. GHCR (issue #460): dispatch `ghcr.yml` with `approve: true`, then `cosign sign
    <digest>` plus attestation on the same trust root; record quotas and
    update the scaffold digest reference. Base-image plus Bazelisk plus
    Cosign plus TUF rebuild and rotation follows the manual on-demand
    [GHCR rebuild plus signing rotation](../contributing/devcontainer.md#ghcr-rebuild-plus-signing-rotation)
    contract (issue #647).
 9. Verify before install: `bazel run //deploy/install:dx_verify --
    --binary <dx> --bundle <bundle> --identity <workflow-id> --issuer
    https://token.actions.githubusercontent.com [--sbom <sbom>
    --sbom-bundle <sbom-bundle>] [--notice <NOTICE>
    --notice-manifest <manifest>]`; checksum-only is rejected, a NOTICE
    missing bundled entries or words fails with `missing-notice-text`,
    and failure happens before install or exec.

## CI SBOM Upload

CI builds plus verifies SBOM plus provenance on every push/PR via the
`sbom` job in `.github/workflows/ci.yml` (issue #612): `bazel build
//deploy/release:sbom_demo` plus `bazel test
//deploy/release:dx_release_tools_test`, staged under `RUNNER_TEMP/sbom` with
`digests.txt` plus `origin.txt` (sha256 bind plus repository, commit, run, ref)
and uploaded as the `sbom-provenance` artifact (SPDX-2.3 plus SLSA v1, publishes
nothing). Push-to-main runs additionally attest the staged pair via
`actions/attest` (Sigstore, fork-safe: PRs never attest); release signing stays
owner-gated human-run via `//deploy/release:signing_demo`. Per-host release evidence for Linux arm64 glibc lands via the
`sbom-arm64` job in `.github/workflows/ci.yml` (issue #803 closed): the same
`sbom_demo` build plus `dx_release_tools_test` verify on the arm64 native
runner (`ubuntu-24.04-arm`, `bazel-arm64-` cache, `needs: [build-arm64]`,
local-only), staged under `RUNNER_TEMP/sbom-arm64` and uploaded as the
`sbom-provenance-linux_arm64` artifact (SPDX-2.3 plus SLSA v1, publishes
nothing). Per-profile release evidence for the two Linux static-musl profiles
lands via the `sbom-musl-x86_64` plus `sbom-musl-arm64` jobs in
`.github/workflows/ci.yml` (issue #804 closed): the same `sbom_demo` build plus
`dx_release_tools_test` verify on the musl profile runners (`ubuntu-latest`
with `bazel-musl-x86_64-` cache, `needs: [build-musl-x86_64]`, plus
`ubuntu-24.04-arm` with `bazel-musl-arm64-` cache, `needs: [build-musl-arm64]`,
local-only), staged under `RUNNER_TEMP/sbom-musl-x86_64` plus
`RUNNER_TEMP/sbom-musl-arm64` and uploaded as the
`sbom-provenance-linux_x86_64_musl` plus `sbom-provenance-linux_arm64_musl`
artifacts (SPDX-2.3 plus SLSA v1, static native closure only with dynamic musl
explicitly out of scope, publishes nothing). Per-host release evidence for macOS arm64 native
lands via the `sbom-macos-arm64` job in `.github/workflows/ci.yml` (issue #805 closed): the same
`sbom_demo` build plus `dx_release_tools_test` verify on the macos arm64 native
runner (`macos-14`, `bazel-macos-arm64-` cache, `needs: [build-macos-arm64]`,
local-only), staged under `RUNNER_TEMP/sbom-macos-arm64` and uploaded as the
`sbom-provenance-macos_arm64` artifact (SPDX-2.3 plus SLSA v1, pinned acquired SDK
with hermetic-llvm Apple-SDK backend provisional and no host-installed SDK fallback never approved,
publishes nothing). Per-host release evidence for Windows
x86_64 MSVC-compatible lands via the `sbom-windows-x86_64` job in
`.github/workflows/ci.yml` (issue #807 closed): the same `sbom_demo` build plus
`dx_release_tools_test` verify on the windows native runner (`windows-latest`
with shell bash, `bazel-windows-x86_64-` cache, `needs: [build-windows-x86_64]`,
local-only), staged under `RUNNER_TEMP/sbom-windows-x86_64` and uploaded as the
`sbom-provenance-windows_x86_64` artifact (SPDX-2.3 plus SLSA v1, hermetic
acquisition plus MSVC compatibility gates unchanged with explicit EULA acceptance
required never automatic and no installed Build Tools fallback, publishes nothing).
Attestation stays owner-gated human-run via
`//deploy/release:signing_demo` (Sigstore keyless plus GitHub attestations);
CI never signs PR code. Pinned by `bazel run
//tools/ci:sbom_upload_qualification` with fixture evidence in
`tools/ci/tests/fixtures/sbom_upload/` plus `bazel run
//tools/ci:release_arm64_qualification` with fixture evidence in
`tools/ci/tests/fixtures/release_arm64/` plus `bazel run
//tools/ci:release_musl_qualification` with fixture evidence in
`tools/ci/tests/fixtures/release_musl/` plus `bazel run
//tools/ci:release_macos_arm64_qualification` with fixture evidence in
`tools/ci/tests/fixtures/release_macos_arm64/` plus `bazel run
//tools/ci:release_windows_qualification` with fixture evidence in
`tools/ci/tests/fixtures/release_windows/`.

All release outputs stay under `RUNNER_TEMP` or the chosen outdir until
published; `dist/` and `release/` stay git-ignored and the checkout is
left clean.
