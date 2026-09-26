# Devcontainer

Implementation status: the `dx init` devcontainer definition
(`.devcontainer/devcontainer.json` with pinned base image, Bazel-delegated
bootstrap `postCreateCommand`, no ambient tools) is dogfooded in this
repository. The checked-in definition is a snapshot of the
scaffold output held by `//.devcontainer:devcontainer_parity_test`
(schema plus byte snapshot, UPDATE_EXPECT refreshes the golden), and CI runs
that gate check-only on every push and pull request. `postCreateCommand`
runs the user-path bootstrap (`bazel run //dx:env`, then `dx setup`)
instead of a full build, so container create pays only
managed-environment setup.

Container boot on the seed host (linux/amd64, devcontainer CLI build plus
`postCreateCommand` execution asserting the managed environment
materializes) is manually verified on every Dockerfile or definition
change (see [Container boot manual](#container-boot-manual)); CI stays
parity plus definition shape only, no boot job. Non-Linux runs plus
linux/arm64 boot plus an arm64 prebuilt variant are wont-fix: the
scaffold `devcontainer.json` itself is arch-independent and the pinned
Ubuntu base digest resolves from a multi-arch index, but the prebuilt image
stays a linux/amd64 seed slice (`.devcontainer/Dockerfile.prebuilt` pins
the amd64 Bazelisk launcher), so arm64 container boot plus an arm64
prebuilt variant stay wont-fix (issue #410 qualifies `dx` and CI natively
on arm64, not container boot; issue #412 qualifies `dx` and CI
natively on macOS arm64, not container boot; macOS x86_64 Not planned per #976
with no CI, not container boot; issue #414 qualifies `dx` and CI natively
on Windows x86_64, not container boot).

## Container boot manual

Manual boot verify on Dockerfile change with non-Linux/arm64 wont-fix
(issue #648): seed-host boot only, no boot job in CI.

As built today: `devcontainer-check` stays parity plus definition shape
(`//.devcontainer:devcontainer_parity_test` plus `postCreateCommand`
shape); `ghcr.yml` builds the image on PRs touching the scaffold inputs
without booting it. Boot is verified manually on linux/amd64: `docker
build -f .devcontainer/Dockerfile.prebuilt -t dx-devcontainer:local .`
locally, then `devcontainer up --workspace-folder .` (devcontainer CLI
plus Docker) with `postCreateCommand` (`bazel run //dx:env`, then `dx
setup`) asserting the managed environment materializes.

Verify is manual on every change to `.devcontainer/Dockerfile.prebuilt`
or `.devcontainer/devcontainer.json`, plus before any gated GHCR push,
with evidence in the same reviewed PR; the sole maintainer owns every
row until delegation (see `CODEOWNERS`).

Qualify locally or on demand with customer flows only:
`bazel run //tools/ci:devcontainer_boot_qualification` (static pins, builds nothing,
boots nothing) plus on-demand local `docker build` plus `devcontainer
up`.

Boot job in CI rejected, non-customer; no extra CI job exists; keep CI
customer-only. Non-Linux runs plus linux/arm64 boot plus an arm64
prebuilt variant are wont-fix, no multi-platform container support
claimed. Fixture evidence is pinned in
`tools/ci/tests/fixtures/devcontainer_boot/pins.bzl` plus
`devcontainer_boot.expected` via
`bazel run //tools/ci:devcontainer_boot_qualification` (issue #648; infra only, no
Supported claim).

## Prebuilt images (GHCR)

Owner: issue #460 (live successor to closed #184 for the GHCR route).

Per-create feature installation (Bazel fetch plus a cold postCreate) is
removed by prebuilt images published to GHCR per the
[release runbook](../deploy/release-runbook.md). The route is a
separate workflow (`.github/workflows/ghcr.yml`), never folded into the
release workflow: image lifecycle is per-scaffold-change, not per-tag, so
base-image rebuilds never block or couple a `dx` release.

As built today (owner-gated push with signing, no push claimed until dispatch):

- CI builds the image on PRs touching the scaffold inputs
  (`.devcontainer/Dockerfile.prebuilt`, `ghcr.yml`,
  `.devcontainer/devcontainer.json`).
- Push runs only on manual `workflow_dispatch` with `approve: true`
  (explicit owner approval; default builds and reports only).
- The base is digest-pinned (never `latest`), Bazel arrives via pinned
  Bazelisk delegation, and no language toolchains are baked in: tools
  resolve via Bazel at container runtime. Image tags track the
  single-version `dx` == module pin (`0.0.0-sha-<sha>`); the digest pin
  (`ghcr.io/...@sha256:<digest>`, never `latest`) is the scaffold
  reference that lands with the first push. The checked-in scaffold still
  references the public base image; switching it now would invent an
  unpublished digest.
- Signing follows the release trust root: `cosign sign --yes
  <image>@<digest>` (Sigstore keyless, OIDC via `id-token: write`) plus
  `cosign verify` and `gh attestation` on the same trust root as
  `//deploy/release:signing_demo` (`deploy/release/signing.bzl`); the
  GHCR workflow prints the would-sign commands in dry-run mode on PRs and
  signs + verifies only on `workflow_dispatch` with `approve: true`.
  Cosign arrives via pinned `curl` fetch (version-pinned, checksum-verified
  against the published release checksums; no `sigstore/*` installer
  action). Nothing here is signed until the gated push runs.
- GHCR quotas and retention (qualified per the infrastructure budget in
  [testing](../testing/strategy-details.md#infrastructure-budget)): container image
  storage and bandwidth are currently free for public repos, with at least
  one month notice before any pricing change (GitHub Packages billing);
  the private-Packages quotas (500 MB storage, 1 GB transfer on Free) do
  not apply to containers today. Retention is manual (untagged cleanup per
  package settings); no retention policy deletes the version-tracked tag
  without owner action. Build-only PRs push nothing, so no quota is
  consumed; the first gated push records its exact image bytes in the job
  summary. Image publication is a publication output: no tags, pushes, or
  retention claims without explicit owner approval per the release
  hygiene in [Contributing](../../CONTRIBUTING.md).

## GHCR Rebuild Plus Signing Rotation

Manual on-demand rebuild plus rotation for the four pins with no rebuild
owner (issue #647): Ubuntu base digest, Bazelisk launcher, Cosign CLI,
TUF trust root. Exact pins stay single-sourced in
`Dockerfile.prebuilt` plus `docs/contributing/local-workflows.md`
plus `deploy/release/signing.bzl` plus `.github/workflows/ghcr.yml`;
this section owns only cadence plus handling plus rejection.

As built today: base `ubuntu:24.04@sha256:69cecf4b...` (resolved
2026-09-17, re-pin deliberately, never `latest`), Bazelisk `v1.29.0`
(canonical version plus linux-amd64 sha in `Dockerfile.prebuilt`, all
five per-OS shas in `docs/contributing/local-workflows.md`, Bazel `9.2.0`
via `USE_BAZEL_VERSION`), Cosign
`v2.4.1` (checksum-verified fetch in `ghcr.yml`, single-sourced with
`SIGNING_COSIGN_VERSION`), trust root
`https://tuf-repo-cdn.sigstore.dev` plus issuer
`https://token.actions.githubusercontent.com` plus bundle media
`application/vnd.dev.sigstore.bundle.v0.3+json` (documented, not
self-hosted).

Rebuild is manual on-demand only: `docker build -f
.devcontainer/Dockerfile.prebuilt -t dx-devcontainer:local .` locally,
then re-pin deliberately with evidence in one reviewed PR (`Dockerfile`
plus action pins plus signing pins plus docs plus fixtures). Triggers
are upstream release notice (Bazelisk, Cosign), base-image refresh or
CVE, plus before any gated push; the sole maintainer owns every row
until delegation (see `CODEOWNERS`).

Qualify locally or on demand with customer flows only: `bazel run //tools/ci:ghcr_rebuild_rotation_qualification` (static pins, builds nothing, pushes nothing) plus on-demand local `docker build`; the gated push itself stays owner-approved `workflow_dispatch` plus `approve: true` with `cosign sign` plus `verify` on the same trust root.

Scheduled CI rebuild rejected; `ghcr.yml` carries no `push` or `schedule` trigger and no extra CI job exists; keep CI customer-only. Fixture evidence is pinned in `tools/ci/tests/fixtures/ghcr_rebuild_rotation/pins.bzl` plus `ghcr_rebuild_rotation.expected` via `bazel run //tools/ci:ghcr_rebuild_rotation_qualification` (issue #647; infra only, no Supported claim).
