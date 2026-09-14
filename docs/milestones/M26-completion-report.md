# M26 Completion Report: Audit And Update (Planning Phase)

Seed host: local Linux x86_64 glibc (same host class as the M00 seed report).
Local execution only, no remote. Non-Linux platforms were unavailable and are
recorded as gaps, not claimed. No capability-term transitions are claimed:
there are no qualified auditor integrations yet, so no working audit/update
support is established; tool selection, advisory acquisition, severity/report
mappings, and native configuration remain unqualified under O11, update
mappings under O12, and the license family under O58. What lands here is the
pure planning layer the audit/update contracts authorize before tools:
family/request selection, exception lifecycle, secrets-artifact
qualification, update selection/continuation/semantics, and license-policy
evaluation, all zero-dep Rust crates unit-tested without a workspace, a Bazel
server, or any auditor binary.

This report closes the planning phase only. Auditor tool wiring, advisory
snapshot acquisition, severity/report mappings, target-to-dependency-set
resolution, CLI registration, and Bazel integration are explicitly deferred
as O11/O12/O58-gated follow-ups (see Open Items). The deferral is
evidence-backed: the owning contracts carry a do-not-implement gate until
those qualifications land, and no CLI command is registered until its
behavior lands (the CLI matrix compares the final registry, not partial CLI).

## WP1: Security Audit Planning (O11 planning; qualification pending)

`dx_audit` owns the audit command surface before any tool integration:
bare `dx audit` plans both families security-first, explicit families run
alone, scopes pass through verbatim with `//...` default independent of the
working directory, and audit is pinned non-mutating (`plan_audit`,
7 tests). Risk-acceptance exception lifecycle is pure over injected records:
field/expiry/obsolescence validation, injected audit date, leap-aware
calendar check, expiry-boundary failure on the date itself, identity-match
obsolescence with version-range evaluation deferred to resolver slices
(`exception`, 6 tests, including shared `check_expiry` reused by WP3).
Secrets qualification plans the Gitleaks route without fetching bytes:
checksummed standalone `ArtifactPin` validation (https URL, 64 lowercase hex
sha256, nonzero size, Gitleaks-only tool gate), SARIF report argv wiring
(format/path plus mandatory `--redact`, optional `--config`/`--exit-code`),
conflated exit-1 triage classification, and the frozen config-discovery order
(`secrets`, 11 tests). No byte acquisition, no SARIF parsing, no
adapter/registry wiring; byte identity and report-file redaction stay
fixture-gated under O11.

## WP2: Resolver-Owned Update Planning (O12 planning; mappings pending)

`dx_update` owns the update command surface before any resolver integration:
bare `dx update` selects all supported sets cwd-independently with no CLI
filesystem scan, selectors pass through verbatim, the run applies
immediately without confirmation and mutates strictly within declared
requirements (`UpdateRequest`, 4 tests). Continuation policy aggregates over
injected per-set results: independent failures preserve attempted successes
and fail the run overall, transitive dependents of failures (cycle-safe)
report blocked without running, unexplained result gaps error instead of
reading as clean, unselected results are ignored, output is sorted and
deterministic (`outcome`, 8 tests; no scheduling, no parallelism, no
exit-code selection). Within-constraint and Git semantics are pinned over
injected requirement descriptors: declared requirements are never rewritten,
lock advance follows shape (exact pins hold; ranges and lockfile-only entries
move under upstream resolution), only declared branches advance locked
commits, and upstream owns prerelease/transitive scope (`semantics`,
4 tests). Selector syntax, identity mappings, non-registry handling, backend
boundaries, and per-set reporting stay O12-gated.

## WP3: License Family Planning (O58 planning; qualification pending)

License evaluation reuses security scope mechanics and the shared exception
lifecycle. SPDX expression boolean math over the allow/review/deny lattice:
`OR` takes the most permissive disjunct (`MIT OR AGPL-3.0-only` passes),
`AND` the strictest conjunct, `WITH` needs verbatim approval (allowed base
alone never approves), `UNKNOWN`/unlisted is denied in distributed and
inventoried in internal, `blocked` denies in both tiers until excepted, empty
`OR` denies, and `fails_in_tier` pins review-fails-distributed
(`license_expr`, 12 tests). Tier policy validates over injected records:
single-listing global tables, additive per-set adjustments with conflict
rejection, fail-closed distribution roots (unlisted default distributed,
unknown labels fail as `unknown_distribution_root`), promotion
re-qualifying under the strict table, and license exceptions sharing
`check_expiry` with package-plus-license identity obsolescence (in-range
upgrades retain acceptance; range narrowing deferred like WP1)
(`license_policy`, 10 tests). Notice-text inputs deny `missing-notice-text`
in distributed unless excepted and inventory it in internal; the SPDX 2.3
JSON shape is frozen to one document per invocation, package-URL IDs,
per-root `DESCRIBES`, known-graph `CONTAINS`, with aggregated NOTICE
assembly pinned out of scope (`license_notice`, 6 tests). No `--report`
format identifier or event mapping is invented (the output protocol keeps
those O58-pending). SPDX text parsing, per-ecosystem identity mappings,
table loading, shared-lock tier attribution, and proof evidence stay
O58-gated.

## Evidence

Exact commands on this host, committed tree:

- `bazel build //...`: success (703 targets).
- `bazel test //...`: 174/174 pass, including `//dx/audit:dx_audit_test`
  (52 passed) and `//dx/update:dx_update_test` (16 passed) plus each
  crate's `rustfmt_test`/`rust_clippy_test` (warnings as errors).
- `bazel run //dx:generate`: no diffs; `bazel run //dx:generate_check`:
  clean.

Coverage inventory: no new uncovered executable lines beyond the reconciled
gate; new crates are zero-dep pure-planning libraries fully covered by their
co-located unit tests. No advisory, SARIF, SPDX-document, TOML-loading, or
resolver evidence exists; none claimed. No remote, non-Linux, or
external-consumer evidence; none claimed. Security-only, license-only, and
default-both selection paths are exercised at the planning layer;
qualified ecosystem identities, tier attribution proofs, and NOTICE
aggregation evidence are deferred gaps, not claims.

## Changed Components

- `dx/audit/` (new crate `dx_audit`): `src/lib.rs` (family/scope
  planning), `src/exception.rs` (risk-exception lifecycle plus shared
  `check_expiry`), `src/secrets.rs` (Gitleaks qualification),
  `src/license_expr.rs` (SPDX lattice), `src/license_policy.rs` (tier
  policy/roots/license exceptions), `src/license_notice.rs`
  (notice-text/SPDX shape); `BUILD.bazel`, `Cargo.toml`.
- `dx/update/` (new crate `dx_update`): `src/lib.rs` (selection
  planning), `src/outcome.rs` (continuation aggregation),
  `src/semantics.rs` (within-constraint/Git pins); `BUILD.bazel`,
  `Cargo.toml`.
- Zero-dep by design: no `MODULE.bazel` manifest changes (`aliases()`
  tolerates unlisted packages); no root `gazelle:resolve` entries (no
  out-of-package consumers yet); no `dx/cli` registration (behavior has
  not landed); no `docs/testing/cli.md` matrix change (matrix compares
  the final registry).
- This report.

## Open Items

- O11: Gitleaks byte acquisition/pins, SARIF parsing and report-file
  redaction proofs, findings-versus-error fixtures, silent-`0` coverage
  cases, `secrets` registry amendment, dependency-vulnerability tools,
  advisory acquisition/snapshot/cache semantics, severity mappings,
  target-to-owner mappings, native configuration. No working audit
  support is claimed.
- O12: selector syntax, ecosystem identity mappings, non-registry
  handling, backend operation boundaries, aggregate exit codes, per-set
  operation/manifest/lockfile reporting.
- O58: SPDX parsing, per-ecosystem license identities, policy-table
  loading, shared-lock tier attribution, approval/report mappings, proof
  evidence, SPDX `--report` identifier/event mapping.
- O53: update-bot scope gate unresolved; no bot deliverables approved or
  built.
- Milestone exclusions respected: no auditor/registry/Bazel integration,
  no CLI surface change, no advisory network access, no osv/upload of
  lockfiles, no NOTICE aggregation artifact, no consumer CI (M27), no
  release qualification/publication (M28/M29).
