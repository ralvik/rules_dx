# M29 Completion Report: Release Publication (Planning Phase)

Seed host: local Linux x86_64 glibc (same host class as the M00 seed report).
Local execution only, no remote. Non-Linux platforms were unavailable and are
recorded as gaps, not claimed. No capability-term transitions are claimed:
no tag, registry entry, release, published bytes, checksum/signature upload,
or public-install verification exists yet, so nothing is published or
advertised; destinations select policy only and O45 credentials/procedures
remain open under `docs/open-decisions.md`. What lands here is the pure
planning layer the M29 contract authorizes before publication: input,
destination, credential-scope, and digest/policy verification, the
no-rebuild/no-substitution rule, public-artifact smoke-test shape, incident
disposition, and public consumer-CI pin resolution, all in the zero-dep
`dx_qual` Rust crate unit-tested without credentials, registries, release
hosts, or public networks.

This report closes the planning phase only. Credential issuance and use,
registry submission, release creation, checksum/provenance upload, public
host smoke runs, and clean-consumer runs through published pins are
explicitly deferred as O45-gated follow-ups (see Open Items). The deferral
is evidence-backed: the owning contract forbids credentialed operations
until O45 resolves and M28 qualifies the exact bytes, and no CLI surface is
registered until its behavior lands.

## WP1: Publication Inputs (planning; credentialed ops deferred)

Publication starts only when destinations, credential scopes, digests, and
policy all verify (`publication_inputs_verified`). The used destination must
be a verbatim member of the approved set
(`publish_destination_approved`); granted scopes must cover every required
scope (`credential_scopes_cover`). Membership and coverage compare injected
strings only; they approve no destination and grant nothing (slices 1-2,
extending the M28 handoff gates `may_hand_off_to_publication` and
`handoff_identity_matches`).

## WP2/WP4: Immutable Metadata And Incident Discipline (planning)

Published bytes are never rebuilt or substituted silently: digests must
still match the M28-qualified digests (`rebuild_reproduces`,
`handoff_identity_matches`), and incidents are recorded with bytes
unchanged (`incident_disposition_ok`). Recording alone never authorizes new
bytes; any byte change needs a new qualified release.

## WP3: Public Installation Smoke Tests (planning; runs deferred)

One smoke triple per advertised host: the run must use the public artifact
from an approved destination (never a local substitute), digests must
match, and the host run must pass (`public_install_accepts`). Standalone
installation without ambient Rust and the Bazel-first module-matched CLI
path stay execution-gated; no host run is claimed.

## WP5: Public Consumer-CI Pins (planning; clean-consumer runs deferred)

Public reusable-workflow/reporting identities must equal the M28 handoff
and the documented caller pins must resolve to them
(`handoff_identity_matches`, `ci_requalification_accepts`); the published
caller/setup path smoke test runs selected checks with explicit platform
selection through public pins only. No substitution, no local pins.

## Evidence

Exact commands on this host, committed tree:

- `bazel build //...`: success (721 targets).
- `bazel test //...`: 183/183 pass, including `//dx/qual:dx_qual_test`
  plus `rustfmt`/`clippy` gates (warnings as errors).
- `bazel run //dx:generate_check`: clean.

Coverage inventory: no new uncovered executable lines beyond the reconciled
gate; `dx_qual` stays a zero-dep pure-planning library fully covered by its
co-located unit tests. No credentials, registry state, releases, uploads,
public downloads, host smoke runs, or clean-consumer runs; none claimed.

## Changed Components

- `dx/qual/` (crate `dx_qual`): M29 planning gates beside the M28 gates in
  `src/lib.rs` (slices 1-2 plus unit tests); `BUILD.bazel` scope line.
- Zero-dep by design: no `MODULE.bazel` manifest changes; no `dx/cli`
  registration (behavior has not landed); no destinations touched, no
  credentials handled, no registry/release writes.
- This report.

## Open Items

- O45-gated publication execution (M29 WPs 1-5): credential/procedure
  selection, immutable metadata/module/bytes/checksum/provenance/notice
  publication, per-host public-install smoke runs, location/incident
  records, public CI-pin resolution with clean-consumer smoke runs.
- O38/O39 verification inputs (verifier bootstrap, trust roots, assurance
  level, builder identity) still qualify the bytes these gates compare.
- SECURITY.md reporting channel must be enabled and verified before any
  public release; the policy must describe its availability accurately.
- Milestone exclusions respected: no candidate (re)building or
  qualification (M28 owns it), no new scope, no unqualified support cells,
  no post-release adoption (M30b). M30b handoff: published identities and
  the gates above as fixtures.
