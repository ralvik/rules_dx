# ADR 0022: No Standing Benchmarking

## Status

Accepted.

## Context

Standing wall-clock benchmarking ran on shared free runners with noise
above signal for millisecond-scale micros, single-sample cold cases,
and a `rules_lint` comparison that stayed dx-side-only with no parity
claim. Pending measurement-gated choices in
[ADR 0003](./0003-action-granularity.md),
[Generated Code](../environments/codegen.md),
[Quality Testing](../quality/quality-testing.md), and
[Tool Acquisition](../tools/tool-acquisition.md) would have grown the
harness without paying for themselves. The removed surface was
`perf/` (`bench.sh`, `compare.py`, `regenerate.py`, `baseline.json`,
`rules_lint` comparison scripts and results, `BUILD.bazel`,
`README.md`), `.github/workflows/perf.yml`, `docs/performance.md`,
`docs/tools/rules_lint-comparison.md`, and the `tools/ci` perf halves.
See issue #434.

## Decision

There are no standing benchmarks, no baseline or comparison machinery,
and no report-not-gate track. The following choices are made by
reasoning and fiat, not measurement:

- [ADR 0003](./0003-action-granularity.md) action granularity stays on
  its pipeline-per-target/capability rule; promotion to Accepted no
  longer waits on recorded benchmark evidence.
- Repository-wide codegen and environment root selection stays on the
  frozen `//...` correctness baseline in
  [Generated Code](../environments/codegen.md) and
  [Developer Environments](../environments/environment.md); the
  query-pattern-file, aggregate, and shard candidates are rejected
  alternatives, not pending measurements.
- Quality stage order in
  [Quality Testing](../quality/quality-testing.md) is selected from
  correctness and convergence first, then rounds and process starts;
  wall-time ordering is not measured.
- Tool-acquisition overhead in
  [Tool Acquisition](../tools/tool-acquisition.md) and release
  archive-vs-graph choices in
  [Tool And Platform Test Matrix](../testing/tools.md) are decided by
  laziness reasoning (zero operational work for unused foundations),
  not by timing fixtures.
- Future performance regressions surface via user report, not CI.

The benchmark-gated promises in those documents are superseded by this
record where they conflict.

## Consequences

- `//perf:*` targets, `perf/baseline.json`, `docs/performance.md`, and
  the `rules_lint` comparison report no longer exist; external
  documents must not reference them.
- No `dx perf` command and no perf workflow, PR comment, or job
  summary exists.
- Deterministic non-timing checks remain: `generate --check`
  freshness, `aquery` shape and cache evidence, and corpus ownership.
- `//...` remains the repository-wide root baseline until a later ADR
  changes it by reasoning, not by a benchmark run.

## Rejected Alternatives

- Keep and harden (move wall-clock off PRs, add history via
  artifacts or Pages, add decision cases as `aquery`-first):
  rejected, shared-runner noise still dominates and maintenance is not
  justified.
- Keep smoke-only (`dx_startup` gate plus `generate --check`
  freshness): rejected as a standing track; freshness without timing
  stays as a deterministic check, not a benchmark.
- `hyperfine`, `criterion`, `divan`, or a hosted trend service:
  rejected, still shared-runner noise and violates cost or pin
  constraints for no decision value.
