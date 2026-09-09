# ADR 0003: Initial Action Granularity

## Status

Provisional.

## Context

Ruff is naturally incremental over source-owning targets, while Ty may need a
larger import and dependency context. Repository-wide actions invalidate too
broadly; per-file actions may pay excessive startup and scheduling overhead. The
repository currently has no sources or infrastructure with which to measure these
tradeoffs.

## Decision

Begin validation with one pipeline action per applicable source-owning target and mutating
quality capability: lint, typecheck, or format. Each action receives that target's complete
set of nonempty selected adapter stages for that capability and converges them over action-local
writable copies. Each stage receives only the target's direct sources in the semantic file
classes supported by that adapter and allowed by workspace policy. File classes do not create
separate actions.
Capability pipelines are separate: lint order has no relationship to format or typecheck order.
Non-mutating audit tools may retain independent target/tool actions. Ty initially runs within
the typecheck pipeline for each compatible Python target while using transitive provider data
for resolution. Do not create repository-wide or per-file actions by default.

Within one target/capability pipeline, tools run in a stable ruleset-defined order that is
independent of user list order. The order is selected for each capability and selected stage
set from correctness, interaction, and performance fixtures rather than lexical tool IDs or
unmeasured preference. Each tool sees the virtual bytes produced by preceding stages. Complete rounds
repeat until every nonempty stage performs a no-change pass over its fixed source subset.
Repetition of a prior changed
state or continuing changes after ten complete rounds fails convergence. The ten-round limit
is fixed ruleset policy, not workspace or target configuration, and contributes to the action
key. Only the original and stable final state
leave the action; no intermediate state writes the workspace.

Before prototype implementation, record representative workloads
and comparison criteria. After the cache and performance tests in
[Testing Strategy](../testing/), the milestone reports an explicit
accept, revise, or reject recommendation against the measured alternatives. This
decision becomes Accepted only with recorded benchmark evidence for that result.

## Consequences

- A direct source edit should invalidate every owner/capability pipeline action declaring the
  source and every typecheck pipeline whose declared Ty closure contains it.
- Shared configuration edits intentionally fan out to all consuming actions.
- Cold runs may incur multiple tool startups and convergence rounds per target.
- Ty's per-target model may duplicate analysis and may need coarser roots.
- A pipeline has coarser cache invalidation than target/tool actions because every selected
  tool, per-stage source subset, supported-class manifest, runtime, config, ordering rule, and
  runner version contributes to its action key.
- Order selection rejects permutations with incorrect, divergent, or unjustifiably different
  terminal semantics. Among equivalent correct orders, it minimizes non-convergence, rounds,
  process starts, and measured wall time in that priority.
- Multiple owner/configuration pipelines that include one source must produce identical final
  bytes for that path; disagreement rejects that file rather than merging independently
  validated candidates.
- Multiple formatters are supported only when they reach a common no-change state; oscillation
  or an iteration limit rejects the affected pipeline result.
- Batching changes require evidence and must remain Bazel-owned.
- A mixed-class target still has one action per mutating capability; disjoint and overlapping
  adapter subsets coexist in that pipeline.

## Alternatives to Measure

- Package-level batches of several small source targets.
- Repository-wide actions for very small repositories.
- Per-file actions.
- Coarser application/service/root Ty actions.
- Explicit check targets rather than aspect-generated actions.
- Independent target/tool actions with same-snapshot edit merging.
