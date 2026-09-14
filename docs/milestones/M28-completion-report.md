# M28 Completion Report: Stabilization And Release Qualification (Planning Phase)

Seed host: local Linux x86_64 glibc (same host class as the M00 seed report).
Local execution only, no remote. Non-Linux platforms were unavailable and are
recorded as gaps, not claimed. No capability-term transitions are claimed:
no API, support matrix cell, artifact, installer, signature, provenance
profile, coverage gate rerun, consumer-CI matrix run, or publication exists
yet, so nothing is frozen, qualified, or `Supported`; O6/O37/O38/O39
evidencing and O45 credentialed operations remain open under
`docs/open-decisions.md`. What lands here is the pure planning layer the
M28 contracts authorize before qualification: blocker disposition under the
first-release admission policy, the support-label evidence gate, SemVer
classification per distribution, command-registry exactness, artifact
identity and the embedded-vs-detached packaging boundary, reproducibility
comparison, verification binding with the self-attestation cap, the release
coverage gate, aggregate/matrix completeness, consumer-CI requalification,
and the M28-to-M29 publication handoff, all in the zero-dep `dx_qual` Rust
crate unit-tested without platforms, consumers, builders, or credentials.

This report closes the planning phase only. Frozen APIs/schemas/load
labels, compatibility fixtures, tested-stack/support-matrix freezes, docs
completion beyond the M30a handoff, platform/consumer/parity/generation/
environment/cache/remote/laziness suite runs, reproducible-candidate builds
with independent verification, coverage-instrumentation reruns, consumer-CI
matrix runs against candidate identities, installer/signing execution,
SBOM/provenance generation, and any tag/registry/release publication are
explicitly deferred as qualification-gated follow-ups (see Open Items). The
deferral is evidence-backed: the owning contracts carry do-not-implement
gates until those qualifications land, and no CLI surface is registered
until its behavior lands.

## WP1: Release-Blocking Provisional Decisions (planning)

`dx_qual` disposes one blocker under first-release admission
(`plan_blocker_disposition`): waiver attempts are always
`InvalidDeferral`; an additional-foundation blocker with a recorded
admission decision is `DeferredWithDecision`; everything else
`BlocksRelease`, including required-core even with a decision. Deferrals
never waive required-core obligations or the quality-tool baseline
(slice 1).

## WP2: API, Support, And Registry Freeze (planning)

`Supported` needs both required-platform and external-consumer evidence
(`may_claim_supported`); only a qualified `Supported` cell ships required
scope, partial never ships it (`may_ship_required_cell`). Public-API changes
classify per distribution SemVer (`classify_api_change`): incompatible
public requires major, compatible addition/deprecation minor, compatible fix
patch, internal-only and native-tool-output none. The final registry is
exact: verbatim order-insensitive set equality with the accepted commands
(`registry_is_exact`) (slice 1).

## WP5: Reproducible Candidates And Artifact Verification (planning; builds deferred)

Artifact identity is the digest of the exact published bytes; attestation
subjects bind by digest equality only, empty never binds
(`attestation_binds_exact_bytes`). The O39 candidate wire profiles are
pinned as research strings only (SPDX 2.3 JSON, SLSA provenance v1,
Statement v1) with no profile/builder/level freeze. The single correct
packaging path is payload-only embedded manifest plus detached
final-archive attestations with no self-digest entry
(`packaging_uses_single_correct_path`); the manifest must cover every
payload file verbatim (`manifest_covers_payload`); embedded changes require
new attestations (`embedded_change_requires_new_attestation`) (slice 2).
Reproduction is equal-nonempty-digest comparison over injected strings, no
build or threshold claim (`rebuild_reproduces`); verification binds digest,
predicate, signer, and builder to independent expectations in order, first
mismatch reported (`verify_rejection` / `verification_accepts`); no crypto,
log, timestamp, or certificate trust is executed. Self-attested provenance
caps at L0/L1 (`self_attested_level_admissible`) (slice 3).

## WP4/WP6/WP7: Evidence Inventory, Coverage Gate, CI Requalification (planning; runs deferred)

The release coverage gate passes only with a present report, valid
ignore/reason directives, and zero uncovered non-ignored lines; anything
else fails, never waives (`coverage_gate_accepts`). Aggregate success needs
every selected check/platform cell green over a non-empty set
(`aggregate_may_succeed`); the coverage rollup completes only when every
required platform cell is covered, hiding no gap
(`coverage_aggregation_complete`). Handoff identities match verbatim, empty
never matches (`handoff_identity_matches`); CI requalification rejects any
workflow/reporter/caller/module-CLI substitution
(`ci_requalification_accepts`); M28 hands off to M29 only when qualified,
digests match, and CI identities requalified
(`may_hand_off_to_publication`) (slice 4).

## WP3: Documentation Completion (dependency handoff)

M30a delivered the release-blocking docs subset (guides, examples corpus,
IR, site build, `dx docs` planning) and this milestone consumes that
handoff; remaining adoption scope stays post-release (M30b). No doc prose,
examples, changelog, notices, or module metadata lands here.

## Evidence

Exact commands on this host, committed tree:

- `bazel build //...`: success (721 targets).
- `bazel test //...`: 183/183 pass, including `//dx/qual:dx_qual_test`
  (23 passed) plus `rustfmt`/`clippy` gates (warnings as errors).
- `bazel run //dx:generate_check`: clean.

Coverage inventory: no new uncovered executable lines beyond the reconciled
gate; `dx_qual` is a zero-dep pure-planning library fully covered by its
co-located unit tests. No platform runs, external-consumer runs,
reproducibility measurements, signing/provenance generation, coverage
reruns, CI-matrix runs, or publication evidence; none claimed.

## Changed Components

- `dx/qual/` (crate `dx_qual`): `src/lib.rs` (all four planning slices plus
  23 unit tests); `BUILD.bazel`, `Cargo.toml`.
- Zero-dep by design: no `MODULE.bazel` manifest changes; no `dx/cli`
  registration (behavior has not landed); no artifacts, workflow YAML, or
  registry state.
- This report.

## Open Items

- Qualification execution (M28 WPs 4-7): run the full platform, consumer,
  parity, generation, environment, cache, remote, laziness, and release
  suites; build and independently verify reproducible candidates with SPDX
  2.3/SLSA v1/Statement v1 profiles; rerun the release coverage gate without
  blanket exclusions; run the consumer-CI matrix against candidate
  workflow/reporting/caller/module identities.
- O6/O37: granularity, ruleset/toolchain floors, Windows acquisition,
  ABI/runtime floors, allowlists, cross-build routes; platform evidence pending.
- O38/O39: packaging/install path, checksum/signature formats, trusted
  builder, assurance level, inventory completeness rules, reproducibility
  thresholds, trust bootstrap, verification inputs; research profiles above
  are candidates, not frozen.
- O45: publication credentials, permissions, registry procedure, release
  sequence; M29 owns credentialed operations over M28-qualified bytes only.
- Milestone exclusions respected: no publication (M29), no post-release
  adoption (M30b), no new languages/tools. M29 handoff: qualified-candidate
  digests, workflow/reporting/caller identities, and the pure gates above as
  fixtures.
