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
removed by prebuilt images published to GHCR, tracked in
open work. The route is a
separate workflow (`.github/workflows/ghcr.yml`), never folded into the
release workflow: image lifecycle is per-scaffold-change, not per-tag, so
base-image rebuilds never block or couple a `dx` release.

As built today (build-only seed slice, no push claimed):

- CI builds the image on PRs touching the scaffold inputs
  (`.devcontainer/Dockerfile.prebuilt`, `ghcr.yml`,
  `.devcontainer/devcontainer.json`).
- Push runs only on manual `workflow_dispatch` with `approve: true`
  (explicit owner approval; default builds and reports only).
- The base is digest-pinned (never `latest`), Bazel arrives via pinned
  Bazelisk delegation, and no language toolchains are baked in: tools
  resolve via Bazel at container runtime. The checked-in scaffold still
  references the public base image; the `ghcr.io` digest reference lands
  with the first push.
- Signing follows the release trust root: `cosign sign <digest>`
  (Sigstore keyless) plus attestation verification land after the
  human-run signing workflow
  (open work), on the same
  trust root decided under
  open work. Nothing here is
  signed yet.
- GHCR quotas and retention are recorded on the first push (free for
  public repos, qualified not assumed per the infrastructure budget).
  Image publication is a publication output: no tags, pushes, or
  retention claims without explicit owner approval per the release
  hygiene in open work.
