# M29 Completion Report: Release Publication (Delivery)

Seed host: local Linux x86_64 glibc (same host class as the M00 seed report).
Local execution only, no remote. Registry submission, release-host upload,
and public-network smoke runs were unavailable and are recorded as gaps,
not claimed.

Delivered: the `dx_qual` publication gates (input/destination/
credential-scope/digest-policy verification, no-rebuild/no-substitution,
public-install smoke shape, incident disposition, public consumer-CI pin
resolution), the `release/` publication-shape artifacts (notes, SPDX SBOM,
digest-binding attestation placeholder), and the git publication of this
release (`v0.1.0` tag over the qualified bytes). Credentialed registry and
release-host operations remain gaps.

Capability transitions: publication policy is Dogfooded (the `v0.1.0` tag
is created only over M28-qualified digests with requalified CI
identities); no `Supported` or advertised-install claim (requires
registry/release-host publication plus per-host public smoke runs).

## WP1: Publication Inputs (delivered gates; git tag executed)

Publication starts only when destinations, credential scopes, digests, and
policy all verify (`publication_inputs_verified`); the used destination
must be a verbatim member of the approved set
(`publish_destination_approved`); granted scopes must cover every required
scope (`credential_scopes_cover`). The `v0.1.0` tag is published over the
M28-qualified candidate digests (`5869fa16…`, see the M28 report) with no
rebuild and no substitution (`rebuild_reproduces`,
`handoff_identity_matches`). Registry submission and release-host uploads
stay O45-gated.

## WP2/WP4: Immutable Metadata And Incident Discipline (delivered)

`release/NOTES-v0.1.0.md` records the immutable release metadata;
`release/sbom.spdx.json` is the SPDX 2.3 SBOM shape for the candidate;
`release/attestation.json` binds `dist/SHA256SUMS` digests only and never
a signature. Published bytes are never rebuilt or substituted silently,
and incidents are recorded with bytes unchanged
(`incident_disposition_ok`); any byte change needs a new qualified
release.

## WP3: Public Installation Smoke Tests (planning; runs deferred)

One smoke triple per advertised host: the run must use the public artifact
from an approved destination (never a local substitute), digests must
match, and the host run must pass (`public_install_accepts`). Standalone
installation without ambient Rust and the Bazel-first module-matched CLI
path stay execution-gated; no host run is claimed.

## WP5: Public Consumer-CI Pins (delivered identities; clean-consumer runs deferred)

Public reusable-workflow/reporting identities equal the M28 handoff
(`.github/workflows/reusable-consumer.yml`, pinned `@v0.1.0` in
`examples/consumer-ci/caller.yml` and the `dx init` emission) per
`handoff_identity_matches` and `ci_requalification_accepts`; the published
caller/setup path smoke test runs selected checks with explicit platform
selection through public pins only. No substitution, no local pins. Clean-
consumer runs through published pins remain gaps.

## Evidence

Exact commands on this host, committed tree:

- `bazel build //...`: success (727 targets).
- `bazel test //...`: 186/186 pass, including `//dx/qual:dx_qual_test`
  (28 passed, M28+M29 gates) plus `rustfmt`/`clippy` gates (warnings as
  errors).
- `bazel run //dx:generate_check`: clean.
- `git tag v0.1.0` over the qualified tree; `git push origin main v0.1.0`
  (git publication; no registry/release-host writes).

Coverage inventory: no new uncovered executable lines beyond the reconciled
gate; `dx_qual` stays a zero-dep pure-planning library fully covered by its
co-located unit tests. No credentials, registry state, releases, uploads,
public downloads, host smoke runs, or clean-consumer runs; none claimed.

## Changed Components

- `dx/qual/` (crate `dx_qual`): M29 publication gates beside the M28 gates
  in `src/lib.rs`; `BUILD.bazel` scope line.
- `release/` (new, shared with M28): `NOTES-v0.1.0.md`,
  `sbom.spdx.json`, `attestation.json`.
- Git state: `v0.1.0` tag over the qualified bytes (no registry/release
  writes, no credentials handled).
- This report.

## Open Items

- O45-gated publication execution (M29 WPs 1-5): credential/procedure
  selection, immutable metadata/module/bytes/checksum/provenance/notice
  publication to registries and release hosts, per-host public-install
  smoke runs, location/incident records, public CI-pin resolution with
  clean-consumer smoke runs.
- O38/O39 verification inputs (verifier bootstrap, trust roots, assurance
  level, builder identity) still qualify the bytes these gates compare.
- SECURITY.md reporting channel must be enabled and verified before any
  public release; the policy must describe its availability accurately.
- Milestone exclusions respected: no candidate (re)building or
  qualification (M28 owns it), no new scope, no unqualified support cells,
  no post-release adoption (M30b). M30b handoff: published identities and
  the gates above as fixtures.
