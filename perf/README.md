# Performance benchmarks

Bazel-owned benchmark harness with comparison machinery (issue #215).
No `dx perf` command by design; benchmarks are Bazel targets plus docs.

- `bazel run //perf:bench_micro` — microbenchmarks, per-PR (fast).
- `bazel run //perf:bench_scenario_warm` — warm scenario benchmarks.
- `bazel run //perf:bench_cold` — cold `generate --check` after shutdown.
- `bazel run //perf:compare -- results.jsonl` — median vs
  `perf/baseline.json` with tolerance band; warn-only except the single
  gated `dx_startup` absolute budget.
- `bazel run //perf:regenerate` — rewrite `docs/performance.md` from
  `perf/baseline.json` (numbers are regenerated, never hand-written).

Baselines update via human-merged PRs only; see [Performance](../docs/performance.md).
