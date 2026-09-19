# Devcontainer

Implementation status: the `dx init` devcontainer definition
(`.devcontainer/devcontainer.json` with pinned base image, Bazel-delegated
bootstrap `postCreateCommand`, no ambient tools) is dogfooded in this
repository. The checked-in definition is held byte-identical to the
scaffold output by `//.devcontainer:devcontainer_parity_test`, and CI runs
that gate check-only on every push and pull request. `postCreateCommand`
runs the user-path bootstrap (`bazel run //dx:env`, then `dx setup`)
instead of a full build, so container create pays only
managed-environment setup.

Container boot (devcontainer CLI build plus `postCreateCommand` execution
asserting the managed environment materializes) and non-Linux runs remain
open gaps; no working container or multi-platform support is claimed until
qualified execution lands.

## Prebuilt images (GHCR)

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
  [testing](../testing/README.md#infrastructure-budget)): container image
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
