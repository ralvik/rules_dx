# M26 Completion Report: Audit And Update (Delivery)

Seed host: local Linux x86_64 glibc (same host class as the M00 seed report).
Local execution only, no remote. Auditor/updater tool execution,
networked advisory acquisition, and non-Linux runs were unavailable and are
recorded as gaps, not claimed.

Delivered: the O11/O12/O58 frozen mappings (tool route, config discovery,
selector syntax, identity/lock authorities, SPDX shape, tier policy), the
`dx_audit` (52 tests) and `dx_update` (16 tests) planning gates that pin
 selection/exception/secrets/continuation/semantics/license behavior before
 tools, the license-notice/SPDX-shape freeze, and the `dx audit`/`dx update`
 planning dispatch (`--dry-run` plans and exits `0`; live runs fail closed
 `audit_deferred`/`update_deferred`). Auditor binary wiring,
 advisory snapshot acquisition, SARIF/SPDX parsing, and resolver-backend
 execution remain gaps.

Capability transitions: audit/update mappings are Dogfooded at the gate
level (frozen mappings constrain the planning libraries, which are
unit-tested and lint-clean); no `Supported` claim and no working
audit/update support claimed (requires qualified tool execution).

## WP1: Security Audit (O11 mappings frozen; tool execution deferred)

O11 frozen (`docs/open-decisions.md`): Gitleaks v8.30.1 standalone
checksummed artifact route, `secrets` registry amendment, SARIF
`--report-format` with mandatory `--redact`, TOML config discovery order,
conflated exit-1 triage, Cargo/pnpm/Maven/NuGet advisory snapshots with
24h cache, O44 resolver-owned target-to-dependency-set mapping, severity
mappings deferred to fixture qualification. `dx_audit` gates delivered:
family/scope selection (`plan_audit`, 7 tests), risk-exception lifecycle
over injected records with shared leap-aware `check_expiry` (`exception`,
6 tests), secrets qualification without byte fetch (`secrets`, 11 tests).
No byte acquisition, SARIF parsing, or adapter wiring; none claimed.

## WP2: Resolver-Owned Update (O12 mappings frozen; backend execution deferred)

O12 frozen: selector `<set>[/<package>]`, lock authorities
(`Cargo.lock`/`pnpm-lock.yaml`/`maven_install.json`/`paket.lock`/`go.mod`+`go.sum`),
non-registry handling (`skipped-non-registry`), upstream-owned
prerelease/transitive scope, aggregate exit-code and per-set reporting
shapes. `dx_update` gates delivered: immediate-apply selection
(`UpdateRequest`, 4 tests), continuation aggregation preserving successes
and reporting blocked dependents (`outcome`, 8 tests), within-constraint
and Git semantics over injected descriptors (`semantics`, 4 tests). No
resolver-backend runs; none claimed.

## WP3: License Family (O58 mappings frozen; parsing/loading deferred)

O58 frozen: per-root tier attribution, SPDX expression lattice
(allow/review/deny + bounded-version range narrowing), one SPDX 2.3 JSON
document per invocation with package-URL IDs and per-root `DESCRIBES`,
NOTICE assembly out of scope. `dx_audit` license gates delivered:
expression math (`license_expr`, 12 tests), tier policy with fail-closed
distribution roots (`license_policy`, 10 tests), notice-text/SPDX shape
(`license_notice`, 6 tests). No SPDX parsing, table loading, or proof
artifacts; none claimed.

## Evidence

Exact commands on this host, committed tree:

- `bazel build //...`: success (727 targets).
- `bazel test //...`: 186/186 pass, including `//dx/audit:dx_audit_test`
  (52 passed) and `//dx/update:dx_update_test` (16 passed) plus
  `rustfmt`/`clippy` gates (warnings as errors).
- `bazel run //dx:generate_check`: clean.

Coverage inventory: no new uncovered executable lines beyond the reconciled
gate; both crates are zero-dep pure-planning libraries fully covered by
co-located unit tests. No advisory, SARIF-document, TOML-loading, or
resolver evidence; none claimed.

## Changed Components

- `dx/audit/` (crate `dx_audit`): family/scope planning, exception
  lifecycle + shared `check_expiry`, Gitleaks qualification, SPDX lattice,
  tier policy/roots, notice-text/SPDX shape; `BUILD.bazel`, `Cargo.toml`.
- `dx/update/` (crate `dx_update`): selection planning, continuation
  aggregation, within-constraint/Git pins; `BUILD.bazel`, `Cargo.toml`.
- `docs/open-decisions.md` (O11/O12/O58 frozen mappings constraining the
  above).
- `dx/cli/` (planning dispatch only): `audit`/`update` family/selection
  parsing, `--dry-run` plan specs, deferred live execution; `BUILD.bazel`
  deps on `//dx/audit:dx_audit` and `//dx/update:dx_update`.
- `dx/adopt/` (`ALL_COMMANDS`), `docs/cli/commands/audit-update-bazel.md`
  (dispatch status; tool execution still gap).
- No `docs/testing/cli.md` matrix change (matrix compares final behavior,
  not planning dispatch).
- This report.

## Open Items

- Tool execution (O11/O12/O58-gated follow-ups): auditor binary wiring,
  advisory acquisition, SARIF/SPDX parsing, target-to-owner mappings,
  native configuration, backend operation boundaries, policy-table
  loading, shared-lock tier attribution, proof evidence, `dx audit` /
  `dx update` live execution.
- O53: update-bot scope gate unresolved; no bot deliverables approved or
  built.
- Milestone exclusions respected: no auditor/registry/Bazel integration,
  no advisory network access, no osv/upload of lockfiles, no NOTICE
  aggregation artifact, no consumer CI (M27), no release
  qualification/publication (M28/M29).
