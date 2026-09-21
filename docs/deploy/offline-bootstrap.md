# Offline/Airgap Bootstrap

Enterprise and airgapped consumers bootstrap, set up environments, and
audit without network after one connected first fetch. The offline path
reuses the same gates as the online path; it never weakens them.

## First fetch versus steady-state offline

First fetch needs network once: the pinned Bazelisk launcher download
([Local Workflows](../contributing/local-workflows.md#bootstrap)), the
first Bazel module and toolchain fetch behind `bazel run //dx:env`, the
advisory snapshot download through supported upstream tooling, and the
standalone `dx` release fetch. Steady-state offline needs none of them:
the launcher installs from the vendored bundle, `dx setup`/`env`/`codegen`
reuse already-fetched Bazel inputs with no new fetch when inputs are
unchanged, and `dx audit` matches locally against the vendored advisory
mirror with no fetch at all. Anything that still needs network fails
closed with an actionable diagnostic instead of silently degrading.

## Vendored bundle

The bundle is the `dx_standalone` archive plus the pinned per-OS
Bazelisk launchers plus the advisory mirror directory, carried with a
`SHA256SUMS` manifest. Launcher bytes and checksums stay pinned in
`.github/actions/setup-bazelisk/action.yml`; the manifest binds the exact
bytes the airgapped host installs. Population and verification run
through `deploy/offline/bootstrap-offline.sh`, which verifies every
manifest entry before installing anything and performs no download
(no `curl`, no `wget`, no Python URL fetch). A checksum mismatch, a missing
entry, or an unknown argument fails before mutation.

```sh
deploy/offline/bootstrap-offline.sh --bundle <dir> \
  [--install-dir "$HOME/.local/bin"] [--workspace "$PWD"]
```

The script installs the launcher as `bazel` and populates
`.dx/advisory/<set>.json` plus `.dx/advisory/<set>.meta.json` from the
mirror. After it succeeds, `bazel version` and `dx setup --dry-run`
prove the bootstrap without touching the network.

## Vendored advisory mirror

Airgapped workspaces populate `.dx/advisory/` by copying the vendored
bundle bytes plus identity instead of fetching. Mirror identities carry
a `file://` URL naming the vendored source; upstream identities keep
their `https://` database-download URL. Both shapes enforce the same
`sha256` byte binding plus the same same-day freshness gate, and the
same fail-closed mapping: a missing, invalid, tampered, or stale
mirror snapshot fails with `advisory_refresh_failed`, never clean and
never a stale fallback. The `retrieved_at` stamp is written at populate
time from the bundle bytes, so re-copying a refreshed bundle is the
mirror refresh. Live `dx audit` performs no network fetch on either
path and never uploads lockfiles or inventories.

## Offline setup, env, and codegen

`dx setup`, `dx env`, and `dx codegen` run unchanged once the bundle is
installed: `bazel run //dx:env` remains the pre-`dx` bootstrap, and the
normal first complete initialization stays `dx setup`, as specified in
[Environment, Codegen, And Setup](../cli/commands/environment-codegen-setup.md).
Offline changes none of their selection, validation, or atomic-commit
semantics; with unchanged inputs Bazel reuses its already-fetched
modules and toolchains and starts no new fetch. `--dry-run` plans the
request and exits `0` without launching, which is the network-disabled
proof used by the qualification harness below.

## Fixture evidence

Dry-run and local-mirror fixtures only; no live mirror service is
operated. The `offline_airgap` fixtures carry a miniature bundle
(launcher bytes, manifest, one advisory snapshot) plus the expected
pins. `bazel run //tools/ci:offline_airgap_qualification` proves, with
`curl`/`wget` shadowed by failing stubs so any network attempt fails,
that bootstrap installs from the bundle, that advisory population binds
bytes plus freshness, that tampering fails closed, and that setup
planning needs no network. Seed host only; no `Supported` claim.
Platform plus consumer plus release evidence stays an owned gap.
