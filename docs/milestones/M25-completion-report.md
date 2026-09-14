# M25 Completion Report: Codegen, Environments, And Setup

Seed host: local Linux x86_64 glibc (same host class as the M00 seed report).
Local execution only, no remote. Non-Linux platforms were unavailable and are
recorded as gaps, not claimed. No capability-term transitions are claimed:
the codegen/env/setup/clean commands are first-party tested implementation,
not release-qualified product support (M28/M29 own release evidence).

## WP1: Codegen Shards, Collection, Projection (O33 partial)

First pair only: Protocol Buffer schema to Rust through `rust_prost_library`
(dogfooded by `//quality:result_proto_rs` and `//generation:result_proto_rs`).
Private names frozen: `DxCodegenPlanInfo` (per-target direct record),
`DxCodegenPlanCollectedInfo` (aspect-merged carrier, separate because Bazel
rejects an aspect re-providing its target's own provider), shard suffix
`.dxcodegen.pb` with the `DxCodegenShard` schema in `generation/codegen.proto`,
output group `dx_codegen_plans`. `dx_codegen_shard` emission rule, narrow
`dx_codegen_plan_aspect` collector, and the `prost_codegen_shard` adapter
(pinned to the upstream `rust_generated_srcs` output group) land here; Rust
(`dx_codegen`) merges shards by configured-target identity, validates
logical-path and producer conflicts fail-closed, hashes the normalized plan,
and plans the read-only symlink mirror without scanning `bazel-out`.
GraphQL and any Python/Node projection stay deferred, not dropped;
bare-schema/reverse-dependent queries and replacement contracts stay open.

## WP2: Environment Plans (O35 boundary held)

Normalized env plan frozen: `DxEnvPlanInfo` / `DxEnvShard` schema,
`dx_env_plans` output group, `.dxenv.pb` suffix (`env/plan.bzl`,
`env/env_shard/`), Rust adapter fixture, BEP shard collection with
merge/hash in `dx_env_plan`, artifact index plus projection planning.
Env sharing is allowed (ambiguous artifacts, no duplicate-claim rejection).
No public persistent-environment contribution API in v1 (O35 settled);
direnv stays inside the boundary with `PATH_add` for `.dx/bin` only.
Boundary evidence remains pending under O35.

## WP3: Atomic Combined Setup (O36 partial)

`dx_setup` plans one combined request (codegen/env never invoke each other),
resolves the setup pair purely over opaque digests with capability-aware
carry-forward (absent side carries the selected generation, first run uses
managed empty generations), and commits under the shared O36 lock:
`COMMIT_LOCK_TIMEOUT` (10 s) pinned equal to `dx_env::LOCK_TIMEOUT` by test.
Independent commits re-read current under the lock and compose with the
newest opposite side; install is idempotent byte-for-byte; the current
pointer swaps atomically. No new lock helper/file/deadline.

## WP4: Root Benchmark Freeze Plus Concurrency (O34 frozen, O36 partial)

`dx_roots` freezes the `//...` recursive-pattern baseline
(`FROZEN_STRATEGY`, `FROZEN_EVIDENCE` pinning numbers to `select_strategy`
by test): cold 8457 ms vs query-pattern-file 8983 ms over the same 685
targets (equivalent, file-indirection cost on cold), weighted 9201 vs 9683
(`WARM_WEIGHT=2` provisional; warm medians 372 vs 350, within noise);
monolithic-aggregate and package-shards excluded as non-equivalent
(coverage fails closed). Effective roots stay on the baseline; exact scopes
bypass roots; query-file uses `--target_pattern_file` with empty CLI
patterns. Incrementality gap (explicit, not silent): only cold/warm
measured; source/BUILD-edit, target churn, actions, materialized bytes,
projection time, and retained memory plus remote materialization are
unmeasured and need new evidence plus a freeze change to displace this.
Concurrency certified without new machinery: an 8-thread commit race
serializes with idempotent reuse and no staged link surviving; a planted
staged directory refuses preserving current. Remote materialization stays
Bazel-owned via output groups (no seed-host remote infra).

## WP5: Explicit `dx clean` (O60 partial)

`dx clean [--dry-run] [--bazel]` only (no scopes/reports/quality opts;
`--bazel` rejected on every other command). Prune planning is pure over
injected inventory against validated setup records; apply runs under the
shared O36 lock (`CLEAN_LOCK_TIMEOUT` pinned equal), re-reads live
selection, skips became-current entries, treats misses as idempotent, and
never touches the current pointer. Unmanaged/spoofed paths refused,
malformed current fails closed, `--dry-run` deletes nothing and holds no
lock, `--bazel` forwards exactly `bazel clean` plus recovery guidance, exit
code propagates, failures are operational (`clean_failed`) and fail closed.
Open under O60: process-scan in-use detection (v1 takes caller-provided
active sets) and reclaimable-bytes reporting.

## Evidence

Exact commands on this host, committed tree:

- `bazel build //...`: success (691 targets).
- `bazel test //...`: 168/168 pass, including
  `//dx/... //generation/... //env/...` (56/56).
- `bazel run //dx:generate_check`: clean.

Coverage inventory: no new uncovered executable lines beyond the reconciled
gate; `//tools/coverage:coverage_test` passes in the full run. Benchmark
evidence is the WP4 cold/warm table above; all other root dimensions are
unmeasured gaps, not claims. No remote, non-Linux, or external-consumer
evidence; none claimed.

## Changed Components

- `generation/codegen.bzl`, `generation/codegen.proto`, `generation/BUILD`,
  `generation/codegen_shard/`, `generation/result` fixtures (WP1).
- `env/plan.bzl`, `env/plan.proto`, `env/BUILD`, `env/env_shard/` (WP2).
- `dx/codegen/`, `dx/env_plan/`, `dx/setup/`, `dx/env/` lock owner (WP1–WP3).
- `dx/roots/` (strategy, coverage, composition, freeze), `dx/roots/roots.bzl`
  (WP4).
- `dx/clean/`, `dx/cli/` clean args/plan/exec (WP5).
- `docs/environments/codegen.md` (provider, roots, perf model, freeze),
  `docs/environments/managed-state.md` (setup/commit),
  `docs/cli/commands/check-fix-clean.md` plus `environment-codegen-setup.md`
  (contracts), `docs/open-decisions.md` (O33 partial; O34 frozen; O35 open;
  O36 partial; O60 partial).
- This report.

## Open Items

- O33: GraphQL/Python/Node pairs, bare-schema/reverse-dependent queries,
  replacement contracts deferred to later slices.
- O34: incrementality dimensions (edits, churn, actions, bytes, projection,
  memory) plus remote evidence unmeasured; freeze stands until displaced.
- O35: extension-boundary evidence pending.
- O36: Rust pin, NFS/append-only/crash-release platform evidence pending.
- O60: process-scan detection and bytes reporting pending.
- Milestone exclusions respected: no new languages/tools, no automatic
  cleanup policy, no shell-profile mutation, no security audit (M26), no
  consumer CI (M27), no release qualification/publication (M28/M29).
