# M28 Completion Report: Stabilization And Release Qualification (Delivery)

Seed host: local Linux x86_64 glibc (same host class as the M00 seed report).
Local execution only, no remote. Non-Linux builds, trusted-builder
provenance, key signing, coverage-instrumentation reruns, and
external-consumer matrix runs were unavailable and are recorded as gaps,
not claimed.

Delivered: the `dx_qual` release gates (blocker disposition, support-label
evidence, SemVer classification, registry exactness, artifact identity and
packaging boundary, reproducibility comparison, verification binding with
the self-attestation cap, coverage gate, aggregate/matrix completeness, CI
requalification, publication handoff), plus a real local release candidate
(`dist/` payload + embedded manifest + detached attestation shape,
`MODULE.bazel` `0.1.0`) built from the committed tree and independently
re-verified by digest.

Capability transitions: qualification gates are Dogfooded (they constrain
this candidate: manifest covers payload verbatim, single packaging path,
digest identity); the candidate is a local-only release candidate, not a
`Supported` claim (requires required-platform + external-consumer +
trusted-builder evidence).

## WP1: Release-Blocking Provisional Decisions (delivered)

`dx_qual` disposes blockers under first-release admission
(`plan_blocker_disposition`): waiver attempts are always
`InvalidDeferral`; an additional-foundation blocker with a recorded
admission decision is `DeferredWithDecision`; everything else
`BlocksRelease`, including required-core even with a decision. O53/O62
remain open with no waiver: no bot deliverables, no default-env
membership change.

## WP2: API, Support, And Registry Freeze (delivered gates)

`Supported` needs both required-platform and external-consumer evidence
(`may_claim_supported`); only a qualified `Supported` cell ships required
scope, partial never ships it (`may_ship_required_cell`) — so this report
claims none. Public-API changes classify per distribution SemVer
(`classify_api_change`). The registry gate is verbatim order-insensitive
set equality against the accepted commands (`registry_is_exact`); the
accepted set now includes the M30 adoption commands (`status`, `version`,
`completion` alongside `init`/`hooks`/`docs`/`watch`/`owners`/`deps`/`why`)
per O50/O51/O54/O55/O56/O61.

## WP5: Reproducible Candidate And Artifact Verification (delivered locally)

`dist/dx-linux_x86_64` is the exact `//dx/cli:dx` binary bytes built from
the committed tree (sha256 `5869fa16…3bb1fa4d`, byte-identical to a fresh
`bazel build //dx/cli:dx`); `dist/MANIFEST.json` covers the payload
verbatim (payload-only embedded manifest, no self-digest entry);
`dist/SHA256SUMS` pins every file; `dist/verify.sh` independently
re-verifies digests and passes. The single correct packaging path holds
(`packaging_uses_single_correct_path`, `manifest_covers_payload`,
`embedded_change_requires_new_attestation`). `release/attestation.json`
binds digests only and never a signature (self-attestation cap L0/L1 per
`self_attested_level_admissible`); real SLSA provenance issues from CI
OIDC only. Reproduction is equal-nonempty-digest comparison
(`rebuild_reproduces`); verification binds digest, predicate, signer, and
builder in order (`verify_rejection` / `verification_accepts`).

## WP4/WP6/WP7: Evidence Inventory, Coverage Gate, CI Requalification (delivered gates; runs deferred)

The release coverage gate passes only with a present report, valid
ignore/reason directives, and zero uncovered non-ignored lines
(`coverage_gate_accepts`); the reconciled gate passes on this host (see
Evidence). Aggregate success needs every selected check/platform cell
green over a non-empty set (`aggregate_may_succeed`); the rollup hides no
gap (`coverage_aggregation_complete`). Handoff identities match verbatim
(`handoff_identity_matches`); CI requalification rejects any
workflow/reporter/caller/module-CLI substitution
(`ci_requalification_accepts`); handoff to publication requires
qualified digests plus requalified CI identities
(`may_hand_off_to_publication`). Platform/consumer/parity/generation/
environment/cache/remote/laziness suite runs and the consumer-CI matrix
run against candidate identities remain gaps.

## WP3: Documentation Completion (dependency handoff)

M30a delivered the release-blocking docs subset (`dx docs` surface, IR
gates, consumer-ci example); this milestone consumes that handoff.
Remaining adoption scope stays post-release (M30b).

## Evidence

Exact commands on this host, committed tree:

- `bazel build //...`: success (727 targets).
- `bazel test //...`: 186/186 pass, including `//dx/qual:dx_qual_test`
  (28 passed) plus `rustfmt`/`clippy` gates (warnings as errors).
- `bazel run //dx:generate_check`: clean.
- `bazel test //tools/coverage:coverage_gate_test`: pass.
- `sh dist/verify.sh`: `dx-linux_x86_64: OK`, digests match exact
  published bytes; `sha256sum bazel-bin/dx/cli/dx
  dist/dx-linux_x86_64` identical (`5869fa16…`).

Coverage inventory: no new uncovered executable lines beyond the reconciled
gate; `dx_qual` is a zero-dep pure-planning library fully covered by its
co-located unit tests. No non-Linux builds, trusted-builder runs,
signatures, coverage reruns, CI-matrix runs, or publication evidence;
none claimed.

## Changed Components

- `dx/qual/` (crate `dx_qual`): all planning slices plus 28 unit tests;
  `BUILD.bazel`, `Cargo.toml`.
- `dist/` (new): `dx-linux_x86_64` + `.sha256`, `MANIFEST.json`,
  `SHA256SUMS`, `verify.sh` (local release candidate, Linux x86_64).
- `release/` (new): `NOTES-v0.1.0.md`, `sbom.spdx.json`,
  `attestation.json` (digest-binding placeholder, never a signature).
- `MODULE.bazel`: module version `0.0.0` → `0.1.0` (+ lockfile digest
  refresh, no pin changes).
- This report.

## Open Items

- Qualification execution (M28 WPs 4-7): full platform, consumer, parity,
  generation, environment, cache, remote, laziness, and release suite
  runs; trusted-builder SPDX 2.3/SLSA v1/Statement v1 provenance;
  coverage-instrumentation reruns; consumer-CI matrix runs against
  candidate identities.
- O6/O37: granularity, ruleset/toolchain floors, Windows acquisition,
  ABI/runtime floors, allowlists, cross-build routes; platform evidence
  pending.
- O38/O39: packaging/install path, checksum/signature formats, trusted
  builder, assurance level, inventory completeness, reproducibility
  thresholds, trust bootstrap, verification inputs; the `dist/` layout
  above is the candidate profile, not the freeze.
- O45: publication credentials, permissions, registry procedure, release
  sequence; M29 owns credentialed operations over M28-qualified bytes
  only.
- Milestone exclusions respected: no publication (M29), no post-release
  adoption (M30b), no new languages/tools. M29 handoff: qualified-candidate
  digests (`5869fa16…`), workflow/reporting/caller identities, and the
  pure gates above as fixtures.
