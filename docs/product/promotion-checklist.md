# Promotion Checklist

Accepted checklist owned under issue #611. Tag hygiene, versioning, and what
evidence promotes a cell to `Supported` live here; no other open tracker owns
them. Seed-only fixture proof via
`bazel run //tools/ci:promotion_checklist_qualification` with
`tools/ci/tests/fixtures/promotion_checklist/pins.bzl` plus
`promotion_checklist.expected`. No cell is currently `Supported`.

The [status lifecycle](support-matrix.md#status-lifecycle) owns promotion
(`Planned` → `Seed-host-delivered` → `Platform-qualified` → `Supported`).
This checklist is the per-cell evidence bar for the last step. The
[support matrix](support-matrix.md) owns status cells; the
[verification matrix](../testing/verification-matrix.md) owns as-built
layer evidence; the [release runbook](../deploy/release-runbook.md) owns the
human-run path. Link, don't copy: pins below name the owning contract.

## Tag hygiene as-built

- Module stays at `version = "0.0.0"`; consumers pin reviewed commits, never tags.
- No `v*` tags without explicit owner approval; remote stays tag-free until then.
- `/dist/` plus `/release/` stay git-ignored build outputs and are never committed.
- Caller templates pin the reusable workflows at reviewed-commit SHA
  (`reusable-consumer.yml@<sha>` plus `reusable-docs.yml@<sha>`), stay in sync
  across both example callers, and move only together.
- Pinned by `bazel run //tools/ci:release_hygiene` (issue #5) plus
  `bazel run //tools/ci:consumer_ci_qualification` (issue #509, tag hygiene as-built).

## Versioning

- First release bumps `MODULE.bazel` from `0.0.0` to SemVer in a reviewed PR;
  `CHANGELOG.md` still records `No release has been cut` until then.
- The tag is pushed to the remote beforehand with owner approval. No release
  program creates or pushes tags (`--verify-tag` everywhere); the human-run
  driver never creates or pushes tags itself.
- Single-version `dx` == module pin: the released `dx` version equals the module
  version, and callers pin reviewed commits until the cut, never release tags.
- Published bytes are never rebuilt or substituted silently; byte identity stays
  fail-closed (changed upstream bytes are rejected, never silently recorded).
- Callers keep `rules_dx_version` matched to the module version (`0.0.0` today).

## Platform evidence per cell

A cell promotes only with its required-host evidence landed:

- Pins, hosts, floors, JDK/SDK/CRT identities, qualified native plus cross
  routes, per-cell coverage, and consumer plus release evidence under its
  per-host successor (Linux arm64 native issue #410, static-musl profiles
  issue #411, macOS arm64 native issue #412, macOS x86_64 best-effort native
  issue #413, Windows x86_64 MSVC-compatible native issue #414).
- Floors pinned via `bazel run //tools/ci:deployment_floors_qualification`
  (issue #500); routes pinned via
  `bazel run //tools/ci:cross_routes_qualification` (issue #504, Linux-first,
  never every-target-from-every-host); per-cell coverage via
  `bazel run //tools/ci:coverage_qualification` (issue #507, seven cells, same
  scope, no union).
- Unqualified hosts keep the clean `unsupported_platform` refusal; a host flips
  on exactly when its evidence lands. Best-effort gaps never block
  required-host release.

## Consumer evidence per cell

- Nine checks through the reusable workflow with explicit platform selection
  (no implicit default) and the stable `dx-ci` aggregate, qualified seed-only
  under issue #509 via `bazel run //tools/ci:consumer_ci_qualification`.
- All-enabled self-call in `ci.yml` (issue #408, verbatim `//...`): every
  selected check runs at normal repository scope; build-only self-call forever
  rejected.
- Native bump loop stays the sole updater (native-only, issue #461).

## Release evidence per cell

- SBOM plus provenance: SPDX 2.3 JSON plus SLSA v1 via
  `//deploy/release:sbom_demo` with subject digest equal to artifact sha256.
- Signing-first: Sigstore keyless `cosign sign-blob --bundle` plus GitHub
  attestations on the issue #311 trust root via
  `//deploy/release:signing_demo`; nothing is drafted or published unsigned.
- BCR shape checked-not-submitted via `//deploy/release:bcr_demo`
  (`BCR_DRY_RUN=1`, `"submitted": False`); GHCR stays the separate
  `.github/workflows/ghcr.yml` route (issue #460).
- Human-run driver `deploy/release/release.sh` in dry-run mode by default
  (issue #458): tag ceiling plus owner-approval gate with nothing published.
- Verifier refusal proved: `//deploy/install:dx_verify` refuses checksum-only
  inputs on the TUF trust root and installs nothing.
- Dry-run-first owner-gated tooling: `publish-dry-run.yml` stays
  `workflow_dispatch`-only with a default-closed approve gate, no secrets,
  minimal permissions, `RUNNER_TEMP` staging plus a clean-checkout proof.
- Pinned by `bazel run //tools/ci:release_hygiene` (issue #5),
  `bazel run //tools/ci:release_policy` (curated defaults plus parity),
  `bazel run //tools/ci:publish_trust`,
  `bazel run //tools/ci:distribution_closeout_guards`, and
  `bazel run //tools/ci:signing_distribution_qualification` (issue #459).

## Promotion rule

A `Planned` cell becomes `Seed-host-delivered`, then `Platform-qualified`,
then `Supported` only when every box above for that cell is checked with
fixture or harness evidence and the support-matrix row flips in a reviewed PR
with owner approval. `Supported` promotion occurs only during release
qualification. A missing cell remains an explicit gap and blocks that support
claim, enforced by `bazel run //tools/ci:supported_evidence_gate`
(issue #301). Ad-hoc release without this checklist is rejected.
Compatibility: Release only.

Qualified seed-only under issue #611; platform plus consumer plus release
evidence stays owned gap; no Supported claim.
